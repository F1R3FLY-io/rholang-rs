# Nightly Toolchain Requirements

*Last updated: July 22, 2025*

## Recommendation and Core Problem

**We strongly recommend using the nightly-2025-05-08 Rust toolchain for this project.**

This specific version is required to get access to the experimental `#[may_dangle]` attribute for constructors, which is crucial for our project's goals of safely implementing self-referential structs and integrating with libraries that require this feature. Without this nightly toolchain, you will encounter compilation errors when working with our advanced data structures and when using certain external dependencies like `smallvec`.

## Rationale for Choosing nightly-2025-05-08

### Why This Specific Date?

The date 2025-05-08 was carefully chosen to provide the best balance between stability and access to the features we need:

1. **Feature Freeze Alignment**: This date corresponds to the feature freeze of the 1.88.0 stable release, which occurred on May 9, 2025 (ahead of its June 26, 2025 release).

2. **Stability**: By choosing a nightly that's aligned with a feature freeze, we get a toolchain that's as close as possible to a well-tested stable release while still having access to the experimental features we need.

3. **Consistency**: Using a specific date ensures all contributors are using the exact same compiler version, avoiding subtle differences in behavior that could occur with different nightly builds.

### Understanding Rust's Release Cycle

Rust follows a six-week release cycle with three release channels:

- **Nightly**: Released every night, contains the latest features and changes
- **Beta**: Released every six weeks from the nightly channel, undergoes testing
- **Stable**: Released every six weeks from the beta channel, thoroughly tested

The feature freeze for a stable release happens when the corresponding beta is created. For the 1.88.0 release:

- Feature freeze: May 9, 2025 (when 1.88.0-beta.1 was created)
- Stable release: June 26, 2025 (when 1.88.0 was released)

By using nightly-2025-05-08 (one day before the feature freeze), we get a toolchain that's very close to what became the 1.88.0 stable release, but with the additional experimental features we need.

## Feature Spotlight: #[may_dangle] on Constructors

The `#[may_dangle]` attribute is a game-changer for our library's implementation of safe self-referential types and for integrating with other libraries that require this feature. Here's why it's so important:

### What is #[may_dangle]?

The `#[may_dangle]` attribute is part of Rust's "drop check" system, which ensures memory safety when dropping values that contain references. It allows you to tell the compiler that a particular lifetime parameter might not be used during the destruction of a value, even if it's used in the type itself.

This attribute is hidden behind the unstable `dropck_eyepatch` feature gate, which is why we need a nightly compiler to use it.

### Why Do We Need It?

1. **Self-Referential Structs**: Our library heavily relies on data structures that contain references to other parts of themselves or to data they own.

2. **Memory Safety**: Without `#[may_dangle]`, the Rust compiler is overly conservative about dropping values that contain references, which can lead to compilation errors even when the code is actually safe.

3. **External Library Integration**: Some of our dependencies, like `smallvec`, use this feature to implement their own safe drop behavior.

### Example: Self-Referential Structs with Lifetimes

Here's an example from our codebase showing a self-referential struct that benefits from `#[may_dangle]`:

```rust
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AnnProc<'ast> {
    pub proc: &'ast Proc<'ast>,
    pub span: SourceSpan,
}
```

In this example, `AnnProc` contains a reference to `Proc` with the same lifetime. When implementing drop behavior for such structures, the `#[may_dangle]` attribute allows the compiler to understand that the reference won't be used during destruction.

Another example is the `Name` enum:

```rust
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Name<'ast> {
    ProcVar(Var<'ast>),
    Quote(&'ast Proc<'ast>),
}
```

### Integration with External Libraries

The `#[may_dangle]` feature is not only important for our own code but is **essential for using certain external libraries with our parser**. A prime example is the `smallvec` crate, which our parser heavily depends on.

#### SmallVec Integration Example

Our rholang-parser uses `smallvec` extensively for efficient memory usage when dealing with small collections:

```rust
// In rholang-parser/src/ast.rs
use smallvec::SmallVec;

// Define types using SmallVec for efficient storage of small collections
pub type ProcList<'a> = SmallVec<[AnnProc<'a>; 1]>;
pub type Receipts<'a> = SmallVec<[Receipt<'a>; 1]>;
pub type Receipt<'a> = SmallVec<[Bind<'a>; 1]>;

// Example struct using SmallVec
pub struct Names<'ast> {
    pub names: SmallVec<[AnnName<'ast>; 1]>,
    pub remainder: Option<Var<'ast>>,
}
```

In our Cargo.toml, we explicitly enable the `may_dangle` feature for smallvec:

```toml
# Used by: rholang-parser
smallvec = { version = "1.15.1", features = ["union", "const_generics", "const_new", "may_dangle"] }
```

