use crate::interpreter::Interpreter;

use rholang::rust::interpreter::interpreter;
use rholang::rust::interpreter::errors::InterpreterError;
use rholang::rust::interpreter::reduce::DebruijnInterpreter;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use rholang::rust::interpreter::accounting::_cost;
use rholang::rust::interpreter::accounting::costs::Cost as RholangCost;
use rholang::rust::interpreter::dispatch::RholangAndScalaDispatcher;

pub struct RhInterpreter;

#[async_trait]
impl Interpreter for RhInterpreter {
    
        /// Creates a default DebruijnInterpreter instance configured and ready to use
    fn create_default_interpreter() -> DebruijnInterpreter {
        // Create shared resources
        let environment_map = Arc::new(RwLock::new(HashMap::new()));
        let free_map = Arc::new(RwLock::new(HashSet::new()));

        // Create cost accounting with proper initialization
        // Using create method instead of default which doesn't exist
        let cost_accounting = Arc::new(RwLock::new(RholangCost::create(0, "initial".to_string())));

        // Create DebruijnInterpreter with struct initialization
        DebruijnInterpreter {
            space: Arc::new(Mutex::new(())),
            dispatcher: Arc::new(Mutex::new(RholangAndScalaDispatcher {})),
            urn_map: Default::default(),
            merge_chs: Arc::new(Default::default()),
            mergeable_tag_name: Default::default(),
            cost: _cost,
            substitute: Substitute {},
        }
    }

    async fn interpret(&self, code: String) -> Result<String> {
        // Create a new DebruijnInterpreter
        let map = Arc::new(RwLock::new(HashMap::new()));
        let set = Arc::new(RwLock::new(HashSet::new()));
        let cost_accounting = Arc::new(RwLock::new(RholangCost::default()));
        let db_interpreter = DebruijnInterpreter::new(map, set, cost_accounting);

        // Execute the Rholang code
        match db_interpreter.execute(code.as_str()) {
            Ok(result) => Ok(result.pretty_print()),
            Err(error) => Err(anyhow!("Interpreter error: {:?}", error))
        }
    }
}
