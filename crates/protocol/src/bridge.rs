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
