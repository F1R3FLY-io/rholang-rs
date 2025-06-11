use anyhow::Result;
use rstest::rstest;

use shell::providers::{FakeInterpreterProvider, InterpreterProvider};

#[tokio::test]
async fn test_fake_interpreter_with_arithmetic() -> Result<()> {
    let interpreter = FakeInterpreterProvider;

    let input = "1 + 2 * 3";
    let result = interpreter.interpret(input).await?;

    // The fake interpreter just returns the input
    assert_eq!(result, input);
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
    let result = interpreter.interpret(input).await?;

    assert_eq!(result, expected);
    Ok(())
}
