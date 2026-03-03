use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{FromPrimitive, One, Signed, ToPrimitive, Zero};
use rholang_parser::{
    ast::{self, AnnProc, BinaryExpOp, UnaryExpOp},
    SourcePos,
};

#[derive(Debug, Clone)]
pub struct RuntimeEvalError {
    pub message: String,
    pub position: Option<SourcePos>,
}

impl RuntimeEvalError {
    fn at(position: SourcePos, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            position: Some(position),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumericValue {
    SignedInt {
        value: BigInt,
        bits: u32,
        explicit: bool,
    },
    UnsignedInt {
        value: BigInt,
        bits: u32,
    },
    BigInt(BigInt),
    BigRat(BigRational),
    Float {
        value: f64,
        bits: u16,
    },
    FixedPoint {
        unscaled: BigInt,
        scale: u32,
    },
}

impl std::fmt::Display for NumericValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumericValue::SignedInt {
                value,
                bits,
                explicit,
            } if *bits == 64 && !*explicit => write!(f, "{value}"),
            NumericValue::SignedInt { value, bits, .. } => write!(f, "{value}i{bits}"),
            NumericValue::UnsignedInt { value, bits } => write!(f, "{value}u{bits}"),
            NumericValue::BigInt(value) => write!(f, "{value}n"),
            NumericValue::BigRat(value) => {
                let n = value.numer();
                let d = value.denom();
                if d.is_one() {
                    write!(f, "{n}r")
                } else {
                    write!(f, "{n}r/{d}r")
                }
            }
            NumericValue::Float { value, bits } if value.is_nan() => write!(f, "NaNf{bits}"),
            NumericValue::Float { value, bits } if value.is_infinite() => {
                if value.is_sign_negative() {
                    write!(f, "-Inff{bits}")
                } else {
                    write!(f, "Inff{bits}")
                }
            }
            NumericValue::Float { value, bits } => write!(f, "{value}f{bits}"),
            NumericValue::FixedPoint { unscaled, scale } => {
                let rendered = format_fixed(unscaled, *scale);
                write!(f, "{rendered}p{scale}")
            }
        }
    }
}

pub fn try_eval_numeric(root: &AnnProc<'_>) -> Result<Option<NumericValue>, RuntimeEvalError> {
    eval_numeric(root)
}

pub fn validate_runtime_numeric_support<'a>(root: &'a AnnProc<'a>) -> Result<(), RuntimeEvalError> {
    for proc in root.iter_preorder_dfs() {
        if let ast::Proc::FloatLiteral { bits, .. } = proc.proc {
            if !is_supported_runtime_float_width(*bits) {
                return Err(RuntimeEvalError::at(
                    proc.span.start,
                    format!(
                        "float literal f{bits} is not supported by this runtime (supported: f32, f64)"
                    ),
                ));
            }
        }

        if let ast::Proc::Method {
            receiver,
            name,
            args,
        } = proc.proc
        {
            if name.name == "float" && args.len() == 1 {
                if let Some(width_arg) = eval_numeric(&args[0])? {
                    let width_args = [width_arg];
                    let bits = parse_width_arg(&width_args, "float", proc.span.start)?;
                    cast_float(
                        NumericValue::SignedInt {
                            value: BigInt::zero(),
                            bits: 64,
                            explicit: false,
                        },
                        bits,
                        proc.span.start,
                    )?;
                }
            }

            if is_cast_builtin(name.name) {
                let Some(receiver_value) = eval_numeric(receiver)? else {
                    continue;
                };

                let mut evaluated_args = Vec::with_capacity(args.len());
                let mut all_args_evaluable = true;
                for arg in args {
                    match eval_numeric(arg)? {
                        Some(value) => evaluated_args.push(value),
                        None => {
                            all_args_evaluable = false;
                            break;
                        }
                    }
                }

                if all_args_evaluable {
                    apply_cast_builtin(
                        name.name,
                        receiver_value,
                        &evaluated_args,
                        proc.span.start,
                    )?;
                }
            }
        }
    }
    Ok(())
}

/// Rewrites cast builtins from function-call form to method-call form:
/// `int(x, 8)` -> `(x).int(8)`
///
/// Returns `Some(rewritten)` only when at least one rewrite was applied.
pub fn rewrite_cast_builtin_calls(input: &str) -> Option<String> {
    let rewritten = rewrite_segment(input);
    if rewritten != input {
        Some(rewritten)
    } else {
        None
    }
}

