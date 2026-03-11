use num_bigint::BigInt;
use num_rational::BigRational;
use rholang_process::Process;
use rholang_vm::api::{Instruction, Opcode, Value};

fn run_with_constants(prog: Vec<Instruction>, constants: Vec<Value>) -> Result<Value, String> {
    let mut process = Process::new(prog, "numeric_test");
    process.constants = constants;
    process.execute().map_err(|e| e.to_string())
}

fn bigrat(numer: i64, denom: i64) -> Value {
    Value::BigRat(BigRational::new(BigInt::from(numer), BigInt::from(denom)))
}

fn fixed(unscaled: i64, scale: u32) -> Value {
    Value::FixedPoint {
        unscaled: BigInt::from(unscaled),
        scale,
    }
}

/// Binary op helper: push two constants, apply opcode, return result.
fn binop(a: Value, b: Value, op: Opcode) -> Result<Value, String> {
    run_with_constants(
        vec![
            Instruction::unary(Opcode::PUSH_CONST, 0),
            Instruction::unary(Opcode::PUSH_CONST, 1),
            Instruction::nullary(op),
            Instruction::nullary(Opcode::HALT),
        ],
        vec![a, b],
    )
}

/// Binary op that should error.
fn binop_err(a: Value, b: Value, op: Opcode) -> String {
    run_with_constants(
        vec![
            Instruction::unary(Opcode::PUSH_CONST, 0),
            Instruction::unary(Opcode::PUSH_CONST, 1),
            Instruction::nullary(op),
        ],
        vec![a, b],
    )
    .unwrap_err()
}

/// Unary op helper.
fn unaryop(a: Value, op: Opcode) -> Result<Value, String> {
    run_with_constants(
        vec![
            Instruction::unary(Opcode::PUSH_CONST, 0),
            Instruction::nullary(op),
            Instruction::nullary(Opcode::HALT),
        ],
        vec![a],
    )
}

// ==========================================================================
// Float: arithmetic, IEEE 754 edge cases, NaN semantics
// ==========================================================================

#[test]
fn test_float_arithmetic() {
    assert_eq!(binop(Value::Float(1.5), Value::Float(2.25), Opcode::ADD).unwrap(), Value::Float(3.75));
    assert_eq!(binop(Value::Float(5.0), Value::Float(3.0), Opcode::SUB).unwrap(), Value::Float(2.0));
    assert_eq!(binop(Value::Float(2.5), Value::Float(4.0), Opcode::MUL).unwrap(), Value::Float(10.0));
    assert_eq!(binop(Value::Float(10.0), Value::Float(4.0), Opcode::DIV).unwrap(), Value::Float(2.5));
    assert_eq!(unaryop(Value::Float(3.14), Opcode::NEG).unwrap(), Value::Float(-3.14));
}

#[test]
fn test_float_ieee754_edge_cases() {
    // div by zero -> Inf (IEEE 754, not an error)
    assert_eq!(binop(Value::Float(1.0), Value::Float(0.0), Opcode::DIV).unwrap(), Value::Float(f64::INFINITY));
    // MOD on float is an error per spec
    assert!(binop_err(Value::Float(5.0), Value::Float(3.0), Opcode::MOD).contains("not defined on floating point"));
}

#[test]
fn test_float_nan_semantics() {
    // NaN != NaN (IEEE 754)
    assert_eq!(binop(Value::Float(f64::NAN), Value::Float(f64::NAN), Opcode::CMP_EQ).unwrap(), Value::Bool(false));
    assert_eq!(binop(Value::Float(f64::NAN), Value::Float(f64::NAN), Opcode::CMP_NEQ).unwrap(), Value::Bool(true));
    // NaN ordering is undefined -> error
    assert!(binop_err(Value::Float(f64::NAN), Value::Float(1.0), Opcode::CMP_LT).contains("not comparable"));
}

#[test]
fn test_float_comparison() {
    assert_eq!(binop(Value::Float(1.5), Value::Float(2.5), Opcode::CMP_LT).unwrap(), Value::Bool(true));
}

// ==========================================================================
// BigInt: arithmetic, overflow, comparison
// ==========================================================================

#[test]
fn test_bigint_arithmetic() {
    let bi = |n: i64| Value::BigInt(BigInt::from(n));
    assert_eq!(binop(bi(100), bi(200), Opcode::ADD).unwrap(), bi(300));
    assert_eq!(binop(bi(500), bi(200), Opcode::SUB).unwrap(), bi(300));
    assert_eq!(binop(bi(12), bi(13), Opcode::MUL).unwrap(), bi(156));
    // Integer division: 100 / 7 == 14 (spec: `10n / 3n == 3n`)
    assert_eq!(binop(bi(100), bi(7), Opcode::DIV).unwrap(), bi(14));
    // Remainder: 100 % 7 == 2 (spec: `10n % 3n == 1n`)
    assert_eq!(binop(bi(100), bi(7), Opcode::MOD).unwrap(), bi(2));
    assert_eq!(unaryop(bi(42), Opcode::NEG).unwrap(), bi(-42));
}

