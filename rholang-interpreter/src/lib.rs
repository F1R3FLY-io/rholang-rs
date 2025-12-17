use anyhow::anyhow;
use anyhow::Result;

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use rholang_compiler::compile_source_async;
use rholang_vm::api::{Value as VmValue, VM};

#[cfg(feature = "native-runtime")]
use tokio::sync::oneshot;
#[cfg(feature = "native-runtime")]
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone)]
pub struct InterpreterError {
    message: String,
}

impl InterpreterError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub enum InterpretationResult {
    Success(String),
    Error(InterpreterError),
}

impl InterpretationResult {
    pub fn is_success(&self) -> bool {
        matches!(self, InterpretationResult::Success(_))
    }
    pub fn unwrap(self) -> String {
        match self {
            InterpretationResult::Success(s) => s,
            InterpretationResult::Error(e) => panic!("unwrap on error: {}", e),
        }
    }
}

#[async_trait::async_trait(?Send)]
pub trait InterpreterProvider {
    async fn interpret(&self, code: &str) -> InterpretationResult;
    fn list_processes(&self) -> Result<Vec<(usize, String)>>;
    fn kill_process(&self, _pid: usize) -> Result<bool>;
    fn kill_all_processes(&self) -> Result<usize>;
}

struct ProcessInfo {
    code: String,
    #[cfg(feature = "native-runtime")]
    cancel: Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Clone, Default)]
pub struct RholangCompilerInterpreterProvider {
    processes: Arc<Mutex<HashMap<usize, ProcessInfo>>>,
    next_pid: Arc<Mutex<usize>>,
}

impl RholangCompilerInterpreterProvider {
    pub fn new() -> Result<Self> {
        Ok(Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_pid: Arc::new(Mutex::new(1)),
        })
    }

    fn render_value(v: &VmValue) -> String {
        match v {
            VmValue::Int(n) => n.to_string(),
            VmValue::Bool(b) => b.to_string(),
            VmValue::Str(s) => format!("\"{}\"", s),
            VmValue::Name(n) => format!("@{}", n),
            VmValue::List(items) => {
                let inner: Vec<String> = items.iter().map(Self::render_value).collect();
                format!("[{}]", inner.join(", "))
            }
            VmValue::Tuple(items) => {
                let inner: Vec<String> = items.iter().map(Self::render_value).collect();
                format!("({})", inner.join(", "))
            }
            VmValue::Map(entries) => {
                let inner: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", Self::render_value(k), Self::render_value(v)))
                    .collect();
                format!("{{{}}}", inner.join(", "))
            }
            VmValue::Nil => "Nil".to_string(),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl InterpreterProvider for RholangCompilerInterpreterProvider {
    async fn interpret(&self, code: &str) -> InterpretationResult {
        // Allocate a pid and record the process
        let pid = {
            let mut guard = match self.next_pid.lock() {
                Ok(g) => g,
                Err(e) => {
                    return InterpretationResult::Error(InterpreterError::new(format!(
                        "Lock error: {}",
                        e
                    )))
                }
            };
            let pid = *guard;
            *guard += 1;
            pid
        };

        #[cfg(feature = "native-runtime")]
        let (tx, mut rx) = oneshot::channel();

        {
            let mut processes = match self.processes.lock() {
                Ok(g) => g,
                Err(e) => {
                    return InterpretationResult::Error(InterpreterError::new(format!(
                        "Lock error: {}",
                        e
                    )))
                }
            };
            processes.insert(
                pid,
                ProcessInfo {
                    code: code.to_string(),
                    #[cfg(feature = "native-runtime")]
                    cancel: Some(tx),
                },
            );
        }

        // Core async compile + sync execute. Compile all top-level processes and return the
        // result of the last one (mirrors shell semantics and avoids "No process" errors).
        let fut = async move {
            let processes = match compile_source_async(code).await {
                Ok(ps) => ps,
                Err(e) => return InterpretationResult::Error(InterpreterError::new(e.to_string())),
            };

            if processes.is_empty() {
                return InterpretationResult::Success("Nil".to_string());
            }

            let mut vm = VM::new();
            let mut last_val = VmValue::Nil;
            for mut proc in processes.into_iter() {
                match vm.execute(&mut proc) {
                    Ok(val) => last_val = val,
                    Err(e) => {
                        return InterpretationResult::Error(InterpreterError::new(format!(
                            "Execution error: {}",
                            e
                        )))
                    }
                }
            }

            InterpretationResult::Success(Self::render_value(&last_val))
        };

        #[cfg(feature = "native-runtime")]
        let result = {
            let timed = timeout(Duration::from_secs(30), fut);
            tokio::select! {
                r = timed => r.unwrap_or_else(|_| InterpretationResult::Error(InterpreterError::new("Execution timed out"))),
                _ = &mut rx => InterpretationResult::Error(InterpreterError::new("Execution cancelled")),
            }
        };

        #[cfg(not(feature = "native-runtime"))]
        let result = fut.await;

        // Cleanup
        if let Ok(mut procs) = self.processes.lock() {
            procs.remove(&pid);
        }

        result
    }

    fn list_processes(&self) -> Result<Vec<(usize, String)>> {
        let procs = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        let mut out = Vec::new();
        for (pid, info) in procs.iter() {
            out.push((*pid, info.code.clone()));
        }
        Ok(out)
    }

    fn kill_process(&self, pid: usize) -> Result<bool> {
        let mut procs = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        if let Some(mut info) = procs.remove(&pid) {
            #[cfg(feature = "native-runtime")]
            if let Some(sender) = info.cancel.take() {
                let _ = sender.send(());
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn kill_all_processes(&self) -> Result<usize> {
        let mut procs = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        let count = procs.len();
        #[cfg(feature = "native-runtime")]
        for (_, mut info) in procs.drain() {
            if let Some(sender) = info.cancel.take() {
                let _ = sender.send(());
            }
        }
        #[cfg(not(feature = "native-runtime"))]
        {
            procs.clear();
        }
        Ok(count)
    }
}
