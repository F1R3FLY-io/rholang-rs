use rholang_parser::{RholangParser, ast::Proc, parser::errors::ParsingError};
use validated::Validated;

#[test]
fn parses_typed_numeric_literals() {
    let parser = RholangParser::new();
    let input = "-52i64\n65535u16\n10n\n3r\n-1.234e5f32\n3.3p1";
    let parsed = parser.parse(input);

    let procs = match parsed {
        Validated::Good(procs) => procs,
        Validated::Fail(err) => panic!("expected successful parse, got errors: {err:?}"),
    };

    assert!(matches!(
        procs[0].proc,
        Proc::SignedIntLiteral {
            value: "-52",
            bits: 64
        }
    ));
    assert!(matches!(
        procs[1].proc,
        Proc::UnsignedIntLiteral {
            value: "65535",
            bits: 16
        }
    ));
    assert!(matches!(procs[2].proc, Proc::BigIntLiteral("10")));
    assert!(matches!(procs[3].proc, Proc::BigRatLiteral("3")));
    assert!(matches!(
        procs[4].proc,
        Proc::FloatLiteral {
            value: "-1.234e5",
            bits: 32
        }
    ));
    assert!(matches!(
        procs[5].proc,
        Proc::FixedPointLiteral {
            value: "3.3",
            scale: 1
        }
    ));
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
