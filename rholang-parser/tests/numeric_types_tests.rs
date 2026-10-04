use rholang_parser::{RholangParser, ast::Proc, parser::errors::ParsingError};
use validated::Validated;

#[test]
fn parses_typed_numeric_literals() {
    let parser = RholangParser::new();
    let input = include_str!("corpus/numeric_types.rho");
    let parsed = parser.parse(input);

    let procs = match parsed {
        Validated::Good(procs) => procs,
        Validated::Fail(err) => panic!("expected successful parse, got errors: {err:?}"),
    };

    assert_eq!(
        procs.len(),
        10,
        "numeric_types.rho should parse to 10 top-level terms"
    );
    assert!(procs.iter().any(|p| {
        matches!(
            p.proc,
            Proc::SignedIntLiteral {
                value: "-52",
                bits: 64
            }
        )
    }));
    assert!(procs.iter().any(|p| {
        matches!(
            p.proc,
            Proc::UnsignedIntLiteral {
                value: "65535",
                bits: 16
            }
        )
    }));
    assert!(
        procs
            .iter()
            .any(|p| matches!(p.proc, Proc::BigIntLiteral("10")))
    );
    assert!(
        procs
            .iter()
            .any(|p| matches!(p.proc, Proc::BigRatLiteral("3")))
    );
    assert!(procs.iter().any(|p| {
        matches!(
            p.proc,
            Proc::FloatLiteral {
                value: "-1.234e5",
                bits: 32
            }
        )
    }));
    assert!(procs.iter().any(|p| {
        matches!(
            p.proc,
            Proc::FixedPointLiteral {
                value: "3.3",
                scale: 1
            }
        )
    }));
}

#[test]
fn rejects_non_power_of_two_sized_int_literals() {
    let parser = RholangParser::new();
    let parsed = parser.parse("1i7\n1u12");

    let failures = match parsed {
        Validated::Good(_) => panic!("expected parse failure"),
        Validated::Fail(errs) => errs,
    };

    assert!(
        failures
            .iter()
            .flat_map(|failure| failure.errors.iter())
            .next()
            .is_some()
    );
    assert!(
        failures
            .iter()
            .flat_map(|failure| failure.errors.iter())
            .all(|error| matches!(error.error, ParsingError::NumberOutOfRange))
    );
}

/// The send inputs of a source with one send, which must parse.
fn send_inputs<'a>(parser: &'a RholangParser<'a>, source: &'a str) -> Vec<&'a Proc<'a>> {
    let procs = match parser.parse(source) {
        Validated::Good(procs) => procs,
        Validated::Fail(err) => panic!("{source} must parse: {err:?}"),
    };
    match procs.as_slice() {
        [proc] => match proc.proc {
            Proc::Send { inputs, .. } => inputs.iter().map(|input| input.proc).collect(),
            other => panic!("{source}: expected a send, got {other:?}"),
        },
        other => panic!("{source}: expected one process, got {other:?}"),
    }
}

#[test]
fn bare_float_literals_lex_as_f64() {
    let parser = RholangParser::new();
    let expected = [
        "1.5", "0.25", ".5", "1.5e3", "2.5E-3", "1e3", "2E-4", "-7e+2", "-2.5", "-.5",
    ];
    let inputs = send_inputs(
        &parser,
        "x!(1.5, 0.25, .5, 1.5e3, 2.5E-3, 1e3, 2E-4, -7e+2, -2.5, -.5)",
    );
    assert_eq!(inputs.len(), expected.len());
    for (input, text) in inputs.iter().zip(expected) {
        assert!(
            matches!(input, Proc::FloatLiteral { value, bits: 64 } if *value == text),
            "{text}: {input:?}"
        );
    }
}

#[test]
fn trailing_dot_and_suffixed_floats_unchanged() {
    let parser = RholangParser::new();
    // A suffix still gives the width, and the value excludes it.
    let inputs = send_inputs(&parser, "x!(1.5f32, 2.f64, 1e3f128, .5f256, 2.5f64)");
    let expected = [
        ("1.5", 32),
        ("2.", 64),
        ("1e3", 128),
        (".5", 256),
        ("2.5", 64),
    ];
    for (input, (text, width)) in inputs.iter().zip(expected) {
        assert!(
            matches!(input, Proc::FloatLiteral { value, bits } if *value == text && *bits == width),
            "{text}f{width}: {input:?}"
        );
    }
    // A bare float needs a digit after its dot: `5.method()` stays a method
    // call on an integer, and `5.` alone is not a float, as at 02cef80.
    let inputs = send_inputs(&parser, "x!(5.toString())");
    assert!(matches!(
        inputs.as_slice(),
        [Proc::Method { receiver, name, .. }] if matches!(receiver.proc, Proc::LongLiteral(5)) && name.name == "toString"
    ));
    assert!(matches!(parser.parse("x!(5.)"), Validated::Fail(_)));
    // An exponent followed by an identifier character is not a bare float:
    // 02cef80 lexes `1e5x` as `1` and `e5x`.
    assert!(matches!(parser.parse("x!(1e5x)"), Validated::Fail(_)));
}
