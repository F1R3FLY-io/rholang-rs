use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core Rholang data types that can be serialized to/from JSON
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum RholangValue {
    Nil,
    Bool(bool),
    Int(i64),
    String(String),
    List(Vec<RholangValue>),
    Map(HashMap<String, RholangValue>),
    Process(String), // Serialized process as string
}

impl RholangValue {
    /// Convert to JSON string
    pub fn to_json(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(self).map_err(Into::into)
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        serde_json::from_str(json).map_err(Into::into)
    }

    /// Get type name as string
    pub fn type_name(&self) -> &'static str {
        match self {
            RholangValue::Nil => "Nil",
            RholangValue::Bool(_) => "Bool",
            RholangValue::Int(_) => "Int",
            RholangValue::String(_) => "String",
            RholangValue::List(_) => "List",
            RholangValue::Map(_) => "Map",
            RholangValue::Process(_) => "Process",
        }
    }
}

/// Rholang expression that can contain values and operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RholangExpression {
    pub value: RholangValue,
    pub metadata: Option<HashMap<String, String>>,
}

impl RholangExpression {
    pub fn new(value: RholangValue) -> Self {
        Self {
            value,
            metadata: None,
        }
    }

    pub fn with_metadata(value: RholangValue, metadata: HashMap<String, String>) -> Self {
        Self {
            value,
            metadata: Some(metadata),
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(self).map_err(Into::into)
    }

    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        serde_json::from_str(json).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nil_json_serialization() {
        let nil = RholangValue::Nil;
        let json = nil.to_json().unwrap();
        let parsed = RholangValue::from_json(&json).unwrap();
        assert_eq!(nil, parsed);
    }

    #[test]
    fn test_bool_json_serialization() {
        let value = RholangValue::Bool(true);
        let json = value.to_json().unwrap();
        let parsed = RholangValue::from_json(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_int_json_serialization() {
        let value = RholangValue::Int(42);
        let json = value.to_json().unwrap();
        let parsed = RholangValue::from_json(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_string_json_serialization() {
        let value = RholangValue::String("hello".to_string());
        let json = value.to_json().unwrap();
        let parsed = RholangValue::from_json(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_list_json_serialization() {
        let value = RholangValue::List(vec![
            RholangValue::Int(1),
            RholangValue::String("test".to_string()),
            RholangValue::Bool(false),
        ]);
        let json = value.to_json().unwrap();
        let parsed = RholangValue::from_json(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_map_json_serialization() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), RholangValue::Int(42));
        map.insert(
            "key2".to_string(),
            RholangValue::String("value".to_string()),
        );

        let value = RholangValue::Map(map);
        let json = value.to_json().unwrap();
        let parsed = RholangValue::from_json(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_expression_json_serialization() {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "test".to_string());

        let expr = RholangExpression::with_metadata(RholangValue::Int(123), metadata);

        let json = expr.to_json().unwrap();
        let parsed = RholangExpression::from_json(&json).unwrap();
        assert_eq!(expr, parsed);
    }

    #[test]
    fn test_type_names() {
        assert_eq!(RholangValue::Nil.type_name(), "Nil");
        assert_eq!(RholangValue::Bool(true).type_name(), "Bool");
        assert_eq!(RholangValue::Int(1).type_name(), "Int");
        assert_eq!(RholangValue::String("".to_string()).type_name(), "String");
        assert_eq!(RholangValue::List(vec![]).type_name(), "List");
        assert_eq!(RholangValue::Map(HashMap::new()).type_name(), "Map");
        assert_eq!(RholangValue::Process("".to_string()).type_name(), "Process");
    }
}