fn eval_numeric(proc: &AnnProc<'_>) -> Result<Option<NumericValue>, RuntimeEvalError> {
    let pos = proc.span.start;

    match proc.proc {
        ast::Proc::LongLiteral(value) => Ok(Some(NumericValue::SignedInt {
            value: BigInt::from(*value),
            bits: 64,
            explicit: false,
        })),
        ast::Proc::SignedIntLiteral { value, bits } => {
            let parsed = parse_bigint(value, pos)?;
            Ok(Some(NumericValue::SignedInt {
                value: normalize_signed(parsed, *bits),
                bits: *bits,
                explicit: true,
            }))
        }
        ast::Proc::UnsignedIntLiteral { value, bits } => {
            let parsed = parse_bigint(value, pos)?;
            Ok(Some(NumericValue::UnsignedInt {
                value: normalize_unsigned(parsed, *bits),
                bits: *bits,
            }))
        }
        ast::Proc::BigIntLiteral(value) => {
            Ok(Some(NumericValue::BigInt(parse_bigint(value, pos)?)))
        }
        ast::Proc::BigRatLiteral(value) => {
            let n = parse_bigint(value, pos)?;
            Ok(Some(NumericValue::BigRat(BigRational::from_integer(n))))
        }
        ast::Proc::FloatLiteral { value, bits } => {
            if !is_supported_runtime_float_width(*bits) {
                return Err(RuntimeEvalError::at(
                    pos,
                    format!(
                        "float literal f{bits} is not supported by this runtime (supported: f32, f64)"
                    ),
                ));
            }
            let parsed = value.parse::<f64>().map_err(|_| {
                RuntimeEvalError::at(pos, format!("invalid float literal '{value}f{bits}'"))
            })?;
            Ok(Some(NumericValue::Float {
                value: parsed,
                bits: *bits,
            }))
        }
        ast::Proc::FixedPointLiteral { value, scale } => {
            let unscaled = parse_fixed_unscaled(value, *scale, pos)?;
            Ok(Some(NumericValue::FixedPoint {
                unscaled,
                scale: *scale,
            }))
        }
        ast::Proc::UnaryExp {
            op: UnaryExpOp::Neg,
            arg,
        } => {
            let Some(arg) = eval_numeric(arg)? else {
                return Ok(None);
            };
            Ok(Some(negate(arg)))
        }
        ast::Proc::BinaryExp { op, left, right } if is_arithmetic(*op) => {
            let Some(left) = eval_numeric(left)? else {
                return Ok(None);
            };
            let Some(right) = eval_numeric(right)? else {
                return Ok(None);
            };
            Ok(Some(apply_binary(*op, left, right, pos)?))
        }
        ast::Proc::Method {
            receiver,
            name,
            args,
        } if is_cast_builtin(name.name) => {
            let Some(receiver) = eval_numeric(receiver)? else {
                return Ok(None);
            };

            let mut evaluated_args = Vec::with_capacity(args.len());
            for arg in args {
                let Some(value) = eval_numeric(arg)? else {
                    return Ok(None);
                };
                evaluated_args.push(value);
            }

            Ok(Some(apply_cast_builtin(
                name.name,
                receiver,
                &evaluated_args,
                pos,
            )?))
        }
        _ => Ok(None),
    }
}

fn is_arithmetic(op: BinaryExpOp) -> bool {
    matches!(
        op,
        BinaryExpOp::Add
            | BinaryExpOp::Sub
            | BinaryExpOp::Mult
            | BinaryExpOp::Div
            | BinaryExpOp::Mod
    )
}

fn is_cast_builtin(name: &str) -> bool {
    matches!(
        name,
        "int" | "uint" | "bigint" | "bigrat" | "float" | "fixed"
    )
}

fn rewrite_segment(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;

    while i < bytes.len() {
        let ch = bytes[i] as char;
        if is_ident_start(ch) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_continue(bytes[i] as char) {
                i += 1;
            }
            let ident = &input[start..i];

            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }

            if is_cast_builtin(ident)
                && !is_method_invocation(input, start)
                && j < bytes.len()
                && bytes[j] == b'('
            {
                if let Some((close_idx, inner)) = extract_parenthesized(input, j) {
                    let args = split_top_level_args(inner);
                    if !args.is_empty() {
                        let mut rewritten_args = args
                            .into_iter()
                            .map(|arg| rewrite_segment(arg.trim()))
                            .collect::<Vec<_>>();
                        let receiver = rewritten_args.remove(0);
                        let method_args = rewritten_args.join(", ");
                        out.push('(');
                        out.push_str(&receiver);
                        out.push(')');
                        out.push('.');
                        out.push_str(ident);
                        out.push('(');
                        out.push_str(&method_args);
                        out.push(')');
                        i = close_idx + 1;
                        continue;
                    }
                }
            }

            // Not a cast builtin call: preserve original token and any whitespace already consumed.
            out.push_str(&input[start..j]);
            i = j;
            continue;
        }

        // Preserve quoted segments without scanning inside them.
        if ch == '"' || ch == '`' {
            if let Some(end) = scan_quoted(input, i, ch) {
                out.push_str(&input[i..end]);
                i = end;
                continue;
            }
        }

        out.push(ch);
        i += 1;
    }

    out
}

