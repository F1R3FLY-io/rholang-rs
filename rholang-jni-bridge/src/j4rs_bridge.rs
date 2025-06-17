//! JNI bridge for the Rholang parser
//!
//! This module provides functions that can be called from Java using JNI.

use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jstring};
use jni::JNIEnv;
use serde::{Deserialize, Serialize};

use rholang_parser::{errors::ParseResult, RholangParser};

/// Result of parsing Rholang code
#[derive(Serialize, Deserialize)]
pub struct ParserResult {
    /// Whether the code is valid Rholang
    pub valid: bool,
    /// The parse tree as a string (only if valid)
    pub tree: Option<String>,
    /// Error message (only if not valid)
    pub error: Option<String>,
}

/// Check if the given code is valid Rholang
///
/// This function is exposed to Java via JNI.
#[no_mangle]
pub extern "system" fn Java_org_rholang_lang_parser_RholangParserJNI_isValid(
    mut env: JNIEnv,
    _class: JClass,
    code: JString,
) -> jboolean {
    let code: String = match env.get_string(&code) {
        Ok(s) => s.into(),
        Err(_) => return 0, // false
    };

    match RholangParser::new() {
        Ok(mut parser) => {
            if parser.is_valid(&code) {
                1 // true
            } else {
                0 // false
            }
        }
        Err(_) => 0, // false
    }
}

/// Parse the given code and return a result
///
/// This function is exposed to Java via JNI.
/// Returns a JSON string with the parse result.
#[no_mangle]
pub extern "system" fn Java_org_rholang_lang_parser_RholangParserJNI_parse(
    mut env: JNIEnv,
    _class: JClass,
    code: JString,
) -> jstring {
    let code: String = match env.get_string(&code) {
        Ok(s) => s.into(),
        Err(e) => {
            let result = ParserResult {
                valid: false,
                tree: None,
                error: Some(format!("Failed to get string from Java: {:?}", e)),
            };
            return string_to_jstring(&env, &serde_json::to_string(&result).unwrap_or_else(|e| {
                format!("{{\"valid\":false,\"tree\":null,\"error\":\"Failed to serialize error: {}\"}}", e)
            }));
        }
    };

    let result = match RholangParser::new() {
        Ok(mut parser) => match parser.get_tree_string(&code) {
            ParseResult::Success(tree) => ParserResult {
                valid: true,
                tree: Some(tree),
                error: None,
            },
            ParseResult::Error(err) => ParserResult {
                valid: false,
                tree: None,
                error: Some(format!("{}", err)),
            },
        },
        Err(e) => ParserResult {
            valid: false,
            tree: None,
            error: Some(format!("Failed to create parser: {}", e)),
        },
    };

    // Convert the result to a JSON string
    let json = serde_json::to_string(&result).unwrap_or_else(|e| {
        format!(
            "{{\"valid\":false,\"tree\":null,\"error\":\"Failed to serialize result: {}\"}}",
            e
        )
    });

    string_to_jstring(&env, &json)
}

/// Helper function to convert a Rust String to a Java jstring
fn string_to_jstring<'a>(env: &JNIEnv<'a>, string: &str) -> jstring {
    let output = env
        .new_string(string)
        .expect("Couldn't create Java string!");
    output.into_raw()
}
