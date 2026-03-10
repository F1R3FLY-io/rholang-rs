//! Tests for:
//! - List creation
//! - Tuple creation
//! - Nested collections
//! - Empty collections

mod common;

use common::*;
use rholang_vm::api::Value;

// === List Tests ===

#[test]
fn test_empty_list() {
    let source = "[]";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_list_single_element() {
    let source = "[42]";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::List(vec![Value::Int(42)]));
}

#[test]
fn test_list_multiple_elements() {
    let source = "[1, 2, 3]";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_list_mixed_types() {
    let source = r#"[42, true, "hello"]"#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Int(42),
            Value::Bool(true),
            Value::Str("hello".to_string())
        ])
    );
}

#[test]
fn test_list_with_expressions() {
    let source = "[1 + 2, 3 * 4, 10 - 5]";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(3), Value::Int(12), Value::Int(5)])
    );
}

#[test]
fn test_nested_lists() {
    let source = "[[1, 2], [3, 4]]";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::List(vec![Value::Int(3), Value::Int(4)])
        ])
    );
}

#[test]
fn test_deeply_nested_lists() {
    let source = "[[[1]]]";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::List(vec![Value::List(vec![Value::Int(1)])])])
    );
}

// === Tuple Tests ===

#[test]
fn test_empty_tuple() {
    let source = "()";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Tuple(vec![]));
}

#[test]
fn test_tuple_single_element() {
    let source = "(42,)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Tuple(vec![Value::Int(42)]));
}

#[test]
fn test_tuple_multiple_elements() {
    let source = "(1, 2, 3)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_tuple_mixed_types() {
    let source = r#"(42, true, "hello")"#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![
            Value::Int(42),
            Value::Bool(true),
            Value::Str("hello".to_string())
        ])
    );
}

#[test]
fn test_tuple_with_expressions() {
    let source = "(1 + 2, 3 * 4, 10 - 5)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![Value::Int(3), Value::Int(12), Value::Int(5)])
    );
}

#[test]
fn test_nested_tuples() {
    let source = "((1, 2), (3, 4))";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![
            Value::Tuple(vec![Value::Int(1), Value::Int(2)]),
            Value::Tuple(vec![Value::Int(3), Value::Int(4)])
        ])
    );
}

// === Mixed Collections ===

#[test]
fn test_list_of_tuples() {
    let source = "[(1, 2), (3, 4)]";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Tuple(vec![Value::Int(1), Value::Int(2)]),
            Value::Tuple(vec![Value::Int(3), Value::Int(4)])
        ])
    );
}

#[test]
fn test_tuple_of_lists() {
    let source = "([1, 2], [3, 4])";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::List(vec![Value::Int(3), Value::Int(4)])
        ])
    );
}

#[test]
fn test_complex_nested_collections() {
    let source = "[(1, [2, 3]), ([4, 5], 6)]";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Tuple(vec![
                Value::Int(1),
                Value::List(vec![Value::Int(2), Value::Int(3)])
            ]),
            Value::Tuple(vec![
                Value::List(vec![Value::Int(4), Value::Int(5)]),
                Value::Int(6)
            ])
        ])
    );
}

// === Collections with Control Flow ===

#[test]
fn test_list_with_if_expression() {
    let source = "[if (true) { 1 } else { 2 }, 3]";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::List(vec![Value::Int(1), Value::Int(3)]));
}

#[test]
fn test_tuple_with_comparisons() {
    let source = "(5 > 3, 2 == 2, 1 < 0)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![
            Value::Bool(true),
            Value::Bool(true),
            Value::Bool(false)
        ])
    );
}
