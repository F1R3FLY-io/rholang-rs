pub mod providers;

use anyhow::Result;
use bracket_parser::{BracketParser, BracketState};
use clap::Parser;
use providers::{InterpretationResult, InterpreterProvider};
use rustyline_async::{Readline, ReadlineEvent};
use std::io::Write;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Load code from file into the buffer at startup (interactive mode)
    #[arg(short = 'l', long = "load", value_name = "FILE")]
    pub load: Option<std::path::PathBuf>,

    /// Execute the provided code and exit (non-interactive)
    #[arg(short = 'e', long = "exec", value_name = "CODE")]
    pub exec: Option<String>,

    /// Execute code from the provided file and exit (non-interactive)
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: Option<std::path::PathBuf>,

    /// Show disassembly instead of executing (use with -e or -f)
    #[arg(short = 'd', long = "disassemble")]
    pub disassemble: bool,

    /// Show both disassembly and execution result (use with -e or -f)
    #[arg(short = 'b', long = "both")]
    pub both: bool,
}

pub fn help_message() -> String {
    "Available commands:".to_string()
        + "\n  .help, - Show this help message"
        + "\n  .list - List all edited lines"
        + "\n  .delete or .del - Remove the last edited line"
        + "\n  .reset or Ctrl+C - Interrupt current input (clear buffer)"
        + "\n  .load <file> - Load code from file into the buffer"
        + "\n  .dia - Disassemble bytecode for the code in the buffer"
        + "\n  .validate - Validate code in buffer with all rholang-lib validators"
        + "\n  .validate-unused - Validate only unused-variable diagnostics"
        + "\n  .validate-elab - Validate only elaboration diagnostics (types/joins/consumption/patterns)"
        + "\n  .validate-resolver - Run resolver and show its diagnostics only"
        + "\n  .ps - List all running processes"
        + "\n  .kill <index> - Kill a running process by index"
        + "\n  .quit - Exit the rholang-shell"
        + "\n\nNon-interactive CLI:"
        + "\n  --exec, -e <CODE>     Execute the provided code and exit"
        + "\n  --file, -f <FILE>     Execute code loaded from the file and exit"
        + "\n  --disassemble, -d     Show disassembly instead of executing (use with -e or -f)"
        + "\n  --both, -b            Show both disassembly and execution result"
        + "\n  If stdin is piped (non-TTY), the shell reads all input and processes it"
}

const DEFAULT_PROMPT: &str = ">>> ";

// ANSI color helpers (enabled only when writing to a TTY)
fn is_tty_stdout() -> bool {
    atty::is(atty::Stream::Stdout)
}
fn is_tty_stderr() -> bool {
    atty::is(atty::Stream::Stderr)
}

fn colorize(s: &str, code: &str, enable: bool) -> String {
    if enable {
        format!("\x1b[{}m{}\x1b[0m", code, s)
    } else {
        s.to_string()
    }
}

fn label_info(s: &str) -> String {
    colorize(s, "36", is_tty_stdout())
} // cyan
fn label_ok(s: &str) -> String {
    colorize(s, "32", is_tty_stdout())
} // green
#[allow(dead_code)]
fn label_warn(s: &str) -> String {
    colorize(s, "33", is_tty_stdout())
} // yellow
fn label_err_out(s: &str) -> String {
    colorize(s, "31", is_tty_stdout())
} // red for stdout-bound errors
fn label_err_err(s: &str) -> String {
    colorize(s, "31", is_tty_stderr())
} // red for stderr-bound errors

