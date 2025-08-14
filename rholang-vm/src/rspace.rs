// Rholang RSpace Implementation
// Based on the design in BYTECODE_DESIGN.md

use crate::bytecode::{RSpaceType, Value};
use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::Value as JsonValue;

/// A pattern for matching against data in an RSpace
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// The pattern string
    pub pattern: String,
    /// Bound variables in the pattern
    pub bindings: Vec<String>,
}

/// A continuation that can be stored in an RSpace
#[derive(Debug, Clone)]
pub struct Continuation {
    /// The process to execute when the continuation is resumed
    pub process: String,
    /// The environment for the continuation
    pub environment: HashMap<String, Value>,
}

/// A channel name in an RSpace
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelName {
    /// The name of the channel
    pub name: String,
    /// The RSpace type the channel belongs to
    pub rspace_type: RSpaceType,
}

/// A data entry in an RSpace
#[derive(Debug, Clone)]
pub struct DataEntry {
    /// The channel name
    pub channel: ChannelName,
    /// The data stored at the channel
    pub data: Value,
}

/// A continuation entry in an RSpace
#[derive(Debug, Clone)]
pub struct ContinuationEntry {
    /// The channel name
    pub channel: ChannelName,
    /// The pattern to match against
    pub pattern: Pattern,
    /// The continuation to resume when the pattern matches
    pub continuation: Continuation,
}

/// The result of a consume operation
#[derive(Debug, Clone)]
pub struct ConsumeResult {
    /// The data that was consumed
    pub data: Value,
    /// The bindings from the pattern match
    pub bindings: HashMap<String, Value>,
}

/// The RSpace interface
#[async_trait]
pub trait RSpace: Send + Sync {
    /// Support downcasting to concrete types for snapshotting
    fn as_any(&self) -> &dyn std::any::Any;
    /// Get the type of this RSpace
    fn get_type(&self) -> RSpaceType;

    /// Put data into the RSpace
    async fn put(&self, channel: ChannelName, data: Value) -> Result<()>;

    /// Get data from the RSpace (blocking)
    async fn get(&self, channel: ChannelName) -> Result<Value>;

    /// Get data from the RSpace (non-blocking)
    async fn get_nonblock(&self, channel: ChannelName) -> Result<Option<Value>>;

    /// Consume data from the RSpace
    async fn consume(
        &self,
        channel: ChannelName,
        pattern: Pattern,
        continuation: Continuation,
    ) -> Result<Option<ConsumeResult>>;

    /// Produce data to the RSpace
    async fn produce(&self, channel: ChannelName, data: Value) -> Result<()>;

    /// Peek at data without consuming
    async fn peek(&self, channel: ChannelName) -> Result<Option<Value>>;

    /// Pattern match against RSpace data
    async fn pattern_match(
        &self,
        channel: ChannelName,
        pattern: Pattern,
    ) -> Result<Option<HashMap<String, Value>>>;

    /// Create a fresh name in the RSpace
    async fn name_create(&self) -> Result<ChannelName>;

    /// Quote process to name in the RSpace
    async fn name_quote(&self, process: String) -> Result<ChannelName>;

    /// Unquote name to process in the RSpace
    async fn name_unquote(&self, name: ChannelName) -> Result<String>;
}

/// Type alias for data storage in RSpace
type DataStorage = Arc<Mutex<HashMap<ChannelName, Vec<Value>>>>;

/// Type alias for continuations storage in RSpace
type ContinuationStorage = Arc<Mutex<HashMap<ChannelName, Vec<(Pattern, Continuation)>>>>;

/// In-memory sequential RSpace implementation
/// Snapshot provider for guest-visible RSpace state
pub trait RSpaceSnapshotProvider {
    /// Returns a canonical map: channel_name -> JSON object with data and continuations
    fn snapshot_channels(&self) -> BTreeMap<String, JsonValue>;
}

/// In-memory sequential RSpace implementation
pub struct MemorySequentialRSpace {
    /// The data stored in the RSpace
    data: DataStorage,
    /// The continuations stored in the RSpace
    continuations: ContinuationStorage,
}

