use proc_macro::TokenStream;

use quote::{quote, quote_spanned};
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, punctuated::Punctuated, Expr, LitStr,
    Result, Token,
};
use tree_sitter::Language;

/// # Rholang-tree-sitter-proc-macro
///
/// This crate provides procedural macros for compile-time access to rholang-tree-sitter node kinds,
/// keywords, and fields. These macros help you work with tree-sitter parse trees more efficiently
/// by providing compile-time validation of node kinds, keywords, and field names.
///
/// ## Macros
///
/// This crate provides three main macros:
///
/// - [`kind!`]: Returns the node kind ID for a given node kind name
/// - [`kw!`]: Returns the node kind ID for a given keyword
/// - [`field!`]: Returns the field ID for a given field name
///
/// ## Usage
///
/// These macros are particularly useful when working with tree-sitter parse trees in pattern
/// matching contexts or when traversing the tree to find specific nodes or fields.
///
/// ```
/// use rholang_tree_sitter_proc_macro::{kind, kw, field};
/// use tree_sitter::TreeCursor;
///
/// fn process_node(cursor: &TreeCursor, code: &str) {
///     let node = cursor.node();
///     
///     // Match on node kind
///     match node.kind_id() {
///         kind!("new") => {
///             // Process a new declaration
///             let decls_field = field!("decls");
///             if let Some(decls) = node.child_by_field_id(decls_field.get()) {
///                 // Process declarations
///             }
///         },
///         kind!("send") => {
///             // Process a send operation
///             let proc_field = field!("proc");
///             if let Some(proc) = node.child_by_field_id(proc_field.get()) {
///                 // Process proc
///             }
///         },
///         _ => {
///             // Handle other node kinds
///         }
///     }
/// }
/// ```
///
/// ## Error Handling
///
/// All three macros perform compile-time validation of their inputs. If you provide an invalid
/// node kind, keyword, or field name, you'll get a compile-time error with a helpful message.
///
/// ## Performance
///
/// Since these macros resolve to constants at compile time, there is no runtime overhead
/// compared to using hardcoded IDs. This makes them both safe and efficient.
///
/// Returns the node kind ID for a given node kind name.
///
/// This macro is useful for matching against node kinds in pattern matching contexts.
/// It validates the node kind name at compile time, ensuring that you only use valid
/// node kinds from the rholang-tree-sitter grammar.
///
/// # Arguments
///
/// * `kind_name` - A string literal representing the node kind name.
///
/// # Returns
///
/// The node kind ID as a `u16`.
///
/// # Errors
///
/// Generates a compile-time error if the provided node kind name is not valid
/// in the rholang-tree-sitter grammar.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use rholang_tree_sitter_proc_macro::kind;
///
/// let new_id = kind!("new");
/// ```
///
/// Using in a match statement:
///
/// ```
/// use rholang_tree_sitter_proc_macro::kind;
/// use tree_sitter::Node;
///
/// fn process_node(node: &Node) {
///     match node.kind_id() {
///         kind!("new") => println!("Found a new declaration"),
///         kind!("send") => println!("Found a send operation"),
///         kind!("bundle") => println!("Found a bundle declaration"),
///         _ => println!("Found something else"),
///     }
/// }
/// ```
///
/// Using with variables:
///
/// ```
/// use rholang_tree_sitter_proc_macro::kind;
///
/// // These are evaluated at compile time
/// const NEW_ID: u16 = kind!("new");
/// const SEND_ID: u16 = kind!("send");
///
/// // Later in your code
/// fn is_new_declaration(node_kind_id: u16) -> bool {
///     node_kind_id == NEW_ID
/// }
/// ```
#[proc_macro]
pub fn kind(token_stream: TokenStream) -> TokenStream {
    let string_literal: LitStr = parse_macro_input!(token_stream);

    // Get the string value
    let requested_kind = string_literal.value();

    let language: Language = rholang_tree_sitter::LANGUAGE.into();
    let found_id = language.id_for_node_kind(&requested_kind, true);

    if found_id != 0 {
        quote! {
            #found_id
        }
    } else {
        quote_spanned!(
            string_literal.span() =>
            compile_error!("This is not a valid node kind in the rholang-tree-sitter grammar")
        )
    }
    .into()
}

