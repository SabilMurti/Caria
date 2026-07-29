use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    Int,
    Float,
    String,
    Bool,
    Void,
    Object(String),
    Unknown,
}

pub struct Scope {
    symbols: HashMap<String, TypeInfo>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            parent: None,
        }
    }

    pub fn new_child(parent: Scope) -> Self {
        Self {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn insert(&mut self, name: String, ty: TypeInfo) {
        self.symbols.insert(name, ty);
    }

    pub fn lookup(&self, name: &str) -> Option<TypeInfo> {
        if let Some(ty) = self.symbols.get(name) {
            Some(ty.clone())
        } else if let Some(parent) = &self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }
}