fn extract_parenthesized(input: &str, open_idx: usize) -> Option<(usize, &str)> {
    let bytes = input.as_bytes();
    if bytes.get(open_idx).copied()? != b'(' {
        return None;
    }

    let mut depth = 1usize;
    let mut i = open_idx + 1;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '"' || ch == '`' {
            i = scan_quoted(input, i, ch)?;
            continue;
        }

        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((i, &input[open_idx + 1..i]));
                }
            }
            _ => {}
        }
        i += 1;
    }

    None
}

fn split_top_level_args(input: &str) -> Vec<&str> {
    let bytes = input.as_bytes();
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '"' || ch == '`' {
            if let Some(end) = scan_quoted(input, i, ch) {
                i = end;
                continue;
            }
        }

        match bytes[i] {
            b'(' => paren += 1,
            b')' => paren = paren.saturating_sub(1),
            b'[' => bracket += 1,
            b']' => bracket = bracket.saturating_sub(1),
            b'{' => brace += 1,
            b'}' => brace = brace.saturating_sub(1),
            b',' if paren == 0 && bracket == 0 && brace == 0 => {
                args.push(&input[start..i]);
                start = i + 1;
            }
            _ => {}
        }

        i += 1;
    }

    if start <= input.len() {
        args.push(&input[start..]);
    }

    args
}

fn scan_quoted(input: &str, start: usize, quote: char) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut i = start + 1;

    while i < bytes.len() {
        if bytes[i] == b'\\' && quote == '"' {
            i = (i + 2).min(bytes.len());
            continue;
        }
        if bytes[i] as char == quote {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_method_invocation(input: &str, ident_start: usize) -> bool {
    let bytes = input.as_bytes();
    let mut i = ident_start;

    while i > 0 {
        i -= 1;
        let ch = bytes[i];
        if ch.is_ascii_whitespace() {
            continue;
        }
        return ch == b'.';
    }

    false
}

fn is_supported_runtime_float_width(bits: u16) -> bool {
    matches!(bits, 32 | 64)
}

fn parse_bigint(value: &str, pos: SourcePos) -> Result<BigInt, RuntimeEvalError> {
    BigInt::parse_bytes(value.as_bytes(), 10)
        .ok_or_else(|| RuntimeEvalError::at(pos, format!("invalid integer literal '{value}'")))
}

fn parse_fixed_unscaled(
    value: &str,
    scale: u32,
    pos: SourcePos,
) -> Result<BigInt, RuntimeEvalError> {
    let mut digits = value.trim();
    let negative = if let Some(rest) = digits.strip_prefix('-') {
        digits = rest;
        true
    } else {
        false
    };

    let (whole, frac) = digits.split_once('.').map_or((digits, ""), |parts| parts);
    let whole = if whole.is_empty() { "0" } else { whole };

    if !whole.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return Err(RuntimeEvalError::at(
            pos,
            format!("invalid fixed-point literal '{value}p{scale}'"),
        ));
    }

    if frac.len() > scale as usize {
        return Err(RuntimeEvalError::at(
            pos,
            format!("literal '{value}p{scale}' has more than {scale} fractional digits"),
        ));
    }

    let padded = format!(
        "{whole}{frac}{:0<width$}",
        "",
        width = scale as usize - frac.len()
    );
    let mut result = BigInt::parse_bytes(padded.as_bytes(), 10).ok_or_else(|| {
        RuntimeEvalError::at(
            pos,
            format!("invalid fixed-point literal '{value}p{scale}'"),
        )
    })?;
    if negative {
        result = -result;
    }
    Ok(result)
}

fn op_symbol(op: BinaryExpOp) -> &'static str {
    match op {
        BinaryExpOp::Add => "+",
        BinaryExpOp::Sub => "-",
        BinaryExpOp::Mult => "*",
        BinaryExpOp::Div => "/",
        BinaryExpOp::Mod => "%",
        _ => "?",
    }
}

