use ckb_bf_zkvm::code;
use ckb_bf_zkvm::interpreter::Interpreter;
use halo2_bf::main_config::MyCircuit;
use halo2_bf::utils::DOMAIN;
use halo2_proofs::dev::MockProver;
use halo2_proofs::halo2curves::bn256::Fq;

#[test]
fn test_prove_hello_world() {
    let program = code::compile(include_bytes!("../ckb-bf-zkvm/res/hello_world.bf").to_vec());
    let mut vm = Interpreter::new();
    vm.set_code(program);
    vm.run();

    let circuit = MyCircuit::<Fq, {DOMAIN}>::new(vm.matrix);
    let prover = MockProver::run(11, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}

#[test]
fn test_prove_neptune() {
    let program = code::compile(include_bytes!("../ckb-bf-zkvm/res/neptune_tutorial.bf").to_vec());
    let mut vm = Interpreter::new();
    vm.set_code(program);
    vm.set_input(code::easygen("a"));
    vm.run();

    let circuit = MyCircuit::<Fq, {DOMAIN}>::new(vm.matrix);
    let prover = MockProver::run(10, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}

#[test]
fn test_prove_wrapping() {
    let program = code::compile(include_bytes!("../wrapping_op.b").to_vec());
    let mut vm = Interpreter::new();
    vm.set_code(program);
    vm.run();

    let circuit = MyCircuit::<Fq, {DOMAIN}>::new(vm.matrix);
    let prover = MockProver::run(10, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}

// This takes a long time
#[test]
fn test_prove_pearson() {
    let program = code::compile(include_bytes!("../pearson.b").to_vec());
    let mut vm = Interpreter::new();
    vm.set_code(program);
    vm.set_input(code::easygen("a"));
    vm.run();

    let circuit = MyCircuit::<Fq, {DOMAIN}>::new(vm.matrix);
    let prover = MockProver::run(12, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}
