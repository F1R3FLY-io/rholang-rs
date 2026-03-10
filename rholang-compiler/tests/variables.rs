//! Tests for:
//! - Variable binding through new declarations
//! - Variable binding through for-comprehension
//! - Variable references and scoping
//! - Multiple variables
//! - Nested scopes
//! - Variable shadowing

mod common;

use common::*;
use rholang_vm::api::Value;

// === Basic Variable Binding (via channels) ===

#[test]
fn test_variable_in_for_comprehension() {
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
fn test_variable_with_arithmetic() {
    let source = r#"
        new ch in {
            ch!(10) |
            for (x <- ch) { x + 5 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_variable_with_comparison() {
    let source = r#"
        new ch in {
            ch!(10) |
            for (x <- ch) { x > 5 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_variable_in_expression() {
    let source = r#"
        new ch in {
            ch!(5) |
            for (x <- ch) { x * 2 + 3 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(13)); // 5 * 2 + 3
}

// === Multiple Variables ===

#[test]
fn test_two_variables_sequential() {
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
fn test_variable_used_multiple_times() {
    let source = r#"
        new ch in {
            ch!(5) |
            for (x <- ch) { x + x }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_variable_in_nested_expr() {
    let source = r#"
        new ch in {
            ch!(3) |
            for (x <- ch) { (x + 2) * (x - 1) }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(10)); // (3 + 2) * (3 - 1) = 5 * 2
}

// === Variable Scoping ===

#[test]
fn test_nested_new_scopes() {
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
fn test_variable_in_nested_new() {
    let source = r#"
        new outer in {
            outer!(5) |
            for (x <- outer) {
                new inner in {
                    inner!(x) |
                    for (y <- inner) { y }
                }
            }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(5));
}

// === Variables with Different Types ===

#[test]
fn test_variable_string() {
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
fn test_variable_bool() {
    let source = r#"
        new ch in {
            ch!(true) |
            for (b <- ch) { b }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_variable_list() {
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
fn test_variable_tuple() {
    let source = r#"
        new ch in {
            ch!((42, true)) |
            for (tup <- ch) { tup }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![Value::Int(42), Value::Bool(true)])
    );
}

// === Variables in Control Flow ===

#[test]
fn test_variable_in_if_condition() {
    let source = r#"
        new ch in {
            ch!(10) |
            for (x <- ch) {
                if (x > 5) { 1 } else { 0 }
            }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_variable_in_if_branches() {
    let source = r#"
        new ch in {
            ch!(7) |
            for (x <- ch) {
                if (x > 5) { x + 10 } else { x - 10 }
            }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(17));
}

#[test]
fn test_variable_in_nested_if() {
    let source = r#"
        new ch in {
            ch!(8) |
            for (x <- ch) {
                if (x > 5) {
                    if (x < 10) { x * 2 } else { x }
                } else {
                    0
                }
            }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(16)); // 8 * 2
}

// === Complex Variable Usage ===

#[test]
fn test_variable_complex_arithmetic() {
    let source = r#"
        new ch in {
            ch!(4) |
            for (x <- ch) { (x + 1) * (x - 1) }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(15)); // (4 + 1) * (4 - 1) = 5 * 3
}

#[test]
fn test_variable_computed_from_expression() {
    let source = r#"
        new ch in {
            ch!(2 + 3) |
            for (x <- ch) { x * 10 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(50)); // 5 * 10
}

// === Variables with Nil ===

#[test]
fn test_variable_nil() {
    let source = r#"
        new ch in {
            ch!(Nil) |
            for (x <- ch) { x }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Nil);
}

// === Multiple Channels, Multiple Variables ===

#[test]
fn test_two_channels_independent() {
    let source = r#"
        new a, b in {
            a!(10) |
            b!(20) |
            for (x <- a) { x }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_channel_send_computed_value() {
    let source = r#"
        new result in {
            result!(5 * 5) |
            for (x <- result) { x + 10 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(35)); // 25 + 10
}

// === Wildcard Pattern (No Binding) ===

#[test]
fn test_wildcard_pattern() {
    let source = r#"
        new ch in {
            ch!(42) |
            for (_ <- ch) { 100 }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(100));
}

// === Edge Cases ===

#[test]
fn test_variable_zero() {
    let source = r#"
        new ch in {
            ch!(0) |
            for (x <- ch) { x }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_variable_negative() {
    let source = r#"
        new ch in {
            ch!(-42) |
            for (x <- ch) { x }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Int(-42));
}

#[test]
fn test_variable_in_list() {
    let source = r#"
        new ch in {
            ch!(5) |
            for (x <- ch) { [x, x + 1, x + 2] }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(5), Value::Int(6), Value::Int(7)])
    );
}

#[test]
fn test_variable_in_tuple() {
    let source = r#"
        new ch in {
            ch!(10) |
            for (x <- ch) { (x, x * 2) }
        }
    "#;
    let result = compile_and_run(source).unwrap();
    assert_eq!(result, Value::Tuple(vec![Value::Int(10), Value::Int(20)]));
}