fn type_label(value: &NumericValue) -> String {
    match value {
        NumericValue::SignedInt {
            bits: 64,
            explicit: false,
            ..
        } => "int64".to_string(),
        NumericValue::SignedInt { bits, .. } => format!("i{bits}"),
        NumericValue::UnsignedInt { bits, .. } => format!("u{bits}"),
        NumericValue::BigInt(_) => "bigint".to_string(),
        NumericValue::BigRat(_) => "bigrat".to_string(),
        NumericValue::Float { bits, .. } => format!("f{bits}"),
        NumericValue::FixedPoint { scale, .. } => format!("p{scale}"),
    }
}

fn same_numeric_type(lhs: &NumericValue, rhs: &NumericValue) -> bool {
    match (lhs, rhs) {
        (
            NumericValue::SignedInt { bits: l_bits, .. },
            NumericValue::SignedInt { bits: r_bits, .. },
        ) => l_bits == r_bits,
        (
            NumericValue::UnsignedInt { bits: l_bits, .. },
            NumericValue::UnsignedInt { bits: r_bits, .. },
        ) => l_bits == r_bits,
        (NumericValue::BigInt(_), NumericValue::BigInt(_))
        | (NumericValue::BigRat(_), NumericValue::BigRat(_)) => true,
        (NumericValue::Float { bits: l_bits, .. }, NumericValue::Float { bits: r_bits, .. }) => {
            l_bits == r_bits
        }
        (
            NumericValue::FixedPoint { scale: l_scale, .. },
            NumericValue::FixedPoint { scale: r_scale, .. },
        ) => l_scale == r_scale,
        _ => false,
    }
}

fn apply_binary(
    op: BinaryExpOp,
    lhs: NumericValue,
    rhs: NumericValue,
    pos: SourcePos,
) -> Result<NumericValue, RuntimeEvalError> {
    if !same_numeric_type(&lhs, &rhs) {
        return Err(RuntimeEvalError::at(
            pos,
            format!(
                "numeric type mismatch for '{}': {} vs {} (use explicit cast builtins)",
                op_symbol(op),
                type_label(&lhs),
                type_label(&rhs)
            ),
        ));
    }

    match (lhs, rhs) {
        (
            NumericValue::SignedInt {
                value: l,
                bits,
                explicit: l_explicit,
            },
            NumericValue::SignedInt {
                value: r,
                bits: _,
                explicit: r_explicit,
            },
        ) => {
            if (op == BinaryExpOp::Div || op == BinaryExpOp::Mod) && r.is_zero() {
                return Err(RuntimeEvalError::at(pos, "division by zero"));
            }
            let value = match op {
                BinaryExpOp::Add => normalize_signed(l + r, bits),
                BinaryExpOp::Sub => normalize_signed(l - r, bits),
                BinaryExpOp::Mult => normalize_signed(l * r, bits),
                BinaryExpOp::Div => normalize_signed(l / r, bits),
                BinaryExpOp::Mod => normalize_signed(l % r, bits),
                _ => unreachable!("non-arithmetic operator"),
            };
            Ok(NumericValue::SignedInt {
                value,
                bits,
                explicit: l_explicit || r_explicit,
            })
        }
        (
            NumericValue::UnsignedInt { value: l, bits },
            NumericValue::UnsignedInt { value: r, bits: _ },
        ) => {
            if (op == BinaryExpOp::Div || op == BinaryExpOp::Mod) && r.is_zero() {
                return Err(RuntimeEvalError::at(pos, "division by zero"));
            }
            let value = match op {
                BinaryExpOp::Add => normalize_unsigned(l + r, bits),
                BinaryExpOp::Sub => normalize_unsigned(l - r, bits),
                BinaryExpOp::Mult => normalize_unsigned(l * r, bits),
                BinaryExpOp::Div => normalize_unsigned(l / r, bits),
                BinaryExpOp::Mod => normalize_unsigned(l % r, bits),
                _ => unreachable!("non-arithmetic operator"),
            };
            Ok(NumericValue::UnsignedInt { value, bits })
        }
        (NumericValue::BigInt(l), NumericValue::BigInt(r)) => {
            if (op == BinaryExpOp::Div || op == BinaryExpOp::Mod) && r.is_zero() {
                return Err(RuntimeEvalError::at(pos, "division by zero"));
            }

            let value = match op {
                BinaryExpOp::Add => l + r,
                BinaryExpOp::Sub => l - r,
                BinaryExpOp::Mult => l * r,
                BinaryExpOp::Div => l / r,
                BinaryExpOp::Mod => l % r,
                _ => unreachable!("non-arithmetic operator"),
            };
            Ok(NumericValue::BigInt(value))
        }
        (NumericValue::BigRat(l), NumericValue::BigRat(r)) => {
            if op == BinaryExpOp::Div && r.is_zero() {
                return Err(RuntimeEvalError::at(pos, "division by zero"));
            }

            let value = match op {
                BinaryExpOp::Add => l + r,
                BinaryExpOp::Sub => l - r,
                BinaryExpOp::Mult => l * r,
                BinaryExpOp::Div => l / r,
                BinaryExpOp::Mod => BigRational::zero(),
                _ => unreachable!("non-arithmetic operator"),
            };
            Ok(NumericValue::BigRat(value))
        }
        (NumericValue::Float { value: l, bits }, NumericValue::Float { value: r, bits: _ }) => {
            let value = match op {
                BinaryExpOp::Add => l + r,
                BinaryExpOp::Sub => l - r,
                BinaryExpOp::Mult => l * r,
                BinaryExpOp::Div => l / r,
                BinaryExpOp::Mod => {
                    return Err(RuntimeEvalError::at(
                        pos,
                        "operator '%' is not defined for floating-point numbers",
                    ))
                }
                _ => unreachable!("non-arithmetic operator"),
            };

            Ok(NumericValue::Float { value, bits })
        }
        (
            NumericValue::FixedPoint { unscaled: l, scale },
            NumericValue::FixedPoint {
                unscaled: r,
                scale: _,
            },
        ) => {
            if (op == BinaryExpOp::Div || op == BinaryExpOp::Mod) && r.is_zero() {
                return Err(RuntimeEvalError::at(pos, "division by zero"));
            }
            let ten_pow = pow10(scale);
            let value = match op {
                BinaryExpOp::Add => l + r,
                BinaryExpOp::Sub => l - r,
                BinaryExpOp::Mult => (l * r) / ten_pow.clone(),
                BinaryExpOp::Div => (l * ten_pow.clone()) / r,
                BinaryExpOp::Mod => {
                    let q = (l.clone() * ten_pow.clone()) / r.clone();
                    let product = (q * r) / ten_pow;
                    l - product
                }
                _ => unreachable!("non-arithmetic operator"),
            };

            Ok(NumericValue::FixedPoint {
                unscaled: value,
                scale,
            })
        }
        _ => unreachable!("type compatibility should have handled this"),
    }
}