/// Returns the node kind ID for a given keyword.
///
/// This macro is similar to `kind!` but specifically for keywords. It validates
/// the keyword at compile time, ensuring that you only use valid keywords from
/// the rholang-tree-sitter grammar.
///
/// The difference between `kw!` and `kind!` is that `kw!` looks for keywords
/// (which are terminal nodes in the grammar), while `kind!` looks for node kinds
/// (which can be terminal or non-terminal nodes).
///
/// # Arguments
///
/// * `keyword` - A string literal representing the keyword.
///
/// # Returns
///
/// The keyword ID as a `u16`.
///
/// # Errors
///
/// Generates a compile-time error if the provided keyword is not valid
/// in the rholang-tree-sitter grammar.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use rholang_tree_sitter_proc_macro::kw;
///
/// let for_keyword_id = kw!("for");
/// ```
///
/// Using in a match statement:
///
/// ```
/// use rholang_tree_sitter_proc_macro::kw;
/// use tree_sitter::Node;
///
/// fn process_keyword(node: &Node) {
///     match node.kind_id() {
///         kw!("for") => println!("Found a 'for' keyword"),
///         kw!("new") => println!("Found a 'new' keyword"),
///         kw!("in") => println!("Found an 'in' keyword"),
///         _ => println!("Found something else"),
///     }
/// }
/// ```
///
/// Using with variables:
///
/// ```
/// use rholang_tree_sitter_proc_macro::kw;
///
/// // These are evaluated at compile time
/// const FOR_KEYWORD_ID: u16 = kw!("for");
/// const NEW_KEYWORD_ID: u16 = kw!("new");
/// const IN_KEYWORD_ID: u16 = kw!("in");
///
/// // Later in your code
/// fn is_binding_keyword(keyword_id: u16) -> bool {
///     keyword_id == FOR_KEYWORD_ID || keyword_id == NEW_KEYWORD_ID
/// }
/// ```
#[proc_macro]
pub fn kw(token_stream: TokenStream) -> TokenStream {
    let string_literal: LitStr = parse_macro_input!(token_stream);

    // Get the string value
    let requested_keyword = string_literal.value();

    let language: Language = rholang_tree_sitter::LANGUAGE.into();
    let found_id = language.id_for_node_kind(&requested_keyword, false);

    if found_id != 0 {
        quote! {
            #found_id
        }
    } else {
        quote_spanned!(
            string_literal.span() =>
            compile_error!("This is not a valid keyword in the rholang-tree-sitter grammar")
        )
    }
    .into()
}

