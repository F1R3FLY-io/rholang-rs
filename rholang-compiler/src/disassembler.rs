//! Bytecode disassembler for Rholang
//!
//! Provides multiple output formats for inspecting compiled bytecode:
//! - Compact: Simple instruction listing
//! - Verbose: Detailed with addresses and comments
//! - Assembly: Assembly-like format with sections
//! - Hexdump: Raw hex bytes with instruction annotations
//!
//! ## Example
//!
//! ```ignore
//! use rholang_compiler::{Disassembler, DisassemblyFormat};
//!
//! let disasm = Disassembler::new()
//!     .show_hex(true)
//!     .show_comments(true);
//!
//! // let output = disasm.disassemble(&process);
//! // println!("{}", output);
//! ```

use rholang_bytecode::core::instructions::Instruction;
use rholang_bytecode::core::opcodes::Opcode;
use rholang_process::Process;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisassemblyFormat {
    /// Compact format: "PUSH_INT 42"
    Compact,

    /// Verbose format with addresses and comments
    /// "0000: PUSH_INT 0x002a  ; Push integer 42"
    Verbose,

    /// Assembly-like format with sections
    /// .string_pool / .code / .data
    Assembly,

    /// Raw hexadecimal dump
    /// "0000: 01 00 2a 00"
    Hexdump,
}

#[derive(Debug, Clone)]
pub struct DisassemblerConfig {
    pub format: DisassemblyFormat,
    pub show_addresses: bool,
    pub show_string_pool: bool,
    pub show_hex: bool,
    pub show_comments: bool,
    pub use_colors: bool,
    pub show_metadata: bool,
}

impl Default for DisassemblerConfig {
    fn default() -> Self {
        Self {
            format: DisassemblyFormat::Verbose,
            show_addresses: true,
            show_string_pool: true,
            show_hex: false,
            show_comments: true,
            use_colors: false,
            show_metadata: true,
        }
    }
}

pub struct Disassembler {
    config: DisassemblerConfig,
}

impl Disassembler {
    pub fn new() -> Self {
        Self {
            config: DisassemblerConfig::default(),
        }
    }

    pub fn with_format(format: DisassemblyFormat) -> Self {
        Self {
            config: DisassemblerConfig {
                format,
                ..Default::default()
            },
        }
    }

    pub fn show_addresses(mut self, show: bool) -> Self {
        self.config.show_addresses = show;
        self
    }

    pub fn show_string_pool(mut self, show: bool) -> Self {
        self.config.show_string_pool = show;
        self
    }

    pub fn show_hex(mut self, show: bool) -> Self {
        self.config.show_hex = show;
        self
    }

    pub fn show_comments(mut self, show: bool) -> Self {
        self.config.show_comments = show;
        self
    }

    pub fn use_colors(mut self, use_colors: bool) -> Self {
        self.config.use_colors = use_colors;
        self
    }

    pub fn show_metadata(mut self, show: bool) -> Self {
        self.config.show_metadata = show;
        self
    }

    pub fn disassemble(&self, process: &Process) -> String {
        match self.config.format {
            DisassemblyFormat::Compact => self.format_compact(process),
            DisassemblyFormat::Verbose => self.format_verbose(process),
            DisassemblyFormat::Assembly => self.format_assembly(process),
            DisassemblyFormat::Hexdump => self.format_hexdump(process),
        }
    }

    /// Disassemble to a writer (for file output)
    pub fn disassemble_to_writer(
        &self,
        process: &Process,
        writer: &mut dyn Write,
    ) -> std::io::Result<()> {
        write!(writer, "{}", self.disassemble(process))
    }

