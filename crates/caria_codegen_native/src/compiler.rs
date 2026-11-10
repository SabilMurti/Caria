//! Native Codegen Compiler - Cranelift JIT/Object Integration v0.114
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_object::{ObjectBuilder, ObjectModule};
use cranelift_module::{Module, Linkage, FuncId, DataDescription, DataId};
use cranelift_codegen::Context;
use cranelift_codegen::ir::{InstBuilder, AbiParam, Signature, UserFuncName};
use cranelift_codegen::ir::types;
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::isa;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use std::collections::HashMap;
use cranelift_codegen::ir::condcodes::IntCC;

pub enum CariaModule {
    Jit(JITModule),
    Object(ObjectModule),
}

pub struct CariaCompiler {
    module: Option<CariaModule>,
    function_cache: HashMap<String, *const u8>,
    function_ids: HashMap<String, FuncId>,
    string_data_ids: HashMap<String, DataId>,
}

/// Compilation context tracking variables and constants during code generation
struct CompileContext {
    /// Current variables: name → Cranelift Variable
    variables: HashMap<String, Variable>,
    /// Counter for unique Variable IDs
    next_var_idx: u32,
    /// Interned strings: index → string content
    string_constants: Vec<String>,
    /// Whether the current block is terminated
    is_terminated: bool,
}

impl CompileContext {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            next_var_idx: 0,
            string_constants: Vec::new(),
            is_terminated: false,
        }
    }

    fn declare_and_def(&mut self, name: &str, val: cranelift_codegen::ir::Value, builder: &mut FunctionBuilder) {
        let val = if builder.func.dfg.value_type(val) != types::I64 {
            builder.ins().uextend(types::I64, val)
        } else {
            val
        };
        let var = if let Some(&existing) = self.variables.get(name) {
            existing
        } else {
            let v = Variable::from_u32(self.next_var_idx);
            self.next_var_idx += 1;
            builder.declare_var(v, types::I64);
            self.variables.insert(name.to_string(), v);
            v
        };
        builder.def_var(var, val);
    }

    fn use_variable(&self, name: &str, builder: &mut FunctionBuilder) -> Result<cranelift_codegen::ir::Value, String> {
        if let Some(&var) = self.variables.get(name) {
            Ok(builder.use_var(var))
        } else {
            Err(format!("Undefined variable: {}", name))
        }
    }
}

impl CariaCompiler {
    pub fn default_target_triple() -> target_lexicon::Triple {
        use target_lexicon::*;
        let host = Triple::host();
        match host.architecture {
            Architecture::Aarch64(_) => Triple {
                architecture: Architecture::Aarch64(Aarch64Architecture::Aarch64),
                vendor: Vendor::Unknown,
                operating_system: host.operating_system,
                environment: host.environment,
                binary_format: host.binary_format,
            },
            _ => Triple {
                architecture: Architecture::X86_64,
                vendor: Vendor::Unknown,
                operating_system: host.operating_system,
                environment: host.environment,
                binary_format: host.binary_format,
            },
        }
    }

    pub fn new_jit() -> Result<Self, String> {
        let mut flag_builder = cranelift_codegen::settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").map_err(|e| e.to_string())?;
        flag_builder.set("opt_level", "speed").map_err(|e| e.to_string())?;
        
        let triple = Self::default_target_triple();
        
        let isa = isa::lookup(triple)
            .map_err(|e| format!("ISA lookup failed: {}", e))?
            .finish(cranelift_codegen::settings::Flags::new(flag_builder))
            .map_err(|e| format!("ISA finish failed: {}", e))?;
        
        let mut jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        
        // Register runtime symbols with JITBuilder
        Self::register_jit_symbols(&mut jit_builder)?;
        
        let module = JITModule::new(jit_builder);
        
        Ok(Self {
            module: Some(CariaModule::Jit(module)),
            function_cache: HashMap::new(),
            function_ids: HashMap::new(),
            string_data_ids: HashMap::new(),
        })
    }

    pub fn new_object() -> Result<Self, String> {
        let mut flag_builder = cranelift_codegen::settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").map_err(|e| e.to_string())?;
        flag_builder.set("is_pic", "true").map_err(|e| e.to_string())?;
        
        let triple = Self::default_target_triple();
        
        let isa = isa::lookup(triple)
            .map_err(|e| format!("ISA lookup failed: {}", e))?
            .finish(cranelift_codegen::settings::Flags::new(flag_builder))
            .map_err(|e| format!("ISA finish failed: {}", e))?;
        
        let obj_builder = ObjectBuilder::new(
            isa,
            "caria_program",
            cranelift_module::default_libcall_names(),
        ).map_err(|e| format!("ObjectBuilder creation failed: {}", e))?;
        
        let module = ObjectModule::new(obj_builder);
        
        Ok(Self {
            module: Some(CariaModule::Object(module)),
            function_cache: HashMap::new(),
            function_ids: HashMap::new(),
            string_data_ids: HashMap::new(),
        })
    }

    #[inline]
    fn module_mut(&mut self) -> &mut CariaModule {
        self.module.as_mut().expect("Compiler module has been consumed")
    }

    #[inline]
    fn module_ref(&self) -> &CariaModule {
        self.module.as_ref().expect("Compiler module has been consumed")
    }

