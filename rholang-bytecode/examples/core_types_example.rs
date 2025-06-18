//! Example demonstrating the core types in rholang-bytecode
//!
//! This example shows how to use Value, ConstantPool, and Name types

use rholang_bytecode::{
    Value, ConstantPool, ChannelName, UnforgeableName,
    VarRefKind, BundleValue, ContractValue
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Rholang Bytecode Core Types Example");
    println!("===================================\n");

    // Create a constant pool for managing constants
    let mut pool = ConstantPool::new();

    // Add various constants to the pool
    let hello_idx = pool.add_string("Hello, Rholang!");
    let answer_idx = pool.add_integer(42);
    let truth_idx = pool.add_boolean(true);
    let stdout_idx = pool.add_identifier("stdout");

    println!("📊 Constant Pool:");
    println!("  String constant: {} (index: {})",
             pool.get_string(hello_idx)?, hello_idx.get());
    println!("  Integer constant: {} (index: {})",
             pool.get_integer(answer_idx)?, answer_idx.get());
    println!("  Boolean constant: {} (index: {})",
             pool.get_boolean(truth_idx)?, truth_idx.get());
    println!("  Identifier: {} (index: {})",
             pool.get_identifier(stdout_idx)?, stdout_idx.get());
    println!("  Pool size: {} constants\n", pool.len());
    
    println!("🎯 Value Types:");

    // Basic values
    let int_val = Value::Int(100);
    let str_val = Value::String("Rholang String".to_string());
    let bool_val = Value::Bool(false);

    println!("  Integer: {} (truthy: {})", int_val.as_int()?, int_val.is_truthy());
    println!("  String: {:?} (truthy: {})", str_val, str_val.is_truthy());
    println!("  Boolean: {} (truthy: {})", bool_val.as_bool()?, bool_val.is_truthy());

    // Complex values
    let list_val = Value::List(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3)
    ]);

    let map_val = Value::Map(vec![
        ("name".to_string(), Value::String("Alice".to_string())),
        ("age".to_string(), Value::Int(30)),
    ]);

    println!("  List: {:?} (truthy: {})", list_val, list_val.is_truthy());
    println!("  Map: {:?} (truthy: {})", map_val, map_val.is_truthy());

    // Variable reference
    let var_ref = Value::VarRef {
        kind: VarRefKind::Standard,
        name: "myVariable".to_string(),
    };
    println!("  Variable Reference: {:?}", var_ref);

    // Unforgeable name
    let unforgeable = UnforgeableName::generate();
    let name_val = Value::Name(unforgeable.clone());
    println!("  Unforgeable Name: {:?}", name_val);

    // Bundle value
    let bundle_val = Value::Bundle(BundleValue::Read(Box::new(Value::Int(123))));
    println!("  Bundle: {:?}", bundle_val);

    // Contract value
    let contract_val = Value::Contract(ContractValue {
        name: "MyContract".to_string(),
        formals: vec!["input".to_string(), "return".to_string()],
        body: "return!(input * 2)".to_string(),
    });
    println!("  Contract: {:?}\n", contract_val);

    // Channel names
    println!("🔗 Channel Names:");

    let var_channel = ChannelName::from_variable("stdout");
    let unforgeable_channel = ChannelName::from_unforgeable(unforgeable);
    let wildcard_channel = ChannelName::wildcard();

    println!("  Variable channel: {:?} (is_variable: {})",
             var_channel, var_channel.is_variable());
    println!("  Unforgeable channel: {:?} (is_unforgeable: {})",
             unforgeable_channel, unforgeable_channel.is_unforgeable());
    println!("  Wildcard channel: {:?} (is_wildcard: {})",
             wildcard_channel, wildcard_channel.is_wildcard());

    // Type information
    println!("\n📋 Type Information:");
    let values = vec![
        Value::Nil,
        Value::Bool(true),
        Value::Int(42),
        Value::String("test".to_string()),
        Value::Wildcard,
    ];

    for value in values {
        println!("  {} -> type: {}",
                 match &value {
                     Value::Nil => "Nil".to_string(),
                     Value::Bool(b) => format!("Bool({})", b),
                     Value::Int(i) => format!("Int({})", i),
                     Value::String(s) => format!("String(\"{}\")", s),
                     Value::Wildcard => "Wildcard".to_string(),
                     _ => "Other".to_string(),
                 },
                 value.type_name()
        );
    }

    println!("\n✅ Phase 1 Core Types working correctly");
    Ok(())
}
