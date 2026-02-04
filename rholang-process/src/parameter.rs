// Parameter type for Process dependencies
// Parameters are named bindings that relate processes to entries in RSpace.
// A parameter is solved when its entry is in a resolved state.

use rholang_rspace::RSpace;

/// A named binding that relates a process to an entry in RSpace.
///
/// Parameters represent dependencies that must be resolved before a process can execute.
/// The parameter's name identifies an entry in RSpace, and the parameter is "solved"
/// when that entry is in a resolved state:
/// - Channel: solved if queue is non-empty
/// - Process: solved if in `ProcessState::Value` state
/// - Value: always solved
///
/// # Examples
///
/// ```
/// use rholang_process::Parameter;
///
/// // Create a parameter that depends on entry "input"
/// let param = Parameter::new("input");
/// assert_eq!(param.name(), "input");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    /// Entry name to look up in RSpace
    name: String,
}

impl Parameter {
    /// Create a new parameter with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The entry name (e.g., "input", "worker_result")
    ///
    /// # Examples
    ///
    /// ```
    /// use rholang_process::Parameter;
    ///
    /// let param = Parameter::new("channel");
    /// let param_worker = Parameter::new("worker_result");
    /// ```
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self { name: name.into() }
    }

    /// Get the entry name this parameter references.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if this parameter is solved by looking up its entry in RSpace.
    ///
    /// A parameter is solved when:
    /// - The entry is a Channel with non-empty queue
    /// - The entry is a Process in terminal `ProcessState::Value` state
    /// - The entry is a Value (always solved)
    ///
    /// A parameter is unsolved when:
    /// - No entry exists with that name
    /// - The entry is a Channel with empty queue
    /// - The entry is a Process in Wait, Ready, or Error state
    ///
    /// # Arguments
    ///
    /// * `rspace` - Reference to the RSpace to look up the entry in
    ///
    /// # Examples
    ///
    /// ```
    /// use rholang_process::{Parameter, RSpace};
    ///
    /// // With an RSpace instance...
    /// // param.is_solved(&rspace) returns true if the entry is resolved
    /// ```
    pub fn is_solved(&self, rspace: &dyn RSpace) -> bool {
        rspace.is_solved(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_new() {
        let param = Parameter::new("test");
        assert_eq!(param.name(), "test");
    }

    #[test]
    fn test_parameter_new_with_string() {
        let name = String::from("channel");
        let param = Parameter::new(name);
        assert_eq!(param.name(), "channel");
    }

    #[test]
    fn test_parameter_equality() {
        let p1 = Parameter::new("a");
        let p2 = Parameter::new("a");
        let p3 = Parameter::new("b");

        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_parameter_clone() {
        let p1 = Parameter::new("test");
        let p2 = p1.clone();

        assert_eq!(p1, p2);
        assert_eq!(p2.name(), "test");
    }

    #[test]
    fn test_parameter_debug() {
        let param = Parameter::new("test");
        let debug_str = format!("{:?}", param);
        assert!(debug_str.contains("test"));
    }
}