    fn register_jit_symbols(builder: &mut JITBuilder) -> Result<(), String> {
        // Register all runtime FFI symbols with JIT - only functions that actually exist
        macro_rules! add_symbol {
            ($name:expr, $fn:path) => {{
                builder.symbol($name, $fn as *const u8);
            }};
        }

        // Math operations
        add_symbol!("caria_math_abs", caria_runtime::ffi_aot::caria_math_abs);
        add_symbol!("caria_math_min", caria_runtime::ffi_aot::caria_math_min);
        add_symbol!("caria_math_max", caria_runtime::ffi_aot::caria_math_max);
        add_symbol!("caria_math_pow", caria_runtime::ffi_aot::caria_math_pow);
        add_symbol!("caria_math_sqrt", caria_runtime::ffi_aot::caria_math_sqrt);
        add_symbol!("caria_math_random", caria_runtime::ffi_aot::caria_math_random);

        // Core runtime functions
        add_symbol!("caria_last_error", caria_runtime::ffi_aot::caria_last_error);
        add_symbol!("caria_array_alloc", caria_runtime::ffi_aot::caria_array_alloc);
        add_symbol!("caria_array_push", caria_runtime::ffi_aot::caria_array_push);
        add_symbol!("caria_array_pop", caria_runtime::ffi_aot::caria_array_pop);
        add_symbol!("caria_array_len", caria_runtime::ffi_aot::caria_array_len);
        add_symbol!("caria_array_get", caria_runtime::ffi_aot::caria_array_get);
        add_symbol!("caria_array_set", caria_runtime::ffi_aot::caria_array_set);
        add_symbol!("caria_array_free", caria_runtime::ffi_aot::caria_array_free);
        add_symbol!("caria_map_alloc", caria_runtime::ffi_aot::caria_map_alloc);
        add_symbol!("caria_map_set", caria_runtime::ffi_aot::caria_map_set);
        add_symbol!("caria_map_get", caria_runtime::ffi_aot::caria_map_get);
        add_symbol!("caria_map_has", caria_runtime::ffi_aot::caria_map_has);
        add_symbol!("caria_map_len", caria_runtime::ffi_aot::caria_map_len);
        add_symbol!("caria_map_free", caria_runtime::ffi_aot::caria_map_free);
        
        // Hash and crypto
        add_symbol!("caria_hash_make", caria_runtime::ffi_aot::caria_hash_make);
        
        // JSON
        add_symbol!("caria_json_parse", caria_runtime::ffi_aot::caria_json_parse);
        add_symbol!("caria_json_stringify", caria_runtime::ffi_aot::caria_json_stringify);
        add_symbol!("caria_json_get", caria_runtime::ffi_aot::caria_json_get);
        
        // String operations
        add_symbol!("caria_string_concat", caria_runtime::ffi_aot::caria_string_concat);
        add_symbol!("caria_string_len", caria_runtime::ffi_aot::caria_string_len);
        add_symbol!("caria_string_split", caria_runtime::ffi_aot::caria_string_split);
        add_symbol!("caria_string_trim", caria_runtime::ffi_aot::caria_string_trim);
        add_symbol!("caria_string_trim_left", caria_runtime::ffi_aot::caria_string_trim_left);
        add_symbol!("caria_string_trim_right", caria_runtime::ffi_aot::caria_string_trim_right);
        add_symbol!("caria_string_lower", caria_runtime::ffi_aot::caria_string_lower);
        add_symbol!("caria_string_upper", caria_runtime::ffi_aot::caria_string_upper);
        add_symbol!("caria_string_contains", caria_runtime::ffi_aot::caria_string_contains);
        add_symbol!("caria_string_indexof", caria_runtime::ffi_aot::caria_string_indexof);
        add_symbol!("caria_string_lastindexof", caria_runtime::ffi_aot::caria_string_lastindexof);
        add_symbol!("caria_string_substring", caria_runtime::ffi_aot::caria_string_substring);
        add_symbol!("caria_string_replace_all", caria_runtime::ffi_aot::caria_string_replace_all);
        
        // I/O & OS
        add_symbol!("caria_file_read", caria_runtime::ffi_aot::caria_file_read);
        add_symbol!("caria_file_write", caria_runtime::ffi_aot::caria_file_write);
        add_symbol!("caria_file_exists", caria_runtime::ffi_aot::caria_file_exists);
        add_symbol!("caria_env_get", caria_runtime::ffi_aot::caria_env_get);
        add_symbol!("caria_env_set", caria_runtime::ffi_aot::caria_env_set);
        add_symbol!("caria_sleep_millis", caria_runtime::ffi_aot::caria_sleep_millis);
        add_symbol!("caria_spawn", caria_runtime::ffi_aot::caria_spawn);
        add_symbol!("caria_gas_set", caria_runtime::ffi_aot::caria_gas_set);
        add_symbol!("caria_gas_get", caria_runtime::ffi_aot::caria_gas_get);
        add_symbol!("caria_gas_tick", caria_runtime::ffi_aot::caria_gas_tick);
        add_symbol!("caria_time_now", caria_runtime::ffi_aot::caria_time_now);
        add_symbol!("caria_time_now_millis", caria_runtime::ffi_aot::caria_time_now_millis);

        // Database - SQLite & Connection Pool
        add_symbol!("caria_sqlite_open", caria_runtime::ffi_aot::caria_sqlite_open);
        add_symbol!("caria_sqlite_close", caria_runtime::ffi_aot::caria_sqlite_close);
        add_symbol!("caria_sqlite_exec", caria_runtime::ffi_aot::caria_sqlite_exec);
        add_symbol!("caria_sqlite_query", caria_runtime::ffi_aot::caria_sqlite_query);
        add_symbol!("caria_sqlite_exec_params", caria_runtime::ffi_aot::caria_sqlite_exec_params);
        add_symbol!("caria_sqlite_query_params", caria_runtime::ffi_aot::caria_sqlite_query_params);
        add_symbol!("caria_pool_create", caria_runtime::ffi_aot::caria_pool_create);
        add_symbol!("caria_pool_acquire", caria_runtime::ffi_aot::caria_pool_acquire);
        add_symbol!("caria_pool_release", caria_runtime::ffi_aot::caria_pool_release);

        // HTTP Client & Enterprise Microserver Engine
        add_symbol!("caria_http_get", caria_runtime::ffi_aot::caria_http_get);
        add_symbol!("caria_http_post", caria_runtime::ffi_aot::caria_http_post);
        add_symbol!("caria_http_put", caria_runtime::ffi_aot::caria_http_put);
        add_symbol!("caria_http_delete", caria_runtime::ffi_aot::caria_http_delete);
        add_symbol!("caria_http_listen_and_serve", caria_runtime::ffi_aot::caria_http_listen_and_serve);
        add_symbol!("caria_http_serve_router", caria_runtime::ffi_aot::caria_http_serve_router);

        // Redis In-Memory Data Store (std.redis)
        add_symbol!("caria_redis_connect", caria_runtime::ffi_aot::caria_redis_connect);
        add_symbol!("caria_redis_get", caria_runtime::ffi_aot::caria_redis_get);
        add_symbol!("caria_redis_set", caria_runtime::ffi_aot::caria_redis_set);
        add_symbol!("caria_redis_set_ex", caria_runtime::ffi_aot::caria_redis_set_ex);
        add_symbol!("caria_redis_del", caria_runtime::ffi_aot::caria_redis_del);
        add_symbol!("caria_redis_incr", caria_runtime::ffi_aot::caria_redis_incr);
        add_symbol!("caria_redis_close", caria_runtime::ffi_aot::caria_redis_close);

        // Memory Arena & Scopes
        add_symbol!("caria_scope_enter", caria_runtime::ffi_aot::caria_scope_enter);
        add_symbol!("caria_scope_exit", caria_runtime::ffi_aot::caria_scope_exit);
        add_symbol!("caria_free_ptr", caria_runtime::ffi_aot::caria_free_ptr);

        // Telemetry & Metrics
        add_symbol!("caria_metrics_inc", caria_runtime::ffi_aot::caria_metrics_inc);
        add_symbol!("caria_metrics_get", caria_runtime::ffi_aot::caria_metrics_get);
        add_symbol!("caria_metrics_render_prometheus", caria_runtime::ffi_aot::caria_metrics_render_prometheus);
        
        // Concurrency & Channels (std.sync)
        add_symbol!("caria_channel_create", caria_runtime::ffi_aot::caria_channel_create);
        add_symbol!("caria_channel_send", caria_runtime::ffi_aot::caria_channel_send);
        add_symbol!("caria_channel_recv", caria_runtime::ffi_aot::caria_channel_recv);
        add_symbol!("caria_channel_try_recv", caria_runtime::ffi_aot::caria_channel_try_recv);
        add_symbol!("caria_channel_close", caria_runtime::ffi_aot::caria_channel_close);

        // Cryptographic Authentication & JWT (std.auth)
        add_symbol!("caria_crypto_hmac_sha256", caria_runtime::ffi_aot::caria_crypto_hmac_sha256);
        add_symbol!("caria_crypto_safe_equals", caria_runtime::ffi_aot::caria_crypto_safe_equals);
        add_symbol!("caria_crypto_jwt_sign", caria_runtime::ffi_aot::caria_crypto_jwt_sign);
        add_symbol!("caria_crypto_jwt_verify", caria_runtime::ffi_aot::caria_crypto_jwt_verify);
        
        // Encoding
        add_symbol!("caria_base64_encode", caria_runtime::ffi_aot::caria_base64_encode);
        add_symbol!("caria_base64_decode", caria_runtime::ffi_aot::caria_base64_decode);
        
        // Regex
        add_symbol!("caria_regex_validate", caria_runtime::ffi_aot::caria_regex_validate);
        add_symbol!("caria_regex_test", caria_runtime::ffi_aot::caria_regex_test);
        add_symbol!("caria_regex_match", caria_runtime::ffi_aot::caria_regex_match);
        add_symbol!("caria_regex_match_all", caria_runtime::ffi_aot::caria_regex_match_all);
        add_symbol!("caria_regex_find", caria_runtime::ffi_aot::caria_regex_find);
        add_symbol!("caria_regex_replace", caria_runtime::ffi_aot::caria_regex_replace);
        add_symbol!("caria_regex_split", caria_runtime::ffi_aot::caria_regex_split);
        add_symbol!("caria_regex_escape", caria_runtime::ffi_aot::caria_regex_escape);
        
        // Core I/O
        add_symbol!("caria_say", caria_runtime::ffi_aot::caria_say);
        add_symbol!("caria_say_int", caria_runtime::ffi_aot::caria_say_int);
        add_symbol!("caria_say_float", caria_runtime::ffi_aot::caria_say_float);
        add_symbol!("caria_say_bool", caria_runtime::ffi_aot::caria_say_bool);
        add_symbol!("caria_say_str", caria_runtime::ffi_aot::caria_say_str);
        add_symbol!("caria_say_level", caria_runtime::ffi_aot::caria_say_level);

        // Content Equality
        add_symbol!("caria_smart_eq", caria_runtime::ffi_aot::caria_smart_eq);
        add_symbol!("caria_smart_neq", caria_runtime::ffi_aot::caria_smart_neq);

        // Advanced Cryptography Suite
        add_symbol!("caria_crypto_sha512", caria_runtime::ffi_aot::caria_crypto_sha512);
        add_symbol!("caria_crypto_argon2_hash", caria_runtime::ffi_aot::caria_crypto_argon2_hash);
        add_symbol!("caria_crypto_argon2_verify", caria_runtime::ffi_aot::caria_crypto_argon2_verify);
        add_symbol!("caria_crypto_aes_encrypt", caria_runtime::ffi_aot::caria_crypto_aes_encrypt);
        add_symbol!("caria_crypto_aes_decrypt", caria_runtime::ffi_aot::caria_crypto_aes_decrypt);
        add_symbol!("caria_crypto_uuid_v4", caria_runtime::ffi_aot::caria_crypto_uuid_v4);
        add_symbol!("caria_crypto_random_hex", caria_runtime::ffi_aot::caria_crypto_random_hex);
        add_symbol!("caria_hex_encode", caria_runtime::ffi_aot::caria_hex_encode);
        add_symbol!("caria_hex_decode", caria_runtime::ffi_aot::caria_hex_decode);

        // Real-Time WebSockets
        add_symbol!("caria_ws_connect", caria_runtime::ffi_aot::caria_ws_connect);
        add_symbol!("caria_ws_send", caria_runtime::ffi_aot::caria_ws_send);
        add_symbol!("caria_ws_recv", caria_runtime::ffi_aot::caria_ws_recv);
        add_symbol!("caria_ws_close", caria_runtime::ffi_aot::caria_ws_close);

        // Native Task Scheduler
        add_symbol!("caria_task_spawn", caria_runtime::ffi_aot::caria_task_spawn);
        add_symbol!("caria_task_join", caria_runtime::ffi_aot::caria_task_join);
        add_symbol!("caria_task_sleep", caria_runtime::ffi_aot::caria_task_sleep);

        // Enterprise Filesystem
        add_symbol!("caria_file_append", caria_runtime::ffi_aot::caria_file_append);
        add_symbol!("caria_file_remove", caria_runtime::ffi_aot::caria_file_remove);
        add_symbol!("caria_dir_create", caria_runtime::ffi_aot::caria_dir_create);
        add_symbol!("caria_dir_remove", caria_runtime::ffi_aot::caria_dir_remove);
        add_symbol!("caria_dir_list", caria_runtime::ffi_aot::caria_dir_list);
        add_symbol!("caria_file_size", caria_runtime::ffi_aot::caria_file_size);
        add_symbol!("caria_file_copy", caria_runtime::ffi_aot::caria_file_copy);

        // GZIP Compression
        add_symbol!("caria_gzip_compress", caria_runtime::ffi_aot::caria_gzip_compress);
        add_symbol!("caria_gzip_decompress", caria_runtime::ffi_aot::caria_gzip_decompress);

        // System Diagnostics
        add_symbol!("caria_sys_os", caria_runtime::ffi_aot::caria_sys_os);
        add_symbol!("caria_sys_arch", caria_runtime::ffi_aot::caria_sys_arch);
        add_symbol!("caria_sys_cpu_count", caria_runtime::ffi_aot::caria_sys_cpu_count);
        add_symbol!("caria_sys_cwd", caria_runtime::ffi_aot::caria_sys_cwd);
        add_symbol!("caria_sys_exit", caria_runtime::ffi_aot::caria_sys_exit);

        // Time & Chrono
        add_symbol!("caria_time_iso_now", caria_runtime::ffi_aot::caria_time_iso_now);
        add_symbol!("caria_time_format", caria_runtime::ffi_aot::caria_time_format);
        add_symbol!("caria_time_parse_iso", caria_runtime::ffi_aot::caria_time_parse_iso);

        // Safe Subprocess Execution
        add_symbol!("caria_process_exec", caria_runtime::ffi_aot::caria_process_exec);

        // CSV Engine
        add_symbol!("caria_csv_parse", caria_runtime::ffi_aot::caria_csv_parse);
        add_symbol!("caria_csv_stringify", caria_runtime::ffi_aot::caria_csv_stringify);

        // Dynamic HTTP Router
        add_symbol!("caria_http_route_register", caria_runtime::ffi_aot::caria_http_route_register);
        add_symbol!("caria_http_route_dispatch", caria_runtime::ffi_aot::caria_http_route_dispatch);

        Ok(())
    }

