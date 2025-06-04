/// Detects if a string contains two consecutive newlines
pub fn has_double_newline(s: &str) -> bool {
    s.contains("\n\n")
}

/// Processes a line of input and updates the buffer and multiline state
pub fn process_line(line: &str, buffer: &mut String, in_multiline_mode: &mut bool) -> bool {
    // Add the line to our buffer with a newline
    if !buffer.is_empty() {
        buffer.push('\n');
    }
    buffer.push_str(line);

    // Check if we have a double newline or if we're entering multiline mode
    if !*in_multiline_mode && line.is_empty() {
        // Start multiline mode
        *in_multiline_mode = true;
        false // Not ready to execute
    } else if *in_multiline_mode && line.is_empty() && buffer.ends_with("\n\n") {
        // Execute the command when we get two consecutive empty lines
        // Trim the trailing double newlines
        *buffer = buffer.trim_end_matches('\n').to_string();

        // Reset multiline mode
        *in_multiline_mode = false;
        true // Ready to execute
    } else {
        false // Not ready to execute
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_double_newline() {
        assert!(!has_double_newline("line one"));
        assert!(!has_double_newline("line one\nline two"));
        assert!(has_double_newline("line one\n\nline three"));
    }

    #[test]
    fn test_process_line_single_line() {
        let mut buffer = String::new();
        let mut multiline = false;

        let execute = process_line("println(\"hello\")", &mut buffer, &mut multiline);

        assert_eq!(buffer, "println(\"hello\")");
        assert_eq!(multiline, false);
        assert_eq!(execute, false);
    }

    #[test]
    fn test_process_line_enter_multiline() {
        let mut buffer = String::new();
        let mut multiline = false;

        let execute = process_line("println(\"hello\")", &mut buffer, &mut multiline);
        assert_eq!(execute, false);

        let execute = process_line("", &mut buffer, &mut multiline);

        assert_eq!(buffer, "println(\"hello\")\n");
        assert_eq!(multiline, true);
        assert_eq!(execute, false);
    }

    #[test]
    fn test_process_line_execute_multiline() {
        let mut buffer = String::new();
        let mut multiline = false;

        process_line("println(\"hello\")", &mut buffer, &mut multiline);
        process_line("", &mut buffer, &mut multiline);
        process_line("println(\"world\")", &mut buffer, &mut multiline);

        assert_eq!(multiline, true);

        let execute = process_line("", &mut buffer, &mut multiline);

        assert_eq!(buffer, "println(\"hello\")\n\nprintln(\"world\")");
        assert_eq!(multiline, false);
        assert_eq!(execute, true);
    }
}
