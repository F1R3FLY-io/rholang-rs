//! Name types for Rholang bytecode.
//!
//! This module defines the Name type, which represents Rholang names that can be
//! used as channels for communication.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A Rholang name.
///
/// Names in Rholang are used as channels for communication. They can be created
/// using the `new` operator or derived from other values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Name {
    /// A name created with the `new` operator
    New {
        /// The index of the name in the registry
        index: u64,
    },
    /// A name derived from a quoted process
    Quote {
        /// The hash of the quoted process
        hash: Vec<u8>,
    },
    /// A name for a built-in channel
    Builtin {
        /// The name of the built-in channel
        name: String,
    },
    /// A name for a ground term (e.g., an integer or string)
    Ground {
        /// The hash of the ground term
        hash: Vec<u8>,
    },
}

impl Name {
    /// Creates a new name with the given index
    pub fn new(index: u64) -> Self {
        Name::New { index }
    }

    /// Creates a name from a quoted process hash
    pub fn quote(hash: Vec<u8>) -> Self {
        Name::Quote { hash }
    }

    /// Creates a name for a built-in channel
    pub fn builtin<S: Into<String>>(name: S) -> Self {
        Name::Builtin { name: name.into() }
    }

    /// Creates a name for a ground term
    pub fn ground(hash: Vec<u8>) -> Self {
        Name::Ground { hash }
    }

    /// Returns true if this is a name created with the `new` operator
    pub fn is_new(&self) -> bool {
        matches!(self, Name::New { .. })
    }

    /// Returns true if this is a name derived from a quoted process
    pub fn is_quote(&self) -> bool {
        matches!(self, Name::Quote { .. })
    }

    /// Returns true if this is a name for a built-in channel
    pub fn is_builtin(&self) -> bool {
        matches!(self, Name::Builtin { .. })
    }

    /// Returns true if this is a name for a ground term
    pub fn is_ground(&self) -> bool {
        matches!(self, Name::Ground { .. })
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Name::New { index } => write!(f, "@{}", index),
            Name::Quote { hash } => {
                write!(f, "@quote(")?;
                for (i, byte) in hash.iter().enumerate() {
                    if i > 0 && i % 8 == 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{:02x}", byte)?;
                }
                write!(f, ")")
            }
            Name::Builtin { name } => write!(f, "@{}", name),
            Name::Ground { hash } => {
                write!(f, "@ground(")?;
                for (i, byte) in hash.iter().enumerate() {
                    if i > 0 && i % 8 == 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{:02x}", byte)?;
                }
                write!(f, ")")
            }
        }
    }
}