fn negate(value: NumericValue) -> NumericValue {
    match value {
        NumericValue::SignedInt {
            value,
            bits,
            explicit,
        } => NumericValue::SignedInt {
            value: normalize_signed(-value, bits),
            bits,
            explicit,
        },
        NumericValue::UnsignedInt { value, bits } => NumericValue::UnsignedInt {
            value: normalize_unsigned(-value, bits),
            bits,
        },
        NumericValue::BigInt(value) => NumericValue::BigInt(-value),
        NumericValue::BigRat(value) => NumericValue::BigRat(-value),
        NumericValue::Float { value, bits } => NumericValue::Float {
            value: -value,
            bits,
        },
        NumericValue::FixedPoint { unscaled, scale } => NumericValue::FixedPoint {
            unscaled: -unscaled,
            scale,
        },
    }
}

fn apply_cast_builtin(
    name: &str,
    receiver: NumericValue,
    args: &[NumericValue],
    pos: SourcePos,
) -> Result<NumericValue, RuntimeEvalError> {
    match name {
        "int" => {
            let bits = parse_width_arg(args, "int", pos)?;
            cast_int(receiver, bits, pos)
        }
        "uint" => {
            let bits = parse_width_arg(args, "uint", pos)?;
            cast_uint(receiver, bits, pos)
        }
        "bigint" => {
            ensure_arg_count(args, 0, "bigint", pos)?;
            cast_bigint(receiver, pos)
        }
        "bigrat" => {
            ensure_arg_count(args, 0, "bigrat", pos)?;
            cast_bigrat(receiver, pos)
        }
        "float" => {
            let bits = parse_width_arg(args, "float", pos)?;
            cast_float(receiver, bits, pos)
        }
        "fixed" => {
            let scale = parse_scale_arg(args, "fixed", pos)?;
            cast_fixed(receiver, scale, pos)
        }
        _ => Err(RuntimeEvalError::at(pos, "unknown cast builtin")),
    }
}