// Heuristic AST highlighter for pretty-printed debug trees
fn colorize_ast_tree(s: &str, enable: bool) -> String {
    if !enable {
        return s.to_string();
    }
    // Only colorize multi-line, structured outputs to avoid touching normal outputs
    if !s.contains('\n') {
        return s.to_string();
    }

    let mut out = String::with_capacity(s.len() + 32);
    for line in s.lines() {
        let mut i = 0usize;
        let bytes = line.as_bytes();
        // Copy leading whitespace/prefix indentation unchanged
        while i < bytes.len()
            && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'|' || bytes[i] == b'`')
        {
            out.push(bytes[i] as char);
            i += 1;
        }
        // After indentation, try to colorize tokens in a single pass
        while i < bytes.len() {
            let c = bytes[i] as char;
            // Strings: "..."
            if c == '"' {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    if ch == '\\' {
                        // escape next
                        if i + 1 < bytes.len() {
                            i += 2;
                            continue;
                        } else {
                            i += 1;
                            break;
                        }
                    }
                    if ch == '"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let segment = &line[start..i.min(line.len())];
                out.push_str(&colorize(segment, "32", true)); // green strings
                continue;
            }
            // Numbers
            if c.is_ascii_digit()
                || (c == '-' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit())
            {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
                // Optional decimal part
                if i < bytes.len() && (bytes[i] as char) == '.' {
                    i += 1;
                    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                        i += 1;
                    }
                }
                let segment = &line[start..i];
                out.push_str(&colorize(segment, "35", true)); // magenta numbers
                continue;
            }
            // Booleans
            if line[i..].starts_with("true") {
                out.push_str(&colorize("true", "36", true)); // cyan
                i += 4;
                continue;
            }
            if line[i..].starts_with("false") {
                out.push_str(&colorize("false", "36", true)); // cyan
                i += 5;
                continue;
            }
            // Field names of form ident: (until colon)
            if c.is_ascii_alphabetic() || c == '_' {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                // If followed by ':' we consider it a field label
                if i < bytes.len() && (bytes[i] as char) == ':' {
                    let ident = &line[start..i];
                    out.push_str(&colorize(ident, "33", true)); // yellow field name
                    out.push(':');
                    i += 1;
                    continue;
                } else {
                    // Otherwise it's likely a type/variant; color cyan
                    let ident = &line[start..i];
                    out.push_str(&colorize(ident, "36", true));
                    continue;
                }
            }
            // Default: copy char
            out.push(c);
            i += 1;
        }
        out.push('\n');
    }
    out
}

fn handle_kill_command<W: Write, I: InterpreterProvider>(
    arg: &str,
    stdout: &mut W,
    interpreter: &I,
) -> Result<()> {
    let pid_str = arg.trim();
    if pid_str.is_empty() {
        writeln!(stdout, "Usage: .kill <pid>")?;
        return Ok(());
    }
    match pid_str.parse::<usize>() {
        Ok(pid) => match interpreter.kill_process(pid) {
            Ok(true) => writeln!(stdout, "Process {} killed successfully", pid)?,
            Ok(false) => writeln!(stdout, "Process {} not found", pid)?,
            Err(e) => writeln!(stdout, "Error killing process {}: {}", pid, e)?,
        },
        Err(_) => writeln!(stdout, "Invalid process ID: {}", pid_str)?,
    }
    Ok(())
}

fn print_processes<W: Write, I: InterpreterProvider>(
    stdout: &mut W,
    interpreter: &I,
) -> Result<()> {
    match interpreter.list_processes() {
        Ok(processes) => {
            if processes.is_empty() {
                writeln!(stdout, "No running processes")?;
            } else {
                writeln!(stdout, "Running processes:")?;
                for (pid, code) in processes {
                    writeln!(stdout, "  {}: {}", pid, code)?;
                }
            }
        }
        Err(e) => writeln!(stdout, "Error listing processes: {}", e)?,
    }
    Ok(())
}

