use rholang_bytecode::{Constant, Instruction, Literal, Name};
use rstest::rstest;

#[test]
fn test_instruction_display() {
    // Stack Operations
    assert_eq!(
        format!("{}", Instruction::Push(Constant::Literal(Literal::Int(42)))),
        "PUSH 42"
    );
    assert_eq!(format!("{}", Instruction::Pop), "POP");
    assert_eq!(format!("{}", Instruction::Dup), "DUP");
    assert_eq!(format!("{}", Instruction::Swap), "SWAP");
    assert_eq!(format!("{}", Instruction::Rot), "ROT");

    // Arithmetic Instructions
    assert_eq!(format!("{}", Instruction::Add), "ADD");
    assert_eq!(format!("{}", Instruction::Sub), "SUB");
    assert_eq!(format!("{}", Instruction::Mul), "MUL");
    assert_eq!(format!("{}", Instruction::Div), "DIV");
    assert_eq!(format!("{}", Instruction::Mod), "MOD");

    // Logical Instructions
    assert_eq!(format!("{}", Instruction::And), "AND");
    assert_eq!(format!("{}", Instruction::Or), "OR");
    assert_eq!(format!("{}", Instruction::Not), "NOT");
    assert_eq!(format!("{}", Instruction::Eq), "EQ");
    assert_eq!(format!("{}", Instruction::Ne), "NE");
    assert_eq!(format!("{}", Instruction::Lt), "LT");
    assert_eq!(format!("{}", Instruction::Le), "LE");
    assert_eq!(format!("{}", Instruction::Gt), "GT");
    assert_eq!(format!("{}", Instruction::Ge), "GE");

    // Process Instructions
    assert_eq!(format!("{}", Instruction::Par), "PAR");
    assert_eq!(format!("{}", Instruction::Send { arity: 2 }), "SEND 2");
    assert_eq!(
        format!("{}", Instruction::Receive { arity: 3, persistent: false }),
        "RECEIVE 3 ONCE"
    );
    assert_eq!(
        format!("{}", Instruction::Receive { arity: 3, persistent: true }),
        "RECEIVE 3 PERSISTENT"
    );
    assert_eq!(format!("{}", Instruction::New { count: 2 }), "NEW 2");

    // Control Flow Instructions
    assert_eq!(format!("{}", Instruction::Jump { target: 100 }), "JUMP 100");
    assert_eq!(format!("{}", Instruction::JumpIf { target: 200 }), "JUMPIF 200");
    assert_eq!(format!("{}", Instruction::JumpIfNot { target: 300 }), "JUMPIFNOT 300");
    assert_eq!(format!("{}", Instruction::Call { target: 400 }), "CALL 400");
    assert_eq!(format!("{}", Instruction::Return), "RETURN");
    assert_eq!(
        format!("{}", Instruction::CallBuiltin { name: "println".to_string(), arity: 1 }),
        "CALLBUILTIN println 1"
    );
    assert_eq!(format!("{}", Instruction::Match), "MATCH");
    assert_eq!(format!("{}", Instruction::MatchCase { target: 500 }), "MATCHCASE 500");

    // Memory Instructions
    assert_eq!(format!("{}", Instruction::Load { index: 10 }), "LOAD 10");
    assert_eq!(format!("{}", Instruction::Store { index: 20 }), "STORE 20");
    assert_eq!(format!("{}", Instruction::LoadLocal { index: 30 }), "LOADLOCAL 30");
    assert_eq!(format!("{}", Instruction::StoreLocal { index: 40 }), "STORELOCAL 40");
    assert_eq!(format!("{}", Instruction::PushEnv), "PUSHENV");
    assert_eq!(format!("{}", Instruction::PopEnv), "POPENV");

    // Data Structure Instructions
    assert_eq!(format!("{}", Instruction::ListNew), "LISTNEW");
    assert_eq!(format!("{}", Instruction::ListPush), "LISTPUSH");
    assert_eq!(format!("{}", Instruction::ListPop), "LISTPOP");
    assert_eq!(format!("{}", Instruction::ListGet), "LISTGET");
    assert_eq!(format!("{}", Instruction::MapNew), "MAPNEW");
    assert_eq!(format!("{}", Instruction::MapInsert), "MAPINSERT");
    assert_eq!(format!("{}", Instruction::MapGet), "MAPGET");
    assert_eq!(format!("{}", Instruction::MapRemove), "MAPREMOVE");
    assert_eq!(format!("{}", Instruction::TupleNew { size: 3 }), "TUPLENEW 3");
    assert_eq!(format!("{}", Instruction::TupleGet { index: 1 }), "TUPLEGET 1");

    // Built-in Instructions
    assert_eq!(format!("{}", Instruction::StringConcat), "STRINGCONCAT");
    assert_eq!(format!("{}", Instruction::StringLength), "STRINGLENGTH");
    assert_eq!(format!("{}", Instruction::StringSlice), "STRINGSLICE");

    // Quoting Instructions
    assert_eq!(format!("{}", Instruction::Quote), "QUOTE");
    assert_eq!(format!("{}", Instruction::Unquote), "UNQUOTE");
}

