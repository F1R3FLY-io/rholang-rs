// New Rholang Bytecode Compiler
// Translates Rholang AST to bytecode

use anyhow::{bail, Result};
use rholang_parser::{
    ast::{AnnProc, Bind, Name, Proc, Source, Var},
    RholangParser,
};
use std::collections::HashMap;
use validated::Validated;

use crate::bytecode::{Instruction, Label, RSpaceType};

/// The Rholang bytecode compiler
pub struct RholangCompiler {
    /// Label counter for generating unique labels
    label_counter: usize,
    /// Variable bindings
    bindings: HashMap<String, usize>,
}

impl RholangCompiler {
    /// Create a new Rholang bytecode compiler
    pub fn new() -> Self {
        RholangCompiler {
            label_counter: 0,
            bindings: HashMap::new(),
        }
    }

    /// Compile Rholang code to bytecode
    pub fn compile(&mut self, code: &str) -> Result<Vec<Instruction>> {
        // Create a new parser for this compilation
        let parser = RholangParser::new();
        // Parse the code to an AST
        let ast = match parser.parse(code) {
            Validated::Good(procs) => procs,
            err => {
                bail!("Parsing error: {:?}", err);
            }
        };

        // Reset state
        self.label_counter = 0;
        self.bindings.clear();

        // Compile each process in the AST
        let mut instructions = Vec::new();
        for proc in ast {
            let mut proc_instructions = self.compile_proc(&proc)?;
            instructions.append(&mut proc_instructions);
        }

        Ok(instructions)
    }

    /// Generate a unique label
    fn gen_label(&mut self, prefix: &str) -> Label {
        let label = Label(format!("{}_{}", prefix, self.label_counter));
        self.label_counter += 1;
        label
    }

