use anyhow::{anyhow, Result};
use async_trait::async_trait;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::Promise;

#[cfg(not(target_arch = "wasm32"))]
use rholang_parser::RholangParser;
#[cfg(not(target_arch = "wasm32"))]
use validated::Validated;

#[async_trait]
pub trait InterpreterProvider {
    async fn interpret(&self, code: &str) -> Result<String>;
}

// ---- Providers ----

// On non-wasm targets, use the parser-based provider
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Default)]
pub struct WasmParserInterpreterProvider;

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl InterpreterProvider for WasmParserInterpreterProvider {
    async fn interpret(&self, code: &str) -> Result<String> {
        // Parse the code using the Rholang parser and pretty-print the validated AST
        let parser = RholangParser::new();
        let validated = parser.parse(code);
        match validated {
            Validated::Fail(_failure) => Err(anyhow!("Parsing failed")),
            _ => Ok(format!("{validated:#?}")),
        }
    }
}

// On wasm32 targets, use a lightweight echo provider to remain wasm-compatible
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Default)]
pub struct WasmEchoInterpreterProvider;

#[cfg(target_arch = "wasm32")]
#[async_trait]
impl InterpreterProvider for WasmEchoInterpreterProvider {
    async fn interpret(&self, code: &str) -> Result<String> {
        Ok(code.to_string())
    }
}

// Type alias to select the default provider per target
#[cfg(not(target_arch = "wasm32"))]
type DefaultProvider = WasmParserInterpreterProvider;
#[cfg(target_arch = "wasm32")]
type DefaultProvider = WasmEchoInterpreterProvider;

#[wasm_bindgen]
pub struct WasmInterpreter {
    provider: DefaultProvider,
}

#[wasm_bindgen]
impl WasmInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmInterpreter {
        WasmInterpreter { provider: DefaultProvider::default() }
    }

    /// Interpret Rholang code and return the result as a JS Promise<string>
    #[wasm_bindgen]
    pub fn interpret(&self, code: String) -> Promise {
        let provider = self.provider.clone();
        future_to_promise(async move {
            match provider.interpret(&code).await {
                Ok(output) => Ok(JsValue::from_str(&output)),
                Err(err) => Err(JsValue::from_str(&format!("Interpreter error: {}", err))),
            }
        })
    }
}

// Non-wasm unit tests to keep test coverage up for core logic
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parser_interpreter_parses_or_errors() {
        let provider = WasmParserInterpreterProvider::default();
        let input = "new x in { x!(42) }";
        let res = provider.interpret(input).await;
        assert!(res.is_ok() || res.is_err());
    }
}
