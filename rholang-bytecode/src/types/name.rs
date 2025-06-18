use serde::{Deserialize, Serialize};
use crate::types::{UnforgeableName, Value};

/// Channel name for Send/Receive operations
/// Grammar: name: $ => choice($._proc_var, $.quote)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelName {
    /// Unforgeable channel name (from New instruction)
    Unforgeable(UnforgeableName),
    /// Variable reference to a channel (var from grammar)
    Variable(String),
    /// Wildcard channel name ('_' from grammar)
    Wildcard,
    /// Quoted process as channel name ('@' prefix from grammar)
    Quote(Value),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId {
    pub id: String,
}

/// Source types for input operations
/// Grammar: _source: $ => choice($.simple_source, $.receive_send_source, $.send_receive_source)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceName {
    /// Simple source: just a name
    Simple(ChannelName),
    /// Receive-send source: name with '?!' operator
    ReceiveSend(ChannelName),
    /// Send-receive source: name with '!?' operator and inputs
    SendReceive {
        name: ChannelName,
        inputs: Vec<Value>
    },
}

impl ChannelName {
    pub fn from_unforgeable(name: UnforgeableName) -> Self {
        Self::Unforgeable(name)
    }
    
    pub fn from_variable(name: impl Into<String>) -> Self {
        Self::Variable(name.into())
    }
    
    pub fn wildcard() -> Self {
        Self::Wildcard
    }
    
    pub fn from_quote(value: Value) -> Self {
        Self::Quote(value)
    }
    
    pub fn is_unforgeable(&self) -> bool {
        matches!(self, Self::Unforgeable(_))
    }
    
    pub fn is_variable(&self) -> bool {
        matches!(self, Self::Variable(_))
    }
    
    pub fn is_wildcard(&self) -> bool {
        matches!(self, Self::Wildcard)
    }
    
    pub fn is_quote(&self) -> bool {
        matches!(self, Self::Quote(_))
    }
    
    pub fn as_variable(&self) -> Option<&str> {
        match self {
            Self::Variable(name) => Some(name),
            _ => None,
        }
    }
}

impl ProcessId {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
    
    pub fn generate() -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut hasher = DefaultHasher::new();
        SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .hash(&mut hasher);

        let hash = hasher.finish();
        Self::new(format!("proc_{:x}", hash))
    }
    
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

impl SourceName {
    pub fn simple(name: ChannelName) -> Self {
        Self::Simple(name)
    }
    
    pub fn receive_send(name: ChannelName) -> Self {
        Self::ReceiveSend(name)
    }
    
    pub fn send_receive(name: ChannelName, inputs: Vec<Value>) -> Self {
        Self::SendReceive { name, inputs }
    }
    
    pub fn channel_name(&self) -> &ChannelName {
        match self {
            Self::Simple(name) => name,
            Self::ReceiveSend(name) => name,
            Self::SendReceive { name, .. } => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_name_from_variable() {
        let channel = ChannelName::from_variable("stdout");
        assert!(channel.is_variable());
        assert!(!channel.is_unforgeable());
        assert_eq!(channel.as_variable(), Some("stdout"));
    }

    #[test]
    fn test_channel_name_from_unforgeable() {
        let unforgeable = UnforgeableName::generate();
        let channel = ChannelName::from_unforgeable(unforgeable.clone());
        assert!(channel.is_unforgeable());
        assert!(!channel.is_variable());
        assert_eq!(channel, ChannelName::Unforgeable(unforgeable));
    }

    #[test]
    fn test_channel_name_wildcard() {
        let channel = ChannelName::wildcard();
        assert!(channel.is_wildcard());
        assert!(!channel.is_variable());
        assert!(!channel.is_unforgeable());
    }

    #[test]
    fn test_process_id_generation() {
        let proc1 = ProcessId::generate();
        let proc2 = ProcessId::generate();
        assert_ne!(proc1.id, proc2.id);
        assert!(proc1.id.starts_with("proc_"));
        assert!(proc1.as_str().starts_with("proc_"));
    }

    #[test]
    fn test_source_name_types() {
        let channel = ChannelName::from_variable("test");

        let simple = SourceName::simple(channel.clone());
        assert_eq!(simple.channel_name(), &channel);

        let recv_send = SourceName::receive_send(channel.clone());
        assert_eq!(recv_send.channel_name(), &channel);

        let send_recv = SourceName::send_receive(channel.clone(), vec![]);
        assert_eq!(send_recv.channel_name(), &channel);
    }
}