fn ensure_arg_count(
    args: &[NumericValue],
    expected: usize,
    name: &str,
    pos: SourcePos,
) -> Result<(), RuntimeEvalError> {
    if args.len() == expected {
        return Ok(());
    }

    Err(RuntimeEvalError::at(
        pos,
        format!(
            "builtin '{name}' expects {expected} argument(s), got {}",
            args.len()
        ),
    ))
}

fn parse_width_arg(
    args: &[NumericValue],
    name: &str,
    pos: SourcePos,
) -> Result<u32, RuntimeEvalError> {
    ensure_arg_count(args, 1, name, pos)?;
    let raw = to_i64_exact(&args[0], pos, "width")?;
    let bits = u32::try_from(raw).map_err(|_| {
        RuntimeEvalError::at(
            pos,
            format!("width for '{name}' must be a positive integer"),
        )
    })?;
    Ok(bits)
}

fn parse_scale_arg(
    args: &[NumericValue],
    name: &str,
    pos: SourcePos,
) -> Result<u32, RuntimeEvalError> {
    ensure_arg_count(args, 1, name, pos)?;
    let raw = to_i64_exact(&args[0], pos, "scale")?;
    let scale = u32::try_from(raw).map_err(|_| {
        RuntimeEvalError::at(
            pos,
            format!("scale for '{name}' must be a non-negative integer"),
        )
    })?;
    Ok(scale)
}

fn to_i64_exact(value: &NumericValue, pos: SourcePos, kind: &str) -> Result<i64, RuntimeEvalError> {
    let as_bigint = match value {
        NumericValue::SignedInt { value, .. } => value.clone(),
        NumericValue::UnsignedInt { value, .. } => value.clone(),
        NumericValue::BigInt(value) => value.clone(),
        NumericValue::BigRat(value) if value.denom().is_one() => value.numer().clone(),
        NumericValue::FixedPoint { unscaled, scale } if *scale == 0 => unscaled.clone(),
        NumericValue::Float { value, .. }
            if value.is_finite() && value.fract() == 0.0 && value.abs() <= i64::MAX as f64 =>
        {
            BigInt::from(*value as i64)
        }
        _ => {
            return Err(RuntimeEvalError::at(
                pos,
                format!("{kind} argument must be an integer value"),
            ));
        }
    };

    as_bigint.to_i64().ok_or_else(|| {
        RuntimeEvalError::at(
            pos,
            format!("{kind} argument is out of signed 64-bit range"),
        )
    })
}

fn cast_int(
    value: NumericValue,
    bits: u32,
    pos: SourcePos,
) -> Result<NumericValue, RuntimeEvalError> {
    if bits < 8 || !bits.is_power_of_two() {
        return Err(RuntimeEvalError::at(
            pos,
            "int(width): width must be a power of two and at least 8",
        ));
    }

    if let Some(integer) = integer_value(&value) {
        return Ok(NumericValue::SignedInt {
            value: normalize_signed(integer, bits),
            bits,
            explicit: true,
        });
    }

    let floor = floor_to_bigint(&value, pos)?;
    ensure_in_signed_range(&floor, bits, pos)?;
    Ok(NumericValue::SignedInt {
        value: floor,
        bits,
        explicit: true,
    })
}

fn cast_uint(
    value: NumericValue,
    bits: u32,
    pos: SourcePos,
) -> Result<NumericValue, RuntimeEvalError> {
    if bits < 8 || !bits.is_power_of_two() {
        return Err(RuntimeEvalError::at(
            pos,
            "uint(width): width must be a power of two and at least 8",
        ));
    }

    if let Some(integer) = integer_value(&value) {
        return Ok(NumericValue::UnsignedInt {
            value: normalize_unsigned(integer, bits),
            bits,
        });
    }

    let floor = floor_to_bigint(&value, pos)?;
    let bounded = if floor.is_negative() {
        BigInt::zero()
    } else {
        floor
    };
    ensure_in_unsigned_range(&bounded, bits, pos)?;
    Ok(NumericValue::UnsignedInt {
        value: bounded,
        bits,
    })
}

fn cast_bigint(value: NumericValue, pos: SourcePos) -> Result<NumericValue, RuntimeEvalError> {
    Ok(NumericValue::BigInt(floor_to_bigint(&value, pos)?))
}

fn cast_bigrat(value: NumericValue, pos: SourcePos) -> Result<NumericValue, RuntimeEvalError> {
    let rational = match value {
        NumericValue::Float { value, .. } => {
            if !value.is_finite() {
                return Err(RuntimeEvalError::at(pos, "cannot cast NaN/Inf to bigrat"));
            }
            BigRational::from_float(value)
                .ok_or_else(|| RuntimeEvalError::at(pos, "failed to convert float to bigrat"))?
        }
        _ => to_rational_exact(&value),
    };
    Ok(NumericValue::BigRat(rational))
}

