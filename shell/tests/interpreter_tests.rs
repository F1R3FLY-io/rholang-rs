use anyhow::Result;
use rstest::rstest;
use tokio::test;

use shell::interpreter::{FakeInterpreter, Interpreter};

#[tokio::test]
async fn test_fake_interpreter_returns_input() -> Result<()> {
    let interpreter = FakeInterpreter;

    let input = "println(\"Hello, World!\");".to_string();
    let result = interpreter.interpret(input.clone()).await?;

    assert_eq!(result, input);
    Ok(())
}

#[rstest]
#[case("println(\"test\");", "println(\"test\");")]
#[case("1 + 1", "1 + 1")]
#[case("", "")]
async fn test_fake_interpreter_with_various_inputs(
    #[case] input: String,
    #[case] expected: String
) -> Result<()> {
    let interpreter = FakeInterpreter;
    let result = interpreter.interpret(input).await?;

    assert_eq!(result, expected);
    Ok()
}
