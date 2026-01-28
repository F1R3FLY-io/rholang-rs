use rholang_vm::{api::Instruction, api::Opcode, api::Process, api::Value, VM};

fn label(name: &str) -> Value {
    Value::Str(name.to_string())
}

#[test]
fn test_cmp_eq_and_neq_various_types() {
    // Int equality and inequality
    let vm1 = VM::new();
    let prog1 = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::nullary(Opcode::CMP_EQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p1 = Process::new(prog1, "cmp_eq_int");
    p1.vm = Some(vm1);
    let out1 = p1.execute().expect("exec ok");
    assert_eq!(out1, Value::Bool(true));

    let vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::CMP_NEQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "cmp_neq_int");
    p2.vm = Some(vm2);
    let out2 = p2.execute().expect("exec ok");
    assert_eq!(out2, Value::Bool(true));

    // String equality
    let vm3 = VM::new();
    let prog3 = vec![
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::unary(Opcode::PUSH_STR, 1),
        Instruction::nullary(Opcode::CMP_EQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p3 = Process::new(prog3, "cmp_eq_str");
    p3.names = vec![label("hello"), label("hello")];
    p3.vm = Some(vm3);
    let out3 = p3.execute().expect("exec ok");
    assert_eq!(out3, Value::Bool(true));

    // String inequality
    let vm4 = VM::new();
    let prog4 = vec![
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::unary(Opcode::PUSH_STR, 1),
        Instruction::nullary(Opcode::CMP_NEQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p4 = Process::new(prog4, "cmp_neq_str");
    p4.names = vec![label("a"), label("b")];
    p4.vm = Some(vm4);
    let out4 = p4.execute().expect("exec ok");
    assert_eq!(out4, Value::Bool(true));

    // Bool equality/inequality
    let vm5 = VM::new();
    let prog5 = vec![
        Instruction::unary(Opcode::PUSH_BOOL, 1),
        Instruction::unary(Opcode::PUSH_BOOL, 1),
        Instruction::nullary(Opcode::CMP_EQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p5 = Process::new(prog5, "cmp_eq_bool");
    p5.vm = Some(vm5);
    let out5 = p5.execute().expect("exec ok");
    assert_eq!(out5, Value::Bool(true));

    let vm6 = VM::new();
    let prog6 = vec![
        Instruction::unary(Opcode::PUSH_BOOL, 1),
        Instruction::unary(Opcode::PUSH_BOOL, 0),
        Instruction::nullary(Opcode::CMP_NEQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p6 = Process::new(prog6, "cmp_neq_bool");
    p6.vm = Some(vm6);
    let out6 = p6.execute().expect("exec ok");
    assert_eq!(out6, Value::Bool(true));

    // List equality
    let vm7 = VM::new();
    let prog7 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::CREATE_LIST, 1),
        Instruction::nullary(Opcode::CMP_EQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p7 = Process::new(prog7, "cmp_eq_list");
    p7.vm = Some(vm7);
    let out7 = p7.execute().expect("exec ok");
    assert_eq!(out7, Value::Bool(true));

    // Nil equality
    let vm8 = VM::new();
    let prog8 = vec![
        Instruction::nullary(Opcode::PUSH_NIL),
        Instruction::nullary(Opcode::PUSH_NIL),
        Instruction::nullary(Opcode::CMP_EQ),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p8 = Process::new(prog8, "cmp_eq_nil");
    p8.vm = Some(vm8);
    let out8 = p8.execute().expect("exec ok");
    assert_eq!(out8, Value::Bool(true));
}

#[test]
fn test_relational_ops_basic_and_equalities() {
    // LT: 2 < 3 -> true
    let vm1 = VM::new();
    let prog1 = vec![
        Instruction::unary(Opcode::PUSH_INT, 2),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::CMP_LT),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p1 = Process::new(prog1, "lt_true");
    p1.vm = Some(vm1);
    let out1 = p1.execute().expect("exec ok");
    assert_eq!(out1, Value::Bool(true));

    // GT: 5 > 1 -> true
    let vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::nullary(Opcode::CMP_GT),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "gt_true");
    p2.vm = Some(vm2);
    let out2 = p2.execute().expect("exec ok");
    assert_eq!(out2, Value::Bool(true));

    // LTE: 7 <= 7 -> true
    let vm3 = VM::new();
    let prog3 = vec![
        Instruction::unary(Opcode::PUSH_INT, 7),
        Instruction::unary(Opcode::PUSH_INT, 7),
        Instruction::nullary(Opcode::CMP_LTE),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p3 = Process::new(prog3, "lte_eq_true");
    p3.vm = Some(vm3);
    let out3 = p3.execute().expect("exec ok");
    assert_eq!(out3, Value::Bool(true));

    // GTE: 9 >= 9 -> true
    let vm4 = VM::new();
    let prog4 = vec![
        Instruction::unary(Opcode::PUSH_INT, 9),
        Instruction::unary(Opcode::PUSH_INT, 9),
        Instruction::nullary(Opcode::CMP_GTE),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p4 = Process::new(prog4, "gte_eq_true");
    p4.vm = Some(vm4);
    let out4 = p4.execute().expect("exec ok");
    assert_eq!(out4, Value::Bool(true));
}

#[test]
fn test_relational_ops_with_negatives() {
    // -5 < -3 -> true
    let vm1 = VM::new();
    let prog1 = vec![
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::nullary(Opcode::NEG),
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::NEG),
        Instruction::nullary(Opcode::CMP_LT),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p1 = Process::new(prog1, "lt_neg_true");
    p1.vm = Some(vm1);
    let out1 = p1.execute().expect("exec ok");
    assert_eq!(out1, Value::Bool(true));

    // -3 > -5 -> true
    let vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 3),
        Instruction::nullary(Opcode::NEG),
        Instruction::unary(Opcode::PUSH_INT, 5),
        Instruction::nullary(Opcode::NEG),
        Instruction::nullary(Opcode::CMP_GT),
        Instruction::nullary(Opcode::HALT),
    ];
    let mut p2 = Process::new(prog2, "gt_neg_true");
    p2.vm = Some(vm2);
    let out2 = p2.execute().expect("exec ok");
    assert_eq!(out2, Value::Bool(true));
}

#[test]
fn test_relational_ops_type_errors_and_underflow() {
    // Type mismatch: Int vs Str for CMP_LT -> error
    let vm1 = VM::new();
    let prog1 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::unary(Opcode::PUSH_STR, 0),
        Instruction::nullary(Opcode::CMP_LT),
    ];
    let mut p1 = Process::new(prog1, "lt_type_err");
    p1.names = vec![label("x")];
    p1.vm = Some(vm1);
    let err1 = p1
        .execute()
        .expect_err("should error for type mismatch on CMP_LT");
    let msg1 = err1.to_string().to_lowercase();
    assert!(msg1.contains("cmp_lt") && msg1.contains("int"));

    // Stack underflow for CMP_GT -> error (only one operand provided)
    let vm2 = VM::new();
    let prog2 = vec![
        Instruction::unary(Opcode::PUSH_INT, 1),
        Instruction::nullary(Opcode::CMP_GT),
    ];
    let mut p2 = Process::new(prog2, "gt_underflow_err");
    p2.vm = Some(vm2);
    let err2 = p2
        .execute()
        .expect_err("should error for underflow on CMP_GT");
    let msg2 = err2.to_string().to_lowercase();
    assert!(msg2.contains("cmp_gt") && msg2.contains("int"));
}
