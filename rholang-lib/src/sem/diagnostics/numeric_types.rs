use rholang_parser::ast::{self, BinaryExpOp, UnaryExpOp};

use crate::sem::{
    Diagnostic, DiagnosticPass, ErrorKind, NumericType, Pass, ProcRef, SemanticDb,
    diagnostics::NumericTypeConsistencyCheck,
};
use std::borrow::Cow;

impl Pass for NumericTypeConsistencyCheck {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Numeric Type Consistency Check")
    }
}

impl DiagnosticPass for NumericTypeConsistencyCheck {
    fn run(&self, db: &SemanticDb) -> Vec<Diagnostic> {
        let mut result = Vec::new();

        for (pid, proc) in db {
            let ast::Proc::BinaryExp { op, left, right } = proc.proc else {
                continue;
            };
            if !is_arithmetic_operator(*op) {
                continue;
            }

            let left_type = infer_numeric_type(left);
            let right_type = infer_numeric_type(right);
            let (Some(left_type), Some(right_type)) = (left_type, right_type) else {
                continue;
            };

            if left_type != right_type {
                result.push(Diagnostic::error(
                    pid,
                    ErrorKind::MixedNumericTypes {
                        op: *op,
                        left: left_type,
                        right: right_type,
                    },
                    Some(proc.span.start),
                ));
                continue;
            }

            if *op == BinaryExpOp::Mod && is_float_type(left_type) {
                result.push(Diagnostic::error(
                    pid,
                    ErrorKind::UnsupportedNumericOperator {
                        op: *op,
                        arg: left_type,
                    },
                    Some(proc.span.start),
                ));
            }
        }

        result
    }
}

fn infer_numeric_type(proc: ProcRef<'_>) -> Option<NumericType> {
    match proc.proc {
        ast::Proc::UnaryExp {
            op: UnaryExpOp::Neg,
            arg,
        } => infer_numeric_type(arg),
        ast::Proc::BinaryExp { op, left, right } if is_arithmetic_operator(*op) => {
            let left_type = infer_numeric_type(left);
            let right_type = infer_numeric_type(right);
            match (left_type, right_type) {
                (Some(left_type), Some(right_type)) if left_type == right_type => Some(left_type),
                _ => None,
            }
        }
        ast::Proc::LongLiteral(_) => Some(NumericType::Int64),
        ast::Proc::SignedIntLiteral { bits, .. } => Some(NumericType::SignedInt { bits: *bits }),
        ast::Proc::UnsignedIntLiteral { bits, .. } => {
            Some(NumericType::UnsignedInt { bits: *bits })
        }
        ast::Proc::BigIntLiteral(_) => Some(NumericType::BigInt),
        ast::Proc::BigRatLiteral(_) => Some(NumericType::BigRat),
        ast::Proc::FloatLiteral { bits, .. } => Some(NumericType::Float { bits: *bits }),
        ast::Proc::FixedPointLiteral { scale, .. } => {
            Some(NumericType::FixedPoint { scale: *scale })
        }
        _ => None,
    }
}

fn is_arithmetic_operator(op: BinaryExpOp) -> bool {
    matches!(
        op,
        BinaryExpOp::Add
            | BinaryExpOp::Sub
            | BinaryExpOp::Mult
            | BinaryExpOp::Div
            | BinaryExpOp::Mod
    )
}

fn is_float_type(numeric_type: NumericType) -> bool {
    matches!(numeric_type, NumericType::Float { .. })
}