/// Returns the field ID for a given field name.
///
/// This macro is useful for checking if a node has a specific field and for
/// navigating the parse tree by field name. It validates the field name at
/// compile time, ensuring that you only use valid fields from the rholang-tree-sitter grammar.
///
/// Fields in tree-sitter represent named children of a node. For example, a
/// `send` node might have fields like `channel` and `arguments`.
///
/// # Arguments
///
/// * `field_name` - A string literal representing the field name.
///
/// # Returns
///
/// The field ID as a `std::num::NonZeroU16`. This is the type expected by
/// tree-sitter's `child_by_field_id` method.
///
/// # Errors
///
/// Generates a compile-time error if the provided field name is not valid
/// in the rholang-tree-sitter grammar.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use rholang_tree_sitter_proc_macro::field;
///
/// let decls_field_id = field!("decls");
/// ```
///
/// Using to navigate a parse tree:
///
/// ```
/// use rholang_tree_sitter_proc_macro::{field, kind};
/// use tree_sitter::Node;
///
/// fn get_new_declarations(new_node: &Node, source_code: &str) -> Option<String> {
///     // Check if this is a new declaration
///     if new_node.kind_id() != kind!("new") {
///         return None;
///     }
///
///     // Get the decls field
///     let decls_field = field!("decls");
///     let decls = new_node.child_by_field_id(decls_field.get())?;
///
///     // Extract the declarations text
///     decls.utf8_text(source_code.as_bytes())
///         .ok()
///         .map(|s| s.to_string())
/// }
/// ```
///
/// Using with variables:
///
/// ```
/// use rholang_tree_sitter_proc_macro::field;
/// use std::num::NonZeroU16;
///
/// // These are evaluated at compile time
/// const DECLS_FIELD: NonZeroU16 = field!("decls");
/// const PROC_FIELD: NonZeroU16 = field!("proc");
/// const BUNDLE_TYPE_FIELD: NonZeroU16 = field!("bundle_type");
///
/// // Later in your code
/// fn extract_field_by_name<'a>(node: &'a tree_sitter::Node<'a>, field_name: &str) -> Option<tree_sitter::Node<'a>> {
///     let field_id = match field_name {
///         "decls" => DECLS_FIELD,
///         "proc" => PROC_FIELD,
///         "bundle_type" => BUNDLE_TYPE_FIELD,
///         _ => return None,
///     };
///
///     node.child_by_field_id(field_id.get())
/// }
/// ```
#[proc_macro]
pub fn field(token_stream: TokenStream) -> TokenStream {
    let string_literal: LitStr = parse_macro_input!(token_stream);

    // Get the string value
    let requested_field = string_literal.value();

    let language: Language = rholang_tree_sitter::LANGUAGE.into();
    let found_id = language.field_id_for_name(&requested_field);

    if let Some(found_id) = found_id {
        let id_number: u16 = found_id.get();
        quote! {
            std::num::NonZeroU16::new(#id_number).unwrap()
        }
    } else {
        quote_spanned!(
            string_literal.span() =>
            compile_error!("This is not a valid field in the rholang-tree-sitter grammar")
        )
    }
    .into()
}

/// A structure to represent a pattern-handler pair in the match_node macro
struct MatchNodeArm {
    pattern: LitStr,
    handler: Expr,
}

impl Parse for MatchNodeArm {
    fn parse(input: ParseStream) -> Result<Self> {
        let pattern = input.parse()?;
        input.parse::<Token![=>]>()?;
        let handler = input.parse()?;
        Ok(MatchNodeArm { pattern, handler })
    }
}

/// A structure to represent the input to the match_node macro
struct MatchNodeInput {
    node_expr: Expr,
    arms: Punctuated<MatchNodeArm, Token![,]>,
}

impl Parse for MatchNodeInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let node_expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let arms = Punctuated::parse_terminated(input)?;
        Ok(MatchNodeInput { node_expr, arms })
    }
}

/// Matches a node's kind string against a series of patterns.
///
/// This macro is useful for pattern matching on node kinds using string literals.
/// It generates code that compares the node's kind string with each pattern and
/// executes the corresponding handler if there's a match.
///
/// # Arguments
///
/// * `node_expr` - An expression that evaluates to a tree-sitter Node.
/// * `pattern => handler` - A series of pattern-handler pairs, where each pattern is a string literal
///   and each handler is an expression to be executed if the node's kind matches the pattern.
///
/// # Returns
///
/// The result of the handler expression for the first matching pattern, or the result of the
/// default handler if no pattern matches.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use rholang_tree_sitter_proc_macro::match_node;
/// use tree_sitter::Node;
///
/// fn process_node(node: &Node) {
///     match_node!(node,
///         "par" => println!("Found a parallel composition"),
///         "new" => println!("Found a new declaration"),
///         "send" => println!("Found a send operation"),
///         _ => println!("Found something else: {}", node.kind())
///     );
/// }
/// ```
///
/// Using with variables:
///
/// ```
/// use rholang_tree_sitter_proc_macro::match_node;
/// use tree_sitter::Node;
///
/// fn get_node_description(node: &Node) -> String {
///     match_node!(node,
///         "par" => "parallel composition".to_string(),
///         "new" => "new declaration".to_string(),
///         "send" => "send operation".to_string(),
///         _ => format!("other node type: {}", node.kind())
///     )
/// }
/// ```
#[proc_macro]
pub fn match_node(token_stream: TokenStream) -> TokenStream {
    let input = parse_macro_input!(token_stream as MatchNodeInput);

    let node_expr = &input.node_expr;
    let mut match_arms = Vec::new();
    let mut has_default_arm = false;

    for arm in input.arms.iter() {
        let pattern = &arm.pattern;
        let handler = &arm.handler;

        if pattern.value() == "_" {
            has_default_arm = true;
            match_arms.push(quote! {
                _ => #handler
            });
        } else {
            match_arms.push(quote! {
                kind if kind == #pattern => #handler
            });
        }
    }

    if !has_default_arm {
        match_arms.push(quote! {
            _ => panic!("Unhandled node kind: {}", kind)
        });
    }

    let expanded = quote! {
        {
            let node = #node_expr;
            let kind = node.kind();
            match kind {
                #(#match_arms),*
            }
        }
    };

    expanded.into()
}
