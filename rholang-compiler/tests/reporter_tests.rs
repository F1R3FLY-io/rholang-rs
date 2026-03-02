use librho::sem::{ErrorKind, SemanticDb};
use rholang_compiler::{CompileError, CompileErrorInfo};
use rholang_compiler::{ErrorReporter, ReporterConfig};
use rholang_parser::{RholangParser, SourcePos};
use validated::Validated;

#[test]
fn test_error_formatting_with_position() {
    let reporter = ErrorReporter::new(ReporterConfig { context_lines: 1 });

    // Get a valid PID from a semantic database
    let parser = RholangParser::new();
    let validated = parser.parse("Nil");
    let ast = match validated {
        Validated::Good(ast) => ast,
        _ => panic!("Parse failed"),
    };

    let mut db = SemanticDb::new();
    let pid = db.build_index(&ast[0]);

    let error = CompileError::SemanticErrors(vec![CompileErrorInfo {
        message: "Use of undeclared variable".to_string(),
        position: Some(SourcePos { line: 1, col: 4 }),
        span: None,
        kind: ErrorKind::UnboundVariable,
        pid,
    }]);

    let source = "new x in {\n    undefined!(42)\n}";
    let output = reporter.format_error(&error, source, Some("test.rho"));

    assert!(output.contains("error: Use of undeclared variable"));
    assert!(output.contains("test.rho:2:5"));
    assert!(output.contains("undefined!(42)"));
    assert!(output.contains("^"));
}

#[test]
fn test_multiple_errors_formatting() {
    let reporter = ErrorReporter::default();

    // Get valid PIDs from a semantic database
    let parser = RholangParser::new();
    let validated = parser.parse("Nil");
    let ast = match validated {
        Validated::Good(ast) => ast,
        _ => panic!("Parse failed"),
    };

    let mut db = SemanticDb::new();
    let pid0 = db.build_index(&ast[0]);
    let pid1 = db.build_index(&ast[0]);

    let error = CompileError::SemanticErrors(vec![
        CompileErrorInfo {
            message: "Error 1".to_string(),
            position: Some(SourcePos { line: 0, col: 0 }),
            span: None,
            kind: ErrorKind::UnboundVariable,
            pid: pid0,
        },
        CompileErrorInfo {
            message: "Error 2".to_string(),
            position: Some(SourcePos { line: 1, col: 0 }),
            span: None,
            kind: ErrorKind::UnboundVariable,
            pid: pid1,
        },
    ]);

    let source = "a!(1)\nb!(2)";
    let output = reporter.format_error(&error, source, None);

    assert!(output.contains("Error 1"));
    assert!(output.contains("Error 2"));
    assert!(output.contains("2 errors emitted"));
}

#[test]
fn test_utf8_multibyte_column_handling() {
    let reporter = ErrorReporter::default();

    // Get a valid PID from a semantic database
    let parser = RholangParser::new();
    let validated = parser.parse("Nil");
    let ast = match validated {
        Validated::Good(ast) => ast,
        _ => panic!("Parse failed"),
    };

    let mut db = SemanticDb::new();
    let pid = db.build_index(&ast[0]);

    let error = CompileError::SemanticErrors(vec![CompileErrorInfo {
        message: "Error after emoji".to_string(),
        position: Some(SourcePos { line: 0, col: 6 }), // Byte offset after "🔥"
        span: None,
        kind: ErrorKind::UnboundVariable,
        pid,
    }]);

    let source = "🔥x!(1)"; // Emoji is 4 bytes, but 1 character
    let output = reporter.format_error(&error, source, None);

    // Caret should align with 'x', not be offset by byte count
    assert!(output.contains("🔥x!(1)"));
    assert!(output.contains(" ^")); // One space for emoji, then caret
}
