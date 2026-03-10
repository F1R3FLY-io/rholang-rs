//! Tests for:
//! - If-then-else expressions
//! - Nested conditionals
//! - Conditionals with complex expressions
//! - Conditionals with all value types
//! - Edge cases (missing else, nested if)

mod common;

use common::*;
use rholang_vm::api::Value;

// === Basic If-Then-Else ===

#[test]
fn test_if_true_simple() {
    let source = "if (true) { 1 } else { 2 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_if_false_simple() {
    let source = "if (false) { 1 } else { 2 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_if_no_else_true() {
    let source = "if (true) { 42 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_if_no_else_false() {
    let source = "if (false) { 42 }";
    let result = compile_and_run(source).unwrap();
    // When condition is false and no else, the if expression evaluates to Nil
    assert_eq!(result, Value::Nil);
}

// === If with Comparisons ===

#[test]
fn test_if_with_comparison_true() {
    let source = "if (5 > 3) { 10 } else { 20 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_if_with_comparison_false() {
    let source = "if (3 > 5) { 10 } else { 20 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_if_with_equality() {
    let source = "if (5 == 5) { 100 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(100));
}

#[test]
fn test_if_with_inequality() {
    let source = "if (5 != 3) { 1 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(1));
}

// === If with Complex Conditions ===

#[test]
fn test_if_with_and_condition() {
    let source = "if (true and true) { 1 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_if_with_and_condition_false() {
    let source = "if (true and false) { 1 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_if_with_or_condition() {
    let source = "if (false or true) { 1 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_if_with_or_condition_false() {
    let source = "if (false or false) { 1 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_if_with_complex_and_condition() {
    let source = "if ((5 > 3) and (10 < 20)) { 42 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_if_with_complex_or_condition() {
    let source = "if ((5 < 3) or (10 < 20)) { 42 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_if_with_nested_logical_ops() {
    let source = "if ((true and true) or false) { 100 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(100));
}

#[test]
fn test_if_with_arithmetic_condition() {
    let source = "if ((2 + 3) == 5) { 100 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(100));
}

// === If with Different Return Types ===

#[test]
fn test_if_returning_string() {
    let source = r#"if (true) { "yes" } else { "no" }"#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Str("yes".to_string()));
}

#[test]
fn test_if_returning_bool() {
    let source = "if (5 > 3) { true } else { false }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_if_returning_nil() {
    let source = "if (true) { Nil } else { 42 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Nil);
}

// === If with Expressions in Branches ===

#[test]
fn test_if_with_arithmetic_in_then() {
    let source = "if (true) { 2 + 3 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_if_with_arithmetic_in_else() {
    let source = "if (false) { 0 } else { 10 - 3 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_if_with_arithmetic_in_both() {
    let source = "if (true) { 2 * 5 } else { 3 * 3 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_if_with_comparison_in_branch() {
    let source = "if (true) { 10 > 5 } else { false }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

// === Nested If-Then-Else ===

#[test]
fn test_nested_if_in_then() {
    let source = r#"
        if (true) {
            if (true) { 1 } else { 2 }
        } else {
            3
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_nested_if_in_else() {
    let source = r#"
        if (false) {
            1
        } else {
            if (true) { 2 } else { 3 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_nested_if_both_branches() {
    let source = r#"
        if (true) {
            if (false) { 1 } else { 2 }
        } else {
            if (true) { 3 } else { 4 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_deeply_nested_if() {
    let source = r#"
        if (true) {
            if (true) {
                if (true) { 42 } else { 0 }
            } else {
                0
            }
        } else {
            0
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}

// === If-Else-If Chains ===

#[test]
fn test_if_else_if_first() {
    let source = r#"
        if (5 > 10) {
            1
        } else {
            if (3 < 5) { 2 } else { 3 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_if_else_if_last() {
    let source = r#"
        if (false) {
            1
        } else {
            if (false) { 2 } else { 3 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(3));
}

// === Edge Cases ===

#[test]
fn test_if_with_nil_in_branch() {
    let source = "if (true) { Nil } else { Nil }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_if_condition_from_comparison() {
    let source = "if (10 == 10) { 1 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_if_with_zero_vs_nonzero() {
    let source = "if (5 > 0) { 100 } else { -100 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(100));
}

// === If with Collections (Integration) ===

#[test]
fn test_if_returning_list() {
    let source = "if (true) { [1, 2, 3] } else { [] }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_if_returning_tuple() {
    let source = "if (false) { (1, 2) } else { (3, 4) }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Tuple(vec![Value::Int(3), Value::Int(4)]));
}

#[test]
fn test_if_with_list_in_condition() {
    let source = "if (true) { [10, 20] } else { [30, 40] }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::List(vec![Value::Int(10), Value::Int(20)]));
}

// === Complex Branching Scenarios ===

#[test]
fn test_multiple_conditions_combined() {
    let source = r#"
        if ((5 > 3) and (10 < 20)) {
            if (2 == 2) { 42 } else { 0 }
        } else {
            -1
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_multiple_conditions_nested() {
    let source = r#"
        if (5 > 3) {
            if (10 < 20) {
                if (2 == 2) { 42 } else { 0 }
            } else {
                -1
            }
        } else {
            -1
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_if_with_par_in_branches() {
    let source = "if (true) { 1 | 2 } else { 3 | 4 }";
    let result = compile_and_run(source).unwrap();
    // Par returns the right side
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_condition_with_nested_expr() {
    let source = "if (((2 + 3) * 2) > 5) { 100 } else { 0 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(100)); // (5 * 2) = 10 > 5
}
