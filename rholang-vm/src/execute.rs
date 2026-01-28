use rholang_bytecode::core::instructions::Instruction as CoreInst;
use rholang_bytecode::core::opcodes::Opcode;
use std::result::Result;

use crate::error::ExecError;
use crate::process::Process;
use crate::value::Value;
use crate::VM;

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
        Opcode::NEG => match vm.stack.pop() {
            Some(Value::Int(a)) => vm.stack.push(Value::Int(-a)),
            _ => {
                return Err(ExecError::OpcodeParamError {
                    opcode: "NEG",
                    message: "requires Int".to_string(),
                })
            }
        },

        // Comparison
        Opcode::CMP_EQ => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(a), Some(b)) => vm.stack.push(Value::Bool(a == b)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CMP_EQ",
                        message: "requires two values".to_string(),
                    })
                }
            }
        }
        Opcode::CMP_NEQ => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(a), Some(b)) => vm.stack.push(Value::Bool(a != b)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CMP_NEQ",
                        message: "requires two values".to_string(),
                    })
                }
            }
        }
        Opcode::CMP_LT => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Bool(a < b)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CMP_LT",
                        message: "requires Ints".to_string(),
                    })
                }
            }
        }
        Opcode::CMP_LTE => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Bool(a <= b)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CMP_LTE",
                        message: "requires Ints".to_string(),
                    })
                }
            }
        }
        Opcode::CMP_GT => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Bool(a > b)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CMP_GT",
                        message: "requires Ints".to_string(),
                    })
                }
            }
        }
        Opcode::CMP_GTE => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Bool(a >= b)),
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CMP_GTE",
                        message: "requires Ints".to_string(),
                    })
                }
            }
        }

        // Logical operators
        Opcode::NOT => match vm.stack.pop() {
            Some(Value::Bool(b)) => vm.stack.push(Value::Bool(!b)),
            _ => {
                return Err(ExecError::OpcodeParamError {
                    opcode: "NOT",
                    message: "requires Bool".to_string(),
                })
            }
        },
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

        // Stack ops
        Opcode::DUP => {
            if let Some(top) = vm.stack.last() {
                vm.stack.push(top.clone());
            } else {
                return Err(ExecError::OpcodeParamError {
                    opcode: "DUP",
                    message: "stack underflow".to_string(),
                });
            }
        }
        Opcode::SWAP => {
            if vm.stack.len() < 2 {
                return Err(ExecError::OpcodeParamError {
                    opcode: "SWAP",
                    message: "stack underflow".to_string(),
                });
            }
            let len = vm.stack.len();
            vm.stack.swap(len - 1, len - 2);
        }

        // Local variables
        Opcode::ALLOC_LOCAL => {
            process.locals.push(Value::Nil);
        }
        Opcode::LOAD_LOCAL => {
            let idx = inst.op16() as usize;
            match process.locals.get(idx) {
                Some(v) => vm.stack.push(v.clone()),
                None => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "LOAD_LOCAL",
                        message: format!("locals index out of bounds: {}", idx),
                    });
                }
            }
        }
        Opcode::STORE_LOCAL => {
            let idx = inst.op16() as usize;
            let value = vm.stack.pop().unwrap_or(Value::Nil);
            if process.locals.len() <= idx {
                process.locals.resize(idx + 1, Value::Nil);
            }
            process.locals[idx] = value;
        }

        // Conditional and jumps
        Opcode::JUMP => {
            let target = inst.op16() as usize;
            return Ok(StepResult::Jump(target));
        }
        Opcode::BRANCH_TRUE => {
            let target = inst.op16() as usize;
            let cond = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "BRANCH_TRUE",
                message: "expects Bool on stack".to_string(),
            })?;
            match cond {
                Value::Bool(true) => return Ok(StepResult::Jump(target)),
                Value::Bool(false) => {}
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "BRANCH_TRUE",
                        message: "expects Bool on stack".to_string(),
                    });
                }
            }
        }
        Opcode::BRANCH_FALSE => {
            let target = inst.op16() as usize;
            let cond = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "BRANCH_FALSE",
                message: "expects Bool on stack".to_string(),
            })?;
            match cond {
                Value::Bool(false) => return Ok(StepResult::Jump(target)),
                Value::Bool(true) => {}
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "BRANCH_FALSE",
                        message: "expects Bool on stack".to_string(),
                    });
                }
            }
        }
        Opcode::BRANCH_SUCCESS => {
            let target = inst.op16() as usize;
            let cond = vm.stack.pop().ok_or_else(|| ExecError::OpcodeParamError {
                opcode: "BRANCH_SUCCESS",
                message: "expects Bool on stack".to_string(),
            })?;
            match cond {
                Value::Bool(true) => return Ok(StepResult::Jump(target)),
                Value::Bool(false) => {}
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "BRANCH_SUCCESS",
                        message: "expects Bool on stack".to_string(),
                    });
                }
            }
        }

        // List / tuple / map
        Opcode::CREATE_LIST => {
            let len = inst.op16() as usize;
            if vm.stack.len() < len {
                return Err(ExecError::OpcodeParamError {
                    opcode: "CREATE_LIST",
                    message: "stack underflow".to_string(),
                });
            }
            let start = vm.stack.len() - len;
            let list = vm.stack.drain(start..).collect();
            vm.stack.push(Value::List(list));
        }
        Opcode::CREATE_TUPLE => {
            let len = inst.op16() as usize;
            if vm.stack.len() < len {
                return Err(ExecError::OpcodeParamError {
                    opcode: "CREATE_TUPLE",
                    message: "stack underflow".to_string(),
                });
            }
            let start = vm.stack.len() - len;
            let list = vm.stack.drain(start..).collect();
            vm.stack.push(Value::Tuple(list));
        }
        Opcode::CREATE_MAP => {
            let len = inst.op16() as usize;
            if vm.stack.len() < len * 2 {
                return Err(ExecError::OpcodeParamError {
                    opcode: "CREATE_MAP",
                    message: "stack underflow".to_string(),
                });
            }
            let start = vm.stack.len() - len * 2;
            let values: Vec<Value> = vm.stack.drain(start..).collect();
            let mut map = Vec::with_capacity(len);
            for pair in values.chunks(2) {
                if let [k, v] = pair {
                    map.push((k.clone(), v.clone()));
                }
            }
            vm.stack.push(Value::Map(map));
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
                        message: "requires two Strings or two Lists".to_string(),
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
                    for item in a {
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
                        message: "requires two Lists".to_string(),
                    })
                }
            }
        }

        // Process ops
        Opcode::SPAWN_ASYNC => {
            let len = inst.op16() as usize;
            if vm.stack.len() < len {
                return Err(ExecError::OpcodeParamError {
                    opcode: "SPAWN_ASYNC",
                    message: "stack underflow".to_string(),
                });
            }
            let start = vm.stack.len() - len;
            let values: Vec<Value> = vm.stack.drain(start..).collect();
            let mut procs = Vec::with_capacity(len);
            for value in values {
                match value {
                    Value::Par(ps) => procs.extend(ps),
                    Value::Nil => {}
                    other => {
                        return Err(ExecError::OpcodeParamError {
                            opcode: "SPAWN_ASYNC",
                            message: format!("expected process list, got {:?}", other),
                        });
                    }
                }
            }
            vm.stack.push(Value::Par(procs));
        }
        Opcode::NAME_CREATE => {
            let kind = inst.op16();
            let id = vm.next_name_id;
            vm.next_name_id += 1;
            let name = format!("@{}:{}", kind, id);
            vm.stack.push(Value::Name(name));
        }

        // RSpace interactions
        Opcode::TELL => {
            let kind = inst.op16();
            let data = vm.stack.pop().unwrap_or(Value::Nil);
            let chan = vm.stack.pop().unwrap_or(Value::Nil);
            match chan {
                Value::Name(name) => {
                    if let Ok(mut rspace) = vm.rspace.lock() {
                        rspace
                            .tell(kind, name, data)
                            .map_err(|e| ExecError::OpcodeParamError {
                                opcode: "TELL",
                                message: e.to_string(),
                            })?;
                    }
                    vm.stack.push(Value::Bool(true));
                }
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "TELL",
                        message: "requires Name channel".to_string(),
                    })
                }
            }
        }
        Opcode::ASK => {
            let kind = inst.op16();
            let chan = vm.stack.pop().unwrap_or(Value::Nil);
            match chan {
                Value::Name(name) => {
                    if let Ok(mut rspace) = vm.rspace.lock() {
                        let result =
                            rspace
                                .ask(kind, name)
                                .map_err(|e| ExecError::OpcodeParamError {
                                    opcode: "ASK",
                                    message: e.to_string(),
                                })?;
                        vm.stack.push(result.unwrap_or(Value::Nil));
                    } else {
                        vm.stack.push(Value::Nil);
                    }
                }
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "ASK",
                        message: "requires Name channel".to_string(),
                    })
                }
            }
        }
        Opcode::PEEK => {
            let kind = inst.op16();
            let chan = vm.stack.pop().unwrap_or(Value::Nil);
            match chan {
                Value::Name(name) => {
                    if let Ok(rspace) = vm.rspace.lock() {
                        let result =
                            rspace
                                .peek(kind, name)
                                .map_err(|e| ExecError::OpcodeParamError {
                                    opcode: "PEEK",
                                    message: e.to_string(),
                                })?;
                        vm.stack.push(result.unwrap_or(Value::Nil));
                    } else {
                        vm.stack.push(Value::Nil);
                    }
                }
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "PEEK",
                        message: "requires Name channel".to_string(),
                    })
                }
            }
        }

        // Continuations
        Opcode::CONT_STORE => {
            let cont = vm.stack.pop().unwrap_or(Value::Nil);
            let id = vm.next_cont_id;
            vm.next_cont_id += 1;
            vm.cont_last = Some((id, cont));
            vm.stack.push(Value::Int(id as i64));
        }
        Opcode::CONT_RESUME => {
            let id = vm.stack.pop().unwrap_or(Value::Int(0));
            match id {
                Value::Int(id) => {
                    if let Some((stored_id, cont)) = &vm.cont_last {
                        if *stored_id == id as u32 {
                            vm.stack.push(cont.clone());
                        } else {
                            vm.stack.push(Value::Nil);
                        }
                    } else {
                        vm.stack.push(Value::Nil);
                    }
                }
                _ => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "CONT_RESUME",
                        message: "requires Int id".to_string(),
                    })
                }
            }
        }

        // Process invocation / evaluation
        // EVAL handles both:
        // 1. Par values: execute ready processes and return list of results
        // 2. Other values: return them as-is (they're already evaluated)
        Opcode::EVAL => {
            let target = vm.stack.pop().unwrap_or(Value::Nil);
            match target {
                Value::Par(mut procs) => {
                    let mut results = Vec::new();
                    for proc in procs.iter_mut() {
                        if proc.is_ready() {
                            let result = proc.execute()?;
                            results.push(result);
                        }
                    }
                    // If only one result, return it directly; otherwise return list
                    if results.len() == 1 {
                        vm.stack.push(results.pop().unwrap());
                    } else {
                        vm.stack.push(Value::List(results));
                    }
                }
                // Non-Par values are already evaluated, just pass through
                other => vm.stack.push(other),
            }
        }

        // Fallback for unimplemented opcodes
        _ => {
            return Err(ExecError::OpcodeParamError {
                opcode: "UNIMPLEMENTED",
                message: format!("opcode {:?} not implemented", opcode),
            })
        }
    }

    Ok(StepResult::Next)
}
