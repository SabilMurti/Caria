use std::collections::HashMap;
use std::fmt;
use caria_memory::CariaArc;

#[derive(Clone, Debug, PartialEq)]
pub enum CariaValue {
    Nil,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(CariaArc<String>),
    Array(CariaArc<Vec<CariaValue>>),
    Map(CariaArc<HashMap<String, CariaValue>>),
    Void,
}

impl CariaValue {
    pub fn new_string(s: impl Into<String>) -> Self {
        CariaValue::String(CariaArc::new(s.into()))
    }

    pub fn new_array(items: Vec<CariaValue>) -> Self {
        CariaValue::Array(CariaArc::new(items))
    }

    pub fn new_map(map: HashMap<String, CariaValue>) -> Self {
        CariaValue::Map(CariaArc::new(map))
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            CariaValue::Nil | CariaValue::Void => false,
            CariaValue::Bool(b) => *b,
            CariaValue::Int(n) => *n != 0,
            CariaValue::Float(f) => *f != 0.0 && !f.is_nan(),
            CariaValue::String(s) => !s.as_ref().is_empty(),
            CariaValue::Array(a) => !a.as_ref().is_empty(),
            CariaValue::Map(m) => !m.as_ref().is_empty(),
        }
    }
}

impl fmt::Display for CariaValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CariaValue::Nil => write!(f, "nil"),
            CariaValue::Void => write!(f, "void"),
            CariaValue::Int(n) => write!(f, "{}", n),
            CariaValue::Float(fl) => write!(f, "{}", fl),
            CariaValue::Bool(b) => write!(f, "{}", b),
            CariaValue::String(s) => write!(f, "{}", s.as_ref()),
            CariaValue::Array(arr) => {
                let items: Vec<String> = arr.as_ref().iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", items.join(", "))
            }
            CariaValue::Map(map) => {
                let entries: Vec<String> = map.as_ref()
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{{{}}}", entries.join(", "))
            }
        }
    }
}