    pub fn disassemble_to_file(&self, process: &Process, path: &Path) -> std::io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        self.disassemble_to_writer(process, &mut file)
    }

    fn format_compact(&self, process: &Process) -> String {
        let mut output = String::new();

        for inst in &process.code {
            output.push_str(&format!("{:?}\n", inst));
        }

        output
    }

    fn format_verbose(&self, process: &Process) -> String {
        let mut output = String::new();

        // Header
        if self.config.show_metadata {
            output.push_str(&format!("Process: {}\n", process.source_ref));
            output.push_str(&format!("Instructions: {}\n\n", process.code.len()));
        }

        // String pool
        if self.config.show_string_pool && !process.names.is_empty() {
            output.push_str("String Pool:\n");
            for (idx, name) in process.names.iter().enumerate() {
                output.push_str(&format!("  [{}]: {:?}\n", idx, name));
            }
            output.push('\n');
        }

        // Instructions with addresses and optional comments
        output.push_str("Bytecode:\n");
        for (addr, inst) in process.code.iter().enumerate() {
            if self.config.show_addresses {
                output.push_str(&format!("  {:04}: ", addr));
            } else {
                output.push_str("  ");
            }

            if self.config.show_hex {
                let bytes = inst.to_bytes();
                output.push_str(&format!(
                    "{:02x} {:02x} {:02x} {:02x}  ",
                    bytes[0], bytes[1], bytes[2], bytes[3]
                ));
            }

            output.push_str(&format!("{:?}", inst));

            if self.config.show_comments {
                if let Ok(comment) = self.get_instruction_comment(inst) {
                    output.push_str(&format!("  ; {}", comment));
                }
            }

            output.push('\n');
        }

        output
    }

    fn format_assembly(&self, process: &Process) -> String {
        let mut output = String::new();

        // Assembly-like format with sections
        if self.config.show_metadata {
            output.push_str(&format!("; Process: {}\n\n", process.source_ref));
        }

        // String pool section
        if !process.names.is_empty() {
            output.push_str(".string_pool\n");
            for (idx, name) in process.names.iter().enumerate() {
                output.push_str(&format!("  str_{}: {:?}\n", idx, name));
            }
            output.push('\n');
        }

        // Code section
        output.push_str(".code\n");
        for (addr, inst) in process.code.iter().enumerate() {
            output.push_str(&format!("  L{:04}:  {:?}\n", addr, inst));
        }

        output
    }

    fn format_hexdump(&self, process: &Process) -> String {
        let mut output = String::new();

        if self.config.show_metadata {
            output.push_str(&format!("Process: {}\n", process.source_ref));
            output.push_str(&format!(
                "Size: {} instructions ({} bytes)\n\n",
                process.code.len(),
                process.code.len() * 4
            ));
        }

        for (addr, inst) in process.code.iter().enumerate() {
            let bytes = inst.to_bytes();
            output.push_str(&format!(
                "{:04}:  {:02x} {:02x} {:02x} {:02x}  |{:?}|\n",
                addr * 4,
                bytes[0],
                bytes[1],
                bytes[2],
                bytes[3],
                inst
            ));
        }

        output
    }

    /// Get a readable comment for an instruction
    fn get_instruction_comment(&self, inst: &Instruction) -> Result<String, ()> {
        let opcode = inst.opcode().map_err(|_| ())?;

        let comment = match opcode {
            // Stack operations
            Opcode::PUSH_INT => {
                let value = inst.op16() as i16;
                format!("Push integer {}", value)
            }
            Opcode::PUSH_BOOL => format!("Push boolean {}", inst.op16() != 0),
            Opcode::PUSH_STR => format!("Push string at index {}", inst.op16()),
            Opcode::PUSH_PROC => format!("Push process at index {}", inst.op16()),
            Opcode::PUSH_NAME => format!("Push name at index {}", inst.op16()),
            Opcode::PUSH_NIL => "Push nil".to_string(),
            Opcode::POP => "Pop top of stack".to_string(),
            Opcode::DUP => "Duplicate top of stack".to_string(),
            Opcode::SWAP => "Swap top two stack values".to_string(),

            // Variable operations
            Opcode::LOAD_VAR => format!("Load variable #{}", inst.op16()),
            Opcode::LOAD_LOCAL => format!("Load local variable #{}", inst.op16()),
            Opcode::STORE_LOCAL => format!("Store to local variable #{}", inst.op16()),
            Opcode::ALLOC_LOCAL => "Allocate local variable".to_string(),
            Opcode::LOAD_ENV => format!("Load from environment slot #{}", inst.op16()),
            Opcode::STORE_ENV => format!("Store to environment slot #{}", inst.op16()),

            // Arithmetic operations
            Opcode::ADD => "Add top two stack values".to_string(),
            Opcode::SUB => "Subtract top two stack values".to_string(),
            Opcode::MUL => "Multiply top two stack values".to_string(),
            Opcode::DIV => "Divide top two stack values".to_string(),
            Opcode::MOD => "Modulo top two stack values".to_string(),
            Opcode::NEG => "Negate top stack value".to_string(),

            // Comparison operations
            Opcode::CMP_EQ => "Compare equal".to_string(),
            Opcode::CMP_NEQ => "Compare not equal".to_string(),
            Opcode::CMP_LT => "Compare less than".to_string(),
            Opcode::CMP_LTE => "Compare less than or equal".to_string(),
            Opcode::CMP_GT => "Compare greater than".to_string(),
            Opcode::CMP_GTE => "Compare greater than or equal".to_string(),

            // Logical operations
            Opcode::NOT => "Logical NOT".to_string(),
            Opcode::AND => "Logical AND".to_string(),
            Opcode::OR => "Logical OR".to_string(),

            // Control flow
            Opcode::NOP => "No operation".to_string(),
            Opcode::JUMP => format!("Jump to instruction {}", inst.op16()),
            Opcode::BRANCH_TRUE => format!("Branch to {} if true", inst.op16()),
            Opcode::BRANCH_FALSE => format!("Branch to {} if false", inst.op16()),
            Opcode::BRANCH_SUCCESS => format!("Branch to {} on success", inst.op16()),
            Opcode::RETURN => "Return from current context".to_string(),
            Opcode::HALT => "Halt execution".to_string(),

            // Collection operations
            Opcode::CREATE_LIST => format!("Create list with {} elements", inst.op16()),
            Opcode::CREATE_TUPLE => format!("Create tuple with {} elements", inst.op16()),
            Opcode::CREATE_MAP => format!("Create map with {} pairs", inst.op16()),
            Opcode::CONCAT => "Concatenate collections".to_string(),
            Opcode::DIFF => "Set difference".to_string(),
            Opcode::INTERPOLATE => "String interpolation".to_string(),

            // Process operations
            Opcode::SPAWN_ASYNC => "Spawn asynchronous process".to_string(),
            Opcode::EVAL => "Evaluate (unquote) name".to_string(),
            Opcode::EVAL_BOOL => "Evaluate as boolean".to_string(),
            Opcode::EVAL_STAR => "Evaluate all (splat)".to_string(),
            Opcode::EXEC => "Execute process".to_string(),
            Opcode::PROC_NEG => "Process negation".to_string(),

            // RSpace operations
            Opcode::NAME_CREATE => format!("Create name with kind {}", inst.op16()),
            Opcode::NAME_QUOTE => "Quote name".to_string(),
            Opcode::NAME_UNQUOTE => "Unquote name".to_string(),
            Opcode::TELL => format!("Send on channel (kind: {})", inst.op1()),
            Opcode::ASK => format!("Receive from channel (kind: {})", inst.op1()),
            Opcode::ASK_NB => "Non-blocking receive".to_string(),
            Opcode::PEEK => "Peek at channel".to_string(),
            Opcode::CONT_STORE => "Store continuation".to_string(),
            Opcode::CONT_RESUME => "Resume continuation".to_string(),
            Opcode::BUNDLE_BEGIN => "Begin bundle".to_string(),
            Opcode::BUNDLE_END => "End bundle".to_string(),

            // Pattern matching
            Opcode::PATTERN => "Pattern match".to_string(),
            Opcode::MATCH_TEST => "Test pattern match".to_string(),
            Opcode::EXTRACT_BINDINGS => "Extract pattern bindings".to_string(),

            // Reference operations
            Opcode::COPY => "Copy value".to_string(),
            Opcode::MOVE => "Move value".to_string(),
            Opcode::REF => "Create reference".to_string(),

            // Method operations
            Opcode::LOAD_METHOD => format!("Load method #{}", inst.op16()),
            Opcode::INVOKE_METHOD => format!("Invoke method with {} args", inst.op16()),
        };

        Ok(comment)
    }
}

