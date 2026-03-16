//! Tests for:
//! - Nil literal
//! - Boolean literals (true/false)
//! - Integer literals (positive, negative, zero)
//! - String literals (empty, simple, with spaces)
//! - Unit/empty tuple

mod common;

use common::*;
use rholang_vm::api::Value;

// === Nil Tests ===

#[test]
fn test_nil() {
    let source = "Nil";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Nil);
}

// === Boolean Tests ===

#[test]
fn test_bool_true() {
    let source = "true";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_bool_false() {
    let source = "false";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(false));
}

// === Integer Tests ===

#[test]
fn test_int_zero() {
    let source = "0";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_int_positive() {
    let source = "42";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_int_negative() {
    let source = "-123";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(-123));
}

#[test]
fn test_int_large_positive() {
    let source = "32767"; // i16::MAX
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(32767));
}

#[test]
fn test_int_large_negative() {
    let source = "-32768"; // i16::MIN
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(-32768));
}

#[test]
fn test_int_out_of_range_positive_uses_constant_pool() {
    let source = "100000"; // Beyond i16::MAX, uses PUSH_CONST
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(100_000));
}

#[test]
fn test_int_out_of_range_negative_uses_constant_pool() {
    let source = "-100000"; // Beyond i16::MIN, uses PUSH_CONST
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(-100_000));
}

// === String Tests ===

#[test]
fn test_string_empty() {
    let source = r#""""#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Str("".to_string()));
}

#[test]
fn test_string_simple() {
    let source = r#""hello""#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Str("hello".to_string()));
}

#[test]
fn test_string_with_spaces() {
    let source = r#""hello world""#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Str("hello world".to_string()));
}

#[test]
fn test_string_with_numbers() {
    let source = r#""test123""#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Str("test123".to_string()));
}

#[test]
fn test_string_unicode() {
    let source = r#""Hello 世界 🌍""#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Str("Hello 世界 🌍".to_string()));
}

// === Unit Tests ===

#[test]
fn test_unit_empty_tuple() {
    let source = "()";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Tuple(vec![]));
}

// === Multiple Literals (Edge Cases) ===

#[test]
fn test_literal_in_par() {
    let source = "1 | 2";
    let result = compile_and_run(source).unwrap();
    // Par executes left (discards), then right (returns)
    assert_eq!(result, Value::Int(2));
}