    pub fn compile_to_object(&mut self, prog: &caria_parser::ast::Program) -> Result<Vec<u8>, String> {
        // Compile the program first
        self.compile_program(prog)?;
        
        let module = self.module.take().ok_or("Module already consumed")?;
        
        match module {
            CariaModule::Object(obj_module) => {
                let product = obj_module.finish();
                Ok(product.emit().map_err(|e| format!("Failed to emit object: {}", e))?)
            }
            CariaModule::Jit(_) => {
                Err("Cannot produce object file from JIT module. Use new_object() instead.".to_string())
            }
        }
    }

    /// Compile AST to a standalone binary executable using AOT compilation
    /// This avoids runtime CPU feature detection issues by using explicit baseline ISA
    pub fn compile_to_binary(&mut self, prog: &caria_parser::ast::Program, output_path: &str) -> Result<(), String> {
        use std::process::Command;
        use std::path::Path;
        
        // Compile the program first
        self.compile_program(prog)?;
        
        // Generate object file
        let obj_path = format!("{}.o", output_path);
        
        let module = self.module.take().ok_or("Module already consumed")?;
        
        let obj_bytes = match module {
            CariaModule::Object(obj_module) => {
                let product = obj_module.finish();
                product.emit().map_err(|e| format!("Failed to emit object: {}", e))?
            }
            CariaModule::Jit(_) => {
                return Err("Cannot produce binary from JIT module. Use new_object() instead.".to_string());
            }
        };
        
        // Write object file
        std::fs::write(&obj_path, &obj_bytes)
            .map_err(|e| format!("Failed to write object file {}: {}", obj_path, e))?;
        
        // Link using system linker
        // Try to find the runtime library path
        let target_dir = std::env::var("CARGO_TARGET_DIR")
            .or_else(|_| std::env::var("OUT_DIR").map(|p| {
                // OUT_DIR is typically target/debug/build/<pkg>/out
                // We need to go up to target/debug or target/release
                Path::new(&p).ancestors().nth(3).unwrap().to_str().unwrap().to_string()
            }))
            .unwrap_or_else(|_| "target/release".to_string());
        
        let mut cmd = Command::new("cc");
        cmd.arg("-no-pie")
            .arg("-o")
            .arg(output_path)
            .arg(&obj_path)
            .arg(format!("-L{}", target_dir))
            .arg("-L/usr/local/lib")
            .arg("-L/usr/lib")
            .arg("-L/root/projects/Caria/target/debug")
            .arg("-L/root/projects/Caria/target/release")
            .arg("-lcaria_runtime")
            .arg("-lpthread")
            .arg("-ldl")
            .arg("-lm");
        
        let status = cmd.status()
            .map_err(|e| format!("Failed to run linker: {}", e))?;
        
        if !status.success() {
            return Err(format!("Linker failed with status: {}", status));
        }
        
        // Clean up object file
        let _ = std::fs::remove_file(&obj_path);
        
        Ok(())
    }
    
    pub fn compile_to_jit(&mut self, prog: &caria_parser::ast::Program) -> Result<*const u8, String> {
        // Compile the program into JIT module
        self.compile_program(prog)?;
        
        // Find and return pointer to main function
        self.get_function_pointer("main")
    }
    
    pub fn compile_and_get_function(&mut self, prog: &caria_parser::ast::Program, name: &str) -> Result<*const u8, String> {
        // Compile the program
        self.compile_program(prog)?;
        
        // Get specific function pointer
        self.get_function_pointer(name)
    }
    
    fn compile_program(&mut self, prog: &caria_parser::ast::Program) -> Result<(), String> {
        // Pass 1: Declare all functions (runes) first so forward references, recursion, and inter-rune calls resolve
        for stmt in &prog.statements {
            if let caria_parser::ast::Statement::RuneDecl(rune) = stmt {
                let sig = self.create_signature(rune)?;
                let func_id = match self.module_mut() {
                    CariaModule::Jit(jit) => {
                        jit.declare_function(&rune.name, Linkage::Export, &sig)
                            .map_err(|e| format!("Failed to declare function {}: {}", rune.name, e))?
                    }
                    CariaModule::Object(obj) => {
                        obj.declare_function(&rune.name, Linkage::Export, &sig)
                            .map_err(|e| format!("Failed to declare function {}: {}", rune.name, e))?
                    }
                };
                self.function_ids.insert(rune.name.clone(), func_id);
            }
        }

        // Pass 2: Compile each function's body
        for stmt in &prog.statements {
            if let caria_parser::ast::Statement::RuneDecl(rune) = stmt {
                self.compile_rune_body(rune)?;
            }
        }
        
        // Finalize all functions
        match self.module_mut() {
            CariaModule::Jit(jit) => {
                jit.finalize_definitions().map_err(|e| format!("JIT finalize failed: {}", e))?;
            }
            CariaModule::Object(_obj) => { 
                // Object finalization happens in finish()
            }
        }
        
        Ok(())
    }
    
    fn compile_rune_body(&mut self, rune: &caria_parser::ast::RuneDecl) -> Result<(), String> {
        let func_id = *self.function_ids.get(&rune.name)
            .ok_or_else(|| format!("Undeclared function: {}", rune.name))?;
        let sig = self.create_signature(rune)?;
        
        let mut ctx = Context::new();
        ctx.func.signature = sig;
        ctx.func.name = UserFuncName::user(0, func_id.as_u32());
        
        {
            let mut fn_builder_ctx = FunctionBuilderContext::new();
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fn_builder_ctx);
            
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);
            
