//! Core value types for RSpace storage.

use crate::ExecError;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;
use std::any::Any;
use std::cmp::Ordering;
use std::fmt;

/// Process execution state.
///
/// Tracks the lifecycle of a process from creation to completion.
#[derive(Clone, Debug, PartialEq)]
pub enum ProcessState {
    /// Process is blocked, must not be executed.
    Wait,
    /// Process is eligible for execution.
    Ready,
    /// Process finished successfully with a final value (terminal state).
    Value(Value),
    /// Process failed with an error message (terminal state).
    Error(String),
}

/// Opaque process holder for `Value::Par`.
///
/// This trait allows `Process` to be defined in a downstream crate (rholang-process)
/// while `Value` stays in the core abstraction layer (rholang-rspace).
///
/// # SOLID Principles
/// - **Interface Segregation**: Minimal interface for process abstraction
/// - **Dependency Inversion**: High-level Value depends on abstraction, not concrete Process
pub trait ProcessHolder: Send + Sync + fmt::Debug {
    /// Downcast to concrete type.
    fn as_any(&self) -> &dyn Any;

    /// Downcast to concrete type mutably.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Clone into a boxed trait object.
    fn clone_box(&self) -> Box<dyn ProcessHolder>;

    /// Compare equality with another process holder.
    fn eq_box(&self, other: &dyn ProcessHolder) -> bool;

    /// Check if the process is ready to execute.
    fn is_ready(&self) -> bool;

    /// Execute the process and return its result value.
    fn execute(&mut self) -> Result<Value, ExecError>;

    /// Get the source reference (debug/provenance tag).
    fn source_ref(&self) -> &str;

    /// Get the current process state.
    fn state(&self) -> &ProcessState;
}

impl Clone for Box<dyn ProcessHolder> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for Box<dyn ProcessHolder> {
    fn eq(&self, other: &Self) -> bool {
        self.eq_box(other.as_ref())
    }
}


/// Runtime value in RSpace.
///
/// Values are the fundamental data type stored in RSpace channels,
/// passed between processes, and returned from computations.
///
/// # Numeric types
///
/// Following the Rholang numeric types spec (Michael Stay, 2025-11-13):
/// - `Int(i64)` — default signed 64-bit integers (unqualified literals)
/// - `Float(f64)` — IEEE 754 double precision (suffix `f64`, also covers `f32`)
/// - `BigInt(BigInt)` — arbitrary-precision signed integers (suffix `n`)
/// - `BigRat(BigRational)` — exact rationals as ratio of bigints (suffix `r`)
/// - `FixedPoint` — shifted bigints with fixed decimal scale (suffix `p<digits>`)
///
/// No implicit coercion between types. All binary ops require matching types.
#[derive(Clone, Debug)]
pub enum Value {
    /// 64-bit signed integer.
    Int(i64),
    /// IEEE 754 double-precision float. NaN != NaN per IEEE 754.
    Float(f64),
    /// Arbitrary-precision signed integer (suffix `n`).
    BigInt(BigInt),
    /// Exact rational number as ratio of BigInts (suffix `r`).
    BigRat(BigRational),
    /// Fixed-point decimal: actual_value = unscaled / 10^scale (suffix `p<scale>`).
    FixedPoint {
        unscaled: BigInt,
        scale: u32,
    },
    /// Boolean value.
    Bool(bool),
    /// UTF-8 string.
    Str(String),
    /// Channel/name reference.
    Name(String),
    /// Ordered list of values.
    List(Vec<Value>),
    /// Fixed-size tuple of values.
    Tuple(Vec<Value>),
    /// Key-value map (preserves insertion order).
    Map(Vec<(Value, Value)>),
    /// Parallel composition of processes.
    /// Use rholang-process utilities to work with these.
    Par(Vec<Box<dyn ProcessHolder>>),
    /// Null/unit value.
    Nil,
}

