use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use serde::{Deserialize, Serialize};

use rholang_vm::api::{Process, VM, Value};

#[derive(Serialize, Deserialize)]
pub struct EvalResult {
    pub pretty: String,
}

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

fn eval_with_vm(_src: &str) -> EvalResult {
    // TODO: Parse `_src` and translate to `Process` once parser->bytecode path is available.
    let mut vm = VM::new();
    let mut proc = Process::new(vec![], "wasm");
    let pretty = match vm.execute(&mut proc) {
        Ok(val) => pretty_value(&val),
        Err(e) => format!("Error: {}", e),
    };
    EvalResult { pretty }
}

fn js_err<E: core::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Synchronous evaluation function kept for backward-compatibility with native tests.
/// Returns the pretty-printed placeholder string (currently "Nil").
#[wasm_bindgen]
pub fn eval(code: &str) -> String {
    let res = eval_with_vm(code);
    res.pretty
}

/// Promise-returning async evaluation suitable for browsers.
#[wasm_bindgen]
pub fn eval_async(code: String) -> js_sys::Promise {
    future_to_promise(async move {
        // Placeholder: call into VM; replace with real interpreter pipeline when available
        let res = eval_with_vm(&code);
        serde_wasm_bindgen::to_value(&res).map_err(js_err)
    })
}

/// Improve error messages in the browser console.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    // Optionally: log a banner once loaded
    web_sys::console::log_1(&JsValue::from_str("rholang-wasm initialized"));
}