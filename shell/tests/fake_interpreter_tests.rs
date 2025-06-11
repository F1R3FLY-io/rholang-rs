use anyhow::Result;
use shell::providers::{FakeInterpreterProvider, InterpreterProvider};

#[tokio::test]
async fn test_fake_interpreter_with_arithmetic() -> Result<()> {
    let interpreter = FakeInterpreterProvider;
    let input = "1 + 2 * 3";
    let result = interpreter.interpret(input).await?;
    assert_eq!(result, input); // FakeInterpreterProvider just returns the input
    Ok(())
}

#[tokio::test]
async fn test_fake_interpreter_with_print() -> Result<()> {
    let interpreter = FakeInterpreterProvider;
    let input = "@\"stdout\"!(\"Hello, world!\")";
    let result = interpreter.interpret(input).await?;
    assert_eq!(result, input); // FakeInterpreterProvider just returns the input
    Ok(())
}

#[tokio::test]
async fn test_fake_interpreter_with_for_comprehension() -> Result<()> {
    let interpreter = FakeInterpreterProvider;
    let input = "for (msg <- channel) { @\"stdout\"!(msg) }";
    let result = interpreter.interpret(input).await?;
    assert_eq!(result, input); // FakeInterpreterProvider just returns the input
    Ok(())
}

#[tokio::test]
async fn test_fake_interpreter_with_new_declaration() -> Result<()> {
    let interpreter = FakeInterpreterProvider;
    let input = "new channel in { @\"stdout\"!(\"Using channel\") }";
    let result = interpreter.interpret(input).await?;
    assert_eq!(result, input); // FakeInterpreterProvider just returns the input
    Ok(())
}

#[rstest::rstest]
#[case("1 + 2", "1 + 2")]
#[case("3 * 4", "3 * 4")]
#[case("10 - 5", "10 - 5")]
#[case("8 / 2", "8 / 2")]
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
