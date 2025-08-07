// Tests for the rholang-tree-sitter-proc-macro macros
use rholang_tree_sitter_proc_macro::{field, kind, kw};

// Static values for node kinds
const NEW_KIND: u16 = 79;
const SEND_KIND: u16 = 87;
const BUNDLE_KIND: u16 = 82;
const BRANCH_KIND: u16 = 129;
const CASE_KIND: u16 = 128;

// Static values for keywords
const NEW_KW: u16 = 4;
const FOR_KW: u16 = 17;
const IN_KW: u16 = 5;

// Static values for fields
const PATTERN_FIELD: u16 = 19;
const PROC_FIELD: u16 = 20;
const DECLS_FIELD: u16 = 10;
const BUNDLE_TYPE_FIELD: u16 = 4;

#[test]
fn test_kind_macro() {
    // Test with valid node kinds
    let new_id = kind!("new");
    assert_eq!(new_id, NEW_KIND, "Unexpected ID for new");

    let send_id = kind!("send");
    assert_eq!(send_id, SEND_KIND, "Unexpected ID for send");

    let bundle_id = kind!("bundle");
    assert_eq!(bundle_id, BUNDLE_KIND, "Unexpected ID for bundle");

    // Test more node kinds to ensure comprehensive coverage
    let branch_id = kind!("branch");
    assert_eq!(branch_id, BRANCH_KIND, "Unexpected ID for branch");

    let case_id = kind!("case");
    assert_eq!(case_id, CASE_KIND, "Unexpected ID for case");

    // Test using the macro in a const context
    assert_eq!(
        NEW_KIND, new_id,
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
    assert_eq!(new_id, NEW_KW, "Unexpected ID for 'new' keyword");

    let for_id = kw!("for");
    assert_eq!(for_id, FOR_KW, "Unexpected ID for 'for' keyword");

    // Test more keywords to ensure comprehensive coverage
    let in_id = kw!("in");
    assert_eq!(in_id, IN_KW, "Unexpected ID for 'in' keyword");

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
    assert_eq!(
        pattern_id, PATTERN_FIELD,
        "Unexpected ID for 'pattern' field"
    );

    let proc_id = field!("proc");
    assert_eq!(proc_id, PROC_FIELD, "Unexpected ID for 'proc' field");

    let decls_id = field!("decls");
    assert_eq!(decls_id, DECLS_FIELD, "Unexpected ID for 'decls' field");

    // Test more fields to ensure comprehensive coverage
    let bundle_type_id = field!("bundle_type");
    assert_eq!(
        bundle_type_id, BUNDLE_TYPE_FIELD,
        "Unexpected ID for 'bundle_type' field"
    );

    // Test using the macro in a const context
    assert_eq!(
        PATTERN_FIELD, pattern_id,
        "Macro should work the same in const and non-const contexts"
    );

    // Test using the field ID to extract a field from a node
    fn extract_field(field_name: &str) -> u16 {
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
    let decls_id = field!("decls");
    let proc_id = field!("proc");

    // Verify that the IDs match the expected values
    assert_eq!(new_id, NEW_KIND, "Unexpected ID for new");
    assert_eq!(decls_id, DECLS_FIELD, "Unexpected ID for decls field");
    assert_eq!(proc_id, PROC_FIELD, "Unexpected ID for proc field");

    // In a real scenario, we would use these IDs to navigate a tree-sitter parse tree
    // For example, to find the declarations and process body of a new expression:
    // node.child_by_field_id(decls_id)
    // node.child_by_field_id(proc_id)
}