fn load_file_into_buffer<W: Write>(
    path: &str,
    buffer: &mut Vec<String>,
    stdout: &mut W,
    update_prompt: impl FnOnce(&str) -> Result<()>,
) -> Result<()> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let trimmed = contents.trim_end_matches(['\n', '\r']);
            buffer.clear();
            if trimmed.is_empty() {
                update_prompt(DEFAULT_PROMPT)?;
                writeln!(stdout, "Loaded 0 lines (file is empty): {}", path)?;
            } else {
                buffer.extend(trimmed.split('\n').map(|s| s.to_string()));
                update_prompt("... ")?;
                writeln!(stdout, "Loaded {} lines from: {}", buffer.len(), path)?;
                writeln!(
                    stdout,
                    "Press Enter to execute; if brackets are unbalanced, continue typing."
                )?;
            }
        }
        Err(e) => {
            writeln!(stdout, "Error loading file '{}': {}", path, e)?;
        }
    }
    Ok(())
}

/// Process a special command (starting with '.')
/// Returns true if the command was processed, false otherwise
pub fn process_special_command<W: Write, I: InterpreterProvider>(
    command: &str,
    buffer: &mut Vec<String>,
    stdout: &mut W,
    update_prompt: impl FnOnce(&str) -> Result<()>,
    interpreter: &I,
) -> Result<bool> {
    let trimmed = command.trim();
    if !trimmed.starts_with('.') {
        return Ok(false);
    }

    let (cmd, arg) = trimmed
        .split_once(' ')
        .map_or((trimmed, ""), |(c, a)| (c, a.trim()));

    match cmd {
        ".help" => {
            // Keep help text content the same; just color the header line if present
            writeln!(stdout, "{}", help_message())?;
        }
        ".quit" => {
            writeln!(stdout, "Exiting rholang-shell...")?;
            return Ok(true); // Signal to exit
        }
        ".list" => {
            writeln!(stdout, "Edited lines:")?;
            for line in buffer.iter() {
                writeln!(stdout, "{line}")?;
            }
        }
        ".delete" | ".del" => {
            if let Some(removed) = buffer.pop() {
                writeln!(stdout, "Removed last line: {removed}")?;
            } else {
                writeln!(stdout, "Buffer is empty, nothing to delete")?;
            }
        }
        ".reset" => {
            buffer.clear();
            update_prompt(DEFAULT_PROMPT)?;
            writeln!(stdout, "Buffer reset")?;
        }
        ".buffer" => {
            writeln!(stdout, "Current buffer: {:?}", buffer)?;
        }
        ".ps" => {
            print_processes(stdout, interpreter)?;
        }
        ".kill" => {
            handle_kill_command(arg, stdout, interpreter)?;
        }
        ".load" => {
            let path = arg.trim();
            if path.is_empty() {
                writeln!(stdout, "Usage: .load <file>")?;
            } else {
                load_file_into_buffer(path, buffer, stdout, update_prompt)?;
            }
        }
        ".dia" => {
            let code = buffer.join("\n");
            if code.trim().is_empty() {
                writeln!(stdout, "Buffer is empty, nothing to disassemble")?;
            } else {
                match interpreter.disassemble(&code) {
                    Ok(output) => {
                        writeln!(stdout, "{}", output)?;
                    }
                    Err(e) => {
                        writeln!(stdout, "{} {}", label_err_out("Disassembly error:"), e)?;
                    }
                }
            }
        }
        ".validate" => {
            let code = buffer.join("\n");
            if code.trim().is_empty() {
                writeln!(stdout, "Buffer is empty, nothing to validate")?;
            } else {
                run_all_validators_on_code(&code, stdout)?;
            }
        }
        ".validate-unused" => {
            let code = buffer.join("\n");
            if code.trim().is_empty() {
                writeln!(stdout, "Buffer is empty, nothing to validate")?;
            } else {
                run_validation_subset(&code, stdout, ValidationMode::UnusedOnly)?;
            }
        }
        ".validate-elab" => {
            let code = buffer.join("\n");
            if code.trim().is_empty() {
                writeln!(stdout, "Buffer is empty, nothing to validate")?;
            } else {
                run_validation_subset(&code, stdout, ValidationMode::ElabOnly)?;
            }
        }
        ".validate-resolver" => {
            let code = buffer.join("\n");
            if code.trim().is_empty() {
                writeln!(stdout, "Buffer is empty, nothing to validate")?;
            } else {
                run_validation_subset(&code, stdout, ValidationMode::ResolverOnly)?;
            }
        }
        _ => {
            writeln!(stdout, "Unknown command: {command}")?;
        }
    }
    Ok(false) // Don't exit
}

