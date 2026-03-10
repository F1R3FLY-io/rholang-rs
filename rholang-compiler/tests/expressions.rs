//! Tests for:
//! - Arithmetic operators (+, -, *, /)
//! - Comparison operators (==, !=, <, <=, >, >=)
//! - Logical operators (&&, ||)
//! - Operator precedence
//! - Mixed type expressions
//! - Nested expressions

mod common;

use common::*;
use rholang_vm::api::Value;

// === Arithmetic Operators ===

#[test]
fn test_add_simple() {
    let source = "2 + 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_sub_simple() {
    let source = "10 - 7";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_mul_simple() {
    let source = "4 * 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_div_simple() {
    let source = "20 / 4";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_add_negative() {
    let source = "-5 + 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(-2));
}

#[test]
fn test_sub_negative() {
    let source = "5 - 10";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(-5));
}

#[test]
fn test_mul_negative() {
    let source = "-3 * 4";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(-12));
}

#[test]
fn test_div_negative() {
    let source = "-20 / 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(-4));
}

#[test]
fn test_add_zero() {
    let source = "42 + 0";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_mul_zero() {
    let source = "42 * 0";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(0));
}

// === Comparison Operators ===

#[test]
fn test_eq_true() {
    let source = "5 == 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_eq_false() {
    let source = "5 == 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_neq_true() {
    let source = "5 != 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_neq_false() {
    let source = "5 != 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_lt_true() {
    let source = "3 < 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_lt_false() {
    let source = "5 < 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_lt_equal() {
    let source = "5 < 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_lte_true() {
    let source = "3 <= 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_lte_equal() {
    let source = "5 <= 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_lte_false() {
    let source = "7 <= 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_gt_true() {
    let source = "7 > 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_gt_false() {
    let source = "3 > 7";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_gt_equal() {
    let source = "5 > 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_gte_true() {
    let source = "7 >= 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_gte_equal() {
    let source = "5 >= 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_gte_false() {
    let source = "3 >= 7";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

// === String Comparison ===

#[test]
fn test_string_eq_true() {
    let source = r#""hello" == "hello""#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_string_eq_false() {
    let source = r#""hello" == "world""#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_string_neq_true() {
    let source = r#""hello" != "world""#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

// === Boolean Comparison ===

#[test]
fn test_bool_eq_true() {
    let source = "true == true";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_bool_eq_false() {
    let source = "true == false";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

// === Logical Operators ===

#[test]
fn test_and_true_true() {
    let source = "true and true";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_and_true_false() {
    let source = "true and false";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_and_false_true() {
    let source = "false and true";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_and_false_false() {
    let source = "false and false";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_or_true_true() {
    let source = "true or true";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_or_true_false() {
    let source = "true or false";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_or_false_true() {
    let source = "false or true";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_or_false_false() {
    let source = "false or false";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

// === Combined Logical Expressions ===

#[test]
fn test_and_with_comparisons() {
    let source = "(5 > 3) and (10 < 20)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_or_with_comparisons() {
    let source = "(5 < 3) or (10 < 20)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_complex_boolean_expr() {
    let source = "(5 > 3) and (10 == 10)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_complex_boolean_with_or() {
    let source = "(5 < 3) or (10 == 10)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_nested_logical_operators() {
    let source = "((true and false) or true) and true";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_logical_with_arithmetic() {
    let source = "((2 + 3) > 4) and ((10 - 5) == 5)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

// === Nested and Complex Expressions ===

#[test]
fn test_nested_arithmetic() {
    let source = "2 + 3 * 4";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(14)); // 2 + 12
}

#[test]
fn test_nested_with_parens() {
    let source = "(2 + 3) * 4";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(20)); // 5 * 4
}

#[test]
fn test_deeply_nested() {
    let source = "((2 + 3) * (4 + 1))";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(25)); // 5 * 5
}

#[test]
fn test_complex_expression() {
    let source = "(10 + 5) / 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(5)); // 15 / 3
}

#[test]
fn test_mixed_operations() {
    let source = "20 - 5 * 2 + 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(13)); // 20 - 10 + 3
}

// === Comparison Chains ===

#[test]
fn test_comparison_result_as_value() {
    let source = "5 > 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_arithmetic_comparison() {
    let source = "(2 + 3) == 5";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_complex_comparison() {
    let source = "(10 / 2) > (3 * 1)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true)); // 5 > 3
}

// === Edge Cases ===

#[test]
fn test_division_truncation() {
    let source = "7 / 2";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(3)); // Integer division
}

#[test]
fn test_negative_comparison() {
    let source = "-5 < 0";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_zero_comparison() {
    let source = "0 == 0";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}
