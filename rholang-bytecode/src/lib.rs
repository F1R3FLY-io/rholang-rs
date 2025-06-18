//! Rholang Bytecode Library
pub mod types;
pub mod constants;
pub mod errors;

pub use types::*;
pub use constants::ConstantPool;
pub use errors::BytecodeError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_setup() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
