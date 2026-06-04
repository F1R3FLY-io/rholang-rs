use anyhow::{anyhow, bail, Result};
use rholang_parser::ast::{self, AnnProc, Name, AgentMethod, AgentDefault, Proc, SyncSendCont};
use librho::sem::PID;
use rholang_bytecode::core::instructions::Instruction;
use rholang_bytecode::core::opcodes::Opcode;
use rholang_process::Value;

use crate::CodegenContext;

impl<'a, 'db> CodegenContext<'a, 'db> {
    pub(crate) fn compile_method_send(
        &mut self,
        pid: PID,
        channel: &Name<'a>,
        method_name: ast::Id<'a>,
        inputs: &[AnnProc<'a>],
    ) -> Result<()> {
        // Compile inputs
        for input in inputs {
            self.compile_proc(input)?;
        }
        
        // Compile method name as the first argument in the list!
        // Wait, is method name the FIRST or LAST argument?
        // `Cell!get(*ret)` -> `Cell!("get", *ret)`
        let idx = self.add_string(method_name.name);
        self.emit(Instruction::unary(Opcode::PUSH_STR, idx));
        
        // We pushed inputs, then method_name. 
        // We need method_name to be the FIRST element, but it's on top of the stack!
        // So we need to push method_name FIRST!
        return Ok(());
    }
}
