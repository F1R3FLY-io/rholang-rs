use rholang_fake::{ErrorKind, InterpretationResult, InterpreterError, SourcePosition};

#[test]
fn test_source_position_display() {
    let position = SourcePosition {
        line: 10,
        column: 20,
    };
    assert_eq!(position.to_string(), "line 10, column 20");
}

#[test]
fn test_interpreter_error_parsing_error() {
    let error = InterpreterError::parsing_error(
        "Invalid syntax",
        Some(SourcePosition {
            line: 5,
            column: 10,
        }),
        Some("for (x <- y".to_string()),
    );

    assert_eq!(error.kind, ErrorKind::ParsingError);
    assert_eq!(error.message, "Invalid syntax");
    assert!(error.position.is_some());

    // Use clone to avoid moving the value
    if let Some(pos) = error.position.as_ref() {
        assert_eq!(pos.line, 5);
        assert_eq!(pos.column, 10);
    } else {
        panic!("Expected position to be Some");
    }

    assert!(error.source.is_some());
    assert_eq!(error.source.clone().unwrap(), "for (x <- y");

    // Test display formatting
    let error_str = error.to_string();
    assert!(error_str.contains("Parsing error: Invalid syntax"));
    assert!(error_str.contains("line 5, column 10"));
    assert!(error_str.contains("Source: for (x <- y"));
}

#[test]
fn test_interpreter_error_runtime_error() {
    let error =
        InterpreterError::runtime_error("Division by zero", None, Some("5 / 0".to_string()));

    assert_eq!(error.kind, ErrorKind::RuntimeError);
    assert_eq!(error.message, "Division by zero");
    assert!(error.position.is_none());
    assert!(error.source.is_some());
    assert_eq!(error.source.clone().unwrap(), "5 / 0");

    // Test display formatting
    let error_str = error.to_string();
    assert!(error_str.contains("Runtime error: Division by zero"));
    assert!(error_str.contains("Source: 5 / 0"));
}

#[test]
fn test_interpreter_error_timeout_error() {
    let error = InterpreterError::timeout_error("Operation timed out after 30 seconds");

    assert_eq!(error.kind, ErrorKind::TimeoutError);
    assert_eq!(error.message, "Operation timed out after 30 seconds");
    assert!(error.position.is_none());
    assert!(error.source.is_none());

    // Test display formatting
    let error_str = error.to_string();
    assert!(error_str.contains("Timeout error: Operation timed out after 30 seconds"));
}

#[test]
fn test_interpreter_error_cancellation_error() {
    let error = InterpreterError::cancellation_error("Operation was cancelled by user");

    assert_eq!(error.kind, ErrorKind::CancellationError);
    assert_eq!(error.message, "Operation was cancelled by user");
    assert!(error.position.is_none());
    assert!(error.source.is_none());

    // Test display formatting
    let error_str = error.to_string();
    assert!(error_str.contains("Cancellation error: Operation was cancelled by user"));
}

#[test]
fn test_interpreter_error_other_error() {
    let error = InterpreterError::other_error("Unknown error occurred");

    assert_eq!(error.kind, ErrorKind::OtherError);
    assert_eq!(error.message, "Unknown error occurred");
    assert!(error.position.is_none());
    assert!(error.source.is_none());

    // Test display formatting
    let error_str = error.to_string();
    assert!(error_str.contains("Error: Unknown error occurred"));
}

#[test]
fn test_interpretation_result_success() {
    let result = InterpretationResult::success("Result value");

    assert!(result.is_success());
    assert!(!result.is_error());

    match result {
        InterpretationResult::Success(value) => {
            assert_eq!(value, "Result value");
        }
        InterpretationResult::Error(_) => {
            panic!("Expected Success, got Error");
        }
    }

    // Test unwrap
    let unwrapped = InterpretationResult::success("Result value").unwrap();
    assert_eq!(unwrapped, "Result value");
}

#[test]
fn test_interpretation_result_error() {
    let error = InterpreterError::parsing_error("Invalid syntax", None, None);
    let result = InterpretationResult::error(error.clone());

    assert!(!result.is_success());
    assert!(result.is_error());

    match result {
        InterpretationResult::Success(_) => {
            panic!("Expected Error, got Success");
        }
        InterpretationResult::Error(err) => {
            assert_eq!(err.kind, ErrorKind::ParsingError);
            assert_eq!(err.message, "Invalid syntax");
        }
    }

    // Test unwrap_err
    let unwrapped_err = InterpretationResult::error(error).unwrap_err();
    assert_eq!(unwrapped_err.kind, ErrorKind::ParsingError);
    assert_eq!(unwrapped_err.message, "Invalid syntax");
}

#[test]
fn test_interpretation_result_from_error() {
    let error = InterpreterError::runtime_error("Runtime error", None, None);
    let result: InterpretationResult = error.into();

    assert!(result.is_error());

    match result {
        InterpretationResult::Success(_) => {
            panic!("Expected Error, got Success");
        }
        InterpretationResult::Error(err) => {
            assert_eq!(err.kind, ErrorKind::RuntimeError);
            assert_eq!(err.message, "Runtime error");
        }
    }
}

#[test]
fn test_interpretation_result_from_string() {
    let result: InterpretationResult = "Success value".to_string().into();

    assert!(result.is_success());

    match result {
        InterpretationResult::Success(value) => {
            assert_eq!(value, "Success value");
        }
        InterpretationResult::Error(_) => {
            panic!("Expected Success, got Error");
        }
    }
}

#[test]
fn test_interpretation_result_from_str() {
    let result: InterpretationResult = "Success value".into();

    assert!(result.is_success());

    match result {
        InterpretationResult::Success(value) => {
            assert_eq!(value, "Success value");
        }
        InterpretationResult::Error(_) => {
            panic!("Expected Success, got Error");
        }
    }
}

#[test]
#[should_panic(expected = "Called unwrap on an error result")]
fn test_interpretation_result_unwrap_error() {
    let error = InterpreterError::parsing_error("Invalid syntax", None, None);
    let result = InterpretationResult::error(error);

    // This should panic
    result.unwrap();
}

#[test]
#[should_panic(expected = "Called unwrap_err on a success result")]
fn test_interpretation_result_unwrap_err_success() {
    let result = InterpretationResult::success("Success value");

    // This should panic
    result.unwrap_err();
}
