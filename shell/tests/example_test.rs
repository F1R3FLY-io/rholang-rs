use anyhow::Result;
use rstest::rstest;

use shell::providers::{FakeInterpreterProvider, InterpretationResult, InterpreterProvider};

#[tokio::test]
async fn test_fake_interpreter_with_arithmetic() -> Result<()> {
    let interpreter = FakeInterpreterProvider;

    let input = "1 + 2 * 3";
    let result = interpreter.interpret(input).await;

    // The fake interpreter just returns the input
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
#[case("1 + 2", "1 + 2")]
#[case("3 * 4", "3 * 4")]
#[case("5 - 6", "5 - 6")]
#[async_std::test]
async fn test_fake_interpreter_with_various_arithmetic(
    #[case] input: &str,
    #[case] expected: &str,
) -> Result<()> {
    let interpreter = FakeInterpreterProvider;
    let result = interpreter.interpret(input).await;

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