#[test]
fn test_instruction_helper_functions() {
    // Test push_int
    let push_int = Instruction::push_int(42);
    assert_eq!(
        push_int,
        Instruction::Push(Constant::Literal(Literal::Int(42)))
    );
    assert_eq!(format!("{}", push_int), "PUSH 42");

    // Test push_bool
    let push_bool = Instruction::push_bool(true);
    assert_eq!(
        push_bool,
        Instruction::Push(Constant::Literal(Literal::Bool(true)))
    );
    assert_eq!(format!("{}", push_bool), "PUSH true");

    // Test push_string
    let push_string = Instruction::push_string("hello");
    assert_eq!(
        push_string,
        Instruction::Push(Constant::Literal(Literal::String("hello".to_string())))
    );
    assert_eq!(format!("{}", push_string), "PUSH \"hello\"");

    // Test push_uri
    let push_uri = Instruction::push_uri("rho:io:stdout");
    assert_eq!(
        push_uri,
        Instruction::Push(Constant::Literal(Literal::Uri("rho:io:stdout".to_string())))
    );
    assert_eq!(format!("{}", push_uri), "PUSH `rho:io:stdout`");

    // Test push_bytes
    let bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let push_bytes = Instruction::push_bytes(bytes.clone());
    assert_eq!(
        push_bytes,
        Instruction::Push(Constant::Literal(Literal::ByteArray(bytes)))
    );
    assert_eq!(format!("{}", push_bytes), "PUSH 0xdeadbeef");
}

#[rstest]
#[case(42, "PUSH 42")]
#[case(0, "PUSH 0")]
#[case(-1, "PUSH -1")]
fn test_push_int_display(#[case] value: i64, #[case] expected: &str) {
    assert_eq!(format!("{}", Instruction::push_int(value)), expected);
}

#[rstest]
#[case(true, "PUSH true")]
#[case(false, "PUSH false")]
fn test_push_bool_display(#[case] value: bool, #[case] expected: &str) {
    assert_eq!(format!("{}", Instruction::push_bool(value)), expected);
}

#[test]
fn test_instruction_serialization() {
    // Test serialization and deserialization of instructions
    let instructions = vec![
        Instruction::push_int(42),
        Instruction::Push(Constant::Literal(Literal::String("hello".to_string()))),
        Instruction::Add,
        Instruction::Send { arity: 2 },
        Instruction::Jump { target: 100 },
    ];

    // Serialize to JSON
    let json = serde_json::to_string(&instructions).unwrap();
    
    // Deserialize from JSON
    let deserialized: Vec<Instruction> = serde_json::from_str(&json).unwrap();
    
    // Check that the deserialized instructions match the original
    assert_eq!(instructions, deserialized);
}