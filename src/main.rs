use ckb_bf_zkvm::code;
use ckb_bf_zkvm::interpreter::Interpreter;
use halo2_bf::main_config::MyCircuit;
use halo2_bf::utils::DOMAIN;
use halo2_proofs::dev::MockProver;
use halo2_proofs::halo2curves::bn256::Fq;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() >= 3);
    let mut f = std::fs::File::open(&args[1])?;
    let k = args[2].parse::<u32>()?;
    let mut c: Vec<u8> = Vec::new();
    f.read_to_end(&mut c)?;
    let mut i = Interpreter::new();
    i.set_code(code::compile(c));
    i.run();

    let circuit = MyCircuit::<Fq, { DOMAIN }>::new(i.matrix);
    let prover = MockProver::run(k, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
    Ok(())
}
