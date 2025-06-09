use crate::rholang_types::{RholangExpression, RholangValue};
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

/// JSON command handler for Rholang interpreter
pub struct JsonCommands;

impl JsonCommands {
    /// Export Rholang value to JSON file
    pub fn export_to_file(value: &RholangValue, path: &str) -> Result<()> {
        let json = value.to_json()?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Import Rholang value from JSON file
    pub fn import_from_file(path: &str) -> Result<RholangValue> {
        if !Path::new(path).exists() {
            return Err(anyhow!("File not found: {}", path));
        }

        let content = fs::read_to_string(path)?;
        RholangValue::from_json(&content)
    }

    /// Export Rholang expression to JSON file
    pub fn export_expression_to_file(expr: &RholangExpression, path: &str) -> Result<()> {
        let json = expr.to_json()?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Import Rholang expression from JSON file
    pub fn import_expression_from_file(path: &str) -> Result<RholangExpression> {
        if !Path::new(path).exists() {
            return Err(anyhow!("File not found: {}", path));
        }

        let content = fs::read_to_string(path)?;
        RholangExpression::from_json(&content)
    }

    /// Convert Rholang value to JSON string for display
    pub fn to_json_string(value: &RholangValue) -> Result<String> {
        value.to_json()
    }

    /// Parse JSON string to Rholang value
    pub fn from_json_string(json: &str) -> Result<RholangValue> {
        RholangValue::from_json(json)
    }

    /// Validate JSON string can be parsed as Rholang value
    pub fn validate_json(json: &str) -> Result<()> {
        RholangValue::from_json(json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    #[test]
    fn test_export_import_value() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let original = RholangValue::Int(42);
        JsonCommands::export_to_file(&original, path).unwrap();

        let imported = JsonCommands::import_from_file(path).unwrap();
        assert_eq!(original, imported);
    }

    #[test]
    fn test_export_import_expression() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let mut metadata = HashMap::new();
        metadata.insert("test".to_string(), "value".to_string());

        let original =
            RholangExpression::with_metadata(RholangValue::String("test".to_string()), metadata);

        JsonCommands::export_expression_to_file(&original, path).unwrap();

        let imported = JsonCommands::import_expression_from_file(path).unwrap();
        assert_eq!(original, imported);
    }

    #[test]
    fn test_json_string_conversion() {
        let value = RholangValue::List(vec![
            RholangValue::Int(1),
            RholangValue::String("test".to_string()),
        ]);

        let json = JsonCommands::to_json_string(&value).unwrap();
        let parsed = JsonCommands::from_json_string(&json).unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn test_validate_json() {
        let valid_json = r#"{"type": "Int", "value": 42}"#;
        assert!(JsonCommands::validate_json(valid_json).is_ok());

        let invalid_json = r#"{"invalid": "json"}"#;
        assert!(JsonCommands::validate_json(invalid_json).is_err());
    }

    #[test]
    fn test_import_nonexistent_file() {
        let result = JsonCommands::import_from_file("nonexistent.json");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }
}
