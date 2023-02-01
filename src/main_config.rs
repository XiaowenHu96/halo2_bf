use crate::instruction_table::InstructionTableConfig;
use crate::memory_table::MemoryTableConfig;
use crate::processor_table::ProcessorTableConfig;
use crate::utils::*;
use ckb_bf_zkvm::matrix::Matrix;

use halo2_proofs::arithmetic::Field;
use halo2_proofs::circuit::{Layouter, SimpleFloorPlanner};
use halo2_proofs::halo2curves::bn256::Fq;
use halo2_proofs::plonk::*;
use std::marker::PhantomData;

#[derive(Clone, Debug, Copy)]
pub struct MainConfig {
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
pub struct MyCircuit<F: Field> {
    _marker: PhantomData<F>,
    matrix: Matrix,
}

impl MyCircuit<Fq> {
    pub fn new(matrix: Matrix) -> Self {
        Self {
            _marker: PhantomData,
            matrix,
        }
    }
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
