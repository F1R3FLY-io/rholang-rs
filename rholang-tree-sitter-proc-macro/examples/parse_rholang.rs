use anyhow::Result;
use std::io::{self, Read};
use tree_sitter::Parser;

fn main() -> Result<()> {
    // Initialize the parser
    let mut parser = Parser::new();

    // Set the language to Rholang
    let language = rholang_tree_sitter::LANGUAGE.into();
    parser.set_language(&language)?;

    // Read input from stdin or use a default example
    let code = if is_terminal::is_terminal(std::io::stdin()) {
        // No stdin input, use a default example
        r#"
        new stdout(`rho:io:stdout`) in {
            stdout!("Hello, world!")
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
    print_node(&root_node, &code, 0);

    // Demonstrate using the proc macros
    #[cfg(feature = "proc_macros")]
    {
        use rholang_tree_sitter_proc_macro::{field, kind};

        println!("\nDemonstrating proc macros:");

        // Example 1: Find all new declarations using kind! macro
        println!("\nExample 1: Finding new declarations using kind! macro");
        let new_id = kind!("new");
        let mut cursor = tree.walk();

        fn visit_new_nodes(
            node: &tree_sitter::Node,
            new_id: u16,
            code: &str,
            cursor: &mut tree_sitter::TreeCursor,
        ) {
            if node.kind_id() == new_id {
                println!(
                    "Found new declaration: {}",
                    node.utf8_text(code.as_bytes())
                        .unwrap_or("(complex new declaration)")
                );
            }

            if cursor.goto_first_child() {
                loop {
                    visit_new_nodes(&cursor.node(), new_id, code, cursor);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }

        visit_new_nodes(&root_node, new_id, &code, &mut cursor);

        // Example 2: Find all send operations using kind! macro
        println!("\nExample 2: Finding send operations using kind! macro");
        let send_id = kind!("send");
        let mut cursor = tree.walk();

        fn visit_send_nodes(
            node: &tree_sitter::Node,
            send_id: u16,
            code: &str,
            cursor: &mut tree_sitter::TreeCursor,
        ) {
            if node.kind_id() == send_id {
                println!(
                    "Found send operation: {}",
                    node.utf8_text(code.as_bytes()).unwrap_or("(complex send)")
                );
            }

            if cursor.goto_first_child() {
                loop {
                    visit_send_nodes(&cursor.node(), send_id, code, cursor);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }

        visit_send_nodes(&root_node, send_id, &code, &mut cursor);

        // Example 3: Find all bundle declarations using kind! macro
        println!("\nExample 3: Finding bundle declarations using kind! macro");
        let bundle_id = kind!("bundle");
        let mut cursor = tree.walk();

        fn visit_bundle_nodes(
            node: &tree_sitter::Node,
            bundle_id: u16,
            code: &str,
            cursor: &mut tree_sitter::TreeCursor,
        ) {
            if node.kind_id() == bundle_id {
                // Get the bundle_type field using field! macro
                let bundle_type_field = field!("bundle_type");
                if let Some(bundle_type) = node.child_by_field_id(bundle_type_field) {
                    println!(
                        "Found bundle declaration with type: {}",
                        bundle_type
                            .utf8_text(code.as_bytes())
                            .unwrap_or("(complex bundle type)")
                    );
                }

                // Get the proc field using field! macro
                let proc_field = field!("proc");
                if let Some(proc) = node.child_by_field_id(proc_field) {
                    println!(
                        "Found bundle declaration with process: {}",
                        proc.utf8_text(code.as_bytes())
                            .unwrap_or("(complex process)")
                    );
                }
            }

            if cursor.goto_first_child() {
                loop {
                    visit_bundle_nodes(&cursor.node(), bundle_id, code, cursor);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }

        visit_bundle_nodes(&root_node, bundle_id, &code, &mut cursor);
    }

    Ok(())
}

fn print_node(node: &tree_sitter::Node, source: &str, indent: usize) {
    let indent_str = " ".repeat(indent);
    let node_text = if node.child_count() == 0 {
        format!(" \"{}\"", node.utf8_text(source.as_bytes()).unwrap())
    } else {
        String::new()
    };

    println!(
        "{}{} ({}){}",
        indent_str,
        node.kind(),
        node.start_position().row,
        node_text
    );

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            print_node(&cursor.node(), source, indent + 2);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}
