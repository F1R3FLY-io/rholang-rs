use anyhow::{anyhow, bail, Result};
use rholang_bytecode::core::instructions::{Instruction as CoreInst, InstructionData};
use rholang_bytecode::core::opcodes::Opcode;

use crate::process::Process;
use crate::value::Value;
use crate::vm::VM;

pub fn step(vm: &mut VM, process: &mut Process, inst: CoreInst) -> Result<bool> {
    let opcode = inst.opcode()?;
    match opcode {
        Opcode::NOP => {}
        Opcode::HALT => { return Ok(true); }
        Opcode::PUSH_INT => {
            let imm = inst.op16() as i16 as i64;
            vm.stack.push(Value::Int(imm));
        }
        Opcode::PUSH_BOOL => {
            let v = inst.op1() != 0;
            vm.stack.push(Value::Bool(v));
        }
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
                    if b == 0 { bail!("modulo by zero"); }
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

        // Collections
        Opcode::CREATE_LIST => {
            let n = inst.op16() as usize;
            if vm.stack.len() < n { bail!("stack underflow creating list"); }
            let mut buf = Vec::with_capacity(n);
            for _ in 0..n { buf.push(vm.stack.pop().unwrap()); }
            buf.reverse();
            vm.stack.push(Value::List(buf));
        }
        Opcode::CREATE_TUPLE => {
            let n = inst.op16() as usize;
            if vm.stack.len() < n { bail!("stack underflow creating tuple"); }
            let mut buf = Vec::with_capacity(n);
            for _ in 0..n { buf.push(vm.stack.pop().unwrap()); }
            buf.reverse();
            vm.stack.push(Value::Tuple(buf));
        }
        Opcode::CREATE_MAP => {
            let n = inst.op16() as usize;
            if vm.stack.len() < n * 2 { bail!("stack underflow creating map"); }
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
            if let Some(v) = process.locals.get(idx).cloned() { vm.stack.push(v); } else { bail!("LOAD_LOCAL out of bounds: {}", idx); }
        }
        Opcode::STORE_LOCAL => {
            let idx = inst.op16() as usize;
            let v = vm.stack.pop().ok_or_else(|| anyhow!("stack underflow on STORE_LOCAL"))?;
            if idx < process.locals.len() { process.locals[idx] = v; } else { bail!("STORE_LOCAL out of bounds: {}", idx); }
        }

        // Continuations (store/resume)
        Opcode::CONT_STORE => {
            // Pop a value (e.g., Process/String/Int) and store; push Int(id)
            let v = vm.stack.pop().ok_or_else(|| anyhow!("stack underflow on CONT_STORE"))?;
            let id = vm.next_cont_id;
            vm.next_cont_id = vm.next_cont_id.wrapping_add(1).max(1);
            vm.cont_table.insert(id, v);
            vm.stack.push(Value::Int(id as i64));
        }
        Opcode::CONT_RESUME => {
            // Pop Int(id); push stored value (or Nil if missing)
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
            let chan = match vm.stack.pop() { Some(Value::Name(s)) => s, other => bail!("TELL expects Name channel, got {:?}", other), };
            let key = (kind, chan);
            vm.rspace.entry(key).or_default().push(data);
            vm.stack.push(Value::Bool(true));
        }
        Opcode::ASK => {
            let kind = inst.op16();
            let chan = match vm.stack.pop() { Some(Value::Name(s)) => s, other => bail!("ASK expects Name channel, got {:?}", other), };
            let key = (kind, chan);
            let v = vm.rspace.get_mut(&key).and_then(|q| if q.is_empty(){None}else{Some(q.remove(0))});
            vm.stack.push(v.unwrap_or(Value::Nil));
        }
        Opcode::PEEK => {
            let kind = inst.op16();
            let chan = match vm.stack.pop() { Some(Value::Name(s)) => s, other => bail!("PEEK expects Name channel, got {:?}", other), };
            let key = (kind, chan);
            let v = vm.rspace.get(&key).and_then(|q| q.get(0)).cloned();
            vm.stack.push(v.unwrap_or(Value::Nil));
        }

        _ => { /* ignore unhandled opcodes for now */ }
    }
    Ok(false)
}