#### Why SmallVec Needs may_dangle

The `smallvec` library uses the `may_dangle` feature to safely implement drop behavior for vectors that contain borrowed references. This is critical for our parser because:

1. **Memory Efficiency**: SmallVec stores small collections inline, avoiding heap allocations for the common case of small AST nodes.
2. **Borrowed References**: Our AST nodes often contain references to other parts of the syntax tree.
3. **Safe Drop Behavior**: The `may_dangle` feature allows SmallVec to safely drop these collections without violating Rust's borrowing rules.

Without the nightly toolchain and the `may_dangle` feature, we would either:
- Need to use less efficient data structures
- Have to implement unsafe workarounds
- Be unable to use SmallVec's optimized implementation for our parser

This attribute enables us to build the foundation of our library's advanced data structures while maintaining Rust's safety guarantees and efficiently integrating with essential external libraries.

## Installation and Setup Guide

### Installing the Nightly Toolchain

To install the specific nightly toolchain we need, run:

```bash
rustup toolchain install nightly-2025-05-08
```

### Setting Up Your Project

#### Option 1: Directory Override

To use this toolchain for a specific project directory:

```bash
cd /path/to/project
rustup override set nightly-2025-05-08
```

#### Option 2: rust-toolchain.toml File

Alternatively, create a `rust-toolchain.toml` file in your project root:

```toml
[toolchain]
channel = "nightly-2025-05-08"
components = ["rustfmt", "clippy"]
```

This will automatically use the correct toolchain whenever you run Rust commands in the project directory.

### Enabling the Feature in Your Code

To use the `#[may_dangle]` attribute, you need to enable the `dropck_eyepatch` feature in your crate root (`lib.rs` or `main.rs`):

```rust
#![feature(dropck_eyepatch)]
```

### Verifying Your Setup

To verify that you're using the correct toolchain:

```bash
rustc --version
```

You should see output like:

```
rustc 1.89.0-nightly (1a1385f63 2025-05-08)
```

## Crucial Risks and Disclaimers

### Experimental Feature Warning

The `#[may_dangle]` attribute and the `dropck_eyepatch` feature are **experimental** and **unstable**. This means:

1. **API Changes**: The syntax or behavior could change in future Rust versions.
2. **Potential Removal**: The feature could be removed or significantly altered.
3. **Limited Documentation**: Experimental features often have less documentation and fewer examples.

### Nightly Toolchain Risks

Using a nightly toolchain comes with inherent risks:

1. **Known Bugs**: Nightly builds may contain known bugs that haven't been fixed yet.
2. **Unknown Bugs**: They may also contain bugs that haven't been discovered yet.
3. **Not for Production**: Nightly toolchains are not recommended for production use.

### Mitigation Strategy

To mitigate these risks:

1. **Pin to Specific Date**: We've pinned to a specific date (nightly-2025-05-08) rather than using the latest nightly.
2. **Comprehensive Testing**: We maintain a robust test suite to catch any issues.
3. **Migration Plan**: We have a plan to migrate to stable Rust once the features we need are stabilized.

## Sources

For more information on the topics discussed in this guide, refer to these sources:

1. [Rust Release Process](https://forge.rust-lang.org/release/process.html) - Official documentation on Rust's release process
2. [Dropck Eyepatch RFC](https://rust-lang.github.io/rfcs/1327-dropck-param-eyepatch.html) - The RFC that introduced the `may_dangle` attribute
3. [Rust Unstable Book: dropck_eyepatch](https://doc.rust-lang.org/unstable-book/language-features/dropck-eyepatch.html) - Documentation on the `dropck_eyepatch` feature
4. [SmallVec Documentation](https://docs.rs/smallvec/latest/smallvec/) - Documentation for the SmallVec crate, including its use of `may_dangle`
5. [Rust Blog: Nightly Rust](https://blog.rust-lang.org/inside-rust/2020/07/23/upcoming-changes-in-the-nightly-experience.html) - Information about using nightly Rust
6. [Rust Edition Guide](https://doc.rust-lang.org/edition-guide/) - Information about Rust editions and feature stabilization

---

*This guide was last updated: July 22, 2025*

---

## Summary of Key Points

- **Required Toolchain**: nightly-2025-05-08
- **Critical Feature**: `#[may_dangle]` attribute (behind the `dropck_eyepatch` feature gate)
- **Primary Use Cases**:
  1. Implementing safe self-referential types in our own code
  2. **Using external libraries like smallvec with our parser**
- **Installation**: Use `rustup toolchain install nightly-2025-05-08` or create a `rust-toolchain.toml` file
- **Code Activation**: Add `#![feature(dropck_eyepatch)]` to your crate root
- **Remember**: This is an experimental feature with associated risks