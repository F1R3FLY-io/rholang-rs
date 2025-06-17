//! Example demonstrating the use of core data types in the rholang-bytecode crate.

use rholang_bytecode::{Constant, Literal, Name, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Rholang Bytecode Core Types Example");
    println!("===================================");

    // Creating and displaying literals
    println!("\nLiterals:");
    let int_lit = Literal::Int(42);
    let bool_lit = Literal::Bool(true);
    let string_lit = Literal::String("Hello, Rholang!".to_string());
    let uri_lit = Literal::Uri("rho:io:stdout".to_string());
    let bytes_lit = Literal::ByteArray(vec![0xDE, 0xAD, 0xBE, 0xEF]);

    println!("  Integer: {}", int_lit);
    println!("  Boolean: {}", bool_lit);
    println!("  String: {}", string_lit);
    println!("  URI: {}", uri_lit);
    println!("  Bytes: {}", bytes_lit);

    // Creating and displaying constants
    println!("\nConstants:");
    let tuple_const = Constant::Tuple(vec![
        Constant::Literal(Literal::Int(1)),
        Constant::Literal(Literal::Int(2)),
        Constant::Literal(Literal::Int(3)),
    ]);
    let list_const = Constant::List(vec![
        Constant::Literal(Literal::String("a".to_string())),
        Constant::Literal(Literal::String("b".to_string())),
        Constant::Literal(Literal::String("c".to_string())),
    ]);
    let map_const = Constant::Map(vec![
        (
            Constant::Literal(Literal::String("name".to_string())),
            Constant::Literal(Literal::String("Alice".to_string())),
        ),
        (
            Constant::Literal(Literal::String("age".to_string())),
            Constant::Literal(Literal::Int(30)),
        ),
    ]);

    println!("  Tuple: {}", tuple_const);
    println!("  List: {}", list_const);
    println!("  Map: {}", map_const);

    // Creating and displaying names
    println!("\nNames:");
    let new_name = Name::new(42);
    let quote_name = Name::quote(vec![0x01, 0x02, 0x03, 0x04]);
    let builtin_name = Name::builtin("stdout");
    let ground_name = Name::ground(vec![0xAA, 0xBB, 0xCC]);

    println!("  New: {}", new_name);
    println!("  Quote: {}", quote_name);
    println!("  Builtin: {}", builtin_name);
    println!("  Ground: {}", ground_name);

    // Creating and displaying values
    println!("\nValues:");
    let int_val = Value::int(42);
    let bool_val = Value::bool(true);
    let string_val = Value::string("Hello, Rholang!");
    let tuple_val = Value::tuple(vec![
        Value::int(1),
        Value::bool(true),
        Value::string("three"),
    ]);
    let list_val = Value::list(vec![Value::int(1), Value::int(2), Value::int(3)]);
    let map_val = Value::map(vec![
        (Value::string("key1"), Value::int(42)),
        (Value::string("key2"), Value::bool(true)),
    ]);
    let name_val = Value::Name(Name::builtin("stdout"));

    println!("  Integer: {}", int_val);
    println!("  Boolean: {}", bool_val);
    println!("  String: {}", string_val);
    println!("  Tuple: {}", tuple_val);
    println!("  List: {}", list_val);
    println!("  Map: {}", map_val);
    println!("  Name: {}", name_val);

    // Converting between constants and values
    println!("\nConverting Constants to Values:");
    let nested_const = Constant::Tuple(vec![
        Constant::Literal(Literal::Int(1)),
        Constant::List(vec![
            Constant::Literal(Literal::String("a".to_string())),
            Constant::Literal(Literal::String("b".to_string())),
        ]),
        Constant::Map(vec![(
            Constant::Literal(Literal::String("key".to_string())),
            Constant::Literal(Literal::Bool(true)),
        )]),
    ]);
    println!("  Original Constant: {}", nested_const);

    let converted_val = Value::from_constant(nested_const);
    println!("  Converted Value: {}", converted_val);

    Ok(())
}