impl Default for MemorySequentialRSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl MemorySequentialRSpace {
    /// Create a new in-memory sequential RSpace
    pub fn new() -> Self {
        MemorySequentialRSpace {
            data: Arc::new(Mutex::new(HashMap::new())),
            continuations: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl RSpaceSnapshotProvider for MemorySequentialRSpace {
    fn snapshot_channels(&self) -> BTreeMap<String, JsonValue> {
        // Try to take non-blocking locks to avoid async in snapshot path
        let mut map: BTreeMap<String, JsonValue> = BTreeMap::new();
        let data_opt = self.data.try_lock();
        let cont_opt = self.continuations.try_lock();

        let data_map = match data_opt {
            Ok(g) => Some(g),
            Err(_) => None,
        };
        let cont_map = match cont_opt {
            Ok(g) => Some(g),
            Err(_) => None,
        };

        // Collect all channel names from either map
        let mut channel_names: Vec<String> = Vec::new();
        if let Some(dm) = data_map.as_ref() {
            for ch in dm.keys() { channel_names.push(ch.name.clone()); }
        }
        if let Some(cm) = cont_map.as_ref() {
            for ch in cm.keys() { channel_names.push(ch.name.clone()); }
        }
        channel_names.sort();
        channel_names.dedup();

        for ch_name in channel_names {
            // Collect data list
            let data_list = if let Some(dm) = data_map.as_ref() {
                // find by constructed ChannelName key; rspace_type can be taken from self.get_type()
                let key = ChannelName { name: ch_name.clone(), rspace_type: self.get_type() };
                if let Some(values) = dm.get(&key) {
                    let arr: Vec<JsonValue> = values.iter().map(|v| serde_json::to_value(v).unwrap_or(JsonValue::Null)).collect();
                    arr
                } else { vec![] }
            } else { vec![] };

            // Collect continuations list (redacted to guest-visible summary)
            let cont_list = if let Some(cm) = cont_map.as_ref() {
                let key = ChannelName { name: ch_name.clone(), rspace_type: self.get_type() };
                if let Some(conts) = cm.get(&key) {
                    let mut out: Vec<JsonValue> = Vec::new();
                    for (pat, kont) in conts.iter() {
                        let env_json = serde_json::to_value(&kont.environment).unwrap_or(JsonValue::Null);
                        let obj = serde_json::json!({
                            "pattern": pat.pattern,
                            "bindings": pat.bindings,
                            "process": kont.process,
                            "environment": env_json
                        });
                        out.push(obj);
                    }
                    out
                } else { vec![] }
            } else { vec![] };

            let obj = serde_json::json!({
                "data": data_list,
                "continuations": cont_list
            });
            map.insert(ch_name, obj);
        }

        map
    }
}

#[async_trait]
impl RSpace for MemorySequentialRSpace {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_type(&self) -> RSpaceType {
        RSpaceType::MemorySequential
    }

    async fn put(&self, channel: ChannelName, data: Value) -> Result<()> {
        let mut data_map = self.data.lock().await;
        data_map.entry(channel).or_insert_with(Vec::new).push(data);
        Ok(())
    }

    async fn get(&self, channel: ChannelName) -> Result<Value> {
        let mut data_map = self.data.lock().await;
        let values = data_map
            .get_mut(&channel)
            .ok_or_else(|| anyhow!("Channel not found"))?;
        if values.is_empty() {
            bail!("No data available on channel");
        }
        Ok(values.remove(0))
    }

    async fn get_nonblock(&self, channel: ChannelName) -> Result<Option<Value>> {
        let mut data_map = self.data.lock().await;
        let values = match data_map.get_mut(&channel) {
            Some(values) => values,
            None => return Ok(None),
        };
        if values.is_empty() {
            return Ok(None);
        }
        Ok(Some(values.remove(0)))
    }

    async fn consume(
        &self,
        channel: ChannelName,
        pattern: Pattern,
        continuation: Continuation,
    ) -> Result<Option<ConsumeResult>> {
        // Check if there's data available
        let mut data_map = self.data.lock().await;
        let values = match data_map.get_mut(&channel) {
            Some(values) => values,
            None => {
                // No data available, store the continuation
                let mut cont_map = self.continuations.lock().await;
                cont_map
                    .entry(channel)
                    .or_insert_with(Vec::new)
                    .push((pattern, continuation));
                return Ok(None);
            }
        };

        if values.is_empty() {
            // No data available, store the continuation
            let mut cont_map = self.continuations.lock().await;
            cont_map
                .entry(channel)
                .or_insert_with(Vec::new)
                .push((pattern, continuation));
            return Ok(None);
        }

        // For now, just return the first value and empty bindings
        // In a real implementation, we would match the pattern against the data
        let data = values.remove(0);
        let bindings = HashMap::new();
        Ok(Some(ConsumeResult { data, bindings }))
    }

    async fn produce(&self, channel: ChannelName, data: Value) -> Result<()> {
        // First, check if there are continuations waiting for this channel
        let has_continuations = {
            let cont_map = self.continuations.lock().await;
            
            cont_map.get(&channel).is_some_and(|conts| !conts.is_empty())
        };

        if !has_continuations {
            // No continuations waiting, just store the data
            return self.put(channel, data).await;
        }

        // If we have continuations, try to get one
        let continuation_opt = {
            let mut cont_map = self.continuations.lock().await;
            
            cont_map.get_mut(&channel).and_then(|conts| {
                if conts.is_empty() {
                    None
                } else {
                    Some(conts.remove(0))
                }
            })
        };

        match continuation_opt {
            Some((_pattern, _continuation)) => {
                // In a real implementation, we would match the data against the pattern
                // For now, we'll assume the match is successful and create empty bindings
                let _bindings: HashMap<String, Value> = HashMap::new();
                
                // In a real implementation, we would resume the continuation with the data and bindings
                // For now, we'll just return success without storing the data
                // since the continuation has "consumed" it
                Ok(())
            },
            None => {
                // No continuations found (this should be rare due to our check above)
                self.put(channel, data).await
            }
        }
    }

    async fn peek(&self, channel: ChannelName) -> Result<Option<Value>> {
        let data_map = self.data.lock().await;
        let values = match data_map.get(&channel) {
            Some(values) => values,
            None => return Ok(None),
        };
        if values.is_empty() {
            return Ok(None);
        }
        Ok(Some(values[0].clone()))
    }

    async fn pattern_match(
        &self,
        channel: ChannelName,
        _pattern: Pattern,
    ) -> Result<Option<HashMap<String, Value>>> {
        // Check if there's data available
        let data_map = self.data.lock().await;
        let values = match data_map.get(&channel) {
            Some(values) => values,
            None => return Ok(None),
        };
        if values.is_empty() {
            return Ok(None);
        }

        // For now, just return empty bindings
        // In a real implementation, we would match the pattern against the data
        Ok(Some(HashMap::new()))
    }

    async fn name_create(&self) -> Result<ChannelName> {
        // Generate a unique name
        let name = format!("name_{}", uuid::Uuid::new_v4());
        Ok(ChannelName {
            name,
            rspace_type: self.get_type(),
        })
    }

    async fn name_quote(&self, process: String) -> Result<ChannelName> {
        // For now, just create a new name with the process as a prefix
        let name = format!("quote_{}_{}", process, uuid::Uuid::new_v4());
        Ok(ChannelName {
            name,
            rspace_type: self.get_type(),
        })
    }

    async fn name_unquote(&self, name: ChannelName) -> Result<String> {
        // For now, just return a placeholder process
        Ok(format!("unquoted_{}", name.name))
    }
}

/// In-memory concurrent RSpace implementation
pub struct MemoryConcurrentRSpace {
    /// The underlying sequential RSpace
    /// In a real implementation, this would use a concurrent data structure
    sequential: MemorySequentialRSpace,
}

impl Default for MemoryConcurrentRSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryConcurrentRSpace {
    /// Create a new in-memory concurrent RSpace
    pub fn new() -> Self {
        MemoryConcurrentRSpace {
            sequential: MemorySequentialRSpace::new(),
        }
    }
}

impl RSpaceSnapshotProvider for MemoryConcurrentRSpace {
    fn snapshot_channels(&self) -> BTreeMap<String, JsonValue> {
        // Delegate to underlying sequential backend
        self.sequential.snapshot_channels()
    }
}

#[async_trait]
impl RSpace for MemoryConcurrentRSpace {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_type(&self) -> RSpaceType {
        RSpaceType::MemoryConcurrent
    }

    async fn put(&self, channel: ChannelName, data: Value) -> Result<()> {
        self.sequential.put(channel, data).await
    }

    async fn get(&self, channel: ChannelName) -> Result<Value> {
        self.sequential.get(channel).await
    }

    async fn get_nonblock(&self, channel: ChannelName) -> Result<Option<Value>> {
        self.sequential.get_nonblock(channel).await
    }

    async fn consume(
        &self,
        channel: ChannelName,
        pattern: Pattern,
        continuation: Continuation,
    ) -> Result<Option<ConsumeResult>> {
        self.sequential
            .consume(channel, pattern, continuation)
            .await
    }

    async fn produce(&self, channel: ChannelName, data: Value) -> Result<()> {
        self.sequential.produce(channel, data).await
    }

    async fn peek(&self, channel: ChannelName) -> Result<Option<Value>> {
        self.sequential.peek(channel).await
    }

    async fn pattern_match(
        &self,
        channel: ChannelName,
        pattern: Pattern,
    ) -> Result<Option<HashMap<String, Value>>> {
        self.sequential.pattern_match(channel, pattern).await
    }

    async fn name_create(&self) -> Result<ChannelName> {
        // Generate a unique name
        let name = format!("name_{}", uuid::Uuid::new_v4());
        Ok(ChannelName {
            name,
            rspace_type: self.get_type(),
        })
    }

    async fn name_quote(&self, process: String) -> Result<ChannelName> {
        // For now, just create a new name with the process as a prefix
        let name = format!("quote_{}_{}", process, uuid::Uuid::new_v4());
        Ok(ChannelName {
            name,
            rspace_type: self.get_type(),
        })
    }

    async fn name_unquote(&self, name: ChannelName) -> Result<String> {
        // For now, just return a placeholder process
        Ok(format!("unquoted_{}", name.name))
    }
}

/// Factory for creating RSpace instances
pub struct RSpaceFactory;

impl RSpaceFactory {
    /// Create a new RSpace instance of the specified type
    pub fn create(rspace_type: RSpaceType) -> Result<Box<dyn RSpace>> {
        match rspace_type {
            RSpaceType::MemorySequential => Ok(Box::new(MemorySequentialRSpace::new())),
            RSpaceType::MemoryConcurrent => Ok(Box::new(MemoryConcurrentRSpace::new())),
            RSpaceType::StoreSequential => bail!("Store-based RSpace not implemented yet"),
            RSpaceType::StoreConcurrent => bail!("Store-based RSpace not implemented yet"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_sequential_rspace_put_get() -> Result<()> {
        let rspace = MemorySequentialRSpace::new();
        let channel = ChannelName {
            name: "test".to_string(),
            rspace_type: RSpaceType::MemorySequential,
        };
        let data = Value::Int(42);

        rspace.put(channel.clone(), data.clone()).await?;
        let result = rspace.get(channel).await?;

        assert_eq!(result, data);
        Ok(())
    }

    #[tokio::test]
    async fn test_memory_sequential_rspace_peek() -> Result<()> {
        let rspace = MemorySequentialRSpace::new();
        let channel = ChannelName {
            name: "test".to_string(),
            rspace_type: RSpaceType::MemorySequential,
        };
        let data = Value::Int(42);

        rspace.put(channel.clone(), data.clone()).await?;
        let result = rspace.peek(channel.clone()).await?;

        assert_eq!(result, Some(data.clone()));

        // Peek doesn't consume the data
        let result2 = rspace.peek(channel).await?;
        assert_eq!(result2, Some(data));

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_sequential_rspace_name_create() -> Result<()> {
        let rspace = MemorySequentialRSpace::new();
        let name1 = rspace.name_create().await?;
        let name2 = rspace.name_create().await?;

        assert_ne!(name1.name, name2.name);
        assert_eq!(name1.rspace_type, RSpaceType::MemorySequential);
        assert_eq!(name2.rspace_type, RSpaceType::MemorySequential);

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_concurrent_rspace() -> Result<()> {
        let rspace = MemoryConcurrentRSpace::new();
        let channel = ChannelName {
            name: "test".to_string(),
            rspace_type: RSpaceType::MemoryConcurrent,
        };
        let data = Value::Int(42);

        rspace.put(channel.clone(), data.clone()).await?;
        let result = rspace.get(channel).await?;

        assert_eq!(result, data);
        Ok(())
    }

    #[tokio::test]
    async fn test_rspace_factory() -> Result<()> {
        let rspace = RSpaceFactory::create(RSpaceType::MemorySequential)?;
        assert_eq!(rspace.get_type(), RSpaceType::MemorySequential);

        let rspace = RSpaceFactory::create(RSpaceType::MemoryConcurrent)?;
        assert_eq!(rspace.get_type(), RSpaceType::MemoryConcurrent);

        Ok(())
    }
}
