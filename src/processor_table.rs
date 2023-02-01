use crate::utils::*;
use zkbrainfuck::matrix::Matrix;

use halo2_proofs::circuit::{Layouter, Value};
use halo2_proofs::halo2curves::bn256::Fq;
use halo2_proofs::plonk::*;
use halo2_proofs::poly::Rotation;

#[derive(Clone, Debug, Copy)]
pub struct ProcessorTableConfig {
    clk: Column<Advice>,
    ip: Column<Advice>,
    ci: Column<Advice>,
    ni: Column<Advice>,
    mp: Column<Advice>,
    mv: Column<Advice>,
    mvi: Column<Advice>,
    s_p: Selector, // Selector for condition P category (Processor Table)
    s_c: Selector, // Selector for condition C category (Consistency Constraints)
}

impl Config for ProcessorTableConfig {
    fn configure(cs: &mut ConstraintSystem<Fq>) -> Self {
        let zero = Expression::Constant(Fq::zero());
        let one = Expression::Constant(Fq::one());
        let two = Expression::Constant(Fq::from(2));

        let clk = cs.advice_column();
        let ci = cs.advice_column();
        let ip = cs.advice_column();
        let ni = cs.advice_column();
        let mp = cs.advice_column();
        let mv = cs.advice_column();
        let mvi = cs.advice_column();
        let s_c = cs.selector();
        let s_p = cs.selector();

        cs.create_gate("P0: clk increase one per step", |vc| {
            let s_p = vc.query_selector(s_p);
            let cur_clk = vc.query_advice(clk, Rotation::cur());
            let next_clk = vc.query_advice(clk, Rotation::next());
            vec![s_p * (next_clk - cur_clk - one.clone())]
        });

        cs.create_gate("C0: mv is 0 or mvi is the inverse of mv", |vc| {
            let s_c = vc.query_selector(s_c);
            let mv = vc.query_advice(mv, Rotation::cur());
            let mvi = vc.query_advice(mvi, Rotation::cur());
            vec![s_c * mv.clone() * (mv * mvi - one.clone())]
        });

        cs.create_gate("C1: mvi is 0 or mvi is the inverse of mv", |vc| {
            let s_c = vc.query_selector(s_c);
            let mv = vc.query_advice(mv, Rotation::cur());
            let mvi = vc.query_advice(mvi, Rotation::cur());
            vec![s_c * mvi.clone() * (mv * mvi - one.clone())]
        });

        // A deselector for op evalutes to zero iff ci != op
        let create_deselector = |ci: Expression<Fq>, op| {
            OPCODES.iter().fold(ci.clone(), |expr, v| {
                if *v == op {
                    expr
                } else {
                    expr * (ci.clone() - Expression::Constant(Fq::from(*v as u64)))
                }
            })
        };

        cs.create_gate("P_1: instruction mutates state(1) correctly ", |vc| {
            let ci = vc.query_advice(ci, Rotation::cur());
            let deselectors = OPCODES
                .iter()
                .map(|op| create_deselector(ci.clone(), *op))
                .collect::<Vec<_>>();
            let cur_ip = vc.query_advice(ip, Rotation::cur());
            let next_ip = vc.query_advice(ip, Rotation::next());
            let cur_mv = vc.query_advice(mv, Rotation::cur());
            let cur_mvi = vc.query_advice(mvi, Rotation::cur());
            let cur_ni = vc.query_advice(ni, Rotation::cur());
            let s_p = vc.query_selector(s_p);
            // ADD SUB SHR SHL GETCHAR PUTCHAR share the same p1 condition:
            // ip increases by 1
            let expr1 = (deselectors[ADD].clone()
                + deselectors[SUB].clone()
                + deselectors[SHR].clone()
                + deselectors[SHL].clone()
                + deselectors[GETCHAR].clone()
                + deselectors[PUTCHAR].clone())
                * (next_ip.clone() - cur_ip.clone() - one.clone());
            // LB: if mv != 0 ⇒ ip increases by 2 and if mv == 0 ⇒ ip is set to ni
            let expr_lb = deselectors[LB].clone()
                * (cur_mv.clone() * (next_ip.clone() - cur_ip.clone() - two.clone())
                    + (cur_mv.clone() * cur_mvi.clone() - one.clone())
                        * (next_ip.clone() - cur_ni.clone()));
            // RB: if mv == 0 ⇒ ip increases by 2 and if mv != 0 ⇒ ip is set to ni
            let expr_rb = deselectors[RB].clone()
                * ((cur_mv.clone() * cur_mvi.clone() - one.clone())
                    * (next_ip.clone() - cur_ip.clone() - two.clone())
                    + (cur_mv.clone() * (next_ip.clone() - cur_ni.clone())));
            vec![s_p * (expr1 + expr_lb + expr_rb)]
        });

        cs.create_gate("P_2: instruction mutates state(2) correctly", |vc| {
            let ci = vc.query_advice(ci, Rotation::cur());
            let deselectors = OPCODES
                .iter()
                .map(|op| create_deselector(ci.clone(), *op))
                .collect::<Vec<_>>();
            let s_p = vc.query_selector(s_p);
            let cur_mp = vc.query_advice(mp, Rotation::cur());
            let next_mp = vc.query_advice(mp, Rotation::next());
            // ADD, SUB, LB, RB, GETCHAR, PUTCHAR share the same p2 condition:
            // memory pointer stay at the same
            let expr1 = (deselectors[ADD].clone()
                + deselectors[SUB].clone()
                + deselectors[LB].clone()
                + deselectors[RB].clone()
                + deselectors[GETCHAR].clone()
                + deselectors[PUTCHAR].clone())
                * (next_mp.clone() - cur_mp.clone());
            // SHL: mp decreases by one
            let expr_shl =
                deselectors[SHL].clone() * (next_mp.clone() - cur_mp.clone() + one.clone());
            // SHR: mp increases by one
            let expr_shr =
                deselectors[SHR].clone() * (next_mp.clone() - cur_mp.clone() - one.clone());
            vec![s_p * (expr1 + expr_shl + expr_shr)]
        });

        cs.create_gate("P_3: instruction mutates state(3) correctly", |vc| {
            let ci = vc.query_advice(ci, Rotation::cur());
            let deselectors = OPCODES
                .iter()
                .map(|op| create_deselector(ci.clone(), *op))
                .collect::<Vec<_>>();
            let s_p = vc.query_selector(s_p);
            let cur_mv = vc.query_advice(mv, Rotation::cur());
            let next_mv = vc.query_advice(mv, Rotation::next());
            // LB, RB, PUTCHAR share the same p3 condition:
            // memory value stay at the same
            let expr1 =
                (deselectors[LB].clone() + deselectors[RB].clone() + deselectors[PUTCHAR].clone())
                    * (next_mv.clone() - cur_mv.clone());
            // ADD: mv increases by 1
            let expr_add =
                deselectors[ADD].clone() * (next_mv.clone() - cur_mv.clone() - one.clone());
            // sub: mv decreases by 1
            let expr_sub =
                deselectors[SUB].clone() * (next_mv.clone() - cur_mv.clone() + one.clone());
            // SHL, SHR, GETCHAR: always true (check elsewhere)
            let expr2 = (deselectors[SHL].clone()
                + deselectors[SHR].clone()
                + deselectors[GETCHAR].clone())
                * (zero.clone());
            vec![s_p * (expr1 + expr2 + expr_add + expr_sub)]
        });

        Self {
            clk,
            ip,
            ci,
            ni,
            mp,
            mv,
            mvi,
            s_p,
            s_c,
        }
    }

