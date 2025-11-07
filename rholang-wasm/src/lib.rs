use wasm_bindgen::prelude::*;

/// Pure Rust evaluation function used for testing and reuse.
/// For now, this demo implementation simply echoes the input.
pub fn eval_pure(rholang_code: &str) -> String {
    rholang_code.to_string()
}

#[cfg(feature = "vm-eval")]
mod vm_adapter {
    use super::*;
    use rholang_vm::api::{Process, VM, Value};

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

    /// Temporary async adapter stub that executes an empty process and returns final Value pretty-printed.
    pub async fn eval_with_vm_async(_src: &str) -> String {
        // TODO: Parse `_src` and translate to `Process` once parser->bytecode path is available.
        let mut vm = VM::new();
        let mut proc = Process::new(vec![], "wasm");
        match vm.execute(&mut proc) {
            Ok(val) => pretty_value(&val),
            Err(e) => format!("Error: {}", e),
        }
    }
}

/// Evaluate Rholang source code and return a result as string (sync API kept for tests and native callers).
/// Exported to JavaScript via wasm-bindgen. Must return `String` across the WASM boundary.
#[wasm_bindgen]
pub fn eval(rholang_code: &str) -> String {
    // In a future implementation, this would call into the real interpreter.
    eval_pure(rholang_code)
}

/// Async evaluation entry-point for the browser. Exposed as `evalRho` in JS to avoid reserved `eval` name.
#[wasm_bindgen(js_name = evalRho)]
pub async fn eval_async(rholang_code: &str) -> String {
    #[cfg(feature = "vm-eval")]
    {
        return vm_adapter::eval_with_vm_async(rholang_code).await;
    }
    // Fallback when vm-eval is disabled
    eval_pure(rholang_code)
}