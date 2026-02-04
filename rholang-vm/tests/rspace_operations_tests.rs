use rholang_process::Process;
use rholang_vm::api::{Instruction, Opcode, Value};

// Helper constants for kind codes (encoded in op16 immediate)
const MEM_SEQ: u16 = 0;
const MEM_CONC: u16 = 1;
const STORE_CONC: u16 = 3;

#[test]
fn test_name_creation_and_tell() {
    // Top-level names (use STORE_CONC kind code)
    let prog1 = vec![
        // Create x
        Instruction::unary(Opcode::NAME_CREATE, STORE_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        // Create y
        Instruction::unary(Opcode::NAME_CREATE, STORE_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 1),
        // x!([1])
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, STORE_CONC),
        // y!([2])
        Instruction::unary(Opcode::LOAD_LOCAL, 1),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, STORE_CONC),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p1 = Process::new(prog1, "rspace1");
    let out1 = p1.execute().expect("exec ok");
    assert_eq!(out1, Value::Bool(true));

    // Local concurrent name used twice
    let prog2 = vec![
        Instruction::unary(Opcode::NAME_CREATE, MEM_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        // x!([10])
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 10),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, MEM_CONC),
        // x!([20])
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 20),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, MEM_CONC),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "rspace2");
    let out2 = p2.execute().expect("exec ok");
    assert_eq!(out2, Value::Bool(true));

    // Sequential local name single use (we just reuse MEM_SEQ kind code)
    let prog3 = vec![
        Instruction::unary(Opcode::NAME_CREATE, MEM_SEQ),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, MEM_SEQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p3 = Process::new(prog3, "rspace3");
    let out3 = p3.execute().expect("exec ok");
    assert_eq!(out3, Value::Bool(true));
}

#[test]
fn test_send_and_receive() {
    // Top-level: tell then ask should yield the list
    let prog1 = vec![
        Instruction::unary(Opcode::NAME_CREATE, STORE_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, STORE_CONC),
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::ASK, STORE_CONC),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p1 = Process::new(prog1, "send_recv1");
    let out1 = p1.execute().expect("exec ok");
    assert_eq!(out1, Value::List(vec![Value::Int(5)]));

    // Local: tell then ask yields the list
    let prog2 = vec![
        Instruction::unary(Opcode::NAME_CREATE, MEM_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 10),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, MEM_CONC),
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::ASK, MEM_CONC),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "send_recv2");
    let out2 = p2.execute().expect("exec ok");
    assert_eq!(out2, Value::List(vec![Value::Int(10)]));
}

#[test]
fn test_let_binding_and_persistent_peek() {
    // let x = 5; ch!([x]);
    let prog = vec![
        Instruction::unary(Opcode::NAME_CREATE, MEM_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        Instruction::nullary(Opcode::ALLOC_LOCAL), // local 1
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::unary(Opcode::STORE_LOCAL, 1),
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::LOAD_LOCAL, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, MEM_CONC),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p = Process::new(prog, "let_bind");
    let out = p.execute().expect("exec ok");
    assert_eq!(out, Value::Bool(true));

    // Use the same VM (with shared RSpace state) for the next process
    let vm = p.vm.clone();

    // Multiple sends then peek should show the head element without removing it
    let prog2 = vec![
        Instruction::unary(Opcode::NAME_CREATE, MEM_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        // tell [1]
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, MEM_CONC),
        // tell [2]
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, MEM_CONC),
        // peek sees [1]
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PEEK, MEM_CONC),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::with_vm(prog2, "peek", vm);
    let out2 = p2.execute().expect("exec ok");
    assert_eq!(out2, Value::List(vec![Value::Int(1)]));
}
