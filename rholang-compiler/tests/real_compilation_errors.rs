use rholang_compiler::{CompileDriver, CompileError, ErrorReporter};

#[tokio::test]
async fn test_real_unbound_variable_with_context() {
    let driver = CompileDriver::default();
    let reporter = ErrorReporter::default();

    let source = r#"
new x in {
    x!(42) |
    undefined_channel!(100)
}
"#;

    let result = driver
        .compile_async_with_filename(source, Some("test.rho"))
        .await;

    assert!(result.is_err());
    if let Err(e) = result {
        let formatted = reporter.format_error(&e, source, Some("test.rho"));

        // Should show the error line with context
        assert!(formatted.contains("undefined_channel"));
        assert!(formatted.contains("test.rho"));
        assert!(formatted.contains("^"));

        // Print for visual inspection (will be hidden unless test fails)
        println!("Formatted error:\n{}", formatted);
    }
}

#[tokio::test]
async fn test_real_duplicate_definition() {
    let driver = CompileDriver::default();
    let reporter = ErrorReporter::default();

    let source = "new x, x in { Nil }";

    let result = driver.compile_async(source).await;

    assert!(result.is_err());
    if let Err(e) = result {
        let formatted = reporter.format_error(&e, source, None);
        assert!(formatted.contains("Duplicate"));
        println!("Formatted error:\n{}", formatted);
    }
}

#[tokio::test]
async fn test_real_parse_error() {
    let driver = CompileDriver::default();

    let source = "new x in { {{{ }"; // Unbalanced braces

    let result = driver.compile_async(source).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CompileError::ParseError(_) => { /* expected */ }
        other => panic!("Expected ParseError, got {:?}", other),
    }
}
