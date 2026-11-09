use std::collections::HashMap;
use caria_parser::ast::TypeExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    Int,
    Float,
    String,
    Bool,
    Nil,
    Void,
    List(Box<TypeInfo>),
    Map(Box<TypeInfo>, Box<TypeInfo>),
    Function {
        params: Vec<TypeInfo>,
        ret: Box<TypeInfo>,
    },
    Range,
    Custom(String),
    Unknown,
}

impl TypeInfo {
    pub fn from_ast_type(expr: &TypeExpr) -> Self {
        match expr {
            TypeExpr::Named(name) => match name.as_str() {
                "int" | "Int" | "i64" => TypeInfo::Int,
                "dec" | "Dec" | "float" | "Float" | "f64" => TypeInfo::Float,
                "txt" | "Txt" | "str" | "Str" | "string" | "String" => TypeInfo::String,
                "flag" | "Flag" | "bool" | "Bool" => TypeInfo::Bool,
                "nil" | "Nil" => TypeInfo::Nil,
                "void" | "Void" => TypeInfo::Void,
                custom => TypeInfo::Custom(custom.to_string()),
            },
            TypeExpr::List(inner) | TypeExpr::NonEmptyArray(inner) => {
                TypeInfo::List(Box::new(Self::from_ast_type(inner)))
            }
            TypeExpr::Map(k, v) => {
                TypeInfo::Map(Box::new(Self::from_ast_type(k)), Box::new(Self::from_ast_type(v)))
            }
            TypeExpr::Function(params, ret) => {
                let p = params.iter().map(Self::from_ast_type).collect();
                TypeInfo::Function {
                    params: p,
                    ret: Box::new(Self::from_ast_type(ret)),
                }
            }
            TypeExpr::Optional(inner) => Self::from_ast_type(inner),
            TypeExpr::Generic(name, _) => TypeInfo::Custom(name.clone()),
        }
    }

    pub fn is_compatible_with(&self, other: &TypeInfo) -> bool {
        if self == other || *self == TypeInfo::Unknown || *other == TypeInfo::Unknown {
            return true;
        }
        match (self, other) {
            (TypeInfo::Int, TypeInfo::Float) | (TypeInfo::Float, TypeInfo::Int) => true,
            (TypeInfo::List(a), TypeInfo::List(b)) => a.is_compatible_with(b),
            (TypeInfo::Map(k1, v1), TypeInfo::Map(k2, v2)) => {
                k1.is_compatible_with(k2) && v1.is_compatible_with(v2)
            }
            (
                TypeInfo::Function { params: p1, ret: r1 },
                TypeInfo::Function { params: p2, ret: r2 },
            ) => {
                if p1.len() != p2.len() {
                    return false;
                }
                for (a, b) in p1.iter().zip(p2.iter()) {
                    if !a.is_compatible_with(b) {
                        return false;
                    }
                }
                r1.is_compatible_with(r2)
            }
            _ => false,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            TypeInfo::Int => "int".to_string(),
            TypeInfo::Float => "dec".to_string(),
            TypeInfo::String => "txt".to_string(),
            TypeInfo::Bool => "flag".to_string(),
            TypeInfo::Nil => "nil".to_string(),
            TypeInfo::Void => "void".to_string(),
            TypeInfo::List(inner) => format!("[{}]", inner.display_name()),
            TypeInfo::Map(k, v) => format!("[{}:{}]", k.display_name(), v.display_name()),
            TypeInfo::Function { params, ret } => {
                let p: Vec<String> = params.iter().map(|x| x.display_name()).collect();
                format!("({}) -> {}", p.join(", "), ret.display_name())
            }
            TypeInfo::Range => "range".to_string(),
            TypeInfo::Custom(name) => name.clone(),
            TypeInfo::Unknown => "unknown".to_string(),
        }
    }
}

pub struct Scope {
    symbols: HashMap<String, (TypeInfo, bool)>, // (TypeInfo, is_mutable)
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        let mut scope = Self {
            symbols: HashMap::new(),
            parent: None,
        };
        scope.inject_builtins();
        scope
    }

    pub fn new_child(parent: Scope) -> Self {
        Self {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn pop_parent(&mut self) -> Option<Scope> {
        self.parent.take().map(|boxed| *boxed)
    }

    fn inject_builtins(&mut self) {
        // Builtin functions
        self.symbols.insert(
            "say".to_string(),
            (
                TypeInfo::Function {
                    params: vec![TypeInfo::Unknown],
                    ret: Box::new(TypeInfo::Int),
                },
                false,
            ),
        );
        self.symbols.insert(
            "print".to_string(),
            (
                TypeInfo::Function {
                    params: vec![TypeInfo::Unknown],
                    ret: Box::new(TypeInfo::Int),
                },
                false,
            ),
        );
        self.symbols.insert(
            "len".to_string(),
            (
                TypeInfo::Function {
                    params: vec![TypeInfo::Unknown],
                    ret: Box::new(TypeInfo::Int),
                },
                false,
            ),
        );
        self.symbols.insert(
            "int".to_string(),
            (
                TypeInfo::Function {
                    params: vec![TypeInfo::Unknown],
                    ret: Box::new(TypeInfo::Int),
                },
                false,
            ),
        );
        self.symbols.insert(
            "dec".to_string(),
            (
                TypeInfo::Function {
                    params: vec![TypeInfo::Unknown],
                    ret: Box::new(TypeInfo::Float),
                },
                false,
            ),
        );
        self.symbols.insert(
            "str".to_string(),
            (
                TypeInfo::Function {
                    params: vec![TypeInfo::Unknown],
                    ret: Box::new(TypeInfo::String),
                },
                false,
            ),
        );
    }

    pub fn insert(&mut self, name: String, ty: TypeInfo, is_mutable: bool) {
        self.symbols.insert(name, (ty, is_mutable));
    }

    pub fn lookup(&self, name: &str) -> Option<(TypeInfo, bool)> {
        if let Some(entry) = self.symbols.get(name) {
            Some(entry.clone())
        } else if let Some(parent) = &self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }

    pub fn update(&mut self, name: &str, ty: TypeInfo) -> bool {
        if let Some(entry) = self.symbols.get_mut(name) {
            entry.0 = ty;
            true
        } else if let Some(parent) = &mut self.parent {
            parent.update(name, ty)
        } else {
            false
        }
    }
}
