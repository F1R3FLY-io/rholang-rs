use crate::error::{CompileError, CompileErrorInfo};
use librho::sem::{Diagnostic, DiagnosticKind, SemanticDb};
use rholang_parser::SourcePos;
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct ReporterConfig {
    /// Number of context lines to show before/after error
    pub context_lines: usize,
}

impl Default for ReporterConfig {
    fn default() -> Self {
        Self { context_lines: 2 }
    }
}

/// Error reporter for displaying compilation diagnostics
pub struct ErrorReporter {
    config: ReporterConfig,
}

impl ErrorReporter {
    pub fn new(config: ReporterConfig) -> Self {
        Self { config }
    }

    /// Format a compile error with source context
    pub fn format_error(
        &self,
        error: &CompileError,
        source: &str,
        filename: Option<&str>,
    ) -> String {
        let mut output = String::new();

        match error {
            CompileError::ParseError(msg) => {
                let _ = writeln!(&mut output, "error: {}", msg);
            }
            CompileError::SemanticErrors(errors) => {
                for (i, err) in errors.iter().enumerate() {
                    if i > 0 {
                        output.push('\n');
                    }
                    self.format_single_error(&mut output, err, source, filename);
                }

                // Summary
                let count = errors.len();
                let _ = writeln!(
                    &mut output,
                    "\nerror: {} error{} emitted",
                    count,
                    if count == 1 { "" } else { "s" }
                );
            }
            CompileError::CodegenError(e) => {
                let _ = writeln!(&mut output, "error: code generation failed\n  {}", e);
            }
            CompileError::InternalError(msg) => {
                let _ = writeln!(
                    &mut output,
                    "error: internal compiler error\n  {}\n  \
                    This is a bug in the compiler, please report it",
                    msg
                );
            }
        }

        output
    }

    /// Format a warning diagnostic with source context
    pub fn format_warning(
        &self,
        warning: &Diagnostic,
        _db: &SemanticDb,
        source: &str,
        filename: Option<&str>,
    ) -> String {
        let mut output = String::new();

        let file = filename.unwrap_or("<input>");

        // Extract warning message
        let message = match &warning.kind {
            DiagnosticKind::Warning(kind) => format!("{:?}", kind), // TODO: Better formatting
            _ => return output,
        };

        // Warning header
        if let Some(pos) = warning.exact_position {
            let _ = writeln!(&mut output, "warning: {}", message);
            let _ = writeln!(
                &mut output,
                "  --> {}:{}:{}",
                file,
                pos.line + 1,
                pos.col + 1
            );
            self.render_source_context(&mut output, source, pos, None);
        } else {
            let _ = writeln!(&mut output, "warning: {}", message);
        }

        output
    }

    fn format_single_error(
        &self,
        output: &mut String,
        error: &CompileErrorInfo,
        source: &str,
        filename: Option<&str>,
    ) {
        let file = filename.unwrap_or("<input>");

        // Error header
        let _ = writeln!(output, "error: {}", error.message);

        // Use span if available, otherwise position
        match (error.span, error.position) {
            (Some(span), _) => {
                let _ = writeln!(
                    output,
                    "  --> {}:{}:{}",
                    file,
                    span.start.line + 1,
                    span.start.col + 1
                );
                self.render_source_context(output, source, span.start, Some(span.end));
            }
            (None, Some(pos)) => {
                let _ = writeln!(output, "  --> {}:{}:{}", file, pos.line + 1, pos.col + 1);
                self.render_source_context(output, source, pos, None);
            }
            (None, None) => {
                let _ = writeln!(output, "  --> {}", file);
            }
        }
    }

    fn render_source_context(
        &self,
        output: &mut String,
        source: &str,
        start_pos: SourcePos,
        end_pos: Option<SourcePos>,
    ) {
        let lines: Vec<&str> = source.lines().collect();
        let line_num = start_pos.line;

        if line_num >= lines.len() {
            return;
        }

        // Calculate line number width for alignment
        let max_line = (line_num + self.config.context_lines + 1).min(lines.len());
        let line_width = max_line.to_string().len();

        // Context lines before
        let start = line_num.saturating_sub(self.config.context_lines);
        for i in start..line_num {
            let _ = writeln!(
                output,
                "{:>width$} | {}",
                i + 1,
                lines[i],
                width = line_width
            );
        }

        // Error line
        let error_line = lines[line_num];
        let _ = writeln!(
            output,
            "{:>width$} | {}",
            line_num + 1,
            error_line,
            width = line_width
        );

        // Error pointer/underline - UTF-8 safe
        if let Some(end_pos) = end_pos {
            if start_pos.line == end_pos.line {
                // Single line span - underline the range
                let start_char = error_line[..(start_pos.col).min(error_line.len())]
                    .chars()
                    .count();
                let end_char = error_line[..(end_pos.col).min(error_line.len())]
                    .chars()
                    .count();
                let pointer_padding = " ".repeat(start_char);
                let underline = "^".repeat((end_char - start_char).max(1));

                let _ = writeln!(
                    output,
                    "{:>width$} | {}{}",
                    "",
                    pointer_padding,
                    underline,
                    width = line_width
                );
            } else {
                // Multi-line span - just use caret at start for now
                self.use_caret(output, error_line, start_pos, line_width);
            }
        } else {
            // Single position - use caret (UTF-8 safe)
            self.use_caret(output, error_line, start_pos, line_width);
        }

        // Context lines after
        let end = (line_num + 1 + self.config.context_lines).min(lines.len());
        for i in (line_num + 1)..end {
            let _ = writeln!(
                output,
                "{:>width$} | {}",
                i + 1,
                lines[i],
                width = line_width
            );
        }
    }

    fn use_caret(&self, output: &mut String, error_line: &str, start_pos: SourcePos, line_width: usize) {
        let char_col = error_line[..start_pos.col.min(error_line.len())]
            .chars()
            .count();
        let pointer_padding = " ".repeat(char_col);
        let _ = writeln!(
            output,
            "{:>width$} | {}^",
            "",
            pointer_padding,
            width = line_width
        );
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new(ReporterConfig::default())
    }
}
