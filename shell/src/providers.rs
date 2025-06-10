use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait InterpreterProvider {
    async fn interpret(&self, code: String) -> Result<String>;
}

pub struct FakeInterpreterProvider;

#[async_trait]
impl InterpreterProvider for FakeInterpreterProvider {
    async fn interpret(&self, code: String) -> Result<String> {
        // Fake implementation: just returns the input code
        Ok(code)
    }
}