#[test]
fn test_bigint_errors() {
    let bi = |n: i64| Value::BigInt(BigInt::from(n));
    assert!(binop_err(bi(1), bi(0), Opcode::DIV).contains("division by zero"));

    // Overflow: 2^1023 * 2^1023 exceeds 128-byte cap
    let large: BigInt = BigInt::from(1) << 1023_usize;
    assert!(binop_err(Value::BigInt(large.clone()), Value::BigInt(large), Opcode::MUL).contains("byte limit"));
}

#[test]
fn test_bigint_comparison() {
    let bi = |n: i64| Value::BigInt(BigInt::from(n));
    assert_eq!(binop(bi(100), bi(200), Opcode::CMP_LT).unwrap(), Value::Bool(true));
}

// ==========================================================================
// BigRat: exact rational arithmetic
// ==========================================================================

#[test]
fn test_bigrat_arithmetic() {
    // Uses non-trivial rationals to verify normalization
    assert_eq!(binop(bigrat(1, 3), bigrat(1, 6), Opcode::ADD).unwrap(), bigrat(1, 2));  // 1/3 + 1/6 = 1/2
    assert_eq!(binop(bigrat(3, 4), bigrat(1, 4), Opcode::SUB).unwrap(), bigrat(1, 2));  // 3/4 - 1/4 = 1/2
    assert_eq!(binop(bigrat(2, 3), bigrat(3, 4), Opcode::MUL).unwrap(), bigrat(1, 2));  // 2/3 * 3/4 = 1/2
    // Spec: `10r / 3r == 3r + 1r/3r` i.e. 10/3
    assert_eq!(binop(bigrat(1, 2), bigrat(1, 4), Opcode::DIV).unwrap(), bigrat(2, 1));
    // Spec: modulus always gives 0 since (a/b)*b == a exactly
    assert_eq!(binop(bigrat(7, 3), bigrat(2, 5), Opcode::MOD).unwrap(), bigrat(0, 1));
    assert_eq!(unaryop(bigrat(3, 4), Opcode::NEG).unwrap(), bigrat(-3, 4));
}

#[test]
fn test_bigrat_errors() {
    assert!(binop_err(bigrat(1, 2), bigrat(0, 1), Opcode::DIV).contains("division by zero"));
}

#[test]
fn test_bigrat_comparison() {
    assert_eq!(binop(bigrat(1, 3), bigrat(1, 2), Opcode::CMP_LT).unwrap(), Value::Bool(true));
}

// ==========================================================================
// FixedPoint: scale-aware arithmetic, C99 identity
// ==========================================================================

#[test]
fn test_fixedpoint_add_sub() {
    // Same-scale required; 1.50 + 2.25 = 3.75
    assert_eq!(binop(fixed(150, 2), fixed(225, 2), Opcode::ADD).unwrap(), fixed(375, 2));
    // 5.00 - 3.25 = 1.75
    assert_eq!(binop(fixed(500, 2), fixed(325, 2), Opcode::SUB).unwrap(), fixed(175, 2));
}

#[test]
fn test_fixedpoint_mul_doubles_scale() {
    // Spec: scale doubles on multiplication. 1.5p1 * 2.0p1 = 3.00p2
    assert_eq!(binop(fixed(15, 1), fixed(20, 1), Opcode::MUL).unwrap(), fixed(300, 2));
}

#[test]
fn test_fixedpoint_div_and_mod_c99() {
    // Spec: `10p1 / 3p1 == 3.3p1` (shifted integer division)
    assert_eq!(binop(fixed(100, 1), fixed(30, 1), Opcode::DIV).unwrap(), fixed(33, 1));
    // Spec: `10p1 % 3p1 == 0.1p1`, satisfying C99 identity (a/b)*b + a%b == a
    assert_eq!(binop(fixed(100, 1), fixed(30, 1), Opcode::MOD).unwrap(), fixed(1, 1));
    // Exact division: 6.0p1 % 3.0p1 = 0.0p1
    assert_eq!(binop(fixed(60, 1), fixed(30, 1), Opcode::MOD).unwrap(), fixed(0, 1));
}

#[test]
fn test_fixedpoint_neg_and_errors() {
    assert_eq!(unaryop(fixed(150, 2), Opcode::NEG).unwrap(), fixed(-150, 2));
    // Different scales -> type mismatch
    assert!(binop_err(fixed(150, 2), fixed(15, 1), Opcode::ADD).contains("type mismatch"));
    // Division by zero
    assert!(binop_err(fixed(100, 1), fixed(0, 1), Opcode::DIV).contains("division by zero"));
    assert!(binop_err(fixed(100, 1), fixed(0, 1), Opcode::MOD).contains("division by zero"));
}

// ==========================================================================
// Cross-type errors: no implicit coercion
// ==========================================================================

#[test]
fn test_cross_type_errors() {
    let bi = Value::BigInt(BigInt::from(1));
    // Int + Float
    assert!(binop_err(Value::Int(1), Value::Float(1.0), Opcode::ADD).contains("type mismatch"));
    // BigInt * BigRat
    assert!(binop_err(bi.clone(), bigrat(1, 2), Opcode::MUL).contains("type mismatch"));
    // Float - BigInt
    assert!(binop_err(Value::Float(1.0), bi, Opcode::SUB).contains("type mismatch"));
}
