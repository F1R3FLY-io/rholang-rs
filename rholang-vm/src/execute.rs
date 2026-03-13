use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use rholang_bytecode::core::instructions::Instruction as CoreInst;
use rholang_bytecode::core::opcodes::Opcode;
use std::cmp::Ordering;
use std::result::Result;

use crate::VM;
use rholang_rspace::{ExecError, Value};

pub enum StepResult {
    Next,
    Stop,
    Jump(usize),
    /// EVAL opcode encountered - Process should handle executing the value
    Eval(Value),
}

/// Execute a single bytecode instruction.
///
/// # Arguments
/// * `vm` - The VM state (stack, rspace, continuations, name counter)
/// * `locals` - The process's local variable slots
/// * `names` - The process's string pool for PUSH_STR
/// * `constants` - The process's typed constant pool for PUSH_CONST
/// * `inst` - The instruction to execute
pub fn step(
    vm: &mut VM,
    locals: &mut Vec<Value>,
    names: &[Value],
    constants: &[Value],
    inst: CoreInst,
) -> Result<StepResult, ExecError> {
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
            match names.get(idx) {
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
        Opcode::PUSH_CONST => {
            let idx = inst.op16() as usize;
            match constants.get(idx) {
                Some(val) => vm.stack.push(val.clone()),
                None => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "PUSH_CONST",
                        message: format!("constants index out of bounds: {}", idx),
                    });
                }
            }
        }
        Opcode::POP => {
            let _ = vm.stack.pop();
        }

        // Arithmetic
        Opcode::ADD => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Int(a.wrapping_add(b))),
                (Some(Value::Float(a)), Some(Value::Float(b))) => vm.stack.push(Value::Float(a + b)),
                (Some(Value::BigInt(a)), Some(Value::BigInt(b))) => {
                    vm.stack.push(Value::BigInt(a + b));
                }
                (Some(Value::BigRat(a)), Some(Value::BigRat(b))) => {
                    vm.stack.push(Value::BigRat(a + b));
                }
                (Some(Value::FixedPoint { unscaled: ua, scale: sa }), Some(Value::FixedPoint { unscaled: ub, scale: sb })) => {
                    if sa != sb {
                        return Err(type_mismatch_error("ADD", &format!("FixedPoint(p{})", sa), &format!("FixedPoint(p{})", sb)));
                    }
                    vm.stack.push(Value::FixedPoint { unscaled: ua + ub, scale: sa });
                }
                (Some(Value::Str(a)), Some(Value::Str(b))) => vm.stack.push(Value::Str(a + &b)),
                (Some(Value::List(mut a)), Some(Value::List(b))) => {
                    a.extend(b);
                    vm.stack.push(Value::List(a));
                }
                (Some(a), Some(b)) => return Err(type_mismatch_error("ADD", a.type_name(), b.type_name())),
                _ => return Err(stack_underflow("ADD")),
            }
        }
        Opcode::SUB => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Int(a.wrapping_sub(b))),
                (Some(Value::Float(a)), Some(Value::Float(b))) => vm.stack.push(Value::Float(a - b)),
                (Some(Value::BigInt(a)), Some(Value::BigInt(b))) => {
                    vm.stack.push(Value::BigInt(a - b));
                }
                (Some(Value::BigRat(a)), Some(Value::BigRat(b))) => {
                    vm.stack.push(Value::BigRat(a - b));
                }
                (Some(Value::FixedPoint { unscaled: ua, scale: sa }), Some(Value::FixedPoint { unscaled: ub, scale: sb })) => {
                    if sa != sb {
                        return Err(type_mismatch_error("SUB", &format!("FixedPoint(p{})", sa), &format!("FixedPoint(p{})", sb)));
                    }
                    vm.stack.push(Value::FixedPoint { unscaled: ua - ub, scale: sa });
                }
                (Some(a), Some(b)) => return Err(type_mismatch_error("SUB", a.type_name(), b.type_name())),
                _ => return Err(stack_underflow("SUB")),
            }
        }
        Opcode::MUL => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Int(a.wrapping_mul(b))),
                (Some(Value::Float(a)), Some(Value::Float(b))) => vm.stack.push(Value::Float(a * b)),
                (Some(Value::BigInt(a)), Some(Value::BigInt(b))) => {
                    vm.stack.push(Value::BigInt(a * b));
                }
                (Some(Value::BigRat(a)), Some(Value::BigRat(b))) => {
                    vm.stack.push(Value::BigRat(a * b));
                }
                (Some(Value::FixedPoint { unscaled: ua, scale: sa }), Some(Value::FixedPoint { unscaled: ub, scale: sb })) => {
                    if sa != sb {
                        return Err(type_mismatch_error("MUL", &format!("FixedPoint(p{})", sa), &format!("FixedPoint(p{})", sb)));
                    }
                    // Scale-preserving: (ua * ub) / 10^scale, using floor division
                    let raw = &ua * &ub;
                    let scale_factor = num_traits::pow::pow(BigInt::from(10), sa as usize);
                    let one = BigInt::from(1);
                    let unscaled: BigInt = if raw.is_negative() {
                        // Floor division for negative: -((-raw - 1) / sf + 1)
                        let abs_raw = -&raw;
                        -((&abs_raw - &one) / &scale_factor + &one)
                    } else {
                        &raw / &scale_factor
                    };
                    vm.stack.push(Value::FixedPoint { unscaled, scale: sa });
                }
                (Some(a), Some(b)) => return Err(type_mismatch_error("MUL", a.type_name(), b.type_name())),
                _ => return Err(stack_underflow("MUL")),
            }
        }
        Opcode::DIV => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => {
                    if b == 0 {
                        return Err(div_by_zero("DIV"));
                    }
                    vm.stack.push(Value::Int(a.wrapping_div(b)));
                }
                (Some(Value::Float(a)), Some(Value::Float(b))) => {
                    // IEEE 754: div by zero produces Inf/-Inf/NaN
                    vm.stack.push(Value::Float(a / b));
                }
                (Some(Value::BigInt(a)), Some(Value::BigInt(b))) => {
                    if b.is_zero() {
                        return Err(div_by_zero("DIV"));
                    }
                    let r = a / b;
                    vm.stack.push(Value::BigInt(r));
                }
                (Some(Value::BigRat(a)), Some(Value::BigRat(b))) => {
                    if b.is_zero() {
                        return Err(div_by_zero("DIV"));
                    }
                    vm.stack.push(Value::BigRat(a / b));
                }
                (Some(Value::FixedPoint { unscaled: ua, scale: sa }), Some(Value::FixedPoint { unscaled: ub, scale: sb })) => {
                    if sa != sb {
                        return Err(type_mismatch_error("DIV", &format!("FixedPoint(p{})", sa), &format!("FixedPoint(p{})", sb)));
                    }
                    if ub.is_zero() {
                        return Err(div_by_zero("DIV"));
                    }
                    // Shifted division: (ua * 10^scale) / ub
                    let shifted = ua * num_traits::pow::pow(BigInt::from(10), sa as usize);
                    vm.stack.push(Value::FixedPoint { unscaled: shifted / ub, scale: sa });
                }
                (Some(a), Some(b)) => return Err(type_mismatch_error("DIV", a.type_name(), b.type_name())),
                _ => return Err(stack_underflow("DIV")),
            }
        }
        Opcode::MOD => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => {
                    if b == 0 {
                        return Err(div_by_zero("MOD"));
                    }
                    vm.stack.push(Value::Int(a % b));
                }
                (Some(Value::Float(_)), Some(Value::Float(_))) => {
                    return Err(ExecError::OpcodeParamError {
                        opcode: "MOD",
                        message: "modulus not defined on floating point".to_string(),
                    });
                }
                (Some(Value::BigInt(a)), Some(Value::BigInt(b))) => {
                    if b.is_zero() {
                        return Err(div_by_zero("MOD"));
                    }
                    let r = a % b;
                    vm.stack.push(Value::BigInt(r));
                }
                (Some(Value::BigRat(_)), Some(Value::BigRat(_))) => {
                    // Per spec: (a/b)*b == a exactly, so mod always returns 0
                    vm.stack.push(Value::BigRat(BigRational::zero()));
                }
                (Some(Value::FixedPoint { unscaled: ua, scale: sa }), Some(Value::FixedPoint { unscaled: ub, scale: sb })) => {
                    if sa != sb {
                        return Err(type_mismatch_error("MOD", &format!("FixedPoint(p{})", sa), &format!("FixedPoint(p{})", sb)));
                    }
                    if ub.is_zero() {
                        return Err(div_by_zero("MOD"));
                    }
                    // C99 identity: (a/b)*b + a%b == a
                    // quotient = (ua * 10^scale) / ub (same as DIV)
                    // remainder = ua - quotient * ub
                    let scale_factor = num_traits::pow::pow(BigInt::from(10), sa as usize);
                    let quotient = (&ua * &scale_factor) / &ub;
                    let r = ua - (&quotient * &ub) / scale_factor;
                    vm.stack.push(Value::FixedPoint { unscaled: r, scale: sa });
                }
                (Some(a), Some(b)) => return Err(type_mismatch_error("MOD", a.type_name(), b.type_name())),
                _ => return Err(stack_underflow("MOD")),
            }
        }
        Opcode::NEG => match vm.stack.pop() {
            Some(Value::Int(a)) => vm.stack.push(Value::Int(a.wrapping_neg())),
            Some(Value::Float(a)) => vm.stack.push(Value::Float(-a)),
            Some(Value::BigInt(a)) => vm.stack.push(Value::BigInt(-a)),
            Some(Value::BigRat(a)) => vm.stack.push(Value::BigRat(-a)),
            Some(Value::FixedPoint { unscaled, scale }) => {
                vm.stack.push(Value::FixedPoint { unscaled: -unscaled, scale });
            }
            Some(other) => {
                return Err(ExecError::OpcodeParamError {
                    opcode: "NEG",
                    message: format!("cannot negate {}", other.type_name()),
                });
            }
            None => return Err(stack_underflow("NEG")),
        },

        // Comparison
        Opcode::CMP_EQ => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(a), Some(b)) => vm.stack.push(Value::Bool(a == b)),
                _ => return Err(stack_underflow("CMP_EQ")),
            }
        }
        Opcode::CMP_NEQ => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(a), Some(b)) => vm.stack.push(Value::Bool(a != b)),
                _ => return Err(stack_underflow("CMP_NEQ")),
            }
        }
        Opcode::CMP_LT => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            vm.stack.push(Value::Bool(compare_values("CMP_LT", &a, &b)? == Ordering::Less));
        }
        Opcode::CMP_LTE => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            vm.stack.push(Value::Bool(matches!(compare_values("CMP_LTE", &a, &b)?, Ordering::Less | Ordering::Equal)));
        }
        Opcode::CMP_GT => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            vm.stack.push(Value::Bool(compare_values("CMP_GT", &a, &b)? == Ordering::Greater));
        }
        Opcode::CMP_GTE => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            vm.stack.push(Value::Bool(matches!(compare_values("CMP_GTE", &a, &b)?, Ordering::Greater | Ordering::Equal)));
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
            locals.push(Value::Nil);
        }
        Opcode::LOAD_LOCAL => {
            let idx = inst.op16() as usize;
            match locals.get(idx) {
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
            if locals.len() <= idx {
                locals.resize(idx + 1, Value::Nil);
            }
            locals[idx] = value;
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
        // Note: kind (op16) is ignored in the new unified API - names are unique identifiers
        Opcode::TELL => {
            let _kind = inst.op16(); // Kept for bytecode compatibility
            let data = vm.stack.pop().unwrap_or(Value::Nil);
            let chan = vm.stack.pop().unwrap_or(Value::Nil);
            match chan {
                Value::Name(name) => {
                    if let Ok(mut rspace) = vm.rspace.lock() {
                        rspace
                            .tell(&name, data)
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
            let _kind = inst.op16(); // Kept for bytecode compatibility
            let chan = vm.stack.pop().unwrap_or(Value::Nil);
            match chan {
                Value::Name(name) => {
                    if let Ok(mut rspace) = vm.rspace.lock() {
                        let result =
                            rspace.ask(&name).map_err(|e| ExecError::OpcodeParamError {
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
            let _kind = inst.op16(); // Kept for bytecode compatibility
            let chan = vm.stack.pop().unwrap_or(Value::Nil);
            match chan {
                Value::Name(name) => {
                    if let Ok(rspace) = vm.rspace.lock() {
                        let result =
                            rspace
                                .peek(&name)
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
        // EVAL returns the target value to be handled by Process
        // Process will:
        // 1. For Par values: execute ready processes and return list of results
        // 2. For other values: return them as-is (already evaluated)
        Opcode::EVAL => {
            let target = vm.stack.pop().unwrap_or(Value::Nil);
            return Ok(StepResult::Eval(target));
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

// ---------------------------------------------------------------------------
// Helper functions for arithmetic/comparison opcodes
// ---------------------------------------------------------------------------

fn type_mismatch_error(opcode: &'static str, type_a: &str, type_b: &str) -> ExecError {
    ExecError::OpcodeParamError {
        opcode,
        message: format!("type mismatch: {} vs {}", type_a, type_b),
    }
}

fn stack_underflow(opcode: &'static str) -> ExecError {
    ExecError::OpcodeParamError {
        opcode,
        message: "stack underflow".to_string(),
    }
}

fn div_by_zero(opcode: &'static str) -> ExecError {
    ExecError::OpcodeParamError {
        opcode,
        message: "division by zero".to_string(),
    }
}

fn compare_values(
    opcode: &'static str,
    a: &Option<Value>,
    b: &Option<Value>,
) -> Result<Ordering, ExecError> {
    match (a, b) {
        (Some(a_val), Some(b_val)) => a_val
            .partial_cmp(b_val)
            .ok_or_else(|| ExecError::OpcodeParamError {
                opcode,
                message: format!(
                    "values not comparable: {} vs {}",
                    a_val.type_name(),
                    b_val.type_name()
                ),
            }),
        _ => Err(stack_underflow(opcode)),
    }
}
