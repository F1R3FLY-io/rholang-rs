use wasm_bindgen::prelude::*;

#[cfg(feature = "vm-eval")]
use rholang_compiler::{compile_first_process_async, Disassembler};
#[cfg(feature = "vm-eval")]
use rholang_interpreter::{InterpreterProvider, RholangCompilerInterpreterProvider};
#[cfg(feature = "vm-eval")]
use rholang_vm::api::Value;

// Render VM values similarly to the shell provider so outputs match across targets.
#[cfg(feature = "vm-eval")]
#[allow(dead_code)]
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
        Value::Par(ps) => {
            let elems: Vec<String> = ps.iter().map(|p| format!("<{}>", p.source_ref())).collect();
            format!("Par({})", elems.join(" | "))
        }
        Value::Nil => "Nil".to_string(),
    }
}

/// Evaluate Rholang source code synchronously. This is primarily for compatibility with
/// existing JS tests; it delegates to the async path internally for correctness.
#[cfg(feature = "vm-eval")]
#[wasm_bindgen]
pub fn eval(rholang_code: &str) -> String {
    // Use a minimal current-thread runtime to block on the async interpreter.
    futures::executor::block_on(eval_async(rholang_code))
}

#[cfg(not(feature = "vm-eval"))]
#[wasm_bindgen]
pub fn eval(rholang_code: &str) -> String {
    // Lightweight stub: simply echo the input for now.
    format!("Echo: {}", rholang_code)
}

/// Disassemble Rholang source code into bytecode representation.
/// This mirrors the shell disassembler and uses the compiler's Disassembler with default formatting.
#[cfg(feature = "vm-eval")]
#[wasm_bindgen]
pub fn disassemble(rholang_code: &str) -> String {
    futures::executor::block_on(disassemble_async(rholang_code))
}

#[cfg(not(feature = "vm-eval"))]
#[wasm_bindgen]
pub fn disassemble(rholang_code: &str) -> String {
    format!("EchoDisasm: {}", rholang_code)
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

/// Async disassembly entry-point; exposed as `disassembleRho` in JS for clarity.
#[cfg(feature = "vm-eval")]
#[wasm_bindgen(js_name = disassembleRho)]
pub async fn disassemble_async(rholang_code: &str) -> String {
    let process = match compile_first_process_async(rholang_code).await {
        Ok(proc) => proc,
        Err(e) => return format!("DisasmError: {}", e),
    };

    let disasm = Disassembler::new();
    disasm.disassemble(&process)
}

#[cfg(not(feature = "vm-eval"))]
#[wasm_bindgen(js_name = disassembleRho)]
pub async fn disassemble_async(rholang_code: &str) -> String {
    format!("StubDisasm: {}", rholang_code)
}

// Optional class-style API similar to the draft crate, convenient for JS callers
#[wasm_bindgen]
pub struct WasmInterpreter;

impl Default for WasmInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

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

    #[wasm_bindgen]
    pub async fn disassemble(&self, code: String) -> String {
        disassemble_async(&code).await
    }
}
