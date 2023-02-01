use ckb_bf_zkvm::code;
use ckb_bf_zkvm::interpreter::Interpreter;
use halo2_bf::main_config::MyCircuit;
use halo2_bf::utils::DOMAIN;
use halo2_proofs::dev::MockProver;
use halo2_proofs::halo2curves::bn256::Fq;

fn main() {
    let program = code::compile(include_bytes!("../ckb-bf-zkvm/res/neptune_tutorial.bf").to_vec());
    let mut vm = Interpreter::new();
    vm.set_code(program);
    vm.set_input(code::easygen("a"));
    vm.run();

    let circuit = MyCircuit::<Fq, { DOMAIN }>::new(vm.matrix);
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}
