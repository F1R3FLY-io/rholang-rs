// Tests for the rholang-tree-sitter-proc-macro macros
use rholang_tree_sitter_proc_macro::{field, kind, kw};

#[test]
fn test_kind_macro() {
    // Test with valid node kinds
    let new_id = kind!("new");
    assert!(new_id > 0, "Expected non-zero ID for new");

    let send_id = kind!("send");
    assert!(send_id > 0, "Expected non-zero ID for send");

    let bundle_id = kind!("bundle");
    assert!(bundle_id > 0, "Expected non-zero ID for bundle");

    // Test more node kinds to ensure comprehensive coverage
    let branch_id = kind!("branch");
    assert!(branch_id > 0, "Expected non-zero ID for branch");

    let case_id = kind!("case");
    assert!(case_id > 0, "Expected non-zero ID for case");

    // Verify that different node kinds have different IDs
    assert_ne!(
        new_id, send_id,
        "Different node kinds should have different IDs"
    );
    assert_ne!(
        new_id, bundle_id,
        "Different node kinds should have different IDs"
    );
    assert_ne!(
        send_id, bundle_id,
        "Different node kinds should have different IDs"
    );

    // Test that all node kinds have unique IDs
    let node_kinds = [new_id, send_id, bundle_id, branch_id, case_id];

    for i in 0..node_kinds.len() {
        for j in i + 1..node_kinds.len() {
            assert_ne!(
                node_kinds[i], node_kinds[j],
                "Node kinds should have unique IDs: {} and {}",
                i, j
            );
        }
    }

    // Test using the macro in a const context
    const NEW_ID: u16 = kind!("new");
    assert_eq!(
        NEW_ID, new_id,
        "Macro should work the same in const and non-const contexts"
    );

    // Note: Testing with invalid node kinds would cause compile errors, so we can't test that here
    // Example of what would cause a compile error:
    // kind!("not_a_valid_node_kind") // This would fail at compile time
}

#[test]
fn test_kw_macro() {
    // Test with valid keywords
    let new_id = kw!("new");
    assert!(new_id > 0, "Expected non-zero ID for 'new' keyword");

    let for_id = kw!("for");
    assert!(for_id > 0, "Expected non-zero ID for 'for' keyword");

    // Test more keywords to ensure comprehensive coverage
    let in_id = kw!("in");
    assert!(in_id > 0, "Expected non-zero ID for 'in' keyword");

    // Verify that different keywords have different IDs
    assert_ne!(
        new_id, for_id,
        "Different keywords should have different IDs"
    );
    assert_ne!(
        new_id, in_id,
        "Different keywords should have different IDs"
    );
    assert_ne!(
        for_id, in_id,
        "Different keywords should have different IDs"
    );

    // Test that all keywords have unique IDs
    let keywords = [new_id, for_id, in_id];

    for i in 0..keywords.len() {
        for j in i + 1..keywords.len() {
            assert_ne!(
                keywords[i], keywords[j],
                "Keywords should have unique IDs: {} and {}",
                i, j
            );
        }
    }

    // Test using the macro in a const context
    const NEW_KEYWORD_ID: u16 = kw!("new");
    assert_eq!(
        NEW_KEYWORD_ID, new_id,
        "Macro should work the same in const and non-const contexts"
    );

    // Test using the macro with the matches! macro (compile-time check)
    fn check_keyword(id: u16) -> bool {
        matches!(id, kw!("new") | kw!("for") | kw!("in"))
    }

    assert!(
        check_keyword(new_id),
        "new_id should match in check_keyword"
    );
    assert!(
        check_keyword(for_id),
        "for_id should match in check_keyword"
    );
    assert!(check_keyword(in_id), "in_id should match in check_keyword");

    // Note: Testing with invalid keywords would cause compile errors, so we can't test that here
    // Example of what would cause a compile error:
    // kw!("not_a_valid_keyword") // This would fail at compile time
}

#[test]
fn test_field_macro() {
    // Test with valid fields
    let pattern_id = field!("pattern");
    assert!(
        pattern_id.get() > 0,
        "Expected non-zero ID for 'pattern' field"
    );

    let proc_id = field!("proc");
    assert!(proc_id.get() > 0, "Expected non-zero ID for 'proc' field");

    let decls_id = field!("decls");
    assert!(decls_id.get() > 0, "Expected non-zero ID for 'decls' field");

    // Test more fields to ensure comprehensive coverage
    let bundle_type_id = field!("bundle_type");
    assert!(
        bundle_type_id.get() > 0,
        "Expected non-zero ID for 'bundle_type' field"
    );

    // Verify that different fields have different IDs
    assert_ne!(
        pattern_id, proc_id,
        "Different fields should have different IDs"
    );
    assert_ne!(
        pattern_id, decls_id,
        "Different fields should have different IDs"
    );
    assert_ne!(
        proc_id, decls_id,
        "Different fields should have different IDs"
    );

    // Test that all fields have unique IDs
    let fields = [pattern_id, proc_id, decls_id, bundle_type_id];

    for i in 0..fields.len() {
        for j in i + 1..fields.len() {
            assert_ne!(
                fields[i], fields[j],
                "Fields should have unique IDs: {} and {}",
                i, j
            );
        }
    }

    // Test using the macro in a const context
    const PATTERN_FIELD: std::num::NonZeroU16 = field!("pattern");
    assert_eq!(
        PATTERN_FIELD, pattern_id,
        "Macro should work the same in const and non-const contexts"
    );

    // Test using the field ID to extract a field from a node
    fn extract_field(field_name: &str) -> std::num::NonZeroU16 {
        match field_name {
            "pattern" => field!("pattern"),
            "proc" => field!("proc"),
            "decls" => field!("decls"),
            "bundle_type" => field!("bundle_type"),
            _ => panic!("Unknown field: {}", field_name),
        }
    }

    assert_eq!(
        extract_field("pattern"),
        pattern_id,
        "extract_field should return the correct field ID for 'pattern'"
    );
    assert_eq!(
        extract_field("proc"),
        proc_id,
        "extract_field should return the correct field ID for 'proc'"
    );
    assert_eq!(
        extract_field("decls"),
        decls_id,
        "extract_field should return the correct field ID for 'decls'"
    );

    // Note: Testing with invalid fields would cause compile errors, so we can't test that here
    // Example of what would cause a compile error:
    // field!("not_a_valid_field") // This would fail at compile time
}

#[test]
fn test_macro_integration() {
    // Test using the macros together in a realistic scenario
    let new_id = kind!("new");
    let decls_id = field!("decls").get();
    let proc_id = field!("proc").get();

    // Verify that the IDs are valid
    assert!(new_id > 0, "Expected non-zero ID for new");
    assert!(decls_id > 0, "Expected non-zero ID for decls field");
    assert!(proc_id > 0, "Expected non-zero ID for proc field");

    // In a real scenario, we would use these IDs to navigate a tree-sitter parse tree
    // For example, to find the declarations and process body of a new expression:
    // node.child_by_field_id(decls_id)
    // node.child_by_field_id(proc_id)
}
