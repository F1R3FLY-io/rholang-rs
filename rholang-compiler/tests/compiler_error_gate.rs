use librho::sem::{pipeline::Pipeline, ErrorKind, ResolverPass, SemanticDb};
use rholang_parser::RholangParser;
use validated::Validated;

#[test]
fn test_is_recoverable_logic() {
    // Test recoverable errors
    // For now, just test the non-recoverable errors
    assert!(!ErrorKind::UnboundVariable.is_recoverable());
    assert!(!ErrorKind::ConnectiveOutsidePattern.is_recoverable());
    assert!(!ErrorKind::BadCode.is_recoverable());
}

#[test]
fn test_compile_error_can_be_created() {
    use librho::sem::{ErrorKind, SemanticDb};
    use rholang_compiler::{CompileError, CompileErrorInfo};
    use rholang_parser::RholangParser;
    use validated::Validated;

    // Get a valid PID from a semantic database
    let parser = RholangParser::new();
    let validated = parser.parse("Nil");
    let ast = match validated {
        Validated::Good(ast) => ast,
        _ => panic!("Parse failed"),
    };

    let mut db = SemanticDb::new();
    let pid = db.build_index(&ast[0]);

    let info = CompileErrorInfo {
        message: "test error".to_string(),
        position: None,
        span: None,
        kind: ErrorKind::UnboundVariable,
        pid,
    };

    let err = CompileError::SemanticErrors(vec![info]);
    assert_eq!(err.error_count(), 1);
}

#[test]
fn test_error_count_for_different_types() {
    use rholang_compiler::CompileError;

    assert_eq!(
        CompileError::ParseError("test".to_string()).error_count(),
        1
    );
    assert_eq!(
        CompileError::InternalError("test".to_string()).error_count(),
        1
    );
}

#[tokio::test]
async fn test_compile_checked_halts_on_errors() {
    let parser = RholangParser::new();
    let validated = parser.parse("new x in { undefined!(42) }");
    let ast = match validated {
        Validated::Good(ast) => ast,
        _ => panic!("Parse failed"),
    };

    let mut db = SemanticDb::new();
    let root = db.build_index(&ast[0]);

    // Run resolver to generate errors
    let pipeline = Pipeline::new().add_fact(ResolverPass::new(root));
    pipeline.run(&mut db).await;

    assert!(db.has_errors());

    let compiler = rholang_compiler::Compiler::new(&db);
    let result = compiler.compile_checked(&[&ast[0]]);

    assert!(result.is_err());
    match result.unwrap_err() {
        rholang_compiler::CompileError::SemanticErrors(errors) => {
            assert!(!errors.is_empty());
        }
        _ => panic!("Expected SemanticErrors"),
    }
}

#[tokio::test]
async fn test_compile_unchecked_ignores_errors() {
    let parser = RholangParser::new();
    let validated = parser.parse("new x in { x }");
    let ast = match validated {
        Validated::Good(ast) => ast,
        _ => panic!("Parse failed"),
    };

    let mut db = SemanticDb::new();
    let root = db.build_index(&ast[0]);

    let pipeline = Pipeline::new().add_fact(ResolverPass::new(root));
    pipeline.run(&mut db).await;

    let compiler = rholang_compiler::Compiler::new(&db);
    // Should succeed despite errors
    let result = compiler.compile_unchecked(&[&ast[0]]);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_recoverable_error_filtering() {
    let parser = RholangParser::new();
    let validated = parser.parse("new x in { x }");
    let ast = match validated {
        Validated::Good(ast) => ast,
        _ => panic!("Parse failed"),
    };

    let mut db = SemanticDb::new();
    let root = db.build_index(&ast[0]);

    let pipeline = Pipeline::new().add_fact(ResolverPass::new(root));
    pipeline.run(&mut db).await;

    // Default compiler filters recoverable errors
    let compiler = rholang_compiler::Compiler::new(&db);
    let result = compiler.compile_checked(&[&ast[0]]);
    assert!(result.is_ok()); // Recoverable error filtered

    // Strict compiler does not filter
    let strict_compiler = rholang_compiler::Compiler::strict(&db);
    let strict_result = strict_compiler.compile_checked(&[&ast[0]]);
    assert!(strict_result.is_err()); // Error not filtered
}
