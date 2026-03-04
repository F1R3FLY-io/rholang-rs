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