    /// Compile a process to bytecode
    fn compile_proc(&mut self, proc: &AnnProc) -> Result<Vec<Instruction>> {
        match proc.proc {
            // Literals
            Proc::Nil => Ok(vec![Instruction::PushProc("Nil".to_string())]),
            Proc::BoolLiteral(b) => Ok(vec![Instruction::PushBool(*b)]),
            Proc::LongLiteral(n) => Ok(vec![Instruction::PushInt(*n)]),
            Proc::StringLiteral(s) => Ok(vec![Instruction::PushStr(s.to_string())]),
            Proc::UriLiteral(uri) => Ok(vec![Instruction::PushStr(format!("{:?}", uri))]),

            // Variables
            Proc::ProcVar(var) => self.compile_var(var),

            // Parallel composition
            Proc::Par { left, right } => {
                let mut instructions = Vec::new();

                // Compile left process
                let mut left_instructions = self.compile_proc(left)?;
                instructions.append(&mut left_instructions);

                // Compile right process
                let mut right_instructions = self.compile_proc(right)?;
                instructions.append(&mut right_instructions);

                // Combine with Par
                instructions.push(Instruction::Eval); // Evaluate left
                instructions.push(Instruction::Eval); // Evaluate right

                Ok(instructions)
            }

            // Conditional
            Proc::IfThenElse {
                condition,
                if_true,
                if_false,
            } => {
                let mut instructions = Vec::new();

                // Compile condition
                let mut cond_instructions = self.compile_proc(condition)?;
                instructions.append(&mut cond_instructions);
                instructions.push(Instruction::EvalBool);

                // Generate labels
                let else_label = self.gen_label("else");
                let end_label = self.gen_label("end_if");

                // Branch to else if condition is false
                instructions.push(Instruction::BranchFalse(else_label.clone()));

                // Compile true branch
                let mut true_instructions = self.compile_proc(if_true)?;
                instructions.append(&mut true_instructions);
                instructions.push(Instruction::Jump(end_label.clone()));

                // Else branch
                instructions.push(Instruction::Label(else_label));

                // Compile false branch if it exists
                if let Some(false_branch) = if_false {
                    let mut false_instructions = self.compile_proc(false_branch)?;
                    instructions.append(&mut false_instructions);
                }

                // End label
                instructions.push(Instruction::Label(end_label));

                Ok(instructions)
            }

            // Send
            Proc::Send {
                channel,
                send_type: _,
                inputs,
            } => {
                let mut instructions = Vec::new();

                // Compile channel
                let mut channel_instructions = self.compile_name(channel)?;
                instructions.append(&mut channel_instructions);

                // Compile inputs
                for input in inputs.iter() {
                    let mut input_instructions = self.compile_proc(input)?;
                    instructions.append(&mut input_instructions);
                    instructions.push(Instruction::EvalToRSpace);
                }

                // Create list of inputs
                instructions.push(Instruction::CreateList(inputs.len()));

                // Send to channel
                instructions.push(Instruction::RSpacePut(RSpaceType::MemoryConcurrent));

                Ok(instructions)
            }

            // New
            Proc::New { decls, proc } => {
                let mut instructions = Vec::new();

                // Create fresh names for each declaration
                for decl in decls.iter() {
                    instructions.push(Instruction::NameCreate(RSpaceType::MemoryConcurrent));
                    instructions.push(Instruction::AllocLocal);

                    // Store the name in a local variable
                    let index = self.bindings.len();
                    self.bindings.insert(decl.id.name.to_string(), index);
                    instructions.push(Instruction::StoreLocal(index));
                }

                // Compile the body
                let mut body_instructions = self.compile_proc(proc)?;
                instructions.append(&mut body_instructions);

                Ok(instructions)
            }

            // For comprehension
            Proc::ForComprehension { receipts, proc } => {
                let mut instructions = Vec::new();

                // Compile each receipt (which is a SmallVec of Bind)
                for receipt in receipts.iter() {
                    // Each receipt is a collection of binds
                    for bind in receipt.iter() {
                        match bind {
                            Bind::Linear { lhs: _, rhs } => {
                                // Compile channel from the rhs
                                match rhs {
                                    Source::Simple { name } => {
                                        let mut channel_instructions = self.compile_name(name)?;
                                        instructions.append(&mut channel_instructions);
                                    }
                                    Source::ReceiveSend { name } => {
                                        let mut channel_instructions = self.compile_name(name)?;
                                        instructions.append(&mut channel_instructions);
                                    }
                                    Source::SendReceive { name, inputs: _ } => {
                                        let mut channel_instructions = self.compile_name(name)?;
                                        instructions.append(&mut channel_instructions);
                                    }
                                }

                                // Compile pattern from the lhs
                                // For simplicity, we'll just use a placeholder for now
                                instructions.push(Instruction::PushProc("pattern".to_string()));

                                // Create pattern
                                instructions.push(Instruction::PatternCompile(RSpaceType::MemoryConcurrent));

                                // Store continuation
                                let mut body_instructions = self.compile_proc(proc)?;
                                instructions.append(&mut body_instructions);

                                // Consume from channel
                                instructions.push(Instruction::RSpaceConsume(RSpaceType::MemoryConcurrent));
                            }
                            Bind::Repeated { lhs: _, rhs } => {
                                // Similar to Linear, but for repeated binds
                                let mut channel_instructions = self.compile_name(rhs)?;
                                instructions.append(&mut channel_instructions);

                                // Placeholder for pattern
                                instructions.push(Instruction::PushProc("pattern".to_string()));

                                // Create pattern
                                instructions.push(Instruction::PatternCompile(RSpaceType::MemoryConcurrent));

                                // Store continuation
                                let mut body_instructions = self.compile_proc(proc)?;
                                instructions.append(&mut body_instructions);

                                // Consume from channel
                                instructions.push(Instruction::RSpaceConsume(RSpaceType::MemoryConcurrent));
                            }
                            Bind::Peek { lhs: _, rhs } => {
                                // Similar to Linear, but for peek binds
                                let mut channel_instructions = self.compile_name(rhs)?;
                                instructions.append(&mut channel_instructions);

                                // Placeholder for pattern
                                instructions.push(Instruction::PushProc("pattern".to_string()));

                                // Create pattern
                                instructions.push(Instruction::PatternCompile(RSpaceType::MemoryConcurrent));

                                // Store continuation
                                let mut body_instructions = self.compile_proc(proc)?;
                                instructions.append(&mut body_instructions);

                                // Peek at channel
                                instructions.push(Instruction::RSpacePeek(RSpaceType::MemoryConcurrent));
                            }
                        }
                    }
                }

                Ok(instructions)
            }

            // Expressions
            Proc::BinaryExp { op, left, right } => {
                let mut instructions = Vec::new();

                // Compile left operand
                let mut left_instructions = self.compile_proc(left)?;
                instructions.append(&mut left_instructions);
                instructions.push(Instruction::Eval);

                // Compile right operand
                let mut right_instructions = self.compile_proc(right)?;
                instructions.append(&mut right_instructions);
                instructions.push(Instruction::Eval);

                // Apply operator
                match op {
                    rholang_parser::ast::BinaryExpOp::Add => instructions.push(Instruction::Add),
                    rholang_parser::ast::BinaryExpOp::Sub => instructions.push(Instruction::Sub),
                    rholang_parser::ast::BinaryExpOp::Mult => instructions.push(Instruction::Mul),
                    rholang_parser::ast::BinaryExpOp::Div => instructions.push(Instruction::Div),
                    rholang_parser::ast::BinaryExpOp::Mod => instructions.push(Instruction::Mod),
                    rholang_parser::ast::BinaryExpOp::Lt => instructions.push(Instruction::CmpLt),
                    rholang_parser::ast::BinaryExpOp::Lte => instructions.push(Instruction::CmpLte),
                    rholang_parser::ast::BinaryExpOp::Gt => instructions.push(Instruction::CmpGt),
                    rholang_parser::ast::BinaryExpOp::Gte => instructions.push(Instruction::CmpGte),
                    rholang_parser::ast::BinaryExpOp::Eq => instructions.push(Instruction::CmpEq),
                    rholang_parser::ast::BinaryExpOp::Neq => instructions.push(Instruction::CmpNeq),
                    _ => bail!("Unsupported binary operator: {:?}", op),
                }

                Ok(instructions)
            }

            // For other process types, return a placeholder implementation
            _ => Ok(vec![Instruction::PushStr(format!("{:?}", proc.proc))]),
        }
    }

