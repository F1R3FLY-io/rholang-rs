use rholang_bytecode::{Constant, Instruction, Literal};
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
        format!(
            "{}",
            Instruction::Receive {
                arity: 3,
                persistent: false
            }
        ),
        "RECEIVE 3 ONCE"
    );
    assert_eq!(
        format!(
            "{}",
            Instruction::Receive {
                arity: 3,
                persistent: true
            }
        ),
        "RECEIVE 3 PERSISTENT"
    );
    assert_eq!(format!("{}", Instruction::New { count: 2 }), "NEW 2");

    // Control Flow Instructions
    assert_eq!(format!("{}", Instruction::Jump { target: 100 }), "JUMP 100");
    assert_eq!(
        format!("{}", Instruction::JumpIf { target: 200 }),
        "JUMPIF 200"
    );
    assert_eq!(
        format!("{}", Instruction::JumpIfNot { target: 300 }),
        "JUMPIFNOT 300"
    );
    assert_eq!(format!("{}", Instruction::Call { target: 400 }), "CALL 400");
    assert_eq!(format!("{}", Instruction::Return), "RETURN");
    assert_eq!(
        format!(
            "{}",
            Instruction::CallBuiltin {
                name: "println".to_string(),
                arity: 1
            }
        ),
        "CALLBUILTIN println 1"
    );
    assert_eq!(format!("{}", Instruction::Match), "MATCH");
    assert_eq!(
        format!("{}", Instruction::MatchCase { target: 500 }),
        "MATCHCASE 500"
    );

    // Memory Instructions
    assert_eq!(format!("{}", Instruction::Load { index: 10 }), "LOAD 10");
    assert_eq!(format!("{}", Instruction::Store { index: 20 }), "STORE 20");
    assert_eq!(
        format!("{}", Instruction::LoadLocal { index: 30 }),
        "LOADLOCAL 30"
    );
    assert_eq!(
        format!("{}", Instruction::StoreLocal { index: 40 }),
        "STORELOCAL 40"
    );
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
    assert_eq!(
        format!("{}", Instruction::TupleNew { size: 3 }),
        "TUPLENEW 3"
    );
    assert_eq!(
        format!("{}", Instruction::TupleGet { index: 1 }),
        "TUPLEGET 1"
    );

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

#[rstest]
#[case("hello", "PUSH \"hello\"")]
#[case("", "PUSH \"\"")]
#[case("special \"chars\" \n\t", "PUSH \"special \"chars\" \n\t\"")]
fn test_push_string_display(#[case] value: &str, #[case] expected: &str) {
    assert_eq!(format!("{}", Instruction::push_string(value)), expected);
}

#[rstest]
#[case("rho:io:stdout", "PUSH `rho:io:stdout`")]
#[case("rho:id:1234", "PUSH `rho:id:1234`")]
#[case("", "PUSH ``")]
fn test_push_uri_display(#[case] value: &str, #[case] expected: &str) {
    assert_eq!(format!("{}", Instruction::push_uri(value)), expected);
}

#[rstest]
#[case(vec![0xDE, 0xAD, 0xBE, 0xEF], "PUSH 0xdeadbeef")]
#[case(vec![], "PUSH 0x")]
#[case(vec![0x00, 0x11, 0x22], "PUSH 0x001122")]
fn test_push_bytes_display(#[case] value: Vec<u8>, #[case] expected: &str) {
    assert_eq!(format!("{}", Instruction::push_bytes(value)), expected);
}

#[test]
fn test_instruction_equality() {
    // Test equality for different instruction types
    assert_eq!(Instruction::Add, Instruction::Add);
    assert_eq!(Instruction::push_int(42), Instruction::push_int(42));
    assert_eq!(
        Instruction::Send { arity: 2 },
        Instruction::Send { arity: 2 }
    );
    assert_eq!(
        Instruction::Jump { target: 100 },
        Instruction::Jump { target: 100 }
    );

    // Test inequality
    assert_ne!(Instruction::Add, Instruction::Sub);
    assert_ne!(Instruction::push_int(42), Instruction::push_int(43));
    assert_ne!(
        Instruction::Send { arity: 2 },
        Instruction::Send { arity: 3 }
    );
    assert_ne!(
        Instruction::Jump { target: 100 },
        Instruction::Jump { target: 101 }
    );
}

#[test]
fn test_instruction_cloning() {
    // Test cloning for different instruction types
    let add = Instruction::Add;
    let add_clone = add.clone();
    assert_eq!(add, add_clone);

    let push = Instruction::push_int(42);
    let push_clone = push.clone();
    assert_eq!(push, push_clone);

    let send = Instruction::Send { arity: 2 };
    let send_clone = send.clone();
    assert_eq!(send, send_clone);

    let jump = Instruction::Jump { target: 100 };
    let jump_clone = jump.clone();
    assert_eq!(jump, jump_clone);
}

