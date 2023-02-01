use halo2_bf::main_config::MyCircuit;
use halo2_proofs::dev::MockProver;
use ckb_bf_zkvm::code;
use ckb_bf_zkvm::interpreter::Interpreter;

fn main() {
    let program = code::compile(include_bytes!("../ckb-bf-zkvm/res/neptune_tutorial.bf").to_vec());
    let mut vm = Interpreter::new();
    vm.set_code(program);
    vm.set_input(code::easygen("a"));
    vm.run();

    let circuit = MyCircuit::new(vm.matrix);
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}
