use std::collections::HashMap;
use std::io::{self, Read};
use tree_sitter::{Node, Parser, TreeCursor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the parser
    let mut parser = Parser::new();

    // Set the language to Rholang
    let language = rholang_tree_sitter::LANGUAGE.into();
    parser.set_language(&language)?;

    // Read input from stdin or use a default example
    let code = if atty::is(atty::Stream::Stdin) {
        // No stdin input, use a default example
        r#"
        new alice, bob, logger(`rho:io:stdout`) in {
            // Alice sends a greeting to Bob
            alice!("Hello, Bob!") |

            // Bob receives messages and logs them
            for (@message <- bob) {
                logger!(["Bob received:", message])
            } |

            // Forward messages from Alice to Bob
            for (@message <- alice) {
                bob!(message)
            }
        }
        "#
        .to_string()
    } else {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    };

    // Parse the code
    let tree = parser.parse(&code, None).unwrap();
    let root_node = tree.root_node();

    // Print the syntax tree
    println!("Syntax tree:");
    println!("{}", root_node.to_sexp());

    // Demonstrate using the proc macros
    #[cfg(feature = "proc_macros")]
    {
        use rholang_tree_sitter_proc_macro::{field, kind};

        println!("\nAdvanced usage of proc macros:");

        // Example 1: Analyze new declarations
        println!("\nExample 1: Analyzing new declarations");
        let new_id = kind!("new");
        let mut cursor = tree.walk();
        let mut new_count = 0;

        fn find_new_declarations(
            node: &Node,
            new_id: u16,
            code: &str,
            cursor: &mut TreeCursor,
            count: &mut i32,
        ) {
            if node.kind_id() == new_id {
                *count += 1;
                println!(
                    "Found new declaration {}: {}",
                    *count,
                    node.utf8_text(code.as_bytes())
                        .unwrap_or("(complex new declaration)")
                );

                // Get the decls field
                let decls_field = field!("decls");
                if let Some(decls) = node.child_by_field_id(decls_field.get()) {
                    println!(
                        "  with declarations: {}",
                        decls
                            .utf8_text(code.as_bytes())
                            .unwrap_or("(complex declarations)")
                    );
                }

                // Get the proc field
                let proc_field = field!("proc");
                if let Some(proc) = node.child_by_field_id(proc_field.get()) {
                    println!(
                        "  with process: {}",
                        proc.utf8_text(code.as_bytes())
                            .unwrap_or("(complex process)")
                    );
                }
            }

            if cursor.goto_first_child() {
                loop {
                    find_new_declarations(&cursor.node(), new_id, code, cursor, count);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }

        find_new_declarations(&root_node, new_id, &code, &mut cursor, &mut new_count);

        // Example 2: Analyze send operations
        println!("\nExample 2: Analyzing send operations");
        let send_id = kind!("send");
        let mut cursor = tree.walk();
        let mut send_count = 0;

        fn analyze_send_operations(
            node: &Node,
            send_id: u16,
            code: &str,
            cursor: &mut TreeCursor,
            count: &mut i32,
        ) {
            if node.kind_id() == send_id {
                *count += 1;
                println!(
                    "Found send operation {}: {}",
                    *count,
                    node.utf8_text(code.as_bytes()).unwrap_or("(complex send)")
                );
            }

            if cursor.goto_first_child() {
                loop {
                    analyze_send_operations(&cursor.node(), send_id, code, cursor, count);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }

        analyze_send_operations(&root_node, send_id, &code, &mut cursor, &mut send_count);

        println!("\nOperation statistics:");
        println!("  - New declarations: {}", new_count);
        println!("  - Send operations: {}", send_count);

        // Example 3: Find all bundle declarations
        println!("\nExample 3: Finding bundle declarations");
        let bundle_id = kind!("bundle");
        let mut cursor = tree.walk();
        let mut bundle_types = HashMap::new();

        fn find_bundle_declarations(
            node: &Node,
            bundle_id: u16,
            code: &str,
            cursor: &mut TreeCursor,
            bundle_types: &mut HashMap<String, i32>,
        ) {
            if node.kind_id() == bundle_id {
                // Get the bundle_type field
                let bundle_type_field = field!("bundle_type");
                if let Some(bundle_type) = node.child_by_field_id(bundle_type_field.get()) {
                    let bundle_type_text = bundle_type
                        .utf8_text(code.as_bytes())
                        .unwrap_or("unknown")
                        .to_string();
                    *bundle_types.entry(bundle_type_text.clone()).or_insert(0) += 1;

                    println!("Found bundle declaration with type: {}", bundle_type_text);

                    // Get the proc field
                    let proc_field = field!("proc");
                    if let Some(proc) = node.child_by_field_id(proc_field.get()) {
                        println!(
                            "  with process: {}",
                            proc.utf8_text(code.as_bytes())
                                .unwrap_or("(complex process)")
                        );
                    }
                }
            }

            if cursor.goto_first_child() {
                loop {
                    find_bundle_declarations(&cursor.node(), bundle_id, code, cursor, bundle_types);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }

        find_bundle_declarations(&root_node, bundle_id, &code, &mut cursor, &mut bundle_types);

        println!("\nBundle type statistics:");
        if bundle_types.is_empty() {
            println!("  No bundle declarations found");
        } else {
            for (bundle_type, count) in bundle_types.iter() {
                println!("  - '{}': {} occurrence(s)", bundle_type, count);
            }
        }
    }

    Ok(())
}