// ... existing code ...

/// Process a line of input in multiline mode
/// Returns Some(command) if a command is ready to be executed, None otherwise
pub fn process_multiline_input(
    line: String,
    buffer: &mut Vec<String>,
    update_prompt: impl FnOnce(&str) -> Result<()>,
) -> Result<Option<String>> {
    // If buffer is empty, ignore a leading empty line
    if buffer.is_empty() {
        if line.is_empty() {
            return Ok(None);
        }
        *buffer = vec![line];
        update_prompt("... ")?;
        return Ok(None);
    }

    // Any non-empty line just gets appended (including if the previous line was empty)
    if !line.is_empty() {
        buffer.push(line);
        return Ok(None);
    }

    // Empty line with non-empty buffer: only execute on DOUBLE empty lines when brackets are balanced
    let mut bracket_parser = match BracketParser::new() {
        Ok(parser) => parser,
        Err(_) => {
            // Fallback: if parser cannot be created, keep previous behavior and execute on double empty
            if buffer.last().map(|s| s.is_empty()).unwrap_or(false) {
                // Second empty: execute (exclude the marker empty)
                let _ = buffer.pop();
                let command = buffer.join("\n");
                // Keep buffer after execution for future processing
                update_prompt(DEFAULT_PROMPT)?;
                return Ok(Some(command));
            } else {
                // First empty: remember it and wait for another empty
                buffer.push(String::new());
                update_prompt("... ")?;
                return Ok(None);
            }
        }
    };

    // Build the joined input without any trailing empty marker for bracket parsing
    let joined_no_trailing_empty = if buffer.last().map(|s| s.is_empty()).unwrap_or(false) {
        // There is already a pending empty marker; don't include it in parsing
        buffer[..buffer.len() - 1].join("\n")
    } else {
        buffer.join("\n")
    };

    let state = bracket_parser.get_final_state(&joined_no_trailing_empty);

    if state == BracketState::Inside {
        // Brackets are still open; stay in multiline mode and do not execute
        update_prompt("... ")?;
        return Ok(None);
    }

    // Brackets are balanced here. Execute only on second consecutive empty line.
    if buffer.last().map(|s| s.is_empty()).unwrap_or(false) {
        // This is the second empty; execute now, excluding the marker
        let _ = buffer.pop();
        let command = buffer.join("\n");
        // Keep buffer after execution for future processing
        update_prompt(DEFAULT_PROMPT)?;
        Ok(Some(command))
    } else {
        // First empty after balanced buffer: remember and wait for another empty
        buffer.push(String::new());
        update_prompt("... ")?;
        Ok(None)
    }
}

/// Handle an interrupt event (Ctrl+C)
pub fn handle_interrupt<W: Write, I: InterpreterProvider>(
    buffer: &mut Vec<String>,
    stdout: &mut W,
    update_prompt: impl FnOnce(&str) -> Result<()>,
    interpreter: &I,
) -> Result<()> {
    // Always clear buffer (single line mode removed)
    buffer.clear();
    update_prompt(DEFAULT_PROMPT)?;

    // Kill all running processes
    match interpreter.kill_all_processes() {
        Ok(count) => {
            if count > 0 {
                writeln!(stdout, "Killed {} running processes", count)?;
            }
        }
        Err(e) => writeln!(stdout, "Error killing processes: {}", e)?,
    }

    writeln!(stdout, "Input interrupted with Ctrl+C")?;
    Ok(())
}