            let mut compile_ctx = CompileContext::new();

            // Map function parameters from Cranelift block arguments to local variables
            let block_params = builder.block_params(entry_block).to_vec();
            for (idx, param) in rune.params.iter().enumerate() {
                if idx < block_params.len() {
                    compile_ctx.declare_and_def(&param.name, block_params[idx], &mut builder);
                }
            }
            
            // Compile function body - extract statements from RuneBody
            match &rune.body {
                caria_parser::ast::RuneBody::Block(statements) => {
                    compile_ctx.is_terminated = false;
                    for stmt in statements {
                        if compile_ctx.is_terminated {
                            break;
                        }
                        self.compile_statement(stmt, &mut compile_ctx, &mut builder)?;
                    }
                    if !compile_ctx.is_terminated {
                        if builder.func.signature.returns.first().map(|r| r.value_type) == Some(types::I32) {
                            let zero = builder.ins().iconst(types::I32, 0);
                            builder.ins().return_(&[zero]);
                        } else {
                            let zero = builder.ins().iconst(types::I64, 0);
                            builder.ins().return_(&[zero]);
                        }
                        compile_ctx.is_terminated = true;
                    }
                }
                caria_parser::ast::RuneBody::Inline(expr) => {
                    let val = self.compile_expression(expr, &mut compile_ctx, &mut builder)?;
                    if builder.func.signature.returns.first().map(|r| r.value_type) == Some(types::I32) {
                        let ret_val = builder.ins().ireduce(types::I32, val);
                        builder.ins().return_(&[ret_val]);
                    } else {
                        builder.ins().return_(&[val]);
                    }
                }
            };
            
