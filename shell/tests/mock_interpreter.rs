use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use async_trait::async_trait;

use shell::interpreter::Interpreter;

pub struct MockInterpreter {
    pub responses: Arc<Mutex<HashMap<String, Result<String>>>>,
    pub call_count: Arc<Mutex<usize>>,
    pub calls: Arc<Mutex<Vec<String>>>,
}

impl MockInterpreter {
    pub fn new() -> Self {
        MockInterpreter {
            responses: Arc::new(Mutex::new(HashMap::new())),
            call_count: Arc::new(Mutex::new(0)),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_response(mut self, input: &str, output: Result<String>) -> Self {
        self.responses.lock().unwrap().insert(input.to_string(), output);
        self
    }

    pub fn with_success_response(self, input: &str, output: &str) -> Self {
        self.with_response(input, Ok(output.to_string()))
    }

    pub fn with_error_response(self, input: &str, error: &str) -> Self {
        self.with_response(input, Err(anyhow!(error)))
    }

    pub fn get_call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    pub fn get_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl Interpreter for MockInterpreter {
    async fn interpret(&self, code: String) -> Result<String> {
        // Record the call
        {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;

            let mut calls = self.calls.lock().unwrap();
            calls.push(code.clone());
        }

        // Return the configured response for this input
        let responses = self.responses.lock().unwrap();
        responses
            .get(&code)
            .cloned()
            .unwrap_or_else(|| Ok(format!("Mock default response for: {}", code)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_interpreter() -> Result<()> {
        let mock = MockInterpreter::new()
            .with_success_response("hello", "world")
            .with_error_response("error", "This is an error");

        assert_eq!(mock.interpret("hello".to_string()).await?, "world");
        assert!(mock.interpret("error".to_string()).await.is_err());
        assert_eq!(mock.get_call_count(), 2);

        let calls = mock.get_calls();
        assert_eq!(calls, vec!["hello", "error"]);

        Ok(())
    }
}
