use std::collections::HashMap;
use crate::value::CariaValue;

pub type NativeFunction = fn(Vec<CariaValue>) -> Result<CariaValue, String>;

pub struct FfiRegistry {
    functions: HashMap<String, NativeFunction>,
}

impl FfiRegistry {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, func: NativeFunction) {
        self.functions.insert(name.to_string(), func);
    }

    pub fn call(&self, name: &str, args: Vec<CariaValue>) -> Result<CariaValue, String> {
        if let Some(func) = self.functions.get(name) {
            func(args)
        } else {
            Err(format!("Native function '{}' not found", name))
        }
    }
}
