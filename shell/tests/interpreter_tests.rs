use anyhow::Result;
use rholang_fake::InterpretationResult;
use rstest::rstest;

use shell::providers::{FakeInterpreterProvider, InterpreterProvider};

#[tokio::test]
async fn test_fake_interpreter_returns_input() -> Result<()> {
    let interpreter = FakeInterpreterProvider;

    let input = "println(\"Hello, World!\");".to_string();
    let result = interpreter.interpret(&input).await;

    match result {
        InterpretationResult::Success(output) => {
            assert_eq!(output, input);
        }
        InterpretationResult::Error(err) => {
            panic!("Expected success, got error: {}", err);
        }
    }
    Ok(())
}

#[rstest]
#[case("println(\"Hello, World!\");", "println(\"Hello, World!\");")]
#[case("let x = 42;", "let x = 42;")]
#[case("println(\"test\");", "println(\"test\");")]
#[case("1 + 1", "1 + 1")]
#[case("", "")]
#[async_std::test]
async fn test_fake_interpreter_with_various_inputs(
    #[case] input: String,
    #[case] expected: String,
) -> Result<()> {
    let interpreter = FakeInterpreterProvider;
    let result = interpreter.interpret(&input).await;

    match result {
        InterpretationResult::Success(output) => {
            assert_eq!(output, expected);
        }
        InterpretationResult::Error(err) => {
            panic!("Expected success, got error: {}", err);
        }
    }
    Ok(())
}
