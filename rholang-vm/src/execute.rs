use anyhow::{anyhow, bail, Result};
use rholang_bytecode::core::instructions::Instruction as CoreInst;
use rholang_bytecode::core::opcodes::Opcode;

use crate::process::Process;
use crate::value::Value;
use crate::vm::VM;
use crate::error::ExecError;

pub enum StepResult {
    Next,
    Stop,
    Jump(String),
}

pub fn step(vm: &mut VM, process: &mut Process, inst: CoreInst) -> Result<StepResult> {
    let opcode = inst.opcode()?;
    match opcode {
        Opcode::NOP => {}
        Opcode::HALT => { return Ok(StepResult::Stop); }
        Opcode::PUSH_INT => {
            let imm = inst.op16() as i16 as i64;
            vm.stack.push(Value::Int(imm));
        }
        Opcode::PUSH_BOOL => {
            let v = inst.op1() != 0;
            vm.stack.push(Value::Bool(v));
        }
        Opcode::PUSH_NIL => vm.stack.push(Value::Nil),
        Opcode::POP => { let _ = vm.stack.pop(); }

        // Arithmetic
        Opcode::ADD => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Int(a + b)),
                (Some(Value::Str(a)), Some(Value::Str(b))) => vm.stack.push(Value::Str(a + &b)),
                (Some(Value::List(mut a)), Some(Value::List(b))) => { a.extend(b); vm.stack.push(Value::List(a)); }
                _ => bail!("ADD type mismatch"),
            }
        }
        Opcode::SUB => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Int(a - b)),
                _ => bail!("SUB requires Ints"),
            }
        }
        Opcode::MUL => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Int(a * b)),
                _ => bail!("MUL requires Ints"),
            }
        }
        Opcode::DIV => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => {
                    if b == 0 { bail!("division by zero"); }
                    vm.stack.push(Value::Int(a / b))
                }
                _ => bail!("DIV requires Ints"),
            }
        }
        Opcode::MOD => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => {
                    if b == 0 { bail!("mod by zero"); }
                    vm.stack.push(Value::Int(a % b))
                }
                _ => bail!("MOD requires Ints"),
            }
        }
        Opcode::NEG => {
            let a = vm.stack.pop();
            match a {
                Some(Value::Int(a)) => vm.stack.push(Value::Int(-a)),
                _ => bail!("NEG requires Int"),
            }
        }

        // Comparisons
        Opcode::CMP_EQ => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            vm.stack.push(Value::Bool(a == b));
        }
        Opcode::CMP_NEQ => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            vm.stack.push(Value::Bool(a != b));
        }
        Opcode::CMP_LT => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Bool(a < b)),
                _ => bail!("CMP_LT requires Ints"),
            }
        }
        Opcode::CMP_LTE => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Bool(a <= b)),
                _ => bail!("CMP_LTE requires Ints"),
            }
        }
        Opcode::CMP_GT => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Bool(a > b)),
                _ => bail!("CMP_GT requires Ints"),
            }
        }
        Opcode::CMP_GTE => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Int(a)), Some(Value::Int(b))) => vm.stack.push(Value::Bool(a >= b)),
                _ => bail!("CMP_GTE requires Ints"),
            }
        }

        // Logical
        Opcode::AND => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Bool(a)), Some(Value::Bool(b))) => vm.stack.push(Value::Bool(a && b)),
                _ => bail!("AND requires Bools"),
            }
        }
        Opcode::OR => {
            let (b, a) = (vm.stack.pop(), vm.stack.pop());
            match (a, b) {
                (Some(Value::Bool(a)), Some(Value::Bool(b))) => vm.stack.push(Value::Bool(a || b)),
                _ => bail!("OR requires Bools"),
            }
        }
        Opcode::NOT => {
            let a = vm.stack.pop();
            match a {
                Some(Value::Bool(a)) => vm.stack.push(Value::Bool(!a)),
                _ => bail!("NOT requires Bool"),
            }
        }

        // Collections
        Opcode::CREATE_LIST => {
            let n = inst.op16() as usize;
            if vm.stack.len() < n { bail!("CREATE_LIST underflow"); }
            let mut items = Vec::with_capacity(n);
            for _ in 0..n { items.push(vm.stack.pop().unwrap()); }
            items.reverse();
            vm.stack.push(Value::List(items));
        }
        Opcode::CREATE_TUPLE => {
            let n = inst.op16() as usize;
            if vm.stack.len() < n { bail!("CREATE_TUPLE underflow"); }
            let mut items = Vec::with_capacity(n);
            for _ in 0..n { items.push(vm.stack.pop().unwrap()); }
            items.reverse();
            vm.stack.push(Value::Tuple(items));
        }
        Opcode::CREATE_MAP => {
            let n = inst.op16() as usize;
            if vm.stack.len() < n * 2 { bail!("CREATE_MAP underflow"); }
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
                (Some(Value::List(mut a)), Some(Value::List(b))) => { a.extend(b); vm.stack.push(Value::List(a)); }
                _ => bail!("CONCAT expects (Str,Str) or (List,List)"),
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
                _ => bail!("DIFF expects (List,List)"),
            }
        }

        // Pattern placeholders
        Opcode::PATTERN => { vm.stack.push(Value::Nil); }
        Opcode::MATCH_TEST => { let _val = vm.stack.pop(); let _pat = vm.stack.pop(); vm.stack.push(Value::Bool(true)); }
        Opcode::EXTRACT_BINDINGS => { vm.stack.push(Value::Map(vec![])); }

        // Locals
        Opcode::ALLOC_LOCAL => { process.locals.push(Value::Nil); }
        Opcode::LOAD_LOCAL => {
            let idx = inst.op16() as usize;
            if let Some(v) = process.locals.get(idx).cloned() { vm.stack.push(v); } else { return Err(ExecError::OpcodeParamError { opcode: "LOAD_LOCAL", message: format!("out of bounds: {}", idx) }.into()); }
        }
        Opcode::STORE_LOCAL => {
            let idx = inst.op16() as usize;
            let v = vm.stack.pop().ok_or_else(|| anyhow!("stack underflow on STORE_LOCAL"))?;
            if idx < process.locals.len() { process.locals[idx] = v; } else { return Err(ExecError::OpcodeParamError { opcode: "STORE_LOCAL", message: format!("out of bounds: {}", idx) }.into()); }
        }

        // Continuations (store/resume)
        Opcode::CONT_STORE => {
            let v = vm.stack.pop().ok_or_else(|| anyhow!("stack underflow on CONT_STORE"))?;
            let id = vm.next_cont_id;
            vm.next_cont_id = vm.next_cont_id.wrapping_add(1).max(1);
            vm.cont_table.insert(id, v);
            vm.stack.push(Value::Int(id as i64));
        }
        Opcode::CONT_RESUME => {
            let id = match vm.stack.pop() {
                Some(Value::Int(n)) if n >= 0 => n as u32,
                other => bail!("CONT_RESUME expects non-negative Int id, got {:?}", other),
            };
            let v = vm.cont_table.remove(&id).unwrap_or(Value::Nil);
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
            let data = vm.stack.pop().ok_or_else(|| anyhow!("stack underflow on TELL data"))?;
            let chan = match vm.stack.pop() { Some(Value::Name(s)) => s, other => return Err(ExecError::OpcodeParamError { opcode: "TELL", message: format!("expects Name channel, got {:?}", other) }.into()), };
            let key = (kind, chan);
            vm.rspace.entry(key).or_default().push(data);
            vm.stack.push(Value::Bool(true));
        }
        Opcode::ASK => {
            let kind = inst.op16();
            let chan = match vm.stack.pop() { Some(Value::Name(s)) => s, other => return Err(ExecError::OpcodeParamError { opcode: "ASK", message: format!("expects Name channel, got {:?}", other) }.into()), };
            let key = (kind, chan);
            let v = vm.rspace.get_mut(&key).and_then(|q| if q.is_empty(){None}else{Some(q.remove(0))});
            vm.stack.push(v.unwrap_or(Value::Nil));
        }
        Opcode::PEEK => {
            let kind = inst.op16();
            let chan = match vm.stack.pop() { Some(Value::Name(s)) => s, other => return Err(ExecError::OpcodeParamError { opcode: "PEEK", message: format!("expects Name channel, got {:?}", other) }.into()), };
            let key = (kind, chan);
            let v = vm.rspace.get(&key).and_then(|q| q.get(0)).cloned();
            vm.stack.push(v.unwrap_or(Value::Nil));
        }

        // Control flow
        Opcode::JUMP => {
            // Expect a label name on stack (Value::Str)
            let label = match vm.stack.pop() {
                Some(Value::Str(s)) => s,
                other => return Err(ExecError::OpcodeParamError { opcode: "JUMP", message: format!("expects label String on stack, got {:?}", other) }.into()),
            };
            return Ok(StepResult::Jump(label));
        }
        Opcode::BRANCH_TRUE => {
            // Expect condition bool then label string on stack (label on top like typical assembly push order?)
            // We choose: pop condition first then label beneath? We'll require label on stack (Str) and condition (Bool) on top.
            let cond = match vm.stack.pop() {
                Some(Value::Bool(b)) => b,
                other => return Err(ExecError::OpcodeParamError { opcode: "BRANCH_TRUE", message: format!("expects Bool condition on stack, got {:?}", other) }.into()),
            };
            let label = match vm.stack.pop() {
                Some(Value::Str(s)) => s,
                other => return Err(ExecError::OpcodeParamError { opcode: "BRANCH_TRUE", message: format!("expects label String under condition, got {:?}", other) }.into()),
            };
            if cond { return Ok(StepResult::Jump(label)); }
        }
        Opcode::BRANCH_FALSE => {
            let cond = match vm.stack.pop() {
                Some(Value::Bool(b)) => b,
                other => return Err(ExecError::OpcodeParamError { opcode: "BRANCH_FALSE", message: format!("expects Bool condition on stack, got {:?}", other) }.into()),
            };
            let label = match vm.stack.pop() {
                Some(Value::Str(s)) => s,
                other => return Err(ExecError::OpcodeParamError { opcode: "BRANCH_FALSE", message: format!("expects label String under condition, got {:?}", other) }.into()),
            };
            if !cond { return Ok(StepResult::Jump(label)); }
        }
        Opcode::BRANCH_SUCCESS => {
            // For now, treat success as presence of Bool(true) on stack top; consume it and branch if true.
            let status = match vm.stack.pop() {
                Some(Value::Bool(b)) => b,
                Some(v) => { vm.stack.push(v); false },
                None => false,
            };
            let label = match vm.stack.pop() {
                Some(Value::Str(s)) => s,
                other => return Err(ExecError::OpcodeParamError { opcode: "BRANCH_SUCCESS", message: format!("expects label String under status/stack, got {:?}", other) }.into()),
            };
            if status { return Ok(StepResult::Jump(label)); }
        }

        // Unhandled opcodes default: do nothing
        _ => {}
    }

    Ok(StepResult::Next)
}
