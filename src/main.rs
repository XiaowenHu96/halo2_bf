use halo2_bf::instruction_table::InstructionTableConfig;
use halo2_bf::memory_table::MemoryTableConfig;
use halo2_bf::processor_table::ProcessorTableConfig;
use halo2_bf::utils::*;
use zkbrainfuck::code;
use zkbrainfuck::interpreter::Interpreter;
use zkbrainfuck::matrix::Matrix;

use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::{Layouter, SimpleFloorPlanner};
use halo2_proofs::dev::MockProver;
use halo2_proofs::halo2curves::bn256::Fq;
use halo2_proofs::plonk::*;
use std::marker::PhantomData;

#[derive(Clone, Debug, Copy)]
struct MainConfig {
    p_config: ProcessorTableConfig,
    m_config: MemoryTableConfig,
    i_config: InstructionTableConfig,
}

impl Config for MainConfig {
    fn configure(cs: &mut ConstraintSystem<Fq>) -> Self {
        Self {
            p_config: ProcessorTableConfig::configure(cs),
            m_config: MemoryTableConfig::configure(cs),
            i_config: InstructionTableConfig::configure(cs),
        }
    }

    fn load_table(&self, layouter: &mut impl Layouter<Fq>, matrix: &Matrix) -> Result<(), Error> {
        self.p_config.load_table(layouter, matrix)?;
        self.m_config.load_table(layouter, matrix)?;
        self.i_config.load_table(layouter, matrix)
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
    type Config = MainConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fq>) -> Self::Config {
        MainConfig::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fq>,
    ) -> Result<(), Error> {
        config.load_table(&mut layouter, &self.matrix)?;
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
