use anyhow::Result;
use rholang_tree_sitter_proc_macro::match_node;
use tree_sitter::{Node, Parser, TreeCursor};

fn main() -> Result<()> {
    // Initialize the parser
    let mut parser = Parser::new();

    // Set the language to Rholang
    let language = rholang_tree_sitter::LANGUAGE.into();
    parser.set_language(&language)?;

    // Example Rholang code with various node types
    let code = r#"
    new alice, bob in {
        alice!("Hello") | bob!("World") |
        for (@msg <- alice) {
            bob!(msg)
        }
    }
    "#;

    // Parse the code
    let tree = parser.parse(code, None).unwrap();
    let root_node = tree.root_node();

    println!("Syntax tree:");
    println!("{}", root_node.to_sexp());

    // Example 1: Using match_node! macro to process nodes
    println!("\nExample 1: Using match_node! macro to process nodes");
    let mut cursor = tree.walk();

    fn process_nodes(node: &Node, code: &str, cursor: &mut TreeCursor) {
        // Using match_node! macro instead of match node.kind() { ... }
        match_node!(node,
            "par" => println!("Found parallel composition: {}", node.utf8_text(code.as_bytes()).unwrap_or("(complex par)")),
            "new" => println!("Found new declaration: {}", node.utf8_text(code.as_bytes()).unwrap_or("(complex new)")),
            "send" => println!("Found send operation: {}", node.utf8_text(code.as_bytes()).unwrap_or("(complex send)")),
            "receive" => println!("Found receive operation: {}", node.utf8_text(code.as_bytes()).unwrap_or("(complex receive)")),
            "_" => println!("Found other node type: {}", node.kind())
        );

        if cursor.goto_first_child() {
            loop {
                process_nodes(&cursor.node(), code, cursor);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    process_nodes(&root_node, code, &mut cursor);

    // Example 2: Using match_node! macro to extract information
    println!("\nExample 2: Using match_node! macro to extract information");
    let mut cursor = tree.walk();

    fn get_node_description(node: &Node) -> String {
        match_node!(node,
            "par" => "parallel composition".to_string(),
            "new" => "new declaration".to_string(),
            "send" => "send operation".to_string(),
            "receive" => "receive operation".to_string(),
            "_" => format!("other node type: {}", node.kind())
        )
    }

    fn extract_node_info(node: &Node, _code: &str, cursor: &mut TreeCursor) {
        let description = get_node_description(node);
        println!(
            "Node at {}:{} is a {}",
            node.start_position().row + 1,
            node.start_position().column + 1,
            description
        );

        if cursor.goto_first_child() {
            loop {
                extract_node_info(&cursor.node(), _code, cursor);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    extract_node_info(&root_node, code, &mut cursor);

    // Example 3: Comparing old style with match_node! style
    println!("\nExample 3: Comparing old style with match_node! style");

    fn old_style_match(node: &Node) -> String {
        // Old style: match on node.kind() string
        match node.kind() {
            "par" => "parallel composition".to_string(),
            "new" => "new declaration".to_string(),
            "send" => "send operation".to_string(),
            _ => format!("other: {}", node.kind()),
        }
    }

    fn match_node_style(node: &Node) -> String {
        // New style: using match_node! macro
        match_node!(node,
            "par" => "parallel composition".to_string(),
            "new" => "new declaration".to_string(),
            "send" => "send operation".to_string(),
            "_" => format!("other: {}", node.kind())
        )
    }

    let mut cursor = tree.walk();
    if cursor.goto_first_child() {
        let node = cursor.node();
        println!("Old style result: {}", old_style_match(&node));
        println!("match_node style result: {}", match_node_style(&node));
    }

    Ok(())
}
