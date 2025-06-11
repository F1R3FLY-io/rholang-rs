use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rholang_fake::FakeRholangInterpreter;

/// Trait for interpreter providers
/// This trait defines the interface for interpreters that can be used with the shell
#[async_trait]
pub trait InterpreterProvider {
    /// Interpret a string of code and return the result
    async fn interpret(&self, code: &str) -> Result<String>;
}

/// A fake interpreter provider that simply returns the input code
/// This is used for testing and as a placeholder
pub struct FakeInterpreterProvider;

#[async_trait]
impl InterpreterProvider for FakeInterpreterProvider {
    async fn interpret(&self, code: &str) -> Result<String> {
        // Fake implementation: just returns the input code
        Ok(code.to_string())
    }
}

/// Provider for the fake Rholang interpreter
/// This implements the InterpreterProvider trait
pub struct RholangFakeInterpreterProvider;

impl RholangFakeInterpreterProvider {
    /// Create a new instance of the Rholang fake interpreter provider
    pub fn new() -> Result<Self> {
        Ok(RholangFakeInterpreterProvider)
    }
}

/// Implementation of the InterpreterProvider trait for the fake Rholang interpreter
#[async_trait]
impl InterpreterProvider for RholangFakeInterpreterProvider {
    async fn interpret(&self, code: &str) -> Result<String> {
        // Create a new interpreter for each call to avoid mutability issues
        let mut interpreter = FakeRholangInterpreter::new()?;

        // Use the interpreter to parse and validate the code
        if interpreter.is_valid(code) {
            interpreter.interpret(code)
        } else {
            Err(anyhow!("Invalid Rholang code"))
        }
    }
}
