use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait Interpreter {
    async fn interpret(&self, code: String) -> Result<String>;
}

pub struct FakeInterpreter;

#[async_trait]
impl Interpreter for FakeInterpreter {
    async fn interpret(&self, code: String) -> Result<String> {
        // Fake implementation: just returns the input code
        Ok(code)
    }
}