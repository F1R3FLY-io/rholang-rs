mod common;

use common::*;
use num_bigint::BigInt;
use num_rational::BigRational;
use rholang_vm::api::Value;

fn bigrat(n: i64, d: i64) -> Value {
    Value::BigRat(BigRational::new(BigInt::from(n), BigInt::from(d)))
}

fn fixed(unscaled: i64, scale: u32) -> Value {
    Value::FixedPoint {
        unscaled: BigInt::from(unscaled),
        scale,
    }
}

// ==========================================================================
// Literal parsing + compilation for each numeric type
// ==========================================================================

#[test]
fn test_bigint_literals_and_arithmetic() {
    assert_eq!(compile_and_run("42n").unwrap(), Value::BigInt(BigInt::from(42)));
    assert_eq!(compile_and_run("-7n").unwrap(), Value::BigInt(BigInt::from(-7)));
    assert_eq!(compile_and_run("0n").unwrap(), Value::BigInt(BigInt::from(0)));

    // Spec: 10n / 3n == 3n, 10n % 3n == 1n
    assert_eq!(compile_and_run("10n + 20n").unwrap(), Value::BigInt(BigInt::from(30)));
    assert_eq!(compile_and_run("50n - 30n").unwrap(), Value::BigInt(BigInt::from(20)));
    assert_eq!(compile_and_run("6n * 7n").unwrap(), Value::BigInt(BigInt::from(42)));
    assert_eq!(compile_and_run("100n / 7n").unwrap(), Value::BigInt(BigInt::from(14)));
}

#[test]
fn test_bigrat_literals_and_arithmetic() {
    assert_eq!(compile_and_run("3r").unwrap(), bigrat(3, 1));
    assert_eq!(compile_and_run("-5r").unwrap(), bigrat(-5, 1));

    // Spec: 10r / 3r == 10/3 (exact rational division)
    assert_eq!(compile_and_run("3r + 4r").unwrap(), bigrat(7, 1));
    assert_eq!(compile_and_run("10r - 3r").unwrap(), bigrat(7, 1));
    assert_eq!(compile_and_run("3r * 4r").unwrap(), bigrat(12, 1));
    assert_eq!(compile_and_run("10r / 3r").unwrap(), bigrat(10, 3));
}

#[test]
fn test_float_literals_and_arithmetic() {
    assert_eq!(compile_and_run("3.15f64").unwrap(), Value::Float(3.15));
    assert_eq!(compile_and_run("2.5f32").unwrap(), Value::Float(2.5_f32 as f64)); // f32 stored as f64
    assert_eq!(compile_and_run("-1.5f64").unwrap(), Value::Float(-1.5));
    assert_eq!(compile_and_run("1.5e2f64").unwrap(), Value::Float(150.0)); // scientific notation

    assert_eq!(compile_and_run("1.5f64 + 2.25f64").unwrap(), Value::Float(3.75));
    assert_eq!(compile_and_run("5.0f64 - 3.0f64").unwrap(), Value::Float(2.0));
    assert_eq!(compile_and_run("2.0f64 * 3.0f64").unwrap(), Value::Float(6.0));
    assert_eq!(compile_and_run("10.0f64 / 4.0f64").unwrap(), Value::Float(2.5));
}

#[test]
fn test_fixedpoint_literals_and_arithmetic() {
    // 1.50p2 => unscaled=150, scale=2
    assert_eq!(compile_and_run("1.50p2").unwrap(), fixed(150, 2));
    assert_eq!(compile_and_run("-3.0p1").unwrap(), fixed(-30, 1));

    assert_eq!(compile_and_run("1.50p2 + 2.25p2").unwrap(), fixed(375, 2));
    assert_eq!(compile_and_run("5.00p2 - 3.25p2").unwrap(), fixed(175, 2));
    // Scale-preserving mul: 1.5p1 * 2.0p1 = 3.0p1 (unscaled: (15*20)/10 = 30)
    assert_eq!(compile_and_run("1.5p1 * 2.0p1").unwrap(), fixed(30, 1));
    // Spec: 10p1 / 3p1 == 3.3p1
    assert_eq!(compile_and_run("10.0p1 / 3.0p1").unwrap(), fixed(33, 1));
}

// ==========================================================================
// Signed/Unsigned integer literals (width-qualified)
// ==========================================================================

#[test]
fn test_signed_int_literals() {
    // i64 is same as unqualified: -52i64 == -52
    assert_eq!(compile_and_run("42i64").unwrap(), Value::Int(42));
    assert_eq!(compile_and_run("-52i64").unwrap(), Value::Int(-52));
    // Small width that fits in i64
    assert_eq!(compile_and_run("127i8").unwrap(), Value::Int(127));
}

#[test]
fn test_unsigned_int_literals() {
    assert_eq!(compile_and_run("255u8").unwrap(), Value::Int(255));
    assert_eq!(compile_and_run("0u64").unwrap(), Value::Int(0));
    assert_eq!(compile_and_run("65535u16").unwrap(), Value::Int(65535));
}

// ==========================================================================
// Comparisons across extended types
// ==========================================================================

#[test]
fn test_extended_type_comparisons() {
    assert_eq!(compile_and_run("42n == 42n").unwrap(), Value::Bool(true));
    assert_eq!(compile_and_run("42n != 43n").unwrap(), Value::Bool(true));
    assert_eq!(compile_and_run("1n < 2n").unwrap(), Value::Bool(true));
    assert_eq!(compile_and_run("3.14f64 == 3.14f64").unwrap(), Value::Bool(true));
    assert_eq!(compile_and_run("1.0f64 < 2.0f64").unwrap(), Value::Bool(true));
    assert_eq!(compile_and_run("5r == 5r").unwrap(), Value::Bool(true));
}
