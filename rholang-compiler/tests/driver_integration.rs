use rholang_compiler::{CompileDriver, CompileError};

#[tokio::test]
async fn test_unbound_variable_error() {
    let driver = CompileDriver::default();
    let result = driver.compile_async("undefined_var!(42)").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CompileError::SemanticErrors(errors) => {
            assert_eq!(errors.len(), 1);
            assert!(errors[0].message.contains("undeclared"));
        }
        _ => panic!("Expected SemanticErrors"),
    }
}

#[tokio::test]
async fn test_valid_code_compiles() {
    let driver = CompileDriver::default();
    let result = driver.compile_async("new x in { x!(42) }").await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.processes.len(), 1);
}

#[tokio::test]
async fn test_duplicate_variable_error() {
    let driver = CompileDriver::default();
    let result = driver.compile_async("new x, x in { Nil }").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CompileError::ParseError(msg) => {
            assert!(msg.contains("Duplicate"));
        }
        _ => panic!("Expected ParseError"),
    }
}

#[tokio::test]
async fn test_empty_input_is_error() {
    let driver = CompileDriver::default();
    let result = driver.compile_async("").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CompileError::ParseError(msg) => {
            assert!(msg.contains("Empty"));
        }
        _ => panic!("Expected ParseError"),
    }
}

#[tokio::test]
async fn test_multiple_errors() {
    let driver = CompileDriver::default();
    let result = driver.compile_async("a!(1) | b!(2) | c!(3)").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CompileError::SemanticErrors(errors) => {
            // All three variables are undefined
            assert_eq!(errors.len(), 3);
        }
        _ => panic!("Expected SemanticErrors"),
    }
}