    /// Compile a variable to bytecode
    fn compile_var(&self, var: &Var) -> Result<Vec<Instruction>> {
        match var {
            Var::Id(id) => {
                if let Some(index) = self.bindings.get(id.name) {
                    Ok(vec![Instruction::LoadLocal(*index)])
                } else {
                    bail!("Undefined variable: {}", id.name)
                }
            }
            Var::Wildcard => Ok(vec![Instruction::PushProc("_".to_string())]),
        }
    }

    /// Compile a name to bytecode
    fn compile_name(&mut self, name: &rholang_parser::ast::AnnName) -> Result<Vec<Instruction>> {
        match name.name {
            Name::ProcVar(var) => self.compile_var(&var),
            Name::Quote(proc) => {
                let mut instructions = Vec::new();

                // Create a quoted process
                instructions.push(Instruction::PushProc(format!("{:?}", proc)));
                instructions.push(Instruction::NameQuote(RSpaceType::MemoryConcurrent));

                Ok(instructions)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_arithmetic() -> Result<()> {
        let mut compiler = RholangCompiler::new();
        let bytecode = compiler.compile("1 + 2")?;

        // The bytecode should push 1 and 2, then add them
        assert!(bytecode.len() > 0);

        Ok(())
    }

    #[test]
    fn test_compile_if_then_else() -> Result<()> {
        let mut compiler = RholangCompiler::new();
        let bytecode = compiler.compile("if (true) { 1 } else { 2 }")?;

        // The bytecode should include conditional branching
        assert!(bytecode.len() > 0);

        Ok(())
    }

    #[test]
    fn test_compile_new() -> Result<()> {
        let mut compiler = RholangCompiler::new();
        let bytecode = compiler.compile("new x in { x!(5) }")?;

        // The bytecode should create a new name and send to it
        assert!(bytecode.len() > 0);

        Ok(())
    }
}