use crate::Process;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Name(String),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Par(Vec<Process>),
    Nil,
}

impl Value {
    #[allow(dead_code)]
    pub fn as_int(&self) -> Option<i64> {
        if let Value::Int(n) = self {
            Some(*n)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_as_int() {
        assert_eq!(Value::Int(42).as_int(), Some(42));
        assert_eq!(Value::Bool(true).as_int(), None);
        assert_eq!(Value::Str("test".to_string()).as_int(), None);
        assert_eq!(Value::Nil.as_int(), None);
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Int(1), Value::Int(1));
        assert_ne!(Value::Int(1), Value::Int(2));
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
        assert_eq!(Value::Str("a".into()), Value::Str("a".into()));
        assert_eq!(Value::Name("x".into()), Value::Name("x".into()));
        assert_eq!(Value::Nil, Value::Nil);
        assert_eq!(
            Value::List(vec![Value::Int(1)]),
            Value::List(vec![Value::Int(1)])
        );
        assert_eq!(
            Value::Tuple(vec![Value::Int(1)]),
            Value::Tuple(vec![Value::Int(1)])
        );
        assert_eq!(
            Value::Map(vec![(Value::Int(1), Value::Int(2))]),
            Value::Map(vec![(Value::Int(1), Value::Int(2))])
        );
    }

    #[test]
    fn test_value_clone() {
        let val = Value::List(vec![Value::Int(1), Value::Str("s".into())]);
        assert_eq!(val.clone(), val);
    }

    #[test]
    fn test_value_debug() {
        let val = Value::Int(1);
        assert!(!format!("{:?}", val).is_empty());
    }
}