    fn load_table(&self, layouter: & mut impl Layouter<Fq>, matrix: &Matrix) -> Result<(), Error> {
        layouter.assign_region(
            || "Load Processor Table",
            |mut region| {
                let processor_matrix = &matrix.processor_matrix;
                for (idx, reg) in processor_matrix.iter().enumerate() {
                    if idx < processor_matrix.len() - 1 {
                        // P condition is enabled except last row
                        self.s_p.enable(&mut region, idx)?;
                    }
                    // Enable C condition check
                    self.s_c.enable(&mut region, idx)?;

                    region.assign_advice(|| "clk", self.clk, idx, || Value::known(reg.cycle))?;
                    region.assign_advice(
                        || "ip",
                        self.ip,
                        idx,
                        || Value::known(reg.instruction_pointer),
                    )?;
                    region.assign_advice(
                        || "ci",
                        self.ci,
                        idx,
                        || Value::known(reg.current_instruction),
                    )?;
                    region.assign_advice(
                        || "ni",
                        self.ni,
                        idx,
                        || Value::known(reg.next_instruction),
                    )?;
                    region.assign_advice(
                        || "mp",
                        self.mp,
                        idx,
                        || Value::known(reg.memory_pointer),
                    )?;
                    region.assign_advice(
                        || "mv",
                        self.mv,
                        idx,
                        || Value::known(reg.memory_value),
                    )?;
                    region.assign_advice(
                        || "mvi",
                        self.mvi,
                        idx,
                        || Value::known(reg.memory_value_inverse),
                    )?;
                }
                Ok(())
            },
        )
    }
}
