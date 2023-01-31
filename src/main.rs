use zkbrainfuck::code;
use zkbrainfuck::interpreter::Interpreter;
use zkbrainfuck::matrix::Matrix;

use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::{Layouter, SimpleFloorPlanner, Value};
use halo2_proofs::dev::MockProver;
use halo2_proofs::halo2curves::bn256::Fq;
use halo2_proofs::plonk::*;
use halo2_proofs::poly::Rotation;
use std::marker::PhantomData;

const OPCODES: [u8; 8] = [
    code::SHL,
    code::SHR,
    code::ADD,
    code::SUB,
    code::GETCHAR,
    code::PUTCHAR,
    code::LB,
    code::RB,
];

const SHL: usize = 0;
const SHR: usize = 1;
const ADD: usize = 2;
const SUB: usize = 3;
const GETCHAR: usize = 4;
const PUTCHAR: usize = 5;
const LB: usize = 6;
const RB: usize = 7;

#[derive(Clone, Debug, Copy)]
struct Config {
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

impl Config {
    fn configure(cs: &mut ConstraintSystem<Fq>) -> Self {
        let one = Expression::Constant(Fq::one());
        let two = Expression::Constant(Fq::from(2));

        let clk = cs.advice_column();
        let s_p = cs.selector();
        cs.create_gate("P0: clk increase one per step", |vc| {
            let s = vc.query_selector(s_p);
            let cur_clk = vc.query_advice(clk, Rotation::cur());
            let next_clk = vc.query_advice(clk, Rotation::next());
            vec![s * (next_clk - cur_clk - one.clone())]
        });

        let mv = cs.advice_column();
        let mvi = cs.advice_column();
        let s_c = cs.selector();
        cs.create_gate("C0: mv is 0 or mvi is the inverse of mv", |vc| {
            let s = vc.query_selector(s_c);
            let mv = vc.query_advice(mv, Rotation::cur());
            let mvi = vc.query_advice(mvi, Rotation::cur());
            vec![s * mv.clone() * (mv * mvi - one.clone())]
        });

        cs.create_gate("C1: mvi is 0 or mvi is the inverse of mv", |vc| {
            let s = vc.query_selector(s_c);
            let mv = vc.query_advice(mv, Rotation::cur());
            let mvi = vc.query_advice(mvi, Rotation::cur());
            vec![s * mvi.clone() * (mv * mvi - one.clone())]
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

        let ci = cs.advice_column();
        let ip = cs.advice_column();
        let mp = cs.advice_column();
        let ni = cs.advice_column();
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
            let s = vc.query_selector(s_p);
            // ADD SUB SHR SHL GETCHAR PUTCHAR share the same p1 condition
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
            vec![s * (expr1 + expr_lb + expr_rb)]
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

    pub fn load_table(
        &self,
        mut layouter: impl Layouter<Fq>,
        matrix: &Matrix,
    ) -> Result<(), Error> {
        // Load the table
        layouter.assign_region(
            || "Load Matrix",
            |mut region| {
                let processor_matrix = &matrix.processor_matrix;
                for (idx, reg) in processor_matrix.iter().enumerate() {
                    if idx < processor_matrix.len() - 1 {
                        // P condition is enabled until the second last row
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

#[derive(Default)]
struct MyCircuit<F: Field> {
    _marker: PhantomData<F>,
    matrix: Matrix,
}

// It would be nice if we can use generic type here
// impl <F:Field> Circuit<F> for MyCircuit<F> {...}
impl Circuit<Fq> for MyCircuit<Fq> {
    type Config = Config;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fq>) -> Self::Config {
        Config::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fq>,
    ) -> Result<(), Error> {
        config.load_table(layouter, &self.matrix);
        Ok(())
    }
}

fn main() {
    let program = code::compile(include_bytes!("../zkbrainfuck/res/neptune_tutorial.bf").to_vec());
    let mut vm = Interpreter::new();
    vm.set_code(program);
    vm.set_input(code::easygen("a"));
    vm.run();

    let circuit = MyCircuit {
        _marker: PhantomData,
        matrix: vm.matrix,
    };
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}