/// Run code in non-interactive mode with optional disassembly
async fn run_non_interactive<I: InterpreterProvider>(
    code: &str,
    args: &Args,
    interpreter: &I,
) -> Result<()> {
    let show_disasm = args.disassemble || args.both;
    let show_exec = !args.disassemble || args.both;

    // Show disassembly if requested
    if show_disasm {
        match interpreter.disassemble(code) {
            Ok(disasm) => {
                if args.both {
                    println!("=== Disassembly ===");
                }
                println!("{}", disasm);
            }
            Err(e) => {
                eprintln!("Disassembly error: {}", e);
            }
        }
    }

    // Execute if requested
    if show_exec {
        if args.both {
            println!("\n=== Execution ===");
        }
        let result = interpreter.interpret(code).await;
        match result {
            InterpretationResult::Success(output) => {
                println!("{}", output);
            }
            InterpretationResult::Error(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}

/// Run the rholang-shell with the provided interpreter provider
pub async fn run_shell<I: InterpreterProvider>(args: Args, interpreter: I) -> Result<()> {
    // Highest-priority non-interactive: explicit --exec or --file flags
    if let Some(code) = args.exec.as_ref() {
        return run_non_interactive(code, &args, &interpreter).await;
    }

    if let Some(file_path) = args.file.as_ref() {
        let code = std::fs::read_to_string(file_path)?;
        return run_non_interactive(&code, &args, &interpreter).await;
    }

    // If stdin is not a TTY, run in non-interactive (batch) mode and read from stdin
    if !atty::is(atty::Stream::Stdin) {
        use std::io::{self, Read};
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let input = input.trim().to_string();
        if input.is_empty() {
            return Ok(());
        }
        return run_non_interactive(&input, &args, &interpreter).await;
    }

    writeln!(std::io::stdout(), "{}", help_message())?;

    let prompt = ">>> ".to_string();

    let (mut rl, mut stdout) = Readline::new(prompt.clone())?;
    let mut buffer: Vec<String> = Vec::new();

    rl.should_print_line_on(true, false);

    // If a file was provided via CLI, load it into the buffer now
    if let Some(path) = args.load.as_ref() {
        let path_str = path.to_string_lossy().to_string();
        load_file_into_buffer(&path_str, &mut buffer, &mut stdout, |prompt| {
            Ok(rl.update_prompt(prompt)?)
        })?;
    }

    loop {
        tokio::select! {
            cmd = rl.readline() => match cmd {
                Ok(ReadlineEvent::Line(line)) => {
                    let line = line.trim().to_string();

                    // Process special commands
                    let should_exit = process_special_command(
                        &line,
                        &mut buffer,
                        &mut stdout,
                        |prompt| Ok(rl.update_prompt(prompt)?),
                        &interpreter,
                    )?;

                    if should_exit {
                        break;
                    }

                    if line.starts_with('.') {
                        continue;
                    }

                    rl.add_history_entry(line.clone());

                    // Process input (single line mode removed; always multiline)
                    let command_option = process_multiline_input(
                        line,
                        &mut buffer,
                        |prompt| Ok(rl.update_prompt(prompt)?),
                    )?;

                    // Execute command if one is ready
                    if let Some(command) = command_option {
                        writeln!(stdout, "{} {command}", label_info("Executing code:"))?;
                        let result = interpreter.interpret(&command).await;
                        match result {
                            InterpretationResult::Success(output) => {
                                let rendered = if is_tty_stdout() { colorize_ast_tree(&output, true) } else { output };
                                writeln!(stdout, "{} {}", label_ok("Output:"), rendered)?
                            }
                            InterpretationResult::Error(e) => writeln!(stdout, "{} {e}", label_err_out("Error interpreting line:"))?,
                        }
                    }
                }
                Ok(ReadlineEvent::Eof) => {
                    break;
                }
                Ok(ReadlineEvent::Interrupted) => {
                    handle_interrupt(
                        &mut buffer,
                        &mut stdout,
                        |prompt| Ok(rl.update_prompt(prompt)?),
                        &interpreter,
                    )?;
                    continue;
                }
                Err(e) => {
                    writeln!(stdout, "{} {e:?}", label_err_out("Error:"))?;
                    break;
                }
            }
        }
    }
    rl.flush()?;
    Ok(())
}

// ---- Validation support using rholang-lib validators ----
#[derive(Copy, Clone, Debug)]
enum ValidationMode {
    All,
    UnusedOnly,
    ElabOnly,
    ResolverOnly,
}

fn print_diagnostics<W: Write>(
    stdout: &mut W,
    diags: &[librho::sem::Diagnostic],
    header: &str,
) -> Result<()> {
    if diags.is_empty() {
        writeln!(stdout, "Validation successful: no issues found")?;
        return Ok(());
    }
    writeln!(stdout, "{} {} diagnostic(s):", header, diags.len())?;
    for (i, d) in diags.iter().enumerate() {
        use librho::sem::DiagnosticKind;
        let kind = match d.kind {
            DiagnosticKind::Error(_) => "Error",
            DiagnosticKind::Warning(_) => "Warning",
            DiagnosticKind::Info(_) => "Info",
        };
        writeln!(
            stdout,
            "  {}. {} at pid {}{}: {:?}",
            i + 1,
            kind,
            d.pid,
            match d.exact_position {
                Some(pos) => format!(" @{}:{}", pos.line, pos.col),
                None => String::new(),
            },
            d.kind
        )?;
    }
    Ok(())
}

fn run_validation_subset<W: Write>(code: &str, stdout: &mut W, mode: ValidationMode) -> Result<()> {
    use librho::sem::{
        diagnostics::UnusedVarsPass, DiagnosticPass, FactPass, ForCompElaborationPass,
        ResolverPass, SemanticDb,
    };
    use rholang_parser::RholangParser;

    // Parse code safely using Validated without panicking
    let parser = RholangParser::new();
    let validated = parser.parse(code);

    let ast_vec = match validated {
        validated::Validated::Good(ast) => ast,
        validated::Validated::Fail(_err) => {
            writeln!(
                stdout,
                "Parsing failed: unable to build AST. Please fix syntax errors and try again."
            )?;
            return Ok(());
        }
    };

    if ast_vec.is_empty() {
        writeln!(stdout, "No code to validate (empty AST)")?;
        return Ok(());
    }

    let mut db = SemanticDb::new();

    // Index and validate each top-level proc independently
    for proc in ast_vec.iter() {
        let root = db.build_index(proc);
        // Always run resolver first; it may emit diagnostics itself
        let resolver = ResolverPass::new(root);
        resolver.run(&mut db);

        match mode {
            ValidationMode::ResolverOnly => {
                // only resolver diagnostics, do nothing extra
            }
            ValidationMode::UnusedOnly => {
                let unused = UnusedVarsPass;
                let diags = unused.run(&db);
                db.push_diagnostics(diags);
            }
            ValidationMode::ElabOnly => {
                let forcomp = ForCompElaborationPass::new(root);
                forcomp.run(&mut db);
            }
            ValidationMode::All => {
                let unused = UnusedVarsPass;
                let diags = unused.run(&db);
                db.push_diagnostics(diags);
                let forcomp = ForCompElaborationPass::new(root);
                forcomp.run(&mut db);
            }
        }
    }

    let header = match mode {
        ValidationMode::All => "Validation produced",
        ValidationMode::UnusedOnly => "Unused-vars validation produced",
        ValidationMode::ElabOnly => "Elaboration validation produced",
        ValidationMode::ResolverOnly => "Resolver validation produced",
    };

    print_diagnostics(stdout, db.diagnostics(), header)
}

// (Disassembler functionality moved into InterpreterProvider::disassemble)

fn run_all_validators_on_code<W: Write>(code: &str, stdout: &mut W) -> Result<()> {
    run_validation_subset(code, stdout, ValidationMode::All)
}
