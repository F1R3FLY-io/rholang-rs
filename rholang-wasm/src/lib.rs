use wasm_bindgen::prelude::*;

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

fn eval_with_vm(_src: &str) -> String {
    // TODO: Parse `_src` and translate to `Process` once parser->bytecode path is available.
    let mut vm = VM::new();
    let mut proc = Process::new(vec![], "wasm");
    match vm.execute(&mut proc) {
        Ok(val) => pretty_value(&val),
        Err(e) => format!("Error: {}", e),
    }
}

/// Evaluate Rholang source code and return a result as string (sync API kept for tests and native callers).
/// Exported to JavaScript via wasm-bindgen. Must return `String` across the WASM boundary.
#[wasm_bindgen]
pub fn eval(rholang_code: &str) -> String {
    eval_with_vm(rholang_code)
}

/// Async evaluation entry-point for the browser. Exposed as `evalRho` in JS to avoid reserved `eval` name.
#[wasm_bindgen(js_name = evalRho)]
pub async fn eval_async(rholang_code: &str) -> String {
    // For now the async path reuses the sync VM evaluation.
    eval_with_vm(rholang_code)
}