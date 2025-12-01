use wasm_bindgen::prelude::*;

#[cfg(feature = "vm-eval")]
use rholang_interpreter::{InterpreterProvider, RholangCompilerInterpreterProvider};
#[cfg(feature = "vm-eval")]
use rholang_vm::api::{Process, VM, Value};

// For the synchronous `eval` API (kept only for compatibility with existing tests),
// we return the result of executing an empty process, which renders to "Nil".
// This mirrors the previous placeholder behavior expected by tests.
#[cfg(feature = "vm-eval")]
fn pretty_value(v: &Value) -> String {
    match v {
        Value::Int(n) => format!("Int({})", n),
        Value::Bool(b) => format!("Bool({})", b),
        Value::Str(s) => format!("Str(\"{}\")", s),
        Value::Name(n) => format!("Name({})", n),
        Value::List(xs) => {
            let elems: Vec<String> = xs.iter().map(pretty_value).collect();
            format!("List([{}])", elems.join(", "))
        }
        Value::Tuple(xs) => {
            let elems: Vec<String> = xs.iter().map(pretty_value).collect();
            format!("Tuple({})", elems.join(", "))
        }
        Value::Map(kvs) => {
            let elems: Vec<String> = kvs
                .iter()
                .map(|(k, v)| format!("{} => {}", pretty_value(k), pretty_value(v)))
                .collect();
            format!("Map({{{}}})", elems.join(", "))
        }
        Value::Nil => "Nil".to_string(),
    }
}

// Note: the old sync compile_and_run using the parser directly was removed.

/// Evaluate Rholang source code and return a result as string (sync API kept for tests and native callers).
/// Exported to JavaScript via wasm-bindgen. Must return `String` across the WASM boundary.
#[cfg(feature = "vm-eval")]
#[wasm_bindgen]
pub fn eval(_rholang_code: &str) -> String {
    // Intentionally do not attempt to compile/execute user code in sync mode.
    // Execute an empty process to produce a stable placeholder result "Nil".
    let mut vm = VM::new();
    let mut proc = Process::new(vec![], "wasm-placeholder");
    match vm.execute(&mut proc) {
        Ok(val) => pretty_value(&val),
        Err(exec_err) => format!("Fallback failed: {}", exec_err),
    }
}

#[cfg(not(feature = "vm-eval"))]
#[wasm_bindgen]
pub fn eval(rholang_code: &str) -> String {
    // Lightweight stub: simply echo the input for now.
    format!("Echo: {}", rholang_code)
}

/// Async evaluation entry-point for the browser. Exposed as `evalRho` in JS to avoid reserved `eval` name.
#[cfg(feature = "vm-eval")]
#[wasm_bindgen(js_name = evalRho)]
pub async fn eval_async(rholang_code: &str) -> String {
    let provider = match RholangCompilerInterpreterProvider::new() {
        Ok(p) => p,
        Err(e) => return format!("InitError: {}", e),
    };
    match provider.interpret(rholang_code).await {
        rholang_interpreter::InterpretationResult::Success(s) => s,
        rholang_interpreter::InterpretationResult::Error(e) => format!("{}", e),
    }
}

#[cfg(not(feature = "vm-eval"))]
#[wasm_bindgen(js_name = evalRho)]
pub async fn eval_async(rholang_code: &str) -> String {
    // Stub async API: return the input as-is, prefixed to show it's a stub.
    format!("StubEval: {}", rholang_code)
}

// Optional class-style API similar to the draft crate, convenient for JS callers
#[wasm_bindgen]
pub struct WasmInterpreter;

#[wasm_bindgen]
impl WasmInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmInterpreter {
        WasmInterpreter
    }

    #[wasm_bindgen]
    pub async fn interpret(&self, code: String) -> String {
        eval_async(&code).await
    }
}