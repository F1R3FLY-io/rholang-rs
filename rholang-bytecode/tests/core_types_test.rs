use rholang_bytecode::{Constant, Literal, Name, Value};
use rstest::rstest;

#[test]
fn test_literal_display() {
    let int_lit = Literal::Int(42);
    let bool_lit = Literal::Bool(true);
    let string_lit = Literal::String("hello".to_string());
    let uri_lit = Literal::Uri("rho:io:stdout".to_string());
    let bytes_lit = Literal::ByteArray(vec![0x01, 0x02, 0x03]);

    assert_eq!(format!("{}", int_lit), "42");
    assert_eq!(format!("{}", bool_lit), "true");
    assert_eq!(format!("{}", string_lit), "\"hello\"");
    assert_eq!(format!("{}", uri_lit), "`rho:io:stdout`");
    assert_eq!(format!("{}", bytes_lit), "0x010203");
}

#[test]
fn test_constant_display() {
    let lit_const = Constant::Literal(Literal::Int(42));
    let tuple_const = Constant::Tuple(vec![
        Constant::Literal(Literal::Int(1)),
        Constant::Literal(Literal::Int(2)),
    ]);
    let list_const = Constant::List(vec![
        Constant::Literal(Literal::String("a".to_string())),
        Constant::Literal(Literal::String("b".to_string())),
    ]);
    let map_const = Constant::Map(vec![(
        Constant::Literal(Literal::String("key".to_string())),
        Constant::Literal(Literal::Int(42)),
    )]);

    assert_eq!(format!("{}", lit_const), "42");
    assert_eq!(format!("{}", tuple_const), "(1, 2)");
    assert_eq!(format!("{}", list_const), "[\"a\", \"b\"]");
    assert_eq!(format!("{}", map_const), "{\"key\": 42}");
}

#[test]
fn test_name_display() {
    let new_name = Name::new(42);
    let quote_name = Name::quote(vec![0x01, 0x02, 0x03]);
    let builtin_name = Name::builtin("stdout");
    let ground_name = Name::ground(vec![0x04, 0x05, 0x06]);

    assert_eq!(format!("{}", new_name), "@42");
    assert_eq!(format!("{}", quote_name), "@quote(010203)");
    assert_eq!(format!("{}", builtin_name), "@stdout");
    assert_eq!(format!("{}", ground_name), "@ground(040506)");
}

#[test]
fn test_value_display() {
    let int_val = Value::int(42);
    let bool_val = Value::bool(true);
    let string_val = Value::string("hello");
    let tuple_val = Value::tuple(vec![Value::int(1), Value::int(2)]);
    let list_val = Value::list(vec![Value::string("a"), Value::string("b")]);
    let map_val = Value::map(vec![(Value::string("key"), Value::int(42))]);
    let name_val = Value::Name(Name::new(42));

    assert_eq!(format!("{}", int_val), "42");
    assert_eq!(format!("{}", bool_val), "true");
    assert_eq!(format!("{}", string_val), "\"hello\"");
    assert_eq!(format!("{}", tuple_val), "(1, 2)");
    assert_eq!(format!("{}", list_val), "[\"a\", \"b\"]");
    assert_eq!(format!("{}", map_val), "{\"key\": 42}");
    assert_eq!(format!("{}", name_val), "@42");
}

#[rstest]
#[case(42, "42")]
#[case(0, "0")]
#[case(-1, "-1")]
fn test_int_value_display(#[case] value: i64, #[case] expected: &str) {
    let val = Value::int(value);
    assert_eq!(format!("{}", val), expected);
}

#[rstest]
#[case(true, "true")]
#[case(false, "false")]
fn test_bool_value_display(#[case] value: bool, #[case] expected: &str) {
    let val = Value::bool(value);
    assert_eq!(format!("{}", val), expected);
}

#[test]
fn test_value_from_constant() {
    let constant = Constant::Tuple(vec![
        Constant::Literal(Literal::Int(1)),
        Constant::Literal(Literal::Bool(true)),
        Constant::List(vec![Constant::Literal(Literal::String(
            "hello".to_string(),
        ))]),
    ]);

    let value = Value::from_constant(constant);

    match value {
        Value::Tuple(elements) => {
            assert_eq!(elements.len(), 3);
            match &elements[0] {
                Value::Literal(Literal::Int(i)) => assert_eq!(*i, 1),
                _ => panic!("Expected Int literal"),
            }
            match &elements[1] {
                Value::Literal(Literal::Bool(b)) => assert!(*b),
                _ => panic!("Expected Bool literal"),
            }
            match &elements[2] {
                Value::List(items) => {
                    assert_eq!(items.len(), 1);
                    match &items[0] {
                        Value::Literal(Literal::String(s)) => assert_eq!(s, "hello"),
                        _ => panic!("Expected String literal"),
                    }
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Tuple"),
    }
}

#[test]
fn test_name_type_checks() {
    let new_name = Name::new(42);
    let quote_name = Name::quote(vec![0x01, 0x02, 0x03]);
    let builtin_name = Name::builtin("stdout");
    let ground_name = Name::ground(vec![0x04, 0x05, 0x06]);

    assert!(new_name.is_new());
    assert!(!new_name.is_quote());
    assert!(!new_name.is_builtin());
    assert!(!new_name.is_ground());

    assert!(!quote_name.is_new());
    assert!(quote_name.is_quote());
    assert!(!quote_name.is_builtin());
    assert!(!quote_name.is_ground());

    assert!(!builtin_name.is_new());
    assert!(!builtin_name.is_quote());
    assert!(builtin_name.is_builtin());
    assert!(!builtin_name.is_ground());

    assert!(!ground_name.is_new());
    assert!(!ground_name.is_quote());
    assert!(!ground_name.is_builtin());
    assert!(ground_name.is_ground());
}
