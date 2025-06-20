# Rholang Tree-Sitter Proc Macros

This crate provides procedural macros for compile-time access to rholang-tree-sitter node kinds, keywords, and fields. These macros help you work with tree-sitter parse trees more efficiently by providing compile-time validation of node kinds, keywords, and field names.

## Macros

This crate provides three main macros:

- `kind!`: Returns the node kind ID for a given node kind name
- `kw!`: Returns the node kind ID for a given keyword
- `field!`: Returns the field ID for a given field name

## Usage

These macros are particularly useful when working with tree-sitter parse trees in pattern matching contexts or when traversing the tree to find specific nodes or fields.

```rust
use rholang_tree_sitter_proc_macro::{kind, kw, field};
use tree_sitter::TreeCursor;

fn process_node(cursor: &TreeCursor, code: &str) {
    let node = cursor.node();

    // Match on node kind
    match node.kind_id() {
        kind!("process") => {
            // Process a Rholang process
            let name_field = field!("name");
            if let Some(name) = node.child_by_field_id(name_field.get()) {
                // Process name
            }
        },
        kind!("send") => {
            // Process a send operation
            let channel_field = field!("channel");
            if let Some(channel) = node.child_by_field_id(channel_field.get()) {
                // Process channel
            }
        },
        _ => {
            // Handle other node kinds
        }
    }
}
```

## Error Handling

All three macros perform compile-time validation of their inputs. If you provide an invalid node kind, keyword, or field name, you'll get a compile-time error with a helpful message.

## Performance

Since these macros resolve to constants at compile time, there is no runtime overhead compared to using hardcoded IDs. This makes them both safe and efficient.

## Examples

The crate includes two examples:

1. `parse_rholang.rs`: A simple example that demonstrates how to use the macros to parse and analyze Rholang code.
2. `advanced_usage.rs`: A more complex example that shows how to use the macros for more advanced analysis of Rholang code.

To run the examples:

```bash
# Run the parse_rholang example
cargo run --example parse_rholang --features proc_macros

# Run the advanced_usage example
cargo run --example advanced_usage --features proc_macros
```

## Known Issues

### Doctest Memory Allocation

When running doctests for this crate, you might encounter memory allocation errors:

```
error: could not exec the linker `cc`
  |
  = note: Cannot allocate memory (os error 12)
```

This is due to the memory-intensive nature of compiling the tree-sitter grammar and running the doctests. If you encounter this issue, you can try the following workarounds:

1. Increase the available memory for the compilation process
2. Run the doctests with a smaller number of threads:
   ```bash
   RUST_TEST_THREADS=1 cargo test -p rholang-tree-sitter-proc-macro --doc
   ```
3. Skip the doctests and rely on the examples instead:
   ```bash
   cargo test -p rholang-tree-sitter-proc-macro --examples
   ```

## License

This crate is licensed under the MIT License.