            builder.finalize();
        }
        
        // Define the function in the module
        match self.module_mut() {
            CariaModule::Jit(jit) => {
                jit.define_function(func_id, &mut ctx)
                    .map_err(|e| format!("Failed to define function {}: {}", rune.name, e))?;
            }
            CariaModule::Object(obj) => {
                obj.define_function(func_id, &mut ctx)
                    .map_err(|e| format!("Failed to define function {}: {}", rune.name, e))?;
            }
        }
        
        Ok(())
    }
    
    fn create_signature(&self, rune: &caria_parser::ast::RuneDecl) -> Result<Signature, String> {
        let mut sig = Signature::new(cranelift_codegen::isa::CallConv::SystemV);
        
        // Add parameters
        for _param in &rune.params {
            sig.params.push(AbiParam::new(types::I64));
        }
        
        // Add return type: i32 for main (POSIX exit code), i64 for general runes
        if rune.name == "main" {
            sig.returns.push(AbiParam::new(types::I32));
        } else {
            sig.returns.push(AbiParam::new(types::I64));
        }
        
        Ok(sig)
    }

    pub fn call_named_function(
        &mut self,
        name: &str,
        arg_vals: &[cranelift_codegen::ir::Value],
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::Value, String> {
        // 1. Check if user-defined rune
        if let Some(&func_id) = self.function_ids.get(name) {
            let local_callee = match self.module_mut() {
                CariaModule::Jit(jit) => jit.declare_func_in_func(func_id, builder.func),
                CariaModule::Object(obj) => obj.declare_func_in_func(func_id, builder.func),
            };
            let call_inst = builder.ins().call(local_callee, arg_vals);
            let results = builder.inst_results(call_inst);
            return Ok(if results.is_empty() {
                builder.ins().iconst(types::I64, 0)
            } else {
                let res = results[0];
                if builder.func.dfg.value_type(res) == types::I32 {
                    builder.ins().uextend(types::I64, res)
                } else {
                    res
                }
            });
        }

        // 2. FFI or Runtime standard library function mapping
        let target_symbol = match name {
            // Math
            "abs" | "math_abs" => "caria_math_abs",
            "min" | "math_min" => "caria_math_min",
            "max" | "math_max" => "caria_math_max",
            "pow" | "math_pow" => "caria_math_pow",
            "sqrt" | "math_sqrt" => "caria_math_sqrt",
            "random" | "math_random" => "caria_math_random",
            // Array / Collections
            "array_alloc" => "caria_array_alloc",
            "array_push" => "caria_array_push",
            "array_pop" => "caria_array_pop",
            "array_len" => "caria_array_len",
            "array_get" => "caria_array_get",
            "array_set" => "caria_array_set",
            "array_free" => "caria_array_free",
            // Map
            "map_alloc" => "caria_map_alloc",
            "map_set" => "caria_map_set",
            "map_get" => "caria_map_get",
            "map_has" => "caria_map_has",
            "map_len" => "caria_map_len",
            "map_free" => "caria_map_free",
            // String
            "str_concat" | "concat" => "caria_string_concat",
            "str_len" | "len" => "caria_string_len",
            "str_split" | "split" => "caria_string_split",
            "str_trim" | "trim" => "caria_string_trim",
            "str_lower" | "lower" => "caria_string_lower",
            "str_upper" | "upper" => "caria_string_upper",
            "str_contains" | "contains" => "caria_string_contains",
            "str_substring" | "substring" => "caria_string_substring",
            "str_replace" | "replace" => "caria_string_replace_all",
            // JSON
            "json_parse" => "caria_json_parse",
            "json_stringify" => "caria_json_stringify",
            "json_get" => "caria_json_get",
            // Database (SQLite)
            "db_open" => "caria_sqlite_open",
            "db_close" => "caria_sqlite_close",
            "db_exec" => "caria_sqlite_exec",
            "db_query" => "caria_sqlite_query",
            "db_exec_params" => "caria_sqlite_exec_params",
            "db_query_params" => "caria_sqlite_query_params",
            "pool_create" => "caria_pool_create",
            "pool_acquire" => "caria_pool_acquire",
            "pool_release" => "caria_pool_release",
            // HTTP
            "http_get" => "caria_http_get",
            "http_post" => "caria_http_post",
            "http_put" => "caria_http_put",
            "http_delete" => "caria_http_delete",
            "http_serve" => "caria_http_listen_and_serve",
            "http_serve_router" => "caria_http_serve_router",
            // Redis
            "redis_connect" => "caria_redis_connect",
            "redis_get" => "caria_redis_get",
            "redis_set" => "caria_redis_set",
            "redis_set_ex" => "caria_redis_set_ex",
            "redis_del" => "caria_redis_del",
            "redis_incr" => "caria_redis_incr",
            "redis_close" => "caria_redis_close",
            // Scopes & Arena
            "scope_enter" => "caria_scope_enter",
            "scope_exit" => "caria_scope_exit",
            "free_ptr" => "caria_free_ptr",
            // Metrics & Telemetry
            "metrics_inc" => "caria_metrics_inc",
            "metrics_get" => "caria_metrics_get",
            "metrics_render_prometheus" => "caria_metrics_render_prometheus",
            // Concurrency & Channels
            "channel_create" => "caria_channel_create",
            "channel_send" => "caria_channel_send",
            "channel_recv" => "caria_channel_recv",
            "channel_try_recv" => "caria_channel_try_recv",
            "channel_close" => "caria_channel_close",
            // Cryptographic Authentication & JWT
            "hmac_sha256" => "caria_crypto_hmac_sha256",
            "safe_equals" => "caria_crypto_safe_equals",
            "jwt_sign" => "caria_crypto_jwt_sign",
            "jwt_verify" => "caria_crypto_jwt_verify",
            // I/O & OS
            "file_read" => "caria_file_read",
            "file_write" => "caria_file_write",
            "file_exists" => "caria_file_exists",
            "env_get" => "caria_env_get",
            "env_set" => "caria_env_set",
            "time_now" => "caria_time_now",
            "time_now_millis" => "caria_time_now_millis",
            "sleep_millis" => "caria_sleep_millis",
            "spawn" => "caria_spawn",
            "gas_set" => "caria_gas_set",
            "gas_get" => "caria_gas_get",
            "gas_tick" => "caria_gas_tick",
            // Crypto & Encoding
            "sha256" | "crypto_sha256" => "caria_hash_make",
            "base64_encode" => "caria_base64_encode",
            "base64_decode" => "caria_base64_decode",
            // Regex
            "regex_test" => "caria_regex_test",
            "regex_replace" => "caria_regex_replace",
            "regex_match" => "caria_regex_match",
            "regex_match_all" => "caria_regex_match_all",
            "regex_find" => "caria_regex_find",
            "regex_split" => "caria_regex_split",
            "regex_escape" => "caria_regex_escape",
            // Advanced Cryptography
            "sha512" | "crypto_sha512" => "caria_crypto_sha512",
            "argon2_hash" | "hash_password" => "caria_crypto_argon2_hash",
            "argon2_verify" | "verify_password" => "caria_crypto_argon2_verify",
            "aes_encrypt" | "aes_gcm_encrypt" => "caria_crypto_aes_encrypt",
            "aes_decrypt" | "aes_gcm_decrypt" => "caria_crypto_aes_decrypt",
            "uuid_v4" | "sys_uuid_v4" => "caria_crypto_uuid_v4",
            "random_bytes_hex" | "random_hex" => "caria_crypto_random_hex",
            "hex_encode" | "sys_hex_encode" => "caria_hex_encode",
            "hex_decode" | "sys_hex_decode" => "caria_hex_decode",
            // WebSockets
            "ws_connect" => "caria_ws_connect",
            "ws_send" => "caria_ws_send",
            "ws_recv" => "caria_ws_recv",
            "ws_close" => "caria_ws_close",
            // Task Scheduler
            "task_spawn" => "caria_task_spawn",
            "task_join" => "caria_task_join",
            "task_sleep" => "caria_task_sleep",
            // Filesystem
            "file_append" => "caria_file_append",
            "file_remove" => "caria_file_remove",
            "dir_create" => "caria_dir_create",
            "dir_remove" => "caria_dir_remove",
            "dir_list" => "caria_dir_list",
            "file_size" => "caria_file_size",
            "file_copy" => "caria_file_copy",
            // Compression
            "gzip_compress" => "caria_gzip_compress",
            "gzip_decompress" => "caria_gzip_decompress",
            // System
            "sys_os" => "caria_sys_os",
            "sys_arch" => "caria_sys_arch",
            "sys_cpu_count" => "caria_sys_cpu_count",
            "sys_cwd" => "caria_sys_cwd",
            "sys_exit" => "caria_sys_exit",
            // Time & Chrono
            "time_iso_now" => "caria_time_iso_now",
            "time_format" => "caria_time_format",
            "time_parse_iso" => "caria_time_parse_iso",
            // Process
            "process_exec" => "caria_process_exec",
            // CSV
            "csv_parse" => "caria_csv_parse",
            "csv_stringify" => "caria_csv_stringify",
            // HTTP Router Dispatch
            "http_route" => "caria_http_route_register",
            "http_dispatch" => "caria_http_route_dispatch",
            // Output
            "say" => "caria_say",
            "say_str" => "caria_say_str",
            "say_int" => "caria_say_int",
            "say_bool" => "caria_say_bool",
            "say_float" => "caria_say_float",
            other => other,
        };

        // Create standard C FFI signature (all i64 params, i64 return)
        let mut sig = Signature::new(cranelift_codegen::isa::CallConv::SystemV);
        for _ in 0..arg_vals.len() {
            sig.params.push(AbiParam::new(types::I64));
        }
        sig.returns.push(AbiParam::new(types::I64));

        let callee = match self.module_mut() {
            CariaModule::Jit(jit) => jit.declare_function(target_symbol, Linkage::Import, &sig)
                .map_err(|e| format!("Failed to declare FFI function '{}': {}", target_symbol, e))?,
            CariaModule::Object(obj) => obj.declare_function(target_symbol, Linkage::Import, &sig)
                .map_err(|e| format!("Failed to declare FFI function '{}': {}", target_symbol, e))?,
        };

        let local_callee = match self.module_mut() {
            CariaModule::Jit(jit) => jit.declare_func_in_func(callee, builder.func),
            CariaModule::Object(obj) => obj.declare_func_in_func(callee, builder.func),
        };

        let call_inst = builder.ins().call(local_callee, arg_vals);
        let results = builder.inst_results(call_inst);
        Ok(if results.is_empty() {
            builder.ins().iconst(types::I64, 0)
        } else {
            results[0]
        })
    }
    
    fn get_pointer_type(&self) -> cranelift_codegen::ir::Type {
        match self.module_ref() {
            CariaModule::Jit(jit) => jit.target_config().pointer_type(),
            CariaModule::Object(obj) => obj.target_config().pointer_type(),
        }
    }
    
    fn get_function_pointer(&mut self, name: &str) -> Result<*const u8, String> {
        // Check cache first
        if let Some(&ptr) = self.function_cache.get(name) {
            return Ok(ptr);
        }
        
        let func_id = *self.function_ids.get(name)
            .ok_or_else(|| format!("Function '{}' not found", name))?;

        match self.module_mut() {
            CariaModule::Jit(jit) => {
                // Get code pointer from JIT module
                let ptr = jit.get_finalized_function(func_id);
                
                // Cache it
                self.function_cache.insert(name.to_string(), ptr);
                
                Ok(ptr)
            }
            CariaModule::Object(_) => {
                Err("Cannot get function pointer from Object module. Use compile_to_object() instead.".to_string())
            }
        }
    }
    
    fn compile_statement(
        &mut self,
        stmt: &caria_parser::ast::Statement,
        ctx: &mut CompileContext,
        builder: &mut FunctionBuilder,
    ) -> Result<(), String> {
        if ctx.is_terminated {
            return Ok(());
        }
        match stmt {
            // --- Variable declarations ---
            caria_parser::ast::Statement::ConstDecl(cd) => {
                let val = self.compile_expression(&cd.value, ctx, builder)?;
                ctx.declare_and_def(&cd.name, val, builder);
                Ok(())
            }
            
            caria_parser::ast::Statement::MutDecl(md) => {
                let val = self.compile_expression(&md.value, ctx, builder)?;
                ctx.declare_and_def(&md.name, val, builder);
                Ok(())
            }
            
            // --- Assignment ---
            caria_parser::ast::Statement::AssignStmt(ast) => {
                let val = self.compile_expression(&ast.value, ctx, builder)?;
                ctx.declare_and_def(&ast.name, val, builder);
                Ok(())
            }
            
            // --- Expression statement ---
            caria_parser::ast::Statement::ExprStmt(expr) => {
                self.compile_expression(expr, ctx, builder)?;
                Ok(())
            }
            
            // --- Return ---
            caria_parser::ast::Statement::RetStmt(expr) => {
                let val = self.compile_expression(expr, ctx, builder)?;
                let val = if builder.func.dfg.value_type(val) != types::I64 {
                    builder.ins().uextend(types::I64, val)
                } else {
                    val
                };
                if builder.func.signature.returns.first().map(|r| r.value_type) == Some(types::I32) {
                    let ret_val = builder.ins().ireduce(types::I32, val);
                    builder.ins().return_(&[ret_val]);
                } else {
                    builder.ins().return_(&[val]);
                }
                ctx.is_terminated = true;
                Ok(())
            }
            
            // --- If statement ---
            caria_parser::ast::Statement::IfStmt(if_expr) => {
                let cond = self.compile_expression(&if_expr.condition, ctx, builder)?;
                let cond_bool = if builder.func.dfg.value_type(cond) == types::I64 {
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().icmp(IntCC::NotEqual, cond, zero)
                } else {
                    cond
                };
                
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                
                builder.ins().brif(cond_bool, then_block, &[], else_block, &[]);
                
                // Then branch
                builder.switch_to_block(then_block);
                builder.seal_block(then_block);
                ctx.is_terminated = false;
                for s in &if_expr.consequence {
                    if ctx.is_terminated { break; }
                    self.compile_statement(s, ctx, builder)?;
                }
                let then_term = ctx.is_terminated;
                if !then_term {
                    builder.ins().jump(merge_block, &[]);
                }
                
                // Else branch
                builder.switch_to_block(else_block);
                builder.seal_block(else_block);
                ctx.is_terminated = false;
                if let Some(ref alt) = if_expr.alternative {
                    for s in alt {
                        if ctx.is_terminated { break; }
                        self.compile_statement(s, ctx, builder)?;
                    }
                }
                let else_term = ctx.is_terminated;
                if !else_term {
                    builder.ins().jump(merge_block, &[]);
                }
                
                // Merge point
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                ctx.is_terminated = then_term && else_term;
                Ok(())
            }
            
            // --- While loop ---
            caria_parser::ast::Statement::WhileStmt(ws) => {
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let after_block = builder.create_block();
                
                builder.ins().jump(header_block, &[]);
                
                builder.switch_to_block(header_block);
                ctx.is_terminated = false;
                let cond = self.compile_expression(&ws.condition, ctx, builder)?;
                let cond_bool = if builder.func.dfg.value_type(cond) == types::I64 {
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().icmp(IntCC::NotEqual, cond, zero)
                } else {
                    cond
                };
                builder.ins().brif(cond_bool, body_block, &[], after_block, &[]);
                
                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                ctx.is_terminated = false;
                let one = builder.ins().iconst(types::I64, 1);
                let _ = self.call_named_function("caria_gas_tick", &[one], builder);
                for s in &ws.body {
                    if ctx.is_terminated { break; }
                    self.compile_statement(s, ctx, builder)?;
                }
                if !ctx.is_terminated {
                    builder.ins().jump(header_block, &[]);
                }
                
                builder.seal_block(header_block);
                builder.switch_to_block(after_block);
                builder.seal_block(after_block);
                ctx.is_terminated = false;
                Ok(())
            }
            
            // --- For range loop (for i in start..end step n) ---
            caria_parser::ast::Statement::ForRangeStmt(frs) => {
                // Initialize loop variable
                let start_val = self.compile_expression(&frs.start, ctx, builder)?;
                let end_val = self.compile_expression(&frs.end, ctx, builder)?;
                let step_val = if let Some(ref step_expr) = frs.step {
                    self.compile_expression(step_expr, ctx, builder)?
                } else {
                    builder.ins().iconst(types::I64, 1)
                };
                
                ctx.declare_and_def(&frs.var, start_val, builder);
                
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let after_block = builder.create_block();
                
                builder.ins().jump(header_block, &[]);
                
                builder.switch_to_block(header_block);
                ctx.is_terminated = false;
                
                let current = ctx.use_variable(&frs.var, builder)?;
                
                // Check condition: current < end
                let cond = builder.ins().icmp(IntCC::SignedLessThan, current, end_val);
                builder.ins().brif(cond, body_block, &[], after_block, &[]);
                
                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                ctx.is_terminated = false;
                let one = builder.ins().iconst(types::I64, 1);
                let _ = self.call_named_function("caria_gas_tick", &[one], builder);
                for s in &frs.body {
                    if ctx.is_terminated { break; }
                    self.compile_statement(s, ctx, builder)?;
                }
                
                if !ctx.is_terminated {
                    // Increment loop variable
                    let current = ctx.use_variable(&frs.var, builder)?;
                    let next = builder.ins().iadd(current, step_val);
                    ctx.declare_and_def(&frs.var, next, builder);
                    
                    builder.ins().jump(header_block, &[]);
                }
                
                builder.seal_block(header_block);
                builder.switch_to_block(after_block);
                builder.seal_block(after_block);
                ctx.is_terminated = false;
                Ok(())
            }

            // --- Unless statement (if !cond) ---
            caria_parser::ast::Statement::UnlessStmt(unless_stmt) => {
                let cond = self.compile_expression(&unless_stmt.condition, ctx, builder)?;
                let cond_bool = if builder.func.dfg.value_type(cond) == types::I64 {
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().icmp(IntCC::NotEqual, cond, zero)
                } else {
                    cond
                };
                
                let body_block = builder.create_block();
                let merge_block = builder.create_block();
                
                // brif cond, merge_block, body_block (execute body if cond is zero)
                builder.ins().brif(cond_bool, merge_block, &[], body_block, &[]);
                
                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                ctx.is_terminated = false;
                for s in &unless_stmt.body {
                    if ctx.is_terminated { break; }
                    self.compile_statement(s, ctx, builder)?;
                }
                let body_term = ctx.is_terminated;
                if !body_term {
                    builder.ins().jump(merge_block, &[]);
                }
                
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                ctx.is_terminated = false;
                Ok(())
            }

            // --- Guard statement (guard cond else { fallback }) ---
            caria_parser::ast::Statement::GuardStmt(guard_stmt) => {
                let cond = self.compile_expression(&guard_stmt.condition, ctx, builder)?;
                let cond_bool = if builder.func.dfg.value_type(cond) == types::I64 {
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().icmp(IntCC::NotEqual, cond, zero)
                } else {
                    cond
                };
                
                let proceed_block = builder.create_block();
                let fallback_block = builder.create_block();
                
                builder.ins().brif(cond_bool, proceed_block, &[], fallback_block, &[]);
                
                builder.switch_to_block(fallback_block);
                builder.seal_block(fallback_block);
                ctx.is_terminated = false;
                for s in &guard_stmt.fallback {
                    if ctx.is_terminated { break; }
                    self.compile_statement(s, ctx, builder)?;
                }
                let fallback_term = ctx.is_terminated;
                if !fallback_term {
                    builder.ins().jump(proceed_block, &[]);
                }
                
                builder.switch_to_block(proceed_block);
                builder.seal_block(proceed_block);
                ctx.is_terminated = false;
                Ok(())
            }

            // --- For-in statement (for item [, idx] in collection) ---
            caria_parser::ast::Statement::ForInStmt(for_in) => {
                let arr_ptr = self.compile_expression(&for_in.collection, ctx, builder)?;
                let len_val = self.call_named_function("caria_array_len", &[arr_ptr], builder)?;
                
                let loop_head = builder.create_block();
                let loop_body = builder.create_block();
                let after_block = builder.create_block();
                
                let zero = builder.ins().iconst(types::I64, 0);
                let counter_var = format!("__for_idx_{}", ctx.variables.len());
                ctx.declare_and_def(&counter_var, zero, builder);
                
                builder.ins().jump(loop_head, &[]);
                
                // Loop Head: current_i < len
                builder.switch_to_block(loop_head);
                ctx.is_terminated = false;
                let current_i = ctx.use_variable(&counter_var, builder)?;
                let cond = builder.ins().icmp(IntCC::SignedLessThan, current_i, len_val);
                builder.ins().brif(cond, loop_body, &[], after_block, &[]);
                
                // Loop Body
                builder.switch_to_block(loop_body);
                builder.seal_block(loop_body);
                ctx.is_terminated = false;
                let one = builder.ins().iconst(types::I64, 1);
                let _ = self.call_named_function("caria_gas_tick", &[one], builder);

                let item_val = self.call_named_function("caria_array_get", &[arr_ptr, current_i], builder)?;
                ctx.declare_and_def(&for_in.item_var, item_val, builder);
                if let Some(ref idx_name) = for_in.index_var {
                    ctx.declare_and_def(idx_name, current_i, builder);
                }
                
                for s in &for_in.body {
                    if ctx.is_terminated { break; }
                    self.compile_statement(s, ctx, builder)?;
                }
                
                if !ctx.is_terminated {
                    let next_i = builder.ins().iadd(current_i, one);
                    ctx.declare_and_def(&counter_var, next_i, builder);
                    builder.ins().jump(loop_head, &[]);
                }
                
                builder.seal_block(loop_head);
                builder.switch_to_block(after_block);
                builder.seal_block(after_block);
                ctx.is_terminated = false;
                Ok(())
            }

            // --- Null-assign statement (name ?= value) ---
            caria_parser::ast::Statement::NullAssignStmt { name, value } => {
                let current = ctx.use_variable(name, builder).ok();
                let zero = builder.ins().iconst(types::I64, 0);
                let should_assign = if let Some(cur_val) = current {
                    builder.ins().icmp(IntCC::Equal, cur_val, zero)
                } else {
                    builder.ins().iconst(types::I64, 1)
                };
                
                let assign_block = builder.create_block();
                let merge_block = builder.create_block();
                builder.ins().brif(should_assign, assign_block, &[], merge_block, &[]);
                
                builder.switch_to_block(assign_block);
                builder.seal_block(assign_block);
                let new_val = self.compile_expression(value, ctx, builder)?;
                ctx.declare_and_def(name, new_val, builder);
                builder.ins().jump(merge_block, &[]);
                
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                Ok(())
            }
            
            // --- Say statement ---
            caria_parser::ast::Statement::SayStmt(expr) => {
                let (fn_name, abi_type) = match expr {
                    caria_parser::ast::Expression::Literal(caria_parser::ast::Literal::Str(_)) => ("caria_say_str", types::I64),
                    caria_parser::ast::Expression::Literal(caria_parser::ast::Literal::Float(_)) => ("caria_say_float", types::F64),
                    caria_parser::ast::Expression::Literal(caria_parser::ast::Literal::Bool(_)) => ("caria_say_bool", types::I64),
                    _ => ("caria_say", types::I64),
                };

                // Evaluate the expression to get the value
                let val = self.compile_expression(expr, ctx, builder)?;
                
                // Declare target say function signature
                let mut sig = Signature::new(cranelift_codegen::isa::CallConv::SystemV);
                sig.params.push(AbiParam::new(abi_type));
                sig.returns.push(AbiParam::new(types::I64));
                
                // Import function
                let callee = match self.module_mut() {
                    CariaModule::Jit(jit) => {
                        jit.declare_function(fn_name, Linkage::Import, &sig)
                            .map_err(|e| format!("Failed to declare {}: {}", fn_name, e))?
                    }
                    CariaModule::Object(obj) => {
                        obj.declare_function(fn_name, Linkage::Import, &sig)
                            .map_err(|e| format!("Failed to declare {}: {}", fn_name, e))?
                    }
                };
                
                // Get local function reference for the call
                let local_callee = match self.module_mut() {
                    CariaModule::Jit(jit) => jit.declare_func_in_func(callee, builder.func),
                    CariaModule::Object(obj) => obj.declare_func_in_func(callee, builder.func),
                };
                
                // Call caria_say(val)
                builder.ins().call(local_callee, &[val]);
                Ok(())
            }
            
            caria_parser::ast::Statement::SayLevel { level, args } => {
                let lvl = level.to_uppercase();
                let lvl_lit = caria_parser::ast::Literal::Str(lvl);
                let lvl_val = self.compile_literal(&lvl_lit, ctx, builder)?;

                for arg in args {
                    let val = self.compile_expression(arg, ctx, builder)?;
                    let _ = self.call_named_function("caria_say_level", &[lvl_val, val], builder);
                }
                Ok(())
            }
            
            // --- Unsupported statements ---
            _ => Err(format!("Unsupported statement type: {:?}", stmt)),
        }
    }
    
    fn compile_expression(
        &mut self,
        expr: &caria_parser::ast::Expression,
        ctx: &mut CompileContext,
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::Value, String> {
        match expr {
            // --- Literals ---
            caria_parser::ast::Expression::Literal(lit) => {
                self.compile_literal(lit, ctx, builder)
            }
            
            // --- Identifier ---
            caria_parser::ast::Expression::Identifier(name) => {
                if let Ok(val) = ctx.use_variable(name, builder) {
                    Ok(val)
                } else if let Some(&func_id) = self.function_ids.get(name) {
                    let local_callee = match self.module_mut() {
                        CariaModule::Jit(jit) => jit.declare_func_in_func(func_id, builder.func),
                        CariaModule::Object(obj) => obj.declare_func_in_func(func_id, builder.func),
                    };
                    let func_addr = builder.ins().func_addr(types::I64, local_callee);
                    Ok(func_addr)
                } else {
                    ctx.use_variable(name, builder)
                }
            }
            
            // --- Binary operations ---
            caria_parser::ast::Expression::BinaryExpr { left, op, right } => {
                let left_val = self.compile_expression(left, ctx, builder)?;
                let right_val = self.compile_expression(right, ctx, builder)?;
                
                use caria_parser::ast::BinaryOp;
                match op {
                    BinaryOp::Add => {
                        let is_string_add = match (&**left, &**right) {
                            (caria_parser::ast::Expression::Literal(caria_parser::ast::Literal::Str(_)), _) => true,
                            (_, caria_parser::ast::Expression::Literal(caria_parser::ast::Literal::Str(_))) => true,
                            _ => false,
                        };
                        if is_string_add {
                            self.call_named_function("caria_string_concat", &[left_val, right_val], builder)
                        } else {
                            Ok(builder.ins().iadd(left_val, right_val))
                        }
                    }
                    BinaryOp::Subtract => Ok(builder.ins().isub(left_val, right_val)),
                    BinaryOp::Multiply => Ok(builder.ins().imul(left_val, right_val)),
                    BinaryOp::Divide => Ok(builder.ins().sdiv(left_val, right_val)),
                    BinaryOp::Modulo => Ok(builder.ins().srem(left_val, right_val)),
                    BinaryOp::Greater => Ok(builder.ins().icmp(IntCC::SignedGreaterThan, left_val, right_val)),
                    BinaryOp::Less => Ok(builder.ins().icmp(IntCC::SignedLessThan, left_val, right_val)),
                    BinaryOp::GreaterEq => Ok(builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, left_val, right_val)),
                    BinaryOp::LessEq => Ok(builder.ins().icmp(IntCC::SignedLessThanOrEqual, left_val, right_val)),
                    BinaryOp::Eq => {
                        let eq_val = self.call_named_function("caria_smart_eq", &[left_val, right_val], builder)?;
                        let zero = builder.ins().iconst(types::I64, 0);
                        Ok(builder.ins().icmp(IntCC::NotEqual, eq_val, zero))
                    }
                    BinaryOp::NotEq => {
                        let eq_val = self.call_named_function("caria_smart_eq", &[left_val, right_val], builder)?;
                        let zero = builder.ins().iconst(types::I64, 0);
                        Ok(builder.ins().icmp(IntCC::Equal, eq_val, zero))
                    }
                    BinaryOp::And => {
                        // Logical AND: both must be non-zero
                        let left_bool = if builder.func.dfg.value_type(left_val) == types::I64 {
                            let zero = builder.ins().iconst(types::I64, 0);
                            builder.ins().icmp(IntCC::NotEqual, left_val, zero)
                        } else {
                            left_val
                        };
                        let right_bool = if builder.func.dfg.value_type(right_val) == types::I64 {
                            let zero = builder.ins().iconst(types::I64, 0);
                            builder.ins().icmp(IntCC::NotEqual, right_val, zero)
                        } else {
                            right_val
                        };
                        Ok(builder.ins().band(left_bool, right_bool))
                    }
                    BinaryOp::Or => {
                        // Logical OR: at least one must be non-zero
                        let left_bool = if builder.func.dfg.value_type(left_val) == types::I64 {
                            let zero = builder.ins().iconst(types::I64, 0);
                            builder.ins().icmp(IntCC::NotEqual, left_val, zero)
                        } else {
                            left_val
                        };
                        let right_bool = if builder.func.dfg.value_type(right_val) == types::I64 {
                            let zero = builder.ins().iconst(types::I64, 0);
                            builder.ins().icmp(IntCC::NotEqual, right_val, zero)
                        } else {
                            right_val
                        };
                        Ok(builder.ins().bor(left_bool, right_bool))
                    }
                    BinaryOp::Coalesce => {
                        // Null coalesce: left != 0 ? left : right
                        let zero = builder.ins().iconst(types::I64, 0);
                        let is_null = builder.ins().icmp(IntCC::Equal, left_val, zero);
                        Ok(builder.ins().select(is_null, right_val, left_val))
                    }
                    _ => Err(format!("Unsupported binary operator: {:?}", op)),
                }
            }
            
            // --- Unary operations ---
            caria_parser::ast::Expression::UnaryExpr { op, operand } => {
                let val = self.compile_expression(operand, ctx, builder)?;
                use caria_parser::ast::UnaryOp;
                match op {
                    UnaryOp::Neg => Ok(builder.ins().ineg(val)),
                    UnaryOp::Not => {
                        let zero = builder.ins().iconst(types::I64, 0);
                        let val_i64 = if builder.func.dfg.value_type(val) != types::I64 {
                            builder.ins().uextend(types::I64, val)
                        } else {
                            val
                        };
                        Ok(builder.ins().icmp(IntCC::Equal, val_i64, zero))
                    }
                }
            }
            
            // --- Ternary expression ---
            caria_parser::ast::Expression::TernaryExpr { condition, true_expr, false_expr } => {
                let cond = self.compile_expression(condition, ctx, builder)?;
                let cond_b = if builder.func.dfg.value_type(cond) == types::I64 {
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().icmp(IntCC::NotEqual, cond, zero)
                } else {
                    cond
                };
                let true_val = self.compile_expression(true_expr, ctx, builder)?;
                let false_val = self.compile_expression(false_expr, ctx, builder)?;
                Ok(builder.ins().select(cond_b, true_val, false_val))
            }
            
            // --- If expression ---
            caria_parser::ast::Expression::IfExpr(if_expr) => {
                let cond = self.compile_expression(&if_expr.condition, ctx, builder)?;
                let cond_bool = if builder.func.dfg.value_type(cond) == types::I64 {
                    let zero = builder.ins().iconst(types::I64, 0);
                    builder.ins().icmp(IntCC::NotEqual, cond, zero)
                } else {
                    cond
                };
                
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                
                // Add a block parameter for the result value
                builder.append_block_param(merge_block, types::I64);
                
                builder.ins().brif(cond_bool, then_block, &[], else_block, &[]);
                
                // Then branch
                builder.switch_to_block(then_block);
                builder.seal_block(then_block);
                
                // Execute then body and get last expression value
                let mut then_val = builder.ins().iconst(types::I64, 0);
                for s in &if_expr.consequence {
                    if let caria_parser::ast::Statement::ExprStmt(e) = s {
                        then_val = self.compile_expression(e, ctx, builder)?;
                    } else {
                        self.compile_statement(s, ctx, builder)?;
                    }
                }
                builder.ins().jump(merge_block, &[then_val]);
                
                // Else branch
                builder.switch_to_block(else_block);
                builder.seal_block(else_block);
                
                let mut else_val = builder.ins().iconst(types::I64, 0);
                if let Some(ref alt) = if_expr.alternative {
                    for s in alt {
                        if let caria_parser::ast::Statement::ExprStmt(e) = s {
                            else_val = self.compile_expression(e, ctx, builder)?;
                        } else {
                            self.compile_statement(s, ctx, builder)?;
                        }
                    }
                }
                builder.ins().jump(merge_block, &[else_val]);
                
                // Merge point
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                
                Ok(builder.block_params(merge_block)[0])
            }
            
            // --- When expression (pattern matching) ---
            caria_parser::ast::Expression::WhenExpr { subject, arms } => {
                let subj_val = self.compile_expression(subject, ctx, builder)?;
                let merge_block = builder.create_block();
                builder.append_block_param(merge_block, types::I64);

                let mut next_check_block = builder.create_block();
                builder.ins().jump(next_check_block, &[]);

                for arm in arms {
                    let current_check_block = next_check_block;
                    builder.switch_to_block(current_check_block);
                    builder.seal_block(current_check_block);

                    let arm_body_block = builder.create_block();
                    next_check_block = builder.create_block();

                    match &arm.pattern {
                        caria_parser::ast::WhenPattern::Wildcard => {
                            // wildcard/else arm: unconditionally execute
                            builder.ins().jump(arm_body_block, &[]);
                        }
                        caria_parser::ast::WhenPattern::Literal(lit) => {
                            let lit_val = self.compile_literal(lit, ctx, builder)?;
                            let cond = builder.ins().icmp(IntCC::Equal, subj_val, lit_val);
                            builder.ins().brif(cond, arm_body_block, &[], next_check_block, &[]);
                        }
                        caria_parser::ast::WhenPattern::Range(start_lit, end_lit) => {
                            let start_val = self.compile_literal(start_lit, ctx, builder)?;
                            let end_val = self.compile_literal(end_lit, ctx, builder)?;
                            let gte = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, subj_val, start_val);
                            let lte = builder.ins().icmp(IntCC::SignedLessThanOrEqual, subj_val, end_val);
                            let in_range = builder.ins().band(gte, lte);
                            builder.ins().brif(in_range, arm_body_block, &[], next_check_block, &[]);
                        }
                        caria_parser::ast::WhenPattern::Identifier(name) => {
                            if let Ok(id_val) = ctx.use_variable(name, builder) {
                                let cond = builder.ins().icmp(IntCC::Equal, subj_val, id_val);
                                builder.ins().brif(cond, arm_body_block, &[], next_check_block, &[]);
                            } else {
                                builder.ins().jump(arm_body_block, &[]);
                            }
                        }
                    }

                    // Compile Arm body
                    builder.switch_to_block(arm_body_block);
                    builder.seal_block(arm_body_block);
                    let result_val = self.compile_expression(&arm.body, ctx, builder)?;
                    builder.ins().jump(merge_block, &[result_val]);
                }

                // If fallthrough with no matched arm, default to 0
                builder.switch_to_block(next_check_block);
                builder.seal_block(next_check_block);
                let default_val = builder.ins().iconst(types::I64, 0);
                builder.ins().jump(merge_block, &[default_val]);

                // Merge
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                Ok(builder.block_params(merge_block)[0])
            }
            
            // --- Array literals ---
            caria_parser::ast::Expression::ArrayLiteral(elements) => {
                let cap = builder.ins().iconst(types::I64, elements.len() as i64);
                let arr_ptr = self.call_named_function("caria_array_alloc", &[cap], builder)?;
                for el in elements {
                    let el_val = self.compile_expression(el, ctx, builder)?;
                    self.call_named_function("caria_array_push", &[arr_ptr, el_val], builder)?;
                }
                Ok(arr_ptr)
            }
            
            // --- Map literals ---
            caria_parser::ast::Expression::MapLiteral(entries) => {
                let map_ptr = self.call_named_function("caria_map_alloc", &[], builder)?;
                for (k, v) in entries {
                    let k_val = self.compile_expression(k, ctx, builder)?;
                    let v_val = self.compile_expression(v, ctx, builder)?;
                    self.call_named_function("caria_map_set", &[map_ptr, k_val, v_val], builder)?;
                }
                Ok(map_ptr)
            }

            // --- Index access: arr[i] or map[key] ---
            caria_parser::ast::Expression::IndexExpr { object, index } => {
                let obj_val = self.compile_expression(object, ctx, builder)?;
                let idx_val = self.compile_expression(index, ctx, builder)?;
                match &**index {
                    caria_parser::ast::Expression::Literal(caria_parser::ast::Literal::Str(_)) => {
                        self.call_named_function("caria_map_get", &[obj_val, idx_val], builder)
                    }
                    _ => {
                        self.call_named_function("caria_array_get", &[obj_val, idx_val], builder)
                    }
                }
            }

            // --- Member access: obj.field or arr.len ---
            caria_parser::ast::Expression::MemberAccess { object, member, .. } => {
                let obj_val = self.compile_expression(object, ctx, builder)?;
                if member == "len" || member == "length" {
                    self.call_named_function("caria_array_len", &[obj_val], builder)
                } else {
                    let key_val = self.compile_literal(&caria_parser::ast::Literal::Str(member.clone()), ctx, builder)?;
                    self.call_named_function("caria_map_get", &[obj_val, key_val], builder)
                }
            }

            // --- Function calls ---
            caria_parser::ast::Expression::CallExpr { callee, args } => {
                let mut arg_vals = Vec::new();
                for arg in args {
                    arg_vals.push(self.compile_expression(arg, ctx, builder)?);
                }
                
                if let caria_parser::ast::Expression::Identifier(ref name) = **callee {
                    self.call_named_function(name, &arg_vals, builder)
                } else if let caria_parser::ast::Expression::MemberAccess { ref member, .. } = **callee {
                    self.call_named_function(member, &arg_vals, builder)
                } else {
                    Err("Dynamic expression callees not supported".into())
                }
            }

            // --- Pipe: a |> b ---
            caria_parser::ast::Expression::PipeExpr { left, right } => {
                let left_val = self.compile_expression(left, ctx, builder)?;
                if let caria_parser::ast::Expression::Identifier(ref fn_name) = **right {
                    self.call_named_function(fn_name, &[left_val], builder)
                } else {
                    Err("Pipe target must be a callable identifier".into())
                }
            }

            // --- EnvExpr: env("KEY") ---
            caria_parser::ast::Expression::EnvExpr(key) => {
                let key_lit = caria_parser::ast::Literal::Str(key.clone());
                let key_val = self.compile_literal(&key_lit, ctx, builder)?;
                self.call_named_function("caria_env_get", &[key_val], builder)
            }
            
            // --- Unsupported expressions ---
            _ => Err(format!("Unsupported expression type: {:?}", expr)),
        }
    }
    
    fn compile_literal(
        &mut self,
        lit: &caria_parser::ast::Literal,
        _ctx: &mut CompileContext,
        builder: &mut FunctionBuilder,
    ) -> Result<cranelift_codegen::ir::Value, String> {
        match lit {
            caria_parser::ast::Literal::Int(n) => {
                Ok(builder.ins().iconst(types::I64, *n))
            }
            caria_parser::ast::Literal::Float(f) => {
                Ok(builder.ins().f64const(*f))
            }
            caria_parser::ast::Literal::Bool(b) => {
                Ok(builder.ins().iconst(types::I64, if *b { 1 } else { 0 }))
            }
            caria_parser::ast::Literal::Str(s) => {
                let data_id = if let Some(id) = self.string_data_ids.get(s) {
                    *id
                } else {
                    let mut desc = DataDescription::new();
                    let mut bytes = s.as_bytes().to_vec();
                    bytes.push(0);
                    desc.define(bytes.into_boxed_slice());
                    let name = format!("str_data_{}", self.string_data_ids.len());
                    let id = match self.module_mut() {
                        CariaModule::Jit(jit) => {
                            let id = jit.declare_data(&name, Linkage::Local, false, false)
                                .map_err(|e| format!("declare_data failed: {}", e))?;
                            jit.define_data(id, &desc)
                                .map_err(|e| format!("define_data failed: {}", e))?;
                            id
                        }
                        CariaModule::Object(obj) => {
                            let id = obj.declare_data(&name, Linkage::Local, false, false)
                                .map_err(|e| format!("declare_data failed: {}", e))?;
                            obj.define_data(id, &desc)
                                .map_err(|e| format!("define_data failed: {}", e))?;
                            id
                        }
                    };
                    self.string_data_ids.insert(s.clone(), id);
                    id
                };

                let local_data = match self.module_mut() {
                    CariaModule::Jit(jit) => jit.declare_data_in_func(data_id, builder.func),
                    CariaModule::Object(obj) => obj.declare_data_in_func(data_id, builder.func),
                };

                Ok(builder.ins().symbol_value(types::I64, local_data))
            }
            caria_parser::ast::Literal::Nil => {
                Ok(builder.ins().iconst(types::I64, 0))
            }
            caria_parser::ast::Literal::Unit(value, _unit) => {
                Ok(builder.ins().iconst(types::I64, *value))
            }
        }
    }
}
