use halo2_bf::main_config::MyCircuit;
use halo2_proofs::dev::MockProver;
use zkbrainfuck::code;
use zkbrainfuck::interpreter::Interpreter;

#[test]
fn test_prove_hello_world() {
    let program = code::compile(include_bytes!("../zkbrainfuck/res/hello_world.bf").to_vec());
    let mut vm = Interpreter::new();
    vm.set_code(program);
    vm.run();

    let circuit = MyCircuit::new(vm.matrix);
    let prover = MockProver::run(11, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}

#[test]
fn test_prove_neptune() {
    let program = code::compile(include_bytes!("../zkbrainfuck/res/neptune_tutorial.bf").to_vec());
    let mut vm = Interpreter::new();
    vm.set_code(program);
    vm.set_input(code::easygen("a"));
    vm.run();

    let circuit = MyCircuit::new(vm.matrix);
    let prover = MockProver::run(8, &circuit, vec![]).unwrap();
    prover.assert_satisfied();
}

// Interpreter side is taking too long
// #[test]
// fn test_prove_pearson() {
//     let program = code::compile(include_bytes!("../pearson.b").to_vec());
//     let mut vm = Interpreter::new();
//     vm.set_code(program);
//     vm.set_input(code::easygen("Hello World!"));
//     vm.run();

//     let circuit = MyCircuit::new(vm.matrix);
//     let prover = MockProver::run(12, &circuit, vec![]).unwrap();
//     prover.assert_satisfied();
// }