/// Custom PartialEq: Float uses IEEE 754 semantics where NaN != NaN.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b, // NaN != NaN per IEEE 754
            (Value::BigInt(a), Value::BigInt(b)) => a == b,
            (Value::BigRat(a), Value::BigRat(b)) => a == b,
            (
                Value::FixedPoint {
                    unscaled: ua,
                    scale: sa,
                },
                Value::FixedPoint {
                    unscaled: ub,
                    scale: sb,
                },
            ) => sa == sb && ua == ub,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Name(a), Value::Name(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Par(a), Value::Par(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Some(a.cmp(b)),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b), // NaN → None
            (Value::BigInt(a), Value::BigInt(b)) => Some(a.cmp(b)),
            (Value::BigRat(a), Value::BigRat(b)) => Some(a.cmp(b)),
            (
                Value::FixedPoint {
                    unscaled: ua,
                    scale: sa,
                },
                Value::FixedPoint {
                    unscaled: ub,
                    scale: sb,
                },
            ) => {
                if sa != sb {
                    None // cross-scale comparison not defined
                } else {
                    Some(ua.cmp(ub))
                }
            }
            (Value::Str(a), Value::Str(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }
}

impl Value {
    /// Try to extract an integer value.
    pub fn as_int(&self) -> Option<i64> {
        if let Value::Int(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Try to extract a float value.
    pub fn as_float(&self) -> Option<f64> {
        if let Value::Float(f) = self {
            Some(*f)
        } else {
            None
        }
    }

    /// Try to extract a BigInt reference.
    pub fn as_bigint(&self) -> Option<&BigInt> {
        if let Value::BigInt(n) = self {
            Some(n)
        } else {
            None
        }
    }

    /// Try to extract a BigRational reference.
    pub fn as_bigrat(&self) -> Option<&BigRational> {
        if let Value::BigRat(r) = self {
            Some(r)
        } else {
            None
        }
    }

    /// Try to extract a FixedPoint value.
    pub fn as_fixed_point(&self) -> Option<(&BigInt, u32)> {
        if let Value::FixedPoint { unscaled, scale } = self {
            Some((unscaled, *scale))
        } else {
            None
        }
    }

    /// Try to extract a boolean value.
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// Try to extract a string value.
    pub fn as_str(&self) -> Option<&str> {
        if let Value::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// Returns the type name for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::BigInt(_) => "BigInt",
            Value::BigRat(_) => "BigRat",
            Value::FixedPoint { .. } => "FixedPoint",
            Value::Bool(_) => "Bool",
            Value::Str(_) => "Str",
            Value::Name(_) => "Name",
            Value::List(_) => "List",
            Value::Tuple(_) => "Tuple",
            Value::Map(_) => "Map",
            Value::Par(_) => "Par",
            Value::Nil => "Nil",
        }
    }

    /// Create a BigRat value, returning zero for 0r.
    pub fn new_bigrat(r: BigRational) -> Value {
        Value::BigRat(r)
    }

    /// Create a BigRat zero.
    pub fn bigrat_zero() -> Value {
        Value::BigRat(BigRational::zero())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(v) => write!(f, "{v}f64"),
            Value::BigInt(n) => write!(f, "{n}n"),
            Value::BigRat(r) => {
                if r.denom() == &BigInt::from(1) {
                    write!(f, "{}r", r.numer())
                } else {
                    write!(f, "{}r/{}r", r.numer(), r.denom())
                }
            }
            Value::FixedPoint { unscaled, scale } => {
                let scale = *scale as usize;
                if scale == 0 {
                    return write!(f, "{unscaled}p0");
                }
                let (sign, abs) = if *unscaled < BigInt::ZERO {
                    ("-", (-unscaled).to_string())
                } else {
                    ("", unscaled.to_string())
                };
                if abs.len() > scale {
                    let (integer, frac) = abs.split_at(abs.len() - scale);
                    write!(f, "{sign}{integer}.{frac}p{scale}")
                } else {
                    let padding = "0".repeat(scale - abs.len());
                    write!(f, "{sign}0.{padding}{abs}p{scale}")
                }
            }
            Value::Bool(b) => write!(f, "{b}"),
            Value::Str(s) => write!(f, "\"{s}\""),
            Value::Name(n) => write!(f, "@\"{n}\""),
            Value::List(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", inner.join(", "))
            }
            Value::Tuple(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                write!(f, "({})", inner.join(", "))
            }
            Value::Map(entries) => {
                let inner: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect();
                write!(f, "{{{}}}", inner.join(", "))
            }
            Value::Par(_) => write!(f, "<Par>"),
            Value::Nil => write!(f, "Nil"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use num_rational::BigRational;

    #[test]
    fn test_value_as_int() {
        assert_eq!(Value::Int(42).as_int(), Some(42));
        assert_eq!(Value::Bool(true).as_int(), None);
        assert_eq!(Value::Str("test".to_string()).as_int(), None);
        assert_eq!(Value::Nil.as_int(), None);
    }

    #[test]
    fn test_value_as_float() {
        assert_eq!(Value::Float(3.14).as_float(), Some(3.14));
        assert_eq!(Value::Int(1).as_float(), None);
    }

    #[test]
    fn test_value_as_bigint() {
        let val = Value::BigInt(BigInt::from(42));
        assert_eq!(val.as_bigint(), Some(&BigInt::from(42)));
        assert_eq!(Value::Int(42).as_bigint(), None);
    }

    #[test]
    fn test_value_as_bigrat() {
        let r = BigRational::new(BigInt::from(1), BigInt::from(3));
        let val = Value::BigRat(r.clone());
        assert_eq!(val.as_bigrat(), Some(&r));
        assert_eq!(Value::Int(1).as_bigrat(), None);
    }

    #[test]
    fn test_value_as_fixed_point() {
        let val = Value::FixedPoint {
            unscaled: BigInt::from(33),
            scale: 1,
        };
        assert_eq!(val.as_fixed_point(), Some((&BigInt::from(33), 1)));
        assert_eq!(Value::Int(1).as_fixed_point(), None);
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Int(1), Value::Int(1));
        assert_ne!(Value::Int(1), Value::Int(2));
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
        assert_eq!(Value::Str("a".into()), Value::Str("a".into()));
        assert_eq!(Value::Name("x".into()), Value::Name("x".into()));
        assert_eq!(Value::Nil, Value::Nil);
        assert_eq!(
            Value::List(vec![Value::Int(1)]),
            Value::List(vec![Value::Int(1)])
        );
        assert_eq!(
            Value::Tuple(vec![Value::Int(1)]),
            Value::Tuple(vec![Value::Int(1)])
        );
        assert_eq!(
            Value::Map(vec![(Value::Int(1), Value::Int(2))]),
            Value::Map(vec![(Value::Int(1), Value::Int(2))])
        );
    }

    #[test]
    fn test_float_equality() {
        assert_eq!(Value::Float(1.0), Value::Float(1.0));
        assert_ne!(Value::Float(1.0), Value::Float(2.0));
        // IEEE 754: positive and negative zero are equal
        assert_eq!(Value::Float(0.0), Value::Float(-0.0));
    }

    #[test]
    fn test_float_nan_equality() {
        // IEEE 754: NaN != NaN
        assert_ne!(Value::Float(f64::NAN), Value::Float(f64::NAN));
        assert_ne!(Value::Float(f64::NAN), Value::Float(0.0));
    }

    #[test]
    fn test_float_partial_ord() {
        assert!(Value::Float(1.0) < Value::Float(2.0));
        assert!(Value::Float(2.0) > Value::Float(1.0));
        // NaN comparisons return None (not less, not greater, not equal)
        assert_eq!(
            Value::Float(f64::NAN).partial_cmp(&Value::Float(1.0)),
            None
        );
        assert_eq!(
            Value::Float(f64::NAN).partial_cmp(&Value::Float(f64::NAN)),
            None
        );
    }

    #[test]
    fn test_bigint_equality() {
        assert_eq!(
            Value::BigInt(BigInt::from(100)),
            Value::BigInt(BigInt::from(100))
        );
        assert_ne!(
            Value::BigInt(BigInt::from(100)),
            Value::BigInt(BigInt::from(200))
        );
    }

    #[test]
    fn test_bigint_ordering() {
        assert!(Value::BigInt(BigInt::from(1)) < Value::BigInt(BigInt::from(2)));
        assert!(Value::BigInt(BigInt::from(-1)) < Value::BigInt(BigInt::from(0)));
    }

    #[test]
    fn test_bigrat_equality() {
        let half_a = BigRational::new(BigInt::from(1), BigInt::from(2));
        let half_b = BigRational::new(BigInt::from(2), BigInt::from(4)); // auto-normalized to 1/2
        assert_eq!(Value::BigRat(half_a), Value::BigRat(half_b));
    }

    #[test]
    fn test_bigrat_ordering() {
        let third = BigRational::new(BigInt::from(1), BigInt::from(3));
        let half = BigRational::new(BigInt::from(1), BigInt::from(2));
        assert!(Value::BigRat(third) < Value::BigRat(half));
    }

    #[test]
    fn test_fixed_point_equality() {
        let a = Value::FixedPoint {
            unscaled: BigInt::from(33),
            scale: 1,
        };
        let b = Value::FixedPoint {
            unscaled: BigInt::from(33),
            scale: 1,
        };
        assert_eq!(a, b);

        // Different scale = not equal (even if mathematically equivalent)
        let c = Value::FixedPoint {
            unscaled: BigInt::from(330),
            scale: 2,
        };
        assert_ne!(a, c);
    }

    #[test]
    fn test_fixed_point_ordering() {
        // Same scale: compare by unscaled
        let a = Value::FixedPoint {
            unscaled: BigInt::from(10),
            scale: 1,
        };
        let b = Value::FixedPoint {
            unscaled: BigInt::from(20),
            scale: 1,
        };
        assert!(a < b);

        // Different scale: not comparable
        let c = Value::FixedPoint {
            unscaled: BigInt::from(10),
            scale: 2,
        };
        assert_eq!(a.partial_cmp(&c), None);
    }

    #[test]
    fn test_cross_type_not_equal() {
        assert_ne!(Value::Int(1), Value::Float(1.0));
        assert_ne!(Value::Int(1), Value::BigInt(BigInt::from(1)));
        assert_ne!(Value::Float(1.0), Value::BigRat(BigRational::from(BigInt::from(1))));
    }

    #[test]
    fn test_cross_type_not_comparable() {
        assert_eq!(Value::Int(1).partial_cmp(&Value::Float(1.0)), None);
        assert_eq!(
            Value::Int(1).partial_cmp(&Value::BigInt(BigInt::from(1))),
            None
        );
    }

    #[test]
    fn test_type_name() {
        assert_eq!(Value::Int(1).type_name(), "Int");
        assert_eq!(Value::Float(1.0).type_name(), "Float");
        assert_eq!(Value::BigInt(BigInt::from(1)).type_name(), "BigInt");
        assert_eq!(Value::Nil.type_name(), "Nil");
    }

    #[test]
    fn test_value_clone() {
        let val = Value::List(vec![Value::Int(1), Value::Str("s".into())]);
        assert_eq!(val.clone(), val);
    }

    #[test]
    fn test_value_debug() {
        let val = Value::Int(1);
        assert!(!format!("{:?}", val).is_empty());
    }

    #[test]
    fn test_process_state_equality() {
        assert_eq!(ProcessState::Wait, ProcessState::Wait);
        assert_eq!(ProcessState::Ready, ProcessState::Ready);
        assert_eq!(
            ProcessState::Value(Value::Int(1)),
            ProcessState::Value(Value::Int(1))
        );
        assert_ne!(
            ProcessState::Value(Value::Int(1)),
            ProcessState::Value(Value::Int(2))
        );
        assert_eq!(
            ProcessState::Error("e".into()),
            ProcessState::Error("e".into())
        );
    }

    // =========================================================================
    // Display formatting
    // =========================================================================

    #[test]
    fn test_display_numeric_types() {
        assert_eq!(Value::Int(42).to_string(), "42");
        assert_eq!(Value::Int(-7).to_string(), "-7");
        assert_eq!(Value::Float(3.14).to_string(), "3.14f64");
        assert_eq!(Value::BigInt(BigInt::from(100)).to_string(), "100n");
        assert_eq!(Value::BigInt(BigInt::from(-42)).to_string(), "-42n");

        let whole = BigRational::new(BigInt::from(5), BigInt::from(1));
        assert_eq!(Value::BigRat(whole).to_string(), "5r");
        let frac = BigRational::new(BigInt::from(1), BigInt::from(3));
        assert_eq!(Value::BigRat(frac).to_string(), "1r/3r");
    }

    #[test]
    fn test_display_fixedpoint_edge_cases() {
        let fp = |u: i64, s: u32| Value::FixedPoint { unscaled: BigInt::from(u), scale: s };
        assert_eq!(fp(150, 2).to_string(), "1.50p2");
        assert_eq!(fp(42, 0).to_string(), "42p0");
        assert_eq!(fp(3, 2).to_string(), "0.03p2");       // small positive
        assert_eq!(fp(-150, 2).to_string(), "-1.50p2");    // negative
        assert_eq!(fp(-3, 2).to_string(), "-0.03p2");      // negative small (was buggy)
    }

    #[test]
    fn test_display_non_numeric_types() {
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Str("hello".into()).to_string(), "\"hello\"");
        assert_eq!(Value::Name("ch".into()).to_string(), "@\"ch\"");
        assert_eq!(Value::Nil.to_string(), "Nil");
        assert_eq!(Value::List(vec![Value::Int(1), Value::Int(2)]).to_string(), "[1, 2]");
        assert_eq!(Value::Tuple(vec![Value::Int(1), Value::Bool(true)]).to_string(), "(1, true)");
    }
}