#[test]
fn test_complex_instruction_sequences() {
    // Test a complex arithmetic sequence
    let arithmetic_sequence = [
        Instruction::push_int(10),
        Instruction::push_int(20),
        Instruction::Add,
        Instruction::push_int(5),
        Instruction::Mul,
        Instruction::push_int(2),
        Instruction::Div,
    ];

    // Test a complex control flow sequence
    let control_flow_sequence = [
        Instruction::push_bool(true),
        Instruction::JumpIf { target: 3 },
        Instruction::push_int(0),
        Instruction::Jump { target: 4 },
        Instruction::push_int(1),
    ];

    // Test a complex process sequence
    let process_sequence = [
        Instruction::New { count: 2 },
        Instruction::Dup,
        Instruction::push_string("message"),
        Instruction::Send { arity: 1 },
        Instruction::Receive {
            arity: 1,
            persistent: true,
        },
        Instruction::Par,
    ];

    // Verify the sequences can be created and accessed
    assert_eq!(arithmetic_sequence.len(), 7);
    assert_eq!(control_flow_sequence.len(), 5);
    assert_eq!(process_sequence.len(), 6);

    // Verify specific instructions in the sequences
    assert_eq!(arithmetic_sequence[2], Instruction::Add);
    assert_eq!(control_flow_sequence[1], Instruction::JumpIf { target: 3 });
    assert_eq!(
        process_sequence[4],
        Instruction::Receive {
            arity: 1,
            persistent: true
        }
    );
}

#[test]
fn test_data_structure_instructions() {
    // Test list instructions
    let list_instructions = [
        Instruction::ListNew,
        Instruction::push_int(1),
        Instruction::ListPush,
        Instruction::push_int(2),
        Instruction::ListPush,
        Instruction::ListGet,
        Instruction::ListPop,
    ];

    // Test map instructions
    let map_instructions = [
        Instruction::MapNew,
        Instruction::push_string("key1"),
        Instruction::push_int(42),
        Instruction::MapInsert,
        Instruction::push_string("key2"),
        Instruction::push_bool(true),
        Instruction::MapInsert,
        Instruction::push_string("key1"),
        Instruction::MapGet,
        Instruction::push_string("key2"),
        Instruction::MapRemove,
    ];

    // Test tuple instructions
    let tuple_instructions = [
        Instruction::TupleNew { size: 3 },
        Instruction::push_int(1),
        Instruction::push_string("two"),
        Instruction::push_bool(false),
        Instruction::TupleGet { index: 0 },
        Instruction::TupleGet { index: 1 },
        Instruction::TupleGet { index: 2 },
    ];

    // Verify the instructions can be created and accessed
    assert_eq!(list_instructions.len(), 7);
    assert_eq!(map_instructions.len(), 11);
    assert_eq!(tuple_instructions.len(), 7);

    // Verify specific instructions
    assert_eq!(list_instructions[0], Instruction::ListNew);
    assert_eq!(map_instructions[3], Instruction::MapInsert);
    assert_eq!(tuple_instructions[0], Instruction::TupleNew { size: 3 });
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

#[test]
fn test_complex_instruction_serialization() {
    // Test serialization and deserialization of complex instruction sequences
    let complex_instructions = vec![
        // Arithmetic sequence
        Instruction::push_int(10),
        Instruction::push_int(20),
        Instruction::Add,
        Instruction::push_int(5),
        Instruction::Mul,
        // Control flow sequence
        Instruction::push_bool(true),
        Instruction::JumpIf { target: 10 },
        Instruction::push_int(0),
        Instruction::Jump { target: 15 },
        // Process sequence
        Instruction::New { count: 2 },
        Instruction::Dup,
        Instruction::push_string("message"),
        Instruction::Send { arity: 1 },
        Instruction::Receive {
            arity: 1,
            persistent: true,
        },
        Instruction::Par,
        // Data structure sequence
        Instruction::ListNew,
        Instruction::push_int(1),
        Instruction::ListPush,
        Instruction::MapNew,
        Instruction::push_string("key"),
        Instruction::push_bool(false),
        Instruction::MapInsert,
        Instruction::TupleNew { size: 2 },
        Instruction::push_uri("rho:io:stdout"),
        Instruction::push_bytes(vec![0xCA, 0xFE]),
    ];

    // Serialize to JSON
    let json = serde_json::to_string(&complex_instructions).unwrap();

    // Deserialize from JSON
    let deserialized: Vec<Instruction> = serde_json::from_str(&json).unwrap();

    // Check that the deserialized instructions match the original
    assert_eq!(complex_instructions, deserialized);
}
