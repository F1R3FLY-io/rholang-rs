use rholang_vm::{api::Instruction, api::Opcode, api::Process, api::Value, VM};

fn label(name: &str) -> Value {
    Value::Str(name.to_string())
}

#[test]
fn test_cmp_eq_and_neq_various_types() {
    // Int equality and inequality
    let mut vm1 = VM::new();
    let prog1 = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::nullary(Opcode::CMP_EQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p1 = Process::new(prog1, "cmp_eq_int");
    let out1 = vm1.execute(&mut p1).expect("exec ok");
    assert_eq!(out1, Value::Bool(true));

    let mut vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::CMP_NEQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "cmp_neq_int");
    let out2 = vm2.execute(&mut p2).expect("exec ok");
    assert_eq!(out2, Value::Bool(true));

    // String equality
    let mut vm3 = VM::new();
    let prog3 = vec![
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::unary(Opcode::PUSH_STR, 1),
        Instruction::nullary(Opcode::CMP_EQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p3 = Process::new(prog3, "cmp_eq_str");
    p3.names = vec![label("hello"), label("hello")];
    let out3 = vm3.execute(&mut p3).expect("exec ok");
    assert_eq!(out3, Value::Bool(true));

    // String inequality
    let mut vm4 = VM::new();
    let prog4 = vec![
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::unary(Opcode::PUSH_STR, 1),
        Instruction::nullary(Opcode::CMP_NEQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p4 = Process::new(prog4, "cmp_neq_str");
    p4.names = vec![label("a"), label("b")];
    let out4 = vm4.execute(&mut p4).expect("exec ok");
    assert_eq!(out4, Value::Bool(true));

    // Bool equality/inequality
    let mut vm5 = VM::new();
    let prog5 = vec![
        Instruction::unary(Opcode::PUSH_BOOL, 1),
        Instruction::unary(Opcode::PUSH_BOOL, 1),
        Instruction::nullary(Opcode::CMP_EQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p5 = Process::new(prog5, "cmp_eq_bool");
    let out5 = vm5.execute(&mut p5).expect("exec ok");
    assert_eq!(out5, Value::Bool(true));

    let mut vm6 = VM::new();
    let prog6 = vec![
        Instruction::unary(Opcode::PUSH_BOOL, 1),
        Instruction::unary(Opcode::PUSH_BOOL, 0),
        Instruction::nullary(Opcode::CMP_NEQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p6 = Process::new(prog6, "cmp_neq_bool");
    let out6 = vm6.execute(&mut p6).expect("exec ok");
    assert_eq!(out6, Value::Bool(true));

    // List equality
    let mut vm7 = VM::new();
    let prog7 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::nullary(Opcode::CMP_EQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p7 = Process::new(prog7, "cmp_eq_list");
    let out7 = vm7.execute(&mut p7).expect("exec ok");
    assert_eq!(out7, Value::Bool(true));

    // Nil equality
    let mut vm8 = VM::new();
    let prog8 = vec![
        Instruction::nullary(Opcode::PUSH_NIL),
        Instruction::nullary(Opcode::PUSH_NIL),
        Instruction::nullary(Opcode::CMP_EQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p8 = Process::new(prog8, "cmp_eq_nil");
    let out8 = vm8.execute(&mut p8).expect("exec ok");
    assert_eq!(out8, Value::Bool(true));
}

#[test]
fn test_relational_ops_basic_and_equalities() {
    // LT: 2 < 3 -> true
    let mut vm1 = VM::new();
    let prog1 = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::CMP_LT),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p1 = Process::new(prog1, "lt_true");
    let out1 = vm1.execute(&mut p1).expect("exec ok");
    assert_eq!(out1, Value::Bool(true));

    // GT: 5 > 1 -> true
    let mut vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::nullary(Opcode::CMP_GT),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "gt_true");
    let out2 = vm2.execute(&mut p2).expect("exec ok");
    assert_eq!(out2, Value::Bool(true));

    // LTE: 7 <= 7 -> true
    let mut vm3 = VM::new();
    let prog3 = vec![
        Instruction::unary(Opcode::PUSH_INT, 7),
        Instruction::unary(Opcode::PUSH_INT, 7),
        Instruction::nullary(Opcode::CMP_LTE),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p3 = Process::new(prog3, "lte_eq_true");
    let out3 = vm3.execute(&mut p3).expect("exec ok");
    assert_eq!(out3, Value::Bool(true));

    // GTE: 9 >= 9 -> true
    let mut vm4 = VM::new();
    let prog4 = vec![
        Instruction::unary(Opcode::PUSH_INT, 9),
        Instruction::unary(Opcode::PUSH_INT, 9),
        Instruction::nullary(Opcode::CMP_GTE),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p4 = Process::new(prog4, "gte_eq_true");
    let out4 = vm4.execute(&mut p4).expect("exec ok");
    assert_eq!(out4, Value::Bool(true));
}

#[test]
fn test_relational_ops_with_negatives() {
    // -5 < -3 -> true
    let mut vm1 = VM::new();
    let prog1 = vec![
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::nullary(Opcode::NEG),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::NEG),
        Instruction::nullary(Opcode::CMP_LT),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p1 = Process::new(prog1, "lt_neg_true");
    let out1 = vm1.execute(&mut p1).expect("exec ok");
    assert_eq!(out1, Value::Bool(true));

    // -3 > -5 -> true
    let mut vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::NEG),
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::nullary(Opcode::NEG),
        Instruction::nullary(Opcode::CMP_GT),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "gt_neg_true");
    let out2 = vm2.execute(&mut p2).expect("exec ok");
    assert_eq!(out2, Value::Bool(true));
}

#[test]
fn test_relational_ops_type_errors_and_underflow() {
    // Type mismatch: Int vs Str for CMP_LT -> error
    let mut vm1 = VM::new();
    let prog1 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::nullary(Opcode::CMP_LT),
    ];
    let mut p1 = Process::new(prog1, "lt_type_err");
    p1.names = vec![label("x")];
    let err1 = vm1
        .execute(&mut p1)
        .expect_err("should error for type mismatch on CMP_LT");
    let msg1 = err1.to_string().to_lowercase();
    assert!(msg1.contains("cmp_lt") && msg1.contains("int"));

    // Stack underflow for CMP_GT -> error (only one operand provided)
    let mut vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::nullary(Opcode::CMP_GT),
    ];
    let mut p2 = Process::new(prog2, "gt_underflow_err");
    let err2 = vm2
        .execute(&mut p2)
        .expect_err("should error for underflow on CMP_GT");
    let msg2 = err2.to_string().to_lowercase();
    assert!(msg2.contains("cmp_gt") && msg2.contains("int"));
}
