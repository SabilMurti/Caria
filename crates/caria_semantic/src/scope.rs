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

        // Core FFI & Memory Arena Builtins
        let any_fn = TypeInfo::Function {
            params: vec![],
            ret: Box::new(TypeInfo::Unknown),
        };
        let builtins = [
            "say_str", "say_int", "say_float", "say_bool",
            "scope_enter", "scope_exit", "free_ptr",
            "metrics_inc", "metrics_get", "metrics_render_prometheus",
            "db_open", "db_close", "db_exec", "db_query", "db_exec_params", "db_query_params",
            "pool_create", "pool_acquire", "pool_release",
            "http_get", "http_post", "http_serve", "http_serve_router",
            "file_read", "file_write", "file_exists", "env_get", "env_set",
            "time_now", "time_now_millis", "sleep_millis", "spawn",
            "gas_set", "gas_get", "gas_tick",
            "sha256", "crypto_sha256", "base64_encode", "base64_decode",
            "regex_test", "regex_replace", "regex_match", "regex_match_all", "regex_find", "regex_split", "regex_escape",
            "json_parse", "json_stringify", "json_get",
            "str_concat", "concat", "str_len", "len", "str_split", "split",
            "str_trim", "trim", "str_lower", "lower", "str_upper", "upper",
            "str_contains", "contains", "str_substring", "substring", "str_replace", "replace",
            "array_pop", "array_len", "array_get", "array_set", "array_free",
            "map_alloc", "map_set", "map_get", "map_has", "map_len", "map_free",
            "channel_create", "channel_send", "channel_recv", "channel_try_recv", "channel_close",
            "hmac_sha256", "safe_equals", "jwt_sign", "jwt_verify",
            "http_put", "http_delete",
            "redis_connect", "redis_get", "redis_set", "redis_set_ex", "redis_del", "redis_incr", "redis_close",
            "sha512", "crypto_sha512", "argon2_hash", "hash_password", "argon2_verify", "verify_password",
            "aes_encrypt", "aes_gcm_encrypt", "aes_decrypt", "aes_gcm_decrypt",
            "uuid_v4", "sys_uuid_v4", "random_bytes_hex", "random_hex", "hex_encode", "sys_hex_encode", "hex_decode", "sys_hex_decode",
            "ws_connect", "ws_send", "ws_recv", "ws_close",
            "task_spawn", "task_join", "task_sleep",
            "file_append", "file_remove", "dir_create", "dir_remove", "dir_list", "file_size", "file_copy",
            "gzip_compress", "gzip_decompress",
            "sys_os", "sys_arch", "sys_cpu_count", "sys_cwd", "sys_exit",
            "time_iso_now", "time_format", "time_parse_iso",
            "process_exec", "csv_parse", "csv_stringify",
            "http_route", "http_dispatch", "http_server_start", "http_server_stop", "http_request",
            "caria_smart_eq", "caria_smart_neq",
            // JSON & SQL ORM additions
            "json_extract", "json_set", "json_has",
            "db_migration_run", "db_table_exists", "db_select_where",
            // Asymmetric Cryptography (Ed25519)
            "ed25519_keypair", "ed25519_sign", "ed25519_verify",
            // Archives
            "tar_create", "tar_extract", "zip_create", "zip_extract",
            // UDP Sockets
            "udp_bind", "udp_local_addr", "udp_send_to", "udp_recv_from", "udp_set_timeout", "udp_close",
            // ANSI Terminal
            "term_is_atty", "term_color", "term_clear", "term_cursor_move",
        ];
        for b in builtins {
            self.symbols.insert(b.to_string(), (any_fn.clone(), false));
        }

        // Standard library module namespaces
        let std_modules = [
            "crypto", "net", "db", "ws", "compress", "fs", "string", "regex", "json",
            "environ", "math", "collections", "time", "process", "sync", "task",
            "telemetry", "auth", "redis", "test", "log", "sys", "io", "schema", "udp", "term",
        ];
        for m in std_modules {
            self.symbols.insert(m.to_string(), (TypeInfo::Custom(format!("module:std.{}", m)), false));
        }
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
