use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rholang_parser::RholangParser;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task;
use tokio::time::timeout;
// Compiler/VM imports
use librho::sem::{
    pipeline::Pipeline, DiagnosticKind, EnclosureAnalysisPass, ErrorKind, ForCompElaborationPass,
    ResolverPass, SemanticDb,
};
use rholang_compiler::Compiler;
use rholang_vm::api::Value as VmValue;

/// Remove source position/span information from a pretty-printed AST/debug output
fn strip_sourcepos(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut skip_depth: i32 = 0;
    for line in input.lines() {
        if skip_depth > 0 {
            // Track nested braces within the skipped block
            let opens = line.matches('{').count() as i32;
            let closes = line.matches('}').count() as i32;
            skip_depth += opens - closes;
            if skip_depth <= 0 {
                skip_depth = 0;
            }
            continue;
        }

        let trimmed = line.trim_start();

        // Start skipping when encountering a SourcePos or span block
        if trimmed.contains("SourcePos {") || trimmed.starts_with("span: SourceSpan {") {
            // Initialize skip depth considering current line braces
            let opens = line.matches('{').count() as i32;
            let closes = line.matches('}').count() as i32;
            // We enter a block (at least one opening brace)
            skip_depth = 1 + (opens - closes - 1).max(0);
            continue;
        }

        // Also skip simple fields that directly reference source position labels or span fields
        if trimmed.starts_with("pos:")
            || trimmed.starts_with("start: SourcePos")
            || trimmed.starts_with("end: SourcePos")
            || trimmed.starts_with("span:")
        {
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    if out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Represents an error that occurred during interpretation
#[derive(Debug, Clone)]
pub struct InterpreterError {
    /// A human-readable error message
    pub message: String,
    /// The position in the source code where the error occurred (if available)
    pub position: Option<String>,
    /// The source code that caused the error (if available)
    pub source: Option<String>,
}

impl InterpreterError {
    /// Create a new parsing error
    pub fn parsing_error(
        message: impl Into<String>,
        position: Option<String>,
        source: Option<String>,
    ) -> Self {
        InterpreterError {
            message: message.into(),
            position,
            source,
        }
    }

    /// Create a new timeout error
    pub fn timeout_error(message: impl Into<String>) -> Self {
        InterpreterError {
            message: message.into(),
            position: None,
            source: None,
        }
    }

    /// Create a new cancellation error
    pub fn cancellation_error(message: impl Into<String>) -> Self {
        InterpreterError {
            message: message.into(),
            position: None,
            source: None,
        }
    }

    /// Create a new other error
    pub fn other_error(message: impl Into<String>) -> Self {
        InterpreterError {
            message: message.into(),
            position: None,
            source: None,
        }
    }
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;

        if let Some(position) = &self.position {
            write!(f, " at {}", position)?;
        }

        if let Some(source) = &self.source {
            write!(f, "\nSource: {}", source)?;
        }

        Ok(())
    }
}

/// Represents the result of an interpretation operation
#[derive(Debug, Clone)]
pub enum InterpretationResult {
    /// Successful interpretation with a result value
    Success(String),
    /// Error during interpretation
    Error(InterpreterError),
}

impl InterpretationResult {
    /// Returns true if the result is a success
    pub fn is_success(&self) -> bool {
        matches!(self, InterpretationResult::Success(_))
    }

    /// Returns true if the result is an error
    pub fn is_error(&self) -> bool {
        matches!(self, InterpretationResult::Error(_))
    }

    /// Unwraps the success value, panics if the result is an error
    pub fn unwrap(self) -> String {
        match self {
            InterpretationResult::Success(value) => value,
            InterpretationResult::Error(err) => panic!("Called unwrap on an error result: {}", err),
        }
    }

    /// Unwraps the error value, panics if the result is a success
    pub fn unwrap_err(self) -> InterpreterError {
        match self {
            InterpretationResult::Success(_) => {
                panic!("Called unwrap_err on a success result")
            }
            InterpretationResult::Error(err) => err,
        }
    }
}

/// Trait for interpreter providers
/// This trait defines the interface for interpreters that can be used with the rholang-shell
#[async_trait]
pub trait InterpreterProvider {
    /// Interpret a string of code and return the result
    async fn interpret(&self, code: &str) -> InterpretationResult;

    /// Disassemble the provided code into bytecode representation (as text)
    /// Default providers may return an error if unsupported
    fn disassemble(&self, _code: &str) -> Result<String> {
        Err(anyhow!("Disassembly is not supported by this provider"))
    }

    /// List all running processes
    /// Returns a vector of tuples containing the process ID and the code being executed
    fn list_processes(&self) -> Result<Vec<(usize, String)>>;

    /// Kill a process by ID
    /// Returns true if the process was killed, false if it wasn't found
    fn kill_process(&self, pid: usize) -> Result<bool>;

    /// Kill all running processes
    /// Returns the number of processes that were killed
    fn kill_all_processes(&self) -> Result<usize>;
}

/// A fake interpreter provider that simply returns the input code
/// This is used for testing and as a placeholder
pub struct FakeInterpreterProvider;

#[async_trait]
impl InterpreterProvider for FakeInterpreterProvider {
    async fn interpret(&self, code: &str) -> InterpretationResult {
        // Fake implementation: just returns the input code
        InterpretationResult::Success(code.to_string())
    }

    fn disassemble(&self, _code: &str) -> Result<String> {
        Err(anyhow!(
            "Disassembly not available in FakeInterpreterProvider"
        ))
    }

    /// List all running processes
    /// This is a fake implementation that always returns an empty list
    /// since FakeInterpreterProvider doesn't actually manage processes
    fn list_processes(&self) -> Result<Vec<(usize, String)>> {
        // Fake implementation: no processes to list
        Ok(Vec::new())
    }

    /// Kill a process by ID
    /// This is a fake implementation that always returns false
    /// since FakeInterpreterProvider doesn't actually manage processes
    fn kill_process(&self, _pid: usize) -> Result<bool> {
        // Fake implementation: no processes to kill
        Ok(false)
    }

    /// Kill all running processes
    /// This is a fake implementation that always returns 0
    /// since FakeInterpreterProvider doesn't actually manage processes
    fn kill_all_processes(&self) -> Result<usize> {
        // Fake implementation: no processes to kill
        Ok(0)
    }
}

/// Information about a running interpreter process
struct ProcessInfo {
    /// The code being interpreted
    code: String,
    /// The cancel sender to abort the process
    cancel_sender: Option<oneshot::Sender<()>>,
}

/// Provider for the Rholang parser
/// This implements the InterpreterProvider trait
#[derive(Clone)]
pub struct RholangParserInterpreterProvider {
    /// Map of process ID to process information
    processes: Arc<Mutex<HashMap<usize, ProcessInfo>>>,
    /// Next process ID to assign
    next_pid: Arc<Mutex<usize>>,
    /// Delay for async interpretation (in milliseconds)
    delay_ms: Arc<Mutex<u64>>,
}

impl RholangParserInterpreterProvider {
    /// Create a new instance of the Rholang parser interpreter provider
    pub fn new() -> Result<Self> {
        Ok(RholangParserInterpreterProvider {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_pid: Arc::new(Mutex::new(1)),
            delay_ms: Arc::new(Mutex::new(0)), // Default delay: 0 seconds
        })
    }

    /// Set the delay for async interpretation
    pub fn set_delay(&self, delay_ms: u64) -> Result<&Self> {
        let mut delay = self
            .delay_ms
            .lock()
            .map_err(|e| anyhow!("Failed to lock delay_ms: {}", e))?;
        *delay = delay_ms;
        Ok(self)
    }
}

/// Implementation of the InterpreterProvider trait for the Rholang parser
#[async_trait]
impl InterpreterProvider for RholangParserInterpreterProvider {
    async fn interpret(&self, code: &str) -> InterpretationResult {
        // Clone the code for the process info and for the task
        let code_clone = code.to_string();
        let code_for_task = code.to_string();

        // Clone the Arc<Mutex<>> for the task
        let processes = Arc::clone(&self.processes);
        let next_pid = Arc::clone(&self.next_pid);

        // Create a oneshot channel for cancellation
        let (cancel_sender, cancel_receiver) = oneshot::channel();

        // Get the next process ID
        let pid = {
            let mut next_pid = match next_pid.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    return InterpretationResult::Error(InterpreterError::other_error(format!(
                        "Failed to lock next_pid: {}",
                        e
                    )))
                }
            };
            let pid = *next_pid;
            *next_pid += 1;
            pid
        };

        // Store the process info
        {
            let mut processes = match processes.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    return InterpretationResult::Error(InterpreterError::other_error(format!(
                        "Failed to lock processes: {}",
                        e
                    )))
                }
            };
            processes.insert(
                pid,
                ProcessInfo {
                    code: code_clone,
                    cancel_sender: Some(cancel_sender),
                },
            );
        }

        // Get the delay for the interpreter
        let delay = match self.delay_ms.lock() {
            Ok(guard) => *guard,
            Err(e) => {
                return InterpretationResult::Error(InterpreterError::other_error(format!(
                    "Failed to lock delay_ms: {}",
                    e
                )))
            }
        };

        // Spawn a task to run the parser asynchronously
        let handle = task::spawn(async move {
            // Create a future that completes when the cancel signal is received
            let cancel_future = cancel_receiver;

            // Create a future that completes when the parser finishes
            let interpret_future = async {
                // Add a delay to simulate processing time
                if delay > 0 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }

                // Create a parser locally in the task and parse the code
                let parser = RholangParser::new();
                let validated = parser.parse(&code_for_task);
                // Always pretty-print the parser result (including failures) to match golden snapshots
                let rendered = format!("{validated:#?}");
                let cleaned = strip_sourcepos(&rendered);
                InterpretationResult::Success(cleaned)
            };

            // Run the parser with a timeout
            let timeout_future = timeout(Duration::from_secs(30), interpret_future);

            // Wait for either the parser to finish, the timeout to expire, or the cancel signal to be received
            tokio::select! {
                result = timeout_future => {
                    result.unwrap_or_else(|_| InterpretationResult::Error(InterpreterError::timeout_error("Parser timed out after 30 seconds")))
                }
                _ = cancel_future => {
                    InterpretationResult::Error(InterpreterError::cancellation_error("Parser was cancelled"))
                }
            }
        });

        // Wait for the task to complete
        let result = handle.await.unwrap_or_else(|e| {
            InterpretationResult::Error(InterpreterError::other_error(format!("Task error: {}", e)))
        });

        // Remove the process from the map
        let mut processes = match self.processes.lock() {
            Ok(guard) => guard,
            Err(e) => {
                return InterpretationResult::Error(InterpreterError::other_error(format!(
                    "Failed to lock processes: {}",
                    e
                )))
            }
        };
        processes.remove(&pid);

        result
    }

    fn disassemble(&self, _code: &str) -> Result<String> {
        Err(anyhow!(
            "Disassembly not available in RholangParserInterpreterProvider"
        ))
    }

    /// List all running processes
    /// Returns a vector of tuples containing the process ID and the code being executed
    /// This implementation returns the actual list of running processes managed by this provider
    fn list_processes(&self) -> Result<Vec<(usize, String)>> {
        let processes = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        let mut result = Vec::new();
        for (pid, info) in processes.iter() {
            result.push((*pid, info.code.clone()));
        }
        Ok(result)
    }

    /// Kill a process by ID
    /// Returns true if the process was killed, false if it wasn't found
    /// This implementation sends a cancellation signal to the process and removes it from the process map
    fn kill_process(&self, pid: usize) -> Result<bool> {
        let mut processes = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        if let Some(mut info) = processes.remove(&pid) {
            // Send cancellation signal if the sender is still available
            if let Some(sender) = info.cancel_sender.take() {
                let _ = sender.send(());
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Kill all running processes
    /// Returns the number of processes that were killed
    /// This implementation sends cancellation signals to all processes and removes them from the process map
    fn kill_all_processes(&self) -> Result<usize> {
        let mut processes = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        let count = processes.len();
        for (_, mut info) in processes.drain() {
            // Send cancellation signal if the sender is still available
            if let Some(sender) = info.cancel_sender.take() {
                let _ = sender.send(());
            }
        }
        Ok(count)
    }
}

/// Provider backed by the rholang-compiler and rholang-vm
/// Parses, compiles to bytecode, executes in the VM, and returns the resulting value.
#[derive(Clone)]
pub struct RholangCompilerInterpreterProvider {
    /// Map of process ID to process information
    processes: Arc<Mutex<HashMap<usize, ProcessInfo>>>,
    /// Next process ID to assign
    next_pid: Arc<Mutex<usize>>,
    /// Optional artificial delay (ms) for testing/demo
    delay_ms: Arc<Mutex<u64>>,
}

impl RholangCompilerInterpreterProvider {
    pub fn new() -> Result<Self> {
        Ok(RholangCompilerInterpreterProvider {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_pid: Arc::new(Mutex::new(1)),
            delay_ms: Arc::new(Mutex::new(0)),
        })
    }

    #[allow(dead_code)]
    pub fn set_delay(&self, delay_ms: u64) -> Result<&Self> {
        let mut delay = self
            .delay_ms
            .lock()
            .map_err(|e| anyhow!("Failed to lock delay_ms: {}", e))?;
        *delay = delay_ms;
        Ok(self)
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
                let inner: Vec<String> = procs
                    .iter()
                    .map(|p| format!("<{}>", p.source_ref()))
                    .collect();
                inner.join(" | ")
            }
            VmValue::Nil => "Nil".to_string(),
        }
    }
}

