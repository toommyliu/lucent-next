use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Object(BTreeMap<String, Value>),
    List(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub name: String,
    pub args: Vec<Value>,
}

pub fn to_json(value: Value) -> Option<serde_json::Value> {
    Some(match value {
        Value::Undefined => return None,
        Value::Null => serde_json::Value::Null,
        Value::Bool(value) => value.into(),
        Value::Number(value) => {
            serde_json::Number::from_f64(value).map_or(serde_json::Value::Null, Into::into)
        }
        Value::String(value) => value.into(),
        Value::List(values) => values
            .into_iter()
            .map(|value| to_json(value).unwrap_or(serde_json::Value::Null))
            .collect::<Vec<_>>()
            .into(),
        Value::Object(values) => serde_json::Value::Object(
            values
                .into_iter()
                .filter_map(|(key, value)| to_json(value).map(|value| (key, value)))
                .collect(),
        ),
    })
}

pub fn from_json(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(value) => Value::Bool(value),
        serde_json::Value::Number(value) => Value::Number(value.as_f64().unwrap()),
        serde_json::Value::String(value) => Value::String(value),
        serde_json::Value::Array(values) => {
            Value::List(values.into_iter().map(from_json).collect())
        }
        serde_json::Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, from_json(value)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hydration_keeps_useful_fields_when_optional_values_are_undefined() {
        let value = Value::Object(
            [
                ("uoName".into(), Value::String("player-a".into())),
                ("optional".into(), Value::Undefined),
                (
                    "nested".into(),
                    Value::List(vec![
                        Value::Number(7.0),
                        Value::Undefined,
                        Value::Number(f64::NAN),
                    ]),
                ),
            ]
            .into(),
        );
        assert_eq!(
            to_json(value),
            Some(json!({"uoName":"player-a", "nested":[7.0,null,null]}))
        );
    }

    #[test]
    fn undefined_hydration_is_unavailable_while_null_and_numbers_are_values() {
        assert_eq!(to_json(Value::Undefined), None);
        assert_eq!(to_json(Value::Null), Some(json!(null)));
        assert_eq!(to_json(Value::Number(7.0)), Some(json!(7.0)));
    }
}