impl Default for Disassembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rholang_bytecode::core::instructions::Instruction;
    use rholang_bytecode::core::opcodes::Opcode;
    use rholang_vm::api::{Process, Value};

    fn create_test_process() -> Process {
        let mut process = Process::new(
            vec![
                Instruction::unary(Opcode::PUSH_INT, 42),
                Instruction::unary(Opcode::PUSH_INT, 8),
                Instruction::nullary(Opcode::ADD),
                Instruction::nullary(Opcode::HALT),
            ],
            "test_proc",
        );
        process.names = vec![Value::Str("hello".to_string())];
        process
    }

    #[test]
    fn test_compact_format() {
        let process = create_test_process();
        let disasm = Disassembler::with_format(DisassemblyFormat::Compact);
        let output = disasm.disassemble(&process);

        assert!(output.contains("PUSH_INT"));
        assert!(output.contains("ADD"));
        assert!(output.contains("HALT"));
        assert!(!output.contains("0000:")); // No addresses in compact
    }

    #[test]
    fn test_verbose_format() {
        let process = create_test_process();
        let disasm = Disassembler::with_format(DisassemblyFormat::Verbose);
        let output = disasm.disassemble(&process);

        assert!(output.contains("0000:")); // Addresses
        assert!(output.contains("Process:")); // Metadata
        assert!(output.contains("String Pool:")); // String pool
        assert!(output.contains("hello"));
    }

    #[test]
    fn test_verbose_without_addresses() {
        let process = create_test_process();
        let disasm = Disassembler::with_format(DisassemblyFormat::Verbose).show_addresses(false);
        let output = disasm.disassemble(&process);

        assert!(!output.contains("0000:"));
        assert!(output.contains("PUSH_INT"));
    }

    #[test]
    fn test_verbose_with_hex() {
        let process = create_test_process();
        let disasm = Disassembler::with_format(DisassemblyFormat::Verbose).show_hex(true);
        let output = disasm.disassemble(&process);

        // Should contain hex bytes
        assert!(output.contains("10 00")); // PUSH_INT opcode
    }

    #[test]
    fn test_verbose_with_comments() {
        let process = create_test_process();
        let disasm = Disassembler::with_format(DisassemblyFormat::Verbose).show_comments(true);
        let output = disasm.disassemble(&process);

        assert!(output.contains("; Push integer 42"));
        assert!(output.contains("; Add top two stack values"));
    }

    #[test]
    fn test_assembly_format() {
        let process = create_test_process();
        let disasm = Disassembler::with_format(DisassemblyFormat::Assembly);
        let output = disasm.disassemble(&process);

        assert!(output.contains(".code"));
        assert!(output.contains(".string_pool"));
        assert!(output.contains("L0000:"));
        assert!(output.contains("str_0:"));
    }

    #[test]
    fn test_hexdump_format() {
        let process = create_test_process();
        let disasm = Disassembler::with_format(DisassemblyFormat::Hexdump);
        let output = disasm.disassemble(&process);

        // Check for hex bytes
        assert!(output.contains("10 00")); // PUSH_INT opcode
        assert!(output.contains("30 00")); // ADD opcode
        assert!(output.contains("|PUSH_INT")); // Should have instruction annotation
    }

    #[test]
    fn test_builder_pattern() {
        let disasm = Disassembler::new()
            .show_hex(true)
            .show_comments(false)
            .show_addresses(true);

        assert!(disasm.config.show_hex);
        assert!(!disasm.config.show_comments);
        assert!(disasm.config.show_addresses);
    }

    #[test]
    fn test_empty_string_pool() {
        let process = Process::new(vec![Instruction::nullary(Opcode::PUSH_NIL)], "empty");

        let disasm = Disassembler::new();
        let output = disasm.disassemble(&process);

        assert!(!output.contains("String Pool:")); // Empty pool should not appear
    }

    #[test]
    fn test_file_output() {
        use std::env::temp_dir;

        let process = create_test_process();
        let disasm = Disassembler::new();
        let temp_file = temp_dir().join("test_disasm_output.bc");

        disasm
            .disassemble_to_file(&process, &temp_file)
            .expect("Failed to write file");

        let content = std::fs::read_to_string(&temp_file).expect("Failed to read file");
        assert!(content.contains("Process:"));
        assert!(content.contains("PUSH_INT"));

        std::fs::remove_file(&temp_file).expect("Failed to cleanup");
    }

    #[test]
    fn test_default_config() {
        let config = DisassemblerConfig::default();

        assert_eq!(config.format, DisassemblyFormat::Verbose);
        assert!(config.show_addresses);
        assert!(config.show_string_pool);
        assert!(!config.show_hex);
        assert!(config.show_comments);
        assert!(!config.use_colors);
        assert!(config.show_metadata);
    }

    #[test]
    fn test_metadata_disabled() {
        let process = create_test_process();
        let disasm = Disassembler::with_format(DisassemblyFormat::Verbose).show_metadata(false);
        let output = disasm.disassemble(&process);

        assert!(!output.contains("Process:"));
        assert!(!output.contains("Instructions:"));
    }

    #[test]
    fn test_all_instruction_comments() {
        let disasm = Disassembler::new();

        let test_cases = vec![
            (Opcode::PUSH_INT, "Push integer"),
            (Opcode::ADD, "Add top two"),
            (Opcode::JUMP, "Jump to"),
            (Opcode::NAME_CREATE, "Create name"),
            (Opcode::HALT, "Halt execution"),
        ];

        for (opcode, expected_substr) in test_cases {
            let inst = Instruction::unary(opcode, 0);
            let comment = disasm.get_instruction_comment(&inst);
            assert!(comment.is_ok());
            assert!(comment.unwrap().contains(expected_substr));
        }
    }
}
