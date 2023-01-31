use zkbrainfuck::code;
use zkbrainfuck::interpreter::Interpreter;
use zkbrainfuck::interpreter::Register;

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
    s_clk: Selector,
}

impl Config {
    fn configure<F: Field>(cs: &mut ConstraintSystem<F>) -> Self {
        let clk = cs.advice_column();
        let s_clk = cs.selector();

        cs.create_gate("clk consistency", |vc| {
            let s = vc.query_selector(s_clk);
            let cur_clk = vc.query_advice(clk, Rotation::cur());
            let next_clk = vc.query_advice(clk, Rotation::next());
            let one = Expression::Constant(F::one());
            vec![s * (next_clk - cur_clk - one)]
        });
        Self { clk, s_clk }
    }

    pub fn load_clk<F: Field>(
        &self,
        mut layouter: impl Layouter<F>,
        clks: Vec<F>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "processor_matrix",
            |mut region| {
                for (idx, clk) in clks.iter().enumerate() {
                    // enable until the second last clk
                    if idx < clks.len() - 1 {
                        self.s_clk.enable(&mut region, idx)?;
                    }
                    region.assign_advice(
                        || "clk",
                        self.clk,
                        idx,
                        || Value::known(F::from(*clk)),
                    )?;
                }
                Ok(())
            },
        )
    }
}

struct MyCircuit<F: Field> {
    _marker: PhantomData<F>,
    vm: Interpreter,
}

// interpreter side should implement Field as abstract type
// so I can do:
// impl <F: Field> Circuit<F> for MyCircuit<F> {...}
impl Circuit<Fq> for MyCircuit<Fq> {
    type Config = Config;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        // TODO: default for interpreter
        Self {
            _marker: PhantomData,
            vm: Interpreter::new(),
        }
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
            .vm
            .matrix
            .processor_matrix
            .iter()
            .map(|x| x.cycle)
            .collect();
        config.load_clk(layouter, clks);
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
        vm,
    };
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}
