use rholang_vm::{api::Instruction, api::Opcode, api::Process, api::Value, VM};

// kind codes aligned with other tests
const STORE_CONC: u16 = 3;

#[test]
fn test_contract_operation_examples_minimal() {
    let mut vm = VM::new();

    let prog = vec![
        // Create top-level contract channel
        Instruction::unary(Opcode::NAME_CREATE, STORE_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),

        // Store a continuation-like payload and drop ID (we'll use an Int as payload)
        Instruction::unary(Opcode::PUSH_INT, 7),
        Instruction::nullary(Opcode::CONT_STORE),
        Instruction::nullary(Opcode::POP),

        // Send data to the contract channel: ch!([42])
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 42),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, STORE_CONC),

        // Persistent semantics via peek
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PEEK, STORE_CONC),
        Instruction::nullary(Opcode::HALT),
    ];

    let mut p = Process::new(prog, "contract:minimal");
    let out = vm.execute(&mut p).expect("exec ok");
    assert_eq!(out, Value::List(vec![Value::Int(42)]));
}

#[test]
fn test_contract_persistent_peek_then_consume() {
    let mut vm = VM::new();

    // Setup and persistent peek twice
    let prog1 = vec![
        Instruction::unary(Opcode::NAME_CREATE, STORE_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        // send [1]
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, STORE_CONC),
        // peek twice
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PEEK, STORE_CONC),
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PEEK, STORE_CONC),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p1 = Process::new(prog1, "contract:peek");
    let out1 = vm.execute(&mut p1).expect("exec ok");
    assert_eq!(out1, Value::List(vec![Value::Int(1)]));

    // Now consume then peek -> Nil
    let prog2 = vec![
        Instruction::unary(Opcode::NAME_CREATE, STORE_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, STORE_CONC),
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::ASK, STORE_CONC),
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PEEK, STORE_CONC),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "contract:consume");
    let out2 = vm.execute(&mut p2).expect("exec ok");
    assert_eq!(out2, Value::Nil);
}

#[test]
fn test_contract_multiple_sends_and_persistent_peek() {
    let mut vm = VM::new();
    let prog = vec![
        Instruction::unary(Opcode::NAME_CREATE, STORE_CONC),
        Instruction::nullary(Opcode::ALLOC_LOCAL),
        Instruction::unary(Opcode::STORE_LOCAL, 0),
        // send [1]
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, STORE_CONC),
        // send [2]
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::TELL, STORE_CONC),
        // peek sees [1]
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PEEK, STORE_CONC),
        // consume removes [1]
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::ASK, STORE_CONC),
        // peek now sees [2]
        Instruction::unary(Opcode::LOAD_LOCAL, 0),
        Instruction::unary(Opcode::PEEK, STORE_CONC),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p = Process::new(prog, "contract:multi");
    let out = vm.execute(&mut p).expect("exec ok");
    assert_eq!(out, Value::List(vec![Value::Int(2)]));
}

#[test]
fn test_continuation_store_and_resume() {
    let mut vm = VM::new();
    let prog = vec![
        Instruction::unary(Opcode::PUSH_INT, 99),
        Instruction::nullary(Opcode::CONT_STORE), // -> id
        Instruction::nullary(Opcode::CONT_RESUME), // pops id, pushes stored value
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p = Process::new(prog, "contract:cont");
    let out = vm.execute(&mut p).expect("exec ok");
    assert_eq!(out, Value::Int(99));
}