#[async_trait]
impl InterpreterProvider for RholangCompilerInterpreterProvider {
    async fn interpret(&self, code: &str) -> InterpretationResult {
        // Clone inputs and shared state
        let code_clone = code.to_string();
        let code_for_task = code.to_string();
        let processes = Arc::clone(&self.processes);
        let next_pid = Arc::clone(&self.next_pid);

        let (cancel_sender, cancel_receiver) = oneshot::channel();

        // Allocate PID
        let pid = {
            let mut next_pid = match next_pid.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    return InterpretationResult::Error(InterpreterError::other_error(format!(
                        "Failed to lock next_pid: {}",
                        e
                    )))
                }
            };
            let pid = *next_pid;
            *next_pid += 1;
            pid
        };

        // Track process
        {
            let mut processes = match processes.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    return InterpretationResult::Error(InterpreterError::other_error(format!(
                        "Failed to lock processes: {}",
                        e
                    )))
                }
            };
            processes.insert(
                pid,
                ProcessInfo {
                    code: code_clone,
                    cancel_sender: Some(cancel_sender),
                },
            );
        }

        // Read delay
        let delay = match self.delay_ms.lock() {
            Ok(guard) => *guard,
            Err(e) => {
                return InterpretationResult::Error(InterpreterError::other_error(format!(
                    "Failed to lock delay_ms: {}",
                    e
                )))
            }
        };

        // cancellation future
        let mut cancel_future = cancel_receiver;

        // main future: offload heavy non-Send work to a blocking thread
        let fut = async move {
            if delay > 0 {
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            let res =
                task::spawn_blocking(move || {
                    // Parse
                    let parser = RholangParser::new();
                    let validated = parser.parse(&code_for_task);

                    // Extract AST or pretty-print errors
                    let ast_vec = match validated {
                        validated::Validated::Good(ast) => ast,
                        validated::Validated::Fail(ref err) => {
                            let rendered = format!("{err:#?}");
                            let cleaned = strip_sourcepos(&rendered);
                            return InterpretationResult::Error(InterpreterError::parsing_error(
                                cleaned, None, None,
                            ));
                        }
                    };

                    if ast_vec.is_empty() {
                        return InterpretationResult::Success("".to_string());
                    }

                    let mut db = SemanticDb::new();
                    // For now, execute the first top-level process
                    let first = &ast_vec[0];
                    let root = db.build_index(first);

                    // Run essential semantic passes before compilation using a dedicated runtime
                    let pipeline = Pipeline::new()
                        .add_fact(ResolverPass::new(root))
                        .add_fact(ForCompElaborationPass::new(root))
                        .add_fact(EnclosureAnalysisPass::new(root));

                    // Create a minimal runtime to block_on the async pipeline
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                        .enable_time()
                        .build()
                    {
                        rt.block_on(pipeline.run(&mut db));
                    } else {
                        return InterpretationResult::Error(InterpreterError::other_error(
                            "Failed to initialize runtime for semantic pipeline".to_string(),
                        ));
                    }

                    // Filter out NameInProcPosition errors (handled by compiler emitting EVAL)
                    let real_errors: Vec<_> = db
                        .errors()
                        .filter(|diag| {
                            !matches!(
                                diag.kind,
                                DiagnosticKind::Error(ErrorKind::NameInProcPosition(_, _))
                            )
                        })
                        .collect();

                    if !real_errors.is_empty() {
                        return InterpretationResult::Error(InterpreterError::other_error(
                            format!("Semantic errors: {:?}", real_errors),
                        ));
                    }

                    let compiler = Compiler::new(&db);
                    let mut process = match compiler.compile_single(first) {
                        Ok(p) => p,
                        Err(e) => {
                            return InterpretationResult::Error(InterpreterError::other_error(
                                format!("Compilation error: {}", e),
                            ))
                        }
                    };

                    // Execute the process (VM is initialized by default)
                    let value = match process.execute() {
                        Ok(v) => v,
                        Err(e) => {
                            return InterpretationResult::Error(InterpreterError::other_error(
                                format!("Execution error: {}", e),
                            ))
                        }
                    };

                    let rendered = Self::render_value(&value);
                    InterpretationResult::Success(rendered)
                })
                .await
                .unwrap_or_else(|e| {
                    InterpretationResult::Error(InterpreterError::other_error(format!(
                        "Blocking task error: {}",
                        e
                    )))
                });

            res
        };

        let timeout_future = timeout(Duration::from_secs(30), fut);

        let result = tokio::select! {
            result = timeout_future => {
                result.unwrap_or_else(|_| InterpretationResult::Error(InterpreterError::timeout_error("Execution timed out after 30 seconds")))
            }
            _ = &mut cancel_future => {
                InterpretationResult::Error(InterpreterError::cancellation_error("Execution was cancelled"))
            }
        };

        // Cleanup process tracking
        let mut processes = match self.processes.lock() {
            Ok(guard) => guard,
            Err(e) => {
                return InterpretationResult::Error(InterpreterError::other_error(format!(
                    "Failed to lock processes: {}",
                    e
                )))
            }
        };
        processes.remove(&pid);

        result
    }

    fn disassemble(&self, code: &str) -> Result<String> {
        // Helper that does the entire pipeline on the current thread
        fn do_disassemble(code: &str) -> String {
            // Parse
            let parser = RholangParser::new();
            let validated = parser.parse(code);

            let ast_vec = match validated {
                validated::Validated::Good(ast) => ast,
                validated::Validated::Fail(_err) => {
                    return "Parsing failed: unable to build AST. Please fix syntax errors and try again.".to_string();
                }
            };

            if ast_vec.is_empty() {
                return "No code to disassemble (empty AST)".to_string();
            }

            // Build semantic DB and run essential passes (resolver + elaborations)
            let mut db = SemanticDb::new();
            let first = &ast_vec[0];
            let root = db.build_index(first);

            // Run the pipeline using a lightweight runtime local to this thread
            if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
            {
                let pipeline = Pipeline::new()
                    .add_fact(ResolverPass::new(root))
                    .add_fact(ForCompElaborationPass::new(root))
                    .add_fact(EnclosureAnalysisPass::new(root));
                rt.block_on(pipeline.run(&mut db));
            } else {
                return "Failed to initialize runtime for semantic pipeline".to_string();
            }

            // Filter out NameInProcPosition errors (handled by compiler emitting EVAL)
            let real_errors: Vec<_> = db
                .errors()
                .filter(|diag| {
                    !matches!(
                        diag.kind,
                        DiagnosticKind::Error(ErrorKind::NameInProcPosition(_, _))
                    )
                })
                .collect();

            if !real_errors.is_empty() {
                return format!("Semantic errors: {:?}", real_errors);
            }

            // Compile first top-level process
            let compiler = Compiler::new(&db);
            let process = match compiler.compile_single(first) {
                Ok(p) => p,
                Err(e) => {
                    return format!("Compilation error: {}", e);
                }
            };

            // Disassemble in verbose format by default
            use rholang_compiler::{Disassembler, DisassemblyFormat};
            let disasm = Disassembler::with_format(DisassemblyFormat::Verbose);
            disasm.disassemble(&process)
        }

        // If we're inside a Tokio runtime, offload the entire work to a dedicated OS thread
        // to avoid nested-runtime and blocking issues. Otherwise, run directly.
        if tokio::runtime::Handle::try_current().is_ok() {
            let code_owned = code.to_string();
            let join = std::thread::spawn(move || do_disassemble(&code_owned));
            match join.join() {
                Ok(s) => Ok(s),
                Err(_e) => Ok("Disassembly failed due to thread panic".to_string()),
            }
        } else {
            Ok(do_disassemble(code))
        }
    }

    fn list_processes(&self) -> Result<Vec<(usize, String)>> {
        let processes = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        let mut result = Vec::new();
        for (pid, info) in processes.iter() {
            result.push((*pid, info.code.clone()));
        }
        Ok(result)
    }

    fn kill_process(&self, pid: usize) -> Result<bool> {
        let mut processes = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        if let Some(mut info) = processes.remove(&pid) {
            if let Some(sender) = info.cancel_sender.take() {
                let _ = sender.send(());
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn kill_all_processes(&self) -> Result<usize> {
        let mut processes = self
            .processes
            .lock()
            .map_err(|e| anyhow!("Failed to lock processes: {}", e))?;
        let count = processes.len();
        for (_, mut info) in processes.drain() {
            if let Some(sender) = info.cancel_sender.take() {
                let _ = sender.send(());
            }
        }
        Ok(count)
    }
}
