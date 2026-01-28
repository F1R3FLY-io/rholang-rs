use anyhow::anyhow;
use anyhow::Result;

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use rholang_compiler::compile_source_async;
use rholang_vm::api::Value as VmValue;

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
            VmValue::Par(procs) => {
                let inner: Vec<String> = procs.iter().map(|p| p.to_string()).collect();
                inner.join(" | ")
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

            let mut last_val = VmValue::Nil;
            for mut proc in processes.into_iter() {
                // --- NEW FLOW: store in RSpace then retrieve and execute ---
                let process_id = format!("proc_{}", pid);
                let channel = format!("@0:{}", process_id);

                let vm = proc.vm.take().unwrap_or_default();

                // Store the process in RSpace
                {
                    let mut rspace = match vm.rspace.lock() {
                        Ok(r) => r,
                        Err(e) => {
                            return InterpretationResult::Error(InterpreterError::new(format!(
                                "RSpace lock error: {}",
                                e
                            )))
                        }
                    };
                    if let Err(e) = rspace.tell(0, channel.clone(), VmValue::Par(vec![proc])) {
                        return InterpretationResult::Error(InterpreterError::new(format!(
                            "RSpace tell error: {}",
                            e
                        )));
                    }
                }

                // Retrieve the process from RSpace
                let mut retrieved_proc = {
                    let mut rspace = match vm.rspace.lock() {
                        Ok(r) => r,
                        Err(e) => {
                            return InterpretationResult::Error(InterpreterError::new(format!(
                                "RSpace lock error: {}",
                                e
                            )))
                        }
                    };
                    match rspace.ask(0, channel.clone()) {
                        Ok(Some(VmValue::Par(mut procs))) if !procs.is_empty() => procs.remove(0),
                        Ok(Some(other)) => {
                            return InterpretationResult::Error(InterpreterError::new(format!(
                                "Expected process in RSpace, found: {:?}",
                                other
                            )))
                        }
                        Ok(None) => {
                            return InterpretationResult::Error(InterpreterError::new(
                                "Process not found in RSpace after tell",
                            ))
                        }
                        Err(e) => {
                            return InterpretationResult::Error(InterpreterError::new(format!(
                                "RSpace ask error: {}",
                                e
                            )))
                        }
                    }
                };

                // Execute the retrieved process
                retrieved_proc.vm = Some(vm);
                match retrieved_proc.execute() {
                    Ok(val) => {
                        last_val = val;
                    }
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
        if let Some(_info) = procs.remove(&pid) {
            #[cfg(feature = "native-runtime")]
            {
                // the sender is dropped, which cancels the receiver
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_interpreter_storage_flow() -> Result<()> {
        let provider = RholangCompilerInterpreterProvider::new()?;

        // This code will be compiled to bytecode
        let code = "1 + 2";

        // Execute the code.
        // The implementation now:
        // 1. Compiles "1 + 2" to a Process
        // 2. Stores it in RSpace under "proc_1"
        // 3. Retrieves it from RSpace
        // 4. Executes it
        let result = provider.interpret(code).await;

        assert!(result.is_success());
        assert_eq!(result.unwrap(), "3");

        Ok(())
    }
}