fn cast_float(
    value: NumericValue,
    bits: u32,
    pos: SourcePos,
) -> Result<NumericValue, RuntimeEvalError> {
    if !matches!(bits, 32 | 64 | 80 | 128 | 256) {
        return Err(RuntimeEvalError::at(
            pos,
            "float(width): width must be one of 32, 64, 80, 128, 256",
        ));
    }
    if !is_supported_runtime_float_width(bits as u16) {
        return Err(RuntimeEvalError::at(
            pos,
            format!("float(width): f{bits} is not supported by this runtime (supported: f32, f64)"),
        ));
    }

    let mut rendered = to_f64_with_infinity(&value);
    if bits == 32 {
        rendered = (rendered as f32) as f64;
    }

    Ok(NumericValue::Float {
        value: rendered,
        bits: bits as u16,
    })
}

fn cast_fixed(
    value: NumericValue,
    scale: u32,
    pos: SourcePos,
) -> Result<NumericValue, RuntimeEvalError> {
    let rational = to_rational_for_fixed(&value, pos)?;
    let scaled = rational * BigRational::from_integer(pow10(scale));
    let unscaled = floor_rational(&scaled);

    Ok(NumericValue::FixedPoint { unscaled, scale })
}

fn integer_value(value: &NumericValue) -> Option<BigInt> {
    match value {
        NumericValue::SignedInt { value, .. }
        | NumericValue::UnsignedInt { value, .. }
        | NumericValue::BigInt(value) => Some(value.clone()),
        _ => None,
    }
}

fn floor_to_bigint(value: &NumericValue, pos: SourcePos) -> Result<BigInt, RuntimeEvalError> {
    match value {
        NumericValue::SignedInt { value, .. }
        | NumericValue::UnsignedInt { value, .. }
        | NumericValue::BigInt(value) => Ok(value.clone()),
        NumericValue::BigRat(value) => Ok(floor_rational(value)),
        NumericValue::Float { value, .. } => {
            if !value.is_finite() {
                return Err(RuntimeEvalError::at(
                    pos,
                    "cannot cast NaN/Inf to an integer type",
                ));
            }
            BigInt::from_f64(value.floor())
                .ok_or_else(|| RuntimeEvalError::at(pos, "float value cannot be represented"))
        }
        NumericValue::FixedPoint { unscaled, scale } => Ok(floor_fixed(unscaled, *scale)),
    }
}

fn floor_fixed(unscaled: &BigInt, scale: u32) -> BigInt {
    if scale == 0 {
        return unscaled.clone();
    }
    let denom = pow10(scale);
    floor_div(unscaled, &denom)
}

fn to_rational_exact(value: &NumericValue) -> BigRational {
    match value {
        NumericValue::SignedInt { value, .. }
        | NumericValue::UnsignedInt { value, .. }
        | NumericValue::BigInt(value) => BigRational::from_integer(value.clone()),
        NumericValue::BigRat(value) => value.clone(),
        NumericValue::FixedPoint { unscaled, scale } => {
            BigRational::new(unscaled.clone(), pow10(*scale))
        }
        NumericValue::Float { value, .. } => {
            BigRational::from_float(*value).unwrap_or_else(BigRational::zero)
        }
    }
}

fn to_rational_for_fixed(
    value: &NumericValue,
    pos: SourcePos,
) -> Result<BigRational, RuntimeEvalError> {
    match value {
        NumericValue::Float { value, .. } => {
            if !value.is_finite() {
                return Err(RuntimeEvalError::at(
                    pos,
                    "cannot cast NaN/Inf to fixed-point",
                ));
            }
            BigRational::from_float(*value)
                .ok_or_else(|| RuntimeEvalError::at(pos, "failed to convert float to fixed-point"))
        }
        _ => Ok(to_rational_exact(value)),
    }
}

fn to_f64_with_infinity(value: &NumericValue) -> f64 {
    match value {
        NumericValue::Float { value, .. } => *value,
        NumericValue::SignedInt { value, .. }
        | NumericValue::UnsignedInt { value, .. }
        | NumericValue::BigInt(value) => value.to_f64().unwrap_or_else(|| {
            if value.is_negative() {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }
        }),
        NumericValue::BigRat(value) => value.to_f64().unwrap_or_else(|| {
            if value.numer().is_negative() {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }
        }),
        NumericValue::FixedPoint { unscaled, scale } => {
            BigRational::new(unscaled.clone(), pow10(*scale))
                .to_f64()
                .unwrap_or_else(|| {
                    if unscaled.is_negative() {
                        f64::NEG_INFINITY
                    } else {
                        f64::INFINITY
                    }
                })
        }
    }
}

