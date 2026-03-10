//! Tests for:
//! - Channel creation with `new`
//! - Send operations
//! - Receive operations with for-comprehension
//! - Parallel composition
//! - Combined channel and collection operations

mod common;

use common::*;
use rholang_vm::api::Value;

// === Basic Channel Tests ===

#[test]
fn test_new_channel_simple() {
    let source = "new x in { Nil }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_send_receive_simple() {
    let source = r#"
        new x in {
            x!(42) |
            for (y <- x) { y }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_send_receive_string() {
    let source = r#"
        new ch in {
            ch!("hello") |
            for (msg <- ch) { msg }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Str("hello".to_string()));
}

#[test]
fn test_send_receive_bool() {
    let source = r#"
        new flag in {
            flag!(true) |
            for (b <- flag) { b }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

// === Channel with Expressions ===

#[test]
fn test_send_expression() {
    let source = r#"
        new ch in {
            ch!(2 + 3) |
            for (x <- ch) { x }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_receive_and_compute() {
    let source = r#"
        new ch in {
            ch!(5) |
            for (x <- ch) { x + 10 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_receive_and_multiply() {
    let source = r#"
        new ch in {
            ch!(7) |
            for (x <- ch) { x * 2 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(14));
}

#[test]
fn test_receive_and_compare() {
    let source = r#"
        new ch in {
            ch!(10) |
            for (x <- ch) { x > 5 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

// === Multiple Channels ===

#[test]
fn test_two_channels() {
    let source = r#"
        new a, b in {
            a!(1) |
            b!(2) |
            for (x <- a) { x }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_multiple_sends_same_channel() {
    let source = r#"
        new ch in {
            ch!(1) |
            ch!(2) |
            for (x <- ch) { x }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    // Should receive one of the values (non-deterministic, but VM likely picks first)
    assert!(result == Value::Int(1) || result == Value::Int(2));
}

// === Channels with Collections ===

#[test]
fn test_send_receive_list() {
    let source = r#"
        new ch in {
            ch!([1, 2, 3]) |
            for (list <- ch) { list }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_send_receive_tuple() {
    let source = r#"
        new ch in {
            ch!((1, 2)) |
            for (tup <- ch) { tup }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Tuple(vec![Value::Int(1), Value::Int(2)]));
}

#[test]
fn test_send_list_receive_process() {
    let source = r#"
        new ch in {
            ch!([10, 20, 30]) |
            for (list <- ch) { list }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(10), Value::Int(20), Value::Int(30)])
    );
}

// === Parallel Composition Tests ===

#[test]
fn test_par_simple() {
    let source = "1 | 2";
    let result = compile_and_run(source).unwrap();
    // Par executes left (discards), then right (returns)
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_par_with_expressions() {
    let source = "(1 + 2) | (3 * 4)";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(12));
}

#[test]
fn test_par_chain() {
    let source = "1 | 2 | 3";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_par_with_nil() {
    let source = "Nil | 42";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}

// === Nested Channel Operations ===

#[test]
fn test_nested_new() {
    let source = r#"
        new x in {
            new y in {
                y!(10) |
                for (z <- y) { z }
            }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_channel_in_if() {
    let source = r#"
        new ch in {
            if (true) {
                ch!(1)
            } else {
                ch!(2)
            } |
            for (x <- ch) { x }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(1));
}

// === Complex Scenarios ===

#[test]
fn test_channel_with_computation() {
    let source = r#"
        new result in {
            result!(5 * 5) |
            for (x <- result) {
                if (x > 20) { x + 10 } else { x - 10 }
            }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(35)); // 25 + 10
}

#[test]
fn test_send_comparison_result() {
    let source = r#"
        new ch in {
            ch!(5 > 3) |
            for (result <- ch) { result }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_list_in_par() {
    let source = "[1, 2] | [3, 4]";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::List(vec![Value::Int(3), Value::Int(4)]));
}

// === Edge Cases ===

#[test]
fn test_send_nil() {
    let source = r#"
        new ch in {
            ch!(Nil) |
            for (x <- ch) { x }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_new_unused_channel() {
    let source = "new x, y, z in { 42 }";
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(42));
}
