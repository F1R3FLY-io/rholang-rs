// New minimal Rholang VM scaffold based on rholang-bytecode
// This replaces the previous implementation. The full feature set will be rebuilt incrementally.

use anyhow::{bail, Result};
use rholang_bytecode::core::instructions::Instruction as CoreInst;
use rholang_bytecode::core::opcodes::Opcode;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Nil,
}

impl Value {
    fn as_int(&self) -> Option<i64> { if let Value::Int(n) = self { Some(*n) } else { None } }
}

pub struct VM {
    stack: Vec<Value>,
}

impl VM {
    pub fn new() -> Self { VM { stack: Vec::new() } }

    // Executor: adds collections and keeps arithmetic minimal
    pub fn execute(&mut self, program: &[CoreInst]) -> Result<Value> {
        let mut pc = 0usize;
        while pc < program.len() {
            let inst = &program[pc];
            let opcode = inst.opcode()?;
            match opcode {
                Opcode::NOP => { /* no-op */ }
                Opcode::HALT => { break; }
                Opcode::PUSH_INT => {
                    let imm = inst.op16() as i16 as i64; // sign-extend 16-bit immediate
                    self.stack.push(Value::Int(imm));
                }
                Opcode::PUSH_BOOL => {
                    let v = inst.op1() != 0; // encode bool in op1
                    self.stack.push(Value::Bool(v));
                }
                Opcode::POP => { let _ = self.stack.pop(); }

                // Arithmetic on Value::Int
                Opcode::ADD => {
                    let (b, a) = (self.stack.pop(), self.stack.pop());
                    match (a, b) {
                        (Some(Value::Int(a)), Some(Value::Int(b))) => self.stack.push(Value::Int(a + b)),
                        (Some(Value::Str(a)), Some(Value::Str(b))) => self.stack.push(Value::Str(a + &b)),
                        (Some(Value::List(mut a)), Some(Value::List(b))) => { a.extend(b); self.stack.push(Value::List(a)); }
                        _ => bail!("ADD type mismatch"),
                    }
                }
                Opcode::SUB => {
                    let (b, a) = (self.stack.pop(), self.stack.pop());
                    match (a, b) {
                        (Some(Value::Int(a)), Some(Value::Int(b))) => self.stack.push(Value::Int(a - b)),
                        _ => bail!("SUB requires Ints"),
                    }
                }
                Opcode::MUL => {
                    let (b, a) = (self.stack.pop(), self.stack.pop());
                    match (a, b) {
                        (Some(Value::Int(a)), Some(Value::Int(b))) => self.stack.push(Value::Int(a * b)),
                        _ => bail!("MUL requires Ints"),
                    }
                }
                Opcode::DIV => {
                    let (b, a) = (self.stack.pop(), self.stack.pop());
                    match (a, b) {
                        (Some(Value::Int(a)), Some(Value::Int(b))) => {
                            if b == 0 { bail!("division by zero"); }
                            self.stack.push(Value::Int(a / b))
                        }
                        _ => bail!("DIV requires Ints"),
                    }
                }
                Opcode::MOD => {
                    let (b, a) = (self.stack.pop(), self.stack.pop());
                    match (a, b) {
                        (Some(Value::Int(a)), Some(Value::Int(b))) => {
                            if b == 0 { bail!("modulo by zero"); }
                            self.stack.push(Value::Int(a % b))
                        }
                        _ => bail!("MOD requires Ints"),
                    }
                }
                Opcode::NEG => {
                    let a = self.stack.pop();
                    match a {
                        Some(Value::Int(a)) => self.stack.push(Value::Int(-a)),
                        _ => bail!("NEG requires Int"),
                    }
                }

                // Collections
                Opcode::CREATE_LIST => {
                    let n = inst.op16() as usize;
                    if self.stack.len() < n { bail!("stack underflow creating list"); }
                    let mut buf = Vec::with_capacity(n);
                    for _ in 0..n { buf.push(self.stack.pop().unwrap()); }
                    buf.reverse();
                    self.stack.push(Value::List(buf));
                }
                Opcode::CREATE_TUPLE => {
                    let n = inst.op16() as usize;
                    if self.stack.len() < n { bail!("stack underflow creating tuple"); }
                    let mut buf = Vec::with_capacity(n);
                    for _ in 0..n { buf.push(self.stack.pop().unwrap()); }
                    buf.reverse();
                    self.stack.push(Value::Tuple(buf));
                }
                Opcode::CREATE_MAP => {
                    let n = inst.op16() as usize;
                    if self.stack.len() < n * 2 { bail!("stack underflow creating map"); }
                    let mut entries = Vec::with_capacity(n);
                    for _ in 0..n {
                        let v = self.stack.pop().unwrap();
                        let k = self.stack.pop().unwrap();
                        entries.push((k, v));
                    }
                    entries.reverse();
                    self.stack.push(Value::Map(entries));
                }
                Opcode::CONCAT => {
                    let (b, a) = (self.stack.pop(), self.stack.pop());
                    match (a, b) {
                        (Some(Value::Str(a)), Some(Value::Str(b))) => self.stack.push(Value::Str(a + &b)),
                        (Some(Value::List(mut a)), Some(Value::List(b))) => { a.extend(b); self.stack.push(Value::List(a)); }
                        _ => bail!("CONCAT expects (Str,Str) or (List,List)"),
                    }
                }
                Opcode::DIFF => {
                    let (b, a) = (self.stack.pop(), self.stack.pop());
                    match (a, b) {
                        (Some(Value::List(a)), Some(Value::List(b))) => {
                            // remove elements in b from a (multiset semantics: each occurrence in b removes one in a)
                            let mut result = Vec::new();
                            let mut to_remove = b;
                            for item in a.into_iter() {
                                if let Some(pos) = to_remove.iter().position(|x| x == &item) {
                                    to_remove.remove(pos);
                                } else {
                                    result.push(item);
                                }
                            }
                            self.stack.push(Value::List(result));
                        }
                        _ => bail!("DIFF expects (List,List)"),
                    }
                }

                // Pattern ops: minimal placeholders
                Opcode::PATTERN => {
                    // For now push Nil as a placeholder compiled pattern
                    self.stack.push(Value::Nil);
                }
                Opcode::MATCH_TEST => {
                    // Pop value and pattern placeholder, return true for Nil (catch-all)
                    let _val = self.stack.pop();
                    let _pat = self.stack.pop();
                    self.stack.push(Value::Bool(true));
                }
                Opcode::EXTRACT_BINDINGS => {
                    // Push empty map as "bindings"
                    self.stack.push(Value::Map(vec![]));
                }

                _ => { /* ignore other opcodes for now */ }
            }
            pc += 1;
        }
        Ok(self.stack.last().cloned().unwrap_or(Value::Nil))
    }
}

// Re-export a lightweight API for users
pub mod api {
    pub use rholang_bytecode::core::instructions::Instruction;
    pub use rholang_bytecode::core::opcodes::Opcode;
    pub use crate::Value;
}