fn ensure_in_signed_range(
    value: &BigInt,
    bits: u32,
    pos: SourcePos,
) -> Result<(), RuntimeEvalError> {
    let min = -(BigInt::one() << (bits - 1));
    let max = (BigInt::one() << (bits - 1)) - BigInt::one();
    if *value < min || *value > max {
        Err(RuntimeEvalError::at(
            pos,
            format!("overflow converting value to i{bits}"),
        ))
    } else {
        Ok(())
    }
}

fn ensure_in_unsigned_range(
    value: &BigInt,
    bits: u32,
    pos: SourcePos,
) -> Result<(), RuntimeEvalError> {
    let max = (BigInt::one() << bits) - BigInt::one();
    if value.is_negative() || *value > max {
        Err(RuntimeEvalError::at(
            pos,
            format!("overflow converting value to u{bits}"),
        ))
    } else {
        Ok(())
    }
}

fn normalize_signed(value: BigInt, bits: u32) -> BigInt {
    let modulus = BigInt::one() << bits;
    let mut reduced = value % &modulus;
    if reduced.is_negative() {
        reduced += &modulus;
    }

    let half = BigInt::one() << (bits - 1);
    if reduced >= half {
        reduced - modulus
    } else {
        reduced
    }
}

fn normalize_unsigned(value: BigInt, bits: u32) -> BigInt {
    let modulus = BigInt::one() << bits;
    let mut reduced = value % &modulus;
    if reduced.is_negative() {
        reduced += modulus;
    }
    reduced
}

fn pow10(scale: u32) -> BigInt {
    BigInt::from(10u8).pow(scale)
}

fn floor_rational(value: &BigRational) -> BigInt {
    floor_div(value.numer(), value.denom())
}

fn floor_div(numer: &BigInt, denom: &BigInt) -> BigInt {
    let quotient = numer / denom;
    let remainder = numer % denom;
    if remainder.is_zero() || numer.is_positive() {
        quotient
    } else {
        quotient - BigInt::one()
    }
}

fn format_fixed(unscaled: &BigInt, scale: u32) -> String {
    if scale == 0 {
        return unscaled.to_string();
    }

    let negative = unscaled.is_negative();
    let abs = unscaled.abs();
    let mut digits = abs.to_string();
    let scale = scale as usize;

    if digits.len() <= scale {
        let zeros = "0".repeat(scale + 1 - digits.len());
        digits = format!("{zeros}{digits}");
    }

    let split = digits.len() - scale;
    let whole = &digits[..split];
    let frac = &digits[split..];

    if negative {
        format!("-{whole}.{frac}")
    } else {
        format!("{whole}.{frac}")
    }
}

#[cfg(test)]
mod tests {
    use super::rewrite_cast_builtin_calls;

    #[test]
    fn rewrites_simple_builtin_calls() {
        let rewritten = rewrite_cast_builtin_calls("int(3.5f32, 8)").expect("rewritten");
        assert_eq!(rewritten, "(3.5f32).int(8)");
    }

    #[test]
    fn rewrites_nested_builtin_calls() {
        let rewritten = rewrite_cast_builtin_calls("int(float(1u8, 64), 8)").expect("rewritten");
        assert_eq!(rewritten, "((1u8).float(64)).int(8)");
    }

    #[test]
    fn rewrites_bigrat_and_fixed() {
        let rewritten = rewrite_cast_builtin_calls("fixed(bigrat(3), 2)").expect("rewritten");
        assert_eq!(rewritten, "((3).bigrat()).fixed(2)");
    }

    #[test]
    fn does_not_rewrite_when_no_builtin_call() {
        assert!(rewrite_cast_builtin_calls("1 + 2 * 3").is_none());
    }

    #[test]
    fn does_not_rewrite_method_style_builtin_call() {
        assert!(rewrite_cast_builtin_calls("(3.5f32).int(8)").is_none());
    }

    #[test]
    fn rewrites_outer_builtin_without_corrupting_nested_method_call() {
        let rewritten = rewrite_cast_builtin_calls("int((3.5f32).int(8), 16)").expect("rewritten");
        assert_eq!(rewritten, "((3.5f32).int(8)).int(16)");
    }
}
