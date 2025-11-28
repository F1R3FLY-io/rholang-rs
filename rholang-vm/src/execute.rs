// execute.rs returns ExecError on error
// Avoid anyhow here; map all errors into ExecError
use rholang_bytecode::core::instructions::Instruction as CoreInst;
use rholang_bytecode::core::opcodes::Opcode;
use std::result::Result;

use crate::error::ExecError;
use crate::process::Process;
use crate::value::Value;
use crate::vm::VM;

pub enum StepResult {
    Next,
    Stop,
    Jump(usize),
}

pub fn step(vm: &mut VM, process: &mut Process, inst: CoreInst) -> Result<StepResult, ExecError> {
    let opcode = inst.opcode().map_err(|e| ExecError::OpcodeParamError {
        opcode: "OPCODE",
        message: e.to_string(),
    })?;
    match opcode {
        Opcode::NOP => {}
        Opcode::HALT => {
            return Ok(StepResult::Stop);
        }
        Opcode::PUSH_INT => {
            let imm = inst.op16() as i16 as i64;
            vm.stack.push(Value::Int(imm));
        }
        Opcode::PUSH_BOOL => {
            let v = inst.op1() != 0;
            vm.stack.push(Value::Bool(v));
        }
        Opcode::PUSH_STR => {
            let idx = inst.op16() as usize;
            match process.names.get(idx) {
                Some(Value::Str(s)) => vm.stack.push(Value::Str(s.clone())),
                Some(other) => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "PUSH_STR",
                        message: format!("names[{}] not a String: {:?}", idx, other),
                    });
                }
                None => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "PUSH_STR",
                        message: format!("names index out of bounds: {}", idx),
                    });
                }
            }
        }
        Opcode::PUSH_NIL => vm.stack.push(Value::Nil),
        Opcode::POP => {
            let _ = vm.stack.pop();
        }

        // Arithmetic
        Opcode::ADD => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Int(a + b)),
                (Some(Value::Str(a)), Some(Value::Str(b))) => vm.stack.push(Value::Str(a + &b)),
                (Some(Value::List(mut a)), Some(Value::List(b))) => {
                    a.extend(b);
                    vm.stack.push(Value::List(a));
                }
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "ADD",
                        message: "type mismatch".to_string(),
                    })
                }
            }
        }
        Opcode::SUB => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Int(a - b)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "SUB",
                        message: "requires Ints".to_string(),
                    })
                }
            }
        }
        Opcode::MUL => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Int(a * b)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "MUL",
                        message: "requires Ints".to_string(),
                    })
                }
            }
        }
        Opcode::DIV => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => {
                    if b == 0 {
                        return Err(ExecError::OpcodeParamError {
                            opcode: "DIV",
                            message: "division by zero".to_string(),
                        });
                    }
                    vm.stack.push(Value::Int(a / b))
                }
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "DIV",
                        message: "requires Ints".to_string(),
                    })
                }
            }
        }
        Opcode::MOD => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => {
                    if b == 0 {
                        return Err(ExecError::OpcodeParamError {
                            opcode: "MOD",
                            message: "modulo by zero".to_string(),
                        });
                    }
                    vm.stack.push(Value::Int(a % b))
                }
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "MOD",
                        message: "requires Ints".to_string(),
                    })
                }
            }
        }
        Opcode::NEG => {
            let a = vm.stack.pop();
            match a {
                Some(Value::Int(a)) => vm.stack.push(Value::Int(-a)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "NEG",
                        message: "requires Int".to_string(),
                    })
                }
            }
        }

        // Comparisons
        Opcode::CMP_EQ => {
            let b = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_EQ",
                message: "stack underflow".to_string(),
            })?;
            let a = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_EQ",
                message: "stack underflow".to_string(),
            })?;
            vm.stack.push(Value::Bool(a == b));
        }
        Opcode::CMP_NEQ => {
            let b = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_NEQ",
                message: "stack underflow".to_string(),
            })?;
            let a = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_NEQ",
                message: "stack underflow".to_string(),
            })?;
            vm.stack.push(Value::Bool(a != b));
        }
        Opcode::CMP_LT => {
            let b = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_LT",
                message: "stack underflow rhs (requires Ints)".to_string(),
            })?;
            let a = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_LT",
                message: "stack underflow lhs (requires Ints)".to_string(),
            })?;
            match (a, b) {
                (Value::Int(a), Value::Int(b)) => vm.stack.push(Value::Bool(a < b)),
                (a, b) => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CMP_LT",
                        message: format!("requires Ints, got {:?} and {:?}", a, b),
                    })
                }
            }
        }
        Opcode::CMP_LTE => {
            let b = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_LTE",
                message: "stack underflow rhs (requires Ints)".to_string(),
            })?;
            let a = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_LTE",
                message: "stack underflow lhs (requires Ints)".to_string(),
            })?;
            match (a, b) {
                (Value::Int(a), Value::Int(b)) => vm.stack.push(Value::Bool(a <= b)),
                (a, b) => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CMP_LTE",
                        message: format!("requires Ints, got {:?} and {:?}", a, b),
                    })
                }
            }
        }
        Opcode::CMP_GT => {
            let b = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_GT",
                message: "stack underflow rhs (requires Ints)".to_string(),
            })?;
            let a = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_GT",
                message: "stack underflow lhs (requires Ints)".to_string(),
            })?;
            match (a, b) {
                (Value::Int(a), Value::Int(b)) => vm.stack.push(Value::Bool(a > b)),
                (a, b) => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CMP_GT",
                        message: format!("requires Ints, got {:?} and {:?}", a, b),
                    })
                }
            }
        }
        Opcode::CMP_GTE => {
            let b = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_GTE",
                message: "stack underflow rhs (requires Ints)".to_string(),
            })?;
            let a = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CMP_GTE",
                message: "stack underflow lhs (requires Ints)".to_string(),
            })?;
            match (a, b) {
                (Value::Int(a), Value::Int(b)) => vm.stack.push(Value::Bool(a >= b)),
                (a, b) => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CMP_GTE",
                        message: format!("requires Ints, got {:?} and {:?}", a, b),
                    })
                }
            }
        }

        // Logical
        Opcode::AND => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Bool(a)), Some(Value::Bool(b))) => vm.stack.push(Value::Bool(a && b)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "AND",
                        message: "requires Bools".to_string(),
                    })
                }
            }
        }
        Opcode::OR => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Bool(a)), Some(Value::Bool(b))) => vm.stack.push(Value::Bool(a || b)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "OR",
                        message: "requires Bools".to_string(),
                    })
                }
            }
        }
        Opcode::NOT => {
            let a = vm.stack.pop();
            match a {
                Some(Value::Bool(a)) => vm.stack.push(Value::Bool(!a)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "NOT",
                        message: "requires Bool".to_string(),
                    })
                }
            }
        }

        // Collections
        Opcode::CREATE_LIST => {
            let n = inst.op16() as usize;
            if vm.stack.len() < n {
                return Err(ExecError::OpcodeParamError {
                    opcode: "CREATE_LIST",
                    message: "underflow".to_string(),
                });
            }
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(vm.stack.pop().unwrap());
            }
            items.reverse();
            vm.stack.push(Value::List(items));
        }
        Opcode::CREATE_TUPLE => {
            let n = inst.op16() as usize;
            if vm.stack.len() < n {
                return Err(ExecError::OpcodeParamError {
                    opcode: "CREATE_TUPLE",
                    message: "underflow".to_string(),
                });
            }
            let mut items = Vec::with_capacity(n);
            for _ in 0..n {
                items.push(vm.stack.pop().unwrap());
            }
            items.reverse();
            vm.stack.push(Value::Tuple(items));
        }
        Opcode::CREATE_MAP => {
            let n = inst.op16() as usize;
            if vm.stack.len() < n * 2 {
                return Err(ExecError::OpcodeParamError {
                    opcode: "CREATE_MAP",
                    message: "underflow".to_string(),
                });
            }
            let mut entries = Vec::with_capacity(n);
            for _ in 0..n {
                let v = vm.stack.pop().unwrap();
                let k = vm.stack.pop().unwrap();
                entries.push((k, v));
            }
            entries.reverse();
            vm.stack.push(Value::Map(entries));
        }
        Opcode::CONCAT => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Str(a)), Some(Value::Str(b))) => vm.stack.push(Value::Str(a + &b)),
                (Some(Value::List(mut a)), Some(Value::List(b))) => {
                    a.extend(b);
                    vm.stack.push(Value::List(a));
                }
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CONCAT",
                        message: "expects (Str,Str) or (List,List)".to_string(),
                    })
                }
            }
        }
        Opcode::DIFF => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::List(a)), Some(Value::List(b))) => {
                    let mut result = Vec::new();
                    let mut to_remove = b;
                    for item in a.into_iter() {
                        if let Some(pos) = to_remove.iter().position(|x| x == &item) {
                            to_remove.remove(pos);
                        } else {
                            result.push(item);
                        }
                    }
                    vm.stack.push(Value::List(result));
                }
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "DIFF",
                        message: "expects (List,List)".to_string(),
                    })
                }
            }
        }

        // Pattern placeholders
        Opcode::PATTERN => {
            vm.stack.push(Value::Nil);
        }
        Opcode::MATCH_TEST => {
            let _val = vm.stack.pop();
            let _pat = vm.stack.pop();
            vm.stack.push(Value::Bool(true));
        }
        Opcode::EXTRACT_BINDINGS => {
            vm.stack.push(Value::Map(vec![]));
        }

        // Locals
        Opcode::ALLOC_LOCAL => {
            process.locals.push(Value::Nil);
        }
        Opcode::LOAD_LOCAL => {
            let idx = inst.op16() as usize;
            if let Some(v) = process.locals.get(idx).cloned() {
                vm.stack.push(v);
            } else {
                return Err(ExecError::OpcodeParamError {
                    opcode: "LOAD_LOCAL",
                    message: format!("out of bounds: {}", idx),
                });
            }
        }
        Opcode::STORE_LOCAL => {
            let idx = inst.op16() as usize;
            let v = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "STORE_LOCAL",
                message: "stack underflow".to_string(),
            })?;
            if idx < process.locals.len() {
                process.locals[idx] = v;
            } else {
                return Err(ExecError::OpcodeParamError {
                    opcode: "STORE_LOCAL",
                    message: format!("out of bounds: {}", idx),
                });
            }
        }

        // Continuations (store/resume) - simplified single-slot storage
        Opcode::CONT_STORE => {
            let v = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "CONT_STORE",
                message: "stack underflow".to_string(),
            })?;
            let id = vm.next_cont_id;
            vm.next_cont_id = vm.next_cont_id.wrapping_add(1).max(1);
            vm.cont_last = Some((id, v));
            vm.stack.push(Value::Int(id as i64));
        }
        Opcode::CONT_RESUME => {
            let id = match vm.stack.pop() {
                Some(Value::Int(n)) if n >= 0 => n as u32,
                other => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CONT_RESUME",
                        message: format!("expects non-negative Int id, got {:?}", other),
                    })
                }
            };
            let v = if let Some((saved_id, saved_v)) = vm.cont_last.take() {
                if saved_id == id {
                    saved_v
                } else {
                    Value::Nil
                }
            } else {
                Value::Nil
            };
            vm.stack.push(v);
        }

        // Names and simple RSpace
        Opcode::NAME_CREATE => {
            let kind = inst.op16();
            let id = vm.next_name_id;
            vm.next_name_id = vm.next_name_id.wrapping_add(1).max(1);
            let channel = format!("@{}:{}", kind, id);
            vm.stack.push(Value::Name(channel));
        }
        Opcode::TELL => {
            let kind = inst.op16();
            let data = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "TELL",
                message: "stack underflow on data".to_string(),
            })?;
            let chan = match vm.stack.pop() {
                Some(Value::Name(s)) => s,
                other => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "TELL",
                        message: format!("expects Name channel, got {:?}", other),
                    })
                }
            };
            vm.rspace
                .tell(kind, chan, data)
                .map_err(|e| ExecError::OpcodeParamError {
                    opcode: "TELL",
                    message: e.to_string(),
                })?;
            vm.stack.push(Value::Bool(true));
        }
        Opcode::ASK => {
            let kind = inst.op16();
            let chan = match vm.stack.pop() {
                Some(Value::Name(s)) => s,
                other => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "ASK",
                        message: format!("expects Name channel, got {:?}", other),
                    })
                }
            };
            let v = vm
                .rspace
                .ask(kind, chan)
                .map_err(|e| ExecError::OpcodeParamError {
                    opcode: "TELL",
                    message: e.to_string(),
                })?;
            vm.stack.push(v.unwrap_or(Value::Nil));
        }
        Opcode::PEEK => {
            let kind = inst.op16();
            let chan = match vm.stack.pop() {
                Some(Value::Name(s)) => s,
                other => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "PEEK",
                        message: format!("expects Name channel, got {:?}", other),
                    })
                }
            };
            let v = vm
                .rspace
                .peek(kind, chan)
                .map_err(|e| ExecError::OpcodeParamError {
                    opcode: "TELL",
                    message: e.to_string(),
                })?;
            vm.stack.push(v.unwrap_or(Value::Nil));
        }

        // Control flow
        Opcode::JUMP => {
            // Jump to absolute instruction index provided as immediate operand
            let idx = inst.op16() as usize;
            return Ok(StepResult::Jump(idx));
        }
        Opcode::BRANCH_TRUE => {
            // Conditional branch to absolute index in instruction immediate
            let cond = match vm.stack.pop() {
                Some(Value::Bool(b)) => b,
                other => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "BRANCH_TRUE",
                        message: format!("expects Bool condition on stack, got {:?}", other),
                    })
                }
            };
            if cond {
                let idx = inst.op16() as usize;
                return Ok(StepResult::Jump(idx));
            }
        }
        Opcode::BRANCH_FALSE => {
            let cond = match vm.stack.pop() {
                Some(Value::Bool(b)) => b,
                other => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "BRANCH_FALSE",
                        message: format!("expects Bool condition on stack, got {:?}", other),
                    })
                }
            };
            if !cond {
                let idx = inst.op16() as usize;
                return Ok(StepResult::Jump(idx));
            }
        }
        Opcode::BRANCH_SUCCESS => {
            // Expect Bool status on stack; if true, branch to immediate index
            let status = match vm.stack.pop() {
                Some(Value::Bool(b)) => b,
                other => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "BRANCH_SUCCESS",
                        message: format!("expects Bool status on stack, got {:?}", other),
                    })
                }
            };
            if status {
                let idx = inst.op16() as usize;
                return Ok(StepResult::Jump(idx));
            }
        }

        // Unhandled opcodes default: do nothing
        _ => {}
    }

    Ok(StepResult::Next)
}
