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

#[derive(Clone, Debug, Copy)]
struct Config {
    clk: Column<Advice>,
    mv: Column<Advice>,
    mvi: Column<Advice>,
    s_p0: Selector, // P0: cycle increases by one per step
    s_c0: Selector, // C0: mv is 0 or mvi is the inverse of mv
    s_c1: Selector, // C1: mvi is 0 or mvi is the inverse of mv
}

impl Config {
    fn configure(cs: &mut ConstraintSystem<Fq>) -> Self {
        let one = Expression::Constant(Fq::one());

        let clk = cs.advice_column();
        let s_p0 = cs.selector();
        cs.create_gate("P0: clk increase one per step", |vc| {
            let s = vc.query_selector(s_p0);
            let cur_clk = vc.query_advice(clk, Rotation::cur());
            let next_clk = vc.query_advice(clk, Rotation::next());
            vec![s * (next_clk - cur_clk - one.clone())]
        });

        let mv = cs.advice_column();
        let mvi = cs.advice_column();
        let s_c0 = cs.selector();
        cs.create_gate("C0: mv is 0 or mvi is the inverse of mv", |vc| {
            let s = vc.query_selector(s_c0);
            let mv = vc.query_advice(mv, Rotation::cur());
            let mvi = vc.query_advice(mvi, Rotation::cur());
            vec![s * mv.clone() * (mv * mvi - one.clone())]
        });

        let s_c1 = cs.selector();
        cs.create_gate("C1: mvi is 0 or mvi is the inverse of mv", |vc| {
            let s = vc.query_selector(s_c1);
            let mv = vc.query_advice(mv, Rotation::cur());
            let mvi = vc.query_advice(mvi, Rotation::cur());
            vec![s * mvi.clone() * (mv * mvi - one.clone())]
        });

        Self {
            clk,
            mv,
            mvi,
            s_p0,
            s_c0,
            s_c1,
        }
    }

    pub fn load_table(
        &self,
        mut layouter: impl Layouter<Fq>,
        matrix: &Matrix,
    ) -> Result<(), Error> {
        layouter.assign_region(
            // Constraint P_0: cycle increases by one per step
            || "P_0",
            |mut region| {
                let processor_matrix = &matrix.processor_matrix;
                for (idx, reg) in processor_matrix.iter().enumerate() {
                    // enable until the second last clk
                    if idx < processor_matrix.len() - 1 {
                        self.s_p0.enable(&mut region, idx)?;
                    }
                    region.assign_advice(|| "clk", self.clk, idx, || Value::known(reg.cycle))?;
                }
                Ok(())
            },
        )?;

        layouter.assign_region(
            // Constraint C_0 & C1
            || "C_0 & C_1",
            |mut region| {
                let processor_matrix = &matrix.processor_matrix;
                for (idx, reg) in processor_matrix.iter().enumerate() {
                    self.s_c0.enable(&mut region, idx)?;
                    self.s_c1.enable(&mut region, idx)?;
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
        let clks: Vec<Fq> = self
            .matrix
            .processor_matrix
            .iter()
            .map(|x| x.cycle)
            .collect();
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
