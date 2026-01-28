pub mod in_memory;
pub mod path_map;
pub use in_memory::InMemoryRSpace;
pub use path_map::PathMapRSpace;
pub use rholang_process::{
    step, ExecError, Process, ProcessEvent, ProcessEventHandler, ProcessState, RSpace, StepResult,
    Value, VM,
};

// Set PathMapRSpace as the default implementation
pub type DefaultRSpace = PathMapRSpace;

/// Drain ready processes from a channel and re-store non-ready processes.
pub fn drain_ready_processes(
    rspace: &mut dyn RSpace,
    kind: u16,
    channel: String,
) -> anyhow::Result<Vec<Process>> {
    match rspace.ask(kind, channel.clone())? {
        Some(Value::Par(procs)) => {
            let (ready, pending): (Vec<_>, Vec<_>) = procs.into_iter().partition(|p| p.is_ready());
            if !pending.is_empty() {
                rspace.tell(kind, channel, Value::Par(pending))?;
            }
            Ok(ready)
        }
        Some(other) => {
            rspace.tell(kind, channel, other)?;
            Ok(Vec::new())
        }
        None => Ok(Vec::new()),
    }
}

pub(crate) fn ensure_kind_matches_channel(kind: u16, channel: &str) -> anyhow::Result<()> {
    if !channel.starts_with(&format!("@{}:", kind)) {
        anyhow::bail!(
            "channel-kind mismatch: kind {} does not match channel {}",
            kind,
            channel
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rholang_bytecode::core::instructions::Instruction;
    use rholang_bytecode::core::Opcode;

    #[test]
    fn test_default_rspace() {
        let rspace = DefaultRSpace::default();
        // Just verify it's the right type
        let _: PathMapRSpace = rspace;
    }

    #[test]
    fn test_drain_ready_processes() -> anyhow::Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(DefaultRSpace::default());
        let channel = "@0:test_ready".to_string();

        let ready_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "ready_proc");
        let wait_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "wait_proc")
            .with_state(ProcessState::Wait);

        rspace.tell(0, channel.clone(), Value::Par(vec![ready_proc, wait_proc]))?;

        let ready = drain_ready_processes(rspace.as_mut(), 0, channel.clone())?;
        assert_eq!(ready.len(), 1);
        assert!(ready[0].is_ready());

        let pending = rspace.ask(0, channel.clone())?;
        match pending {
            Some(Value::Par(pending)) => {
                assert_eq!(pending.len(), 1);
                assert!(matches!(pending[0].state, ProcessState::Wait));
            }
            _ => panic!("expected pending process stored back"),
        }

        Ok(())
    }
}
