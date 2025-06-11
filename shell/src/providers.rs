use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait InterpreterProvider {
    async fn interpret(&self, code: &str) -> Result<String>;
}

pub struct FakeInterpreterProvider;

#[async_trait]
impl InterpreterProvider for FakeInterpreterProvider {
    async fn interpret(&self, code: &str) -> Result<String> {
        // Fake implementation: just returns the input code
        Ok(code.to_string())
    }
}
