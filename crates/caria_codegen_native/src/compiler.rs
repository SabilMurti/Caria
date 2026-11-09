//! Native Codegen Compiler - Cranelift JIT/Object Integration v0.114
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_object::{ObjectBuilder, ObjectModule};
use cranelift_module::{Module, Linkage, FuncId, DataDescription, DataId};
use cranelift_codegen::Context;
use cranelift_codegen::ir::{InstBuilder, AbiParam, Signature, UserFuncName};
use cranelift_codegen::ir::types;
use cranelift_codegen::settings::Configurable;
use cranelift_codegen::isa;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use std::collections::HashMap;
use cranelift_codegen::ir::condcodes::IntCC;

pub enum CariaModule {
    Jit(JITModule),
    Object(ObjectModule),
}

pub struct CariaCompiler {
    module: CariaModule,
    function_cache: HashMap<String, *const u8>,
    function_ids: HashMap<String, FuncId>,
    string_data_ids: HashMap<String, DataId>,
}

/// Compilation context tracking variables and constants during code generation
struct CompileContext {
    /// Current variables: name → IR value
    variables: HashMap<String, cranelift_codegen::ir::Value>,
    /// Interned strings: index → string content
    string_constants: Vec<String>,
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
            module: CariaModule::Jit(module),
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
            module: CariaModule::Object(module),
            function_cache: HashMap::new(),
            function_ids: HashMap::new(),
            string_data_ids: HashMap::new(),
        })
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
        add_symbol!("caria_time_now", caria_runtime::ffi_aot::caria_time_now);
        add_symbol!("caria_time_now_millis", caria_runtime::ffi_aot::caria_time_now_millis);

        // Database - SQLite
        add_symbol!("caria_sqlite_open", caria_runtime::ffi_aot::caria_sqlite_open);
        add_symbol!("caria_sqlite_close", caria_runtime::ffi_aot::caria_sqlite_close);
        add_symbol!("caria_sqlite_exec", caria_runtime::ffi_aot::caria_sqlite_exec);
        add_symbol!("caria_sqlite_query", caria_runtime::ffi_aot::caria_sqlite_query);

        // HTTP Client & Native Microserver Engine
        add_symbol!("caria_http_get", caria_runtime::ffi_aot::caria_http_get);
        add_symbol!("caria_http_post", caria_runtime::ffi_aot::caria_http_post);
        add_symbol!("caria_http_listen_and_serve", caria_runtime::ffi_aot::caria_http_listen_and_serve);
        
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

        Ok(())
    }

    pub fn compile_to_object(&mut self, prog: &caria_parser::ast::Program) -> Result<Vec<u8>, String> {
        // Compile the program first
        self.compile_program(prog)?;
        
        // For ObjectModule, we must consume it to get the product
        // We'll temporarily replace it with a dummy JIT module
        let triple = Self::default_target_triple();
        
        let isa = isa::lookup(triple)
            .map_err(|e| format!("Failed to create ISA: {}", e))?
            .finish(cranelift_codegen::settings::Flags::new(cranelift_codegen::settings::builder()))
            .map_err(|e| format!("Failed to finish ISA: {}", e))?;
        
        let dummy_jit = JITModule::new(JITBuilder::with_isa(isa, cranelift_module::default_libcall_names()));
        let module = std::mem::replace(&mut self.module, CariaModule::Jit(dummy_jit));
        
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
        
        // For ObjectModule, we must consume it to get the product
        let isa = {
            use target_lexicon::*;
            let triple = Triple {
                architecture: Architecture::X86_64,
                vendor: Vendor::Unknown,
                operating_system: OperatingSystem::Linux,
                environment: Environment::Gnu,
                binary_format: BinaryFormat::Elf,
            };
            
            let mut flag_builder = cranelift_codegen::settings::builder();
            flag_builder.set("use_colocated_libcalls", "false").map_err(|e| e.to_string())?;
            flag_builder.set("is_pic", "true").map_err(|e| e.to_string())?;
            
            isa::lookup(triple)
                .map_err(|e| format!("ISA lookup failed: {}", e))?
                .finish(cranelift_codegen::settings::Flags::new(flag_builder))
                .map_err(|e| format!("ISA finish failed: {}", e))?
        };
        
        let dummy_jit = JITModule::new(JITBuilder::with_isa(isa, cranelift_module::default_libcall_names()));
        let module = std::mem::replace(&mut self.module, CariaModule::Jit(dummy_jit));
        
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
        // Extract all function declarations (runes) from the program
        for stmt in &prog.statements {
            if let caria_parser::ast::Statement::RuneDecl(rune) = stmt {
                self.compile_rune(rune)?;
            }
        }
        
        // Finalize all functions
        match &mut self.module {
            CariaModule::Jit(jit) => {
                jit.finalize_definitions().map_err(|e| format!("JIT finalize failed: {}", e))?;
            }
            CariaModule::Object(_obj) => { 
                // Object finalization happens in finish()
            }
        }
        
        Ok(())
    }
    
    fn compile_rune(&mut self, rune: &caria_parser::ast::RuneDecl) -> Result<(), String> {
        // Create function signature
        let sig = self.create_signature(rune)?;
        
        // Declare the function in the module
        let func_id = match &mut self.module {
            CariaModule::Jit(jit) => {
                jit.declare_function(&rune.name, Linkage::Export, &sig)
                    .map_err(|e| format!("Failed to declare function {}: {}", rune.name, e))?
            }
            CariaModule::Object(obj) => {
                obj.declare_function(&rune.name, Linkage::Export, &sig)
                    .map_err(|e| format!("Failed to declare function {}: {}", rune.name, e))?
            }
        };
        
        // Store the function ID for later lookup
        self.function_ids.insert(rune.name.clone(), func_id);
        
        // Define the function body
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
            
            // Compile function body - extract statements from RuneBody
            let statements = match &rune.body {
                caria_parser::ast::RuneBody::Block(stmts) => stmts,
                caria_parser::ast::RuneBody::Inline(expr) => {
                    // For inline runes, evaluate expression and return it
                    let mut compile_ctx = CompileContext {
                        variables: HashMap::new(),
                        string_constants: Vec::new(),
                    };
                    
                    match self.compile_expression(expr, &mut compile_ctx, &mut builder) {
                        Ok(val) => {
                            // Convert I64 to I32 for return (signature expects I32)
                            let ret_val = builder.ins().ireduce(types::I32, val);
                            builder.ins().return_(&[ret_val]);
                            builder.finalize();
                            return Ok(());
                        }
                        Err(e) => return Err(format!("Failed to compile inline expression: {}", e)),
                    }
                }
            };
            
            let mut compile_ctx = CompileContext {
                variables: HashMap::new(),
                string_constants: Vec::new(),
            };
            
            // Compile each statement in the function body
            let mut has_return = false;
            for stmt in statements {
                if matches!(stmt, caria_parser::ast::Statement::RetStmt(_)) {
                    has_return = true;
                }
                self.compile_statement(stmt, &mut compile_ctx, &mut builder)?;
            }
            
            // If no explicit return, add implicit return 0
            if !has_return {
                let zero = builder.ins().iconst(types::I32, 0);
                builder.ins().return_(&[zero]);
            }
            
            builder.finalize();
        }
        
        // Define the function in the module
        match &mut self.module {
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
        let _ptr_type = self.get_pointer_type();
        let mut sig = Signature::new(cranelift_codegen::isa::CallConv::SystemV);
        
        // Add parameters
        for _param in &rune.params {
            sig.params.push(AbiParam::new(types::I64));
        }
        
        // Add return type (default to i32 for now)
        sig.returns.push(AbiParam::new(types::I32));
        
        Ok(sig)
    }
    
    fn get_pointer_type(&self) -> cranelift_codegen::ir::Type {
        match &self.module {
            CariaModule::Jit(jit) => jit.target_config().pointer_type(),
            CariaModule::Object(obj) => obj.target_config().pointer_type(),
        }
    }
    
    fn get_function_pointer(&mut self, name: &str) -> Result<*const u8, String> {
        // Check cache first
        if let Some(&ptr) = self.function_cache.get(name) {
            return Ok(ptr);
        }
        
        match &mut self.module {
            CariaModule::Jit(jit) => {
                // Look up the function ID from our tracking map
                let func_id = self.function_ids.get(name)
                    .ok_or_else(|| format!("Function '{}' not found", name))?;
                
                // Get code pointer from JIT module
                let ptr = jit.get_finalized_function(*func_id);
                
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
        match stmt {
            // --- Variable declarations ---
            caria_parser::ast::Statement::ConstDecl(cd) => {
                let val = self.compile_expression(&cd.value, ctx, builder)?;
                ctx.variables.insert(cd.name.clone(), val);
                Ok(())
            }
            
            caria_parser::ast::Statement::MutDecl(md) => {
                let val = self.compile_expression(&md.value, ctx, builder)?;
                ctx.variables.insert(md.name.clone(), val);
                Ok(())
            }
            
            // --- Assignment ---
            caria_parser::ast::Statement::AssignStmt(ast) => {
                let val = self.compile_expression(&ast.value, ctx, builder)?;
                ctx.variables.insert(ast.name.clone(), val);
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
                // Convert I64 to I32 for return (signature expects I32)
                let ret_val = builder.ins().ireduce(types::I32, val);
                builder.ins().return_(&[ret_val]);
                Ok(())
            }
            
            // --- If statement ---
            caria_parser::ast::Statement::IfStmt(if_expr) => {
                let cond = self.compile_expression(&if_expr.condition, ctx, builder)?;
                
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                
                builder.ins().brif(cond, then_block, &[], else_block, &[]);
                
                // Then branch
                builder.switch_to_block(then_block);
                builder.seal_block(then_block);
                for s in &if_expr.consequence {
                    self.compile_statement(s, ctx, builder)?;
                }
                builder.ins().jump(merge_block, &[]);
                
                // Else branch
                builder.switch_to_block(else_block);
                builder.seal_block(else_block);
                if let Some(ref alt) = if_expr.alternative {
                    for s in alt {
                        self.compile_statement(s, ctx, builder)?;
                    }
                }
                builder.ins().jump(merge_block, &[]);
                
                // Merge point
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                Ok(())
            }
            
            // --- While loop ---
            caria_parser::ast::Statement::WhileStmt(ws) => {
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let after_block = builder.create_block();
                
                builder.ins().jump(header_block, &[]);
                
                builder.switch_to_block(header_block);
                builder.seal_block(header_block);
                let cond = self.compile_expression(&ws.condition, ctx, builder)?;
                builder.ins().brif(cond, body_block, &[], after_block, &[]);
                
                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                for s in &ws.body {
                    self.compile_statement(s, ctx, builder)?;
                }
                builder.ins().jump(header_block, &[]);
                
                builder.switch_to_block(after_block);
                builder.seal_block(after_block);
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
                
                ctx.variables.insert(frs.var.clone(), start_val);
                
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let after_block = builder.create_block();
                
                builder.ins().jump(header_block, &[]);
                
                builder.switch_to_block(header_block);
                builder.seal_block(header_block);
                
                let current = ctx.variables.get(&frs.var).cloned()
                    .ok_or_else(|| format!("Loop variable {} not found", frs.var))?;
                
                // Check condition: current < end
                let cond = builder.ins().icmp(IntCC::SignedLessThan, current, end_val);
                builder.ins().brif(cond, body_block, &[], after_block, &[]);
                
                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                for s in &frs.body {
                    self.compile_statement(s, ctx, builder)?;
                }
                
                // Increment loop variable
                let current = ctx.variables.get(&frs.var).cloned()
                    .ok_or_else(|| format!("Loop variable {} not found", frs.var))?;
                let next = builder.ins().iadd(current, step_val);
                ctx.variables.insert(frs.var.clone(), next);
                
                builder.ins().jump(header_block, &[]);
                
                builder.switch_to_block(after_block);
                builder.seal_block(after_block);
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
                let callee = match &mut self.module {
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
                let local_callee = match &mut self.module {
                    CariaModule::Jit(jit) => jit.declare_func_in_func(callee, builder.func),
                    CariaModule::Object(obj) => obj.declare_func_in_func(callee, builder.func),
                };
                
                // Call caria_say(val)
                builder.ins().call(local_callee, &[val]);
                Ok(())
            }
            
            caria_parser::ast::Statement::SayLevel { level: _, args: _ } => {
                // For MVP, say.level statement is a no-op
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
                ctx.variables.get(name)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable: {}", name))
            }
            
            // --- Binary operations ---
            caria_parser::ast::Expression::BinaryExpr { left, op, right } => {
                let left_val = self.compile_expression(left, ctx, builder)?;
                let right_val = self.compile_expression(right, ctx, builder)?;
                
                use caria_parser::ast::BinaryOp;
                match op {
                    BinaryOp::Add => Ok(builder.ins().iadd(left_val, right_val)),
                    BinaryOp::Subtract => Ok(builder.ins().isub(left_val, right_val)),
                    BinaryOp::Multiply => Ok(builder.ins().imul(left_val, right_val)),
                    BinaryOp::Divide => Ok(builder.ins().sdiv(left_val, right_val)),
                    BinaryOp::Modulo => Ok(builder.ins().srem(left_val, right_val)),
                    BinaryOp::Greater => Ok(builder.ins().icmp(IntCC::SignedGreaterThan, left_val, right_val)),
                    BinaryOp::Less => Ok(builder.ins().icmp(IntCC::SignedLessThan, left_val, right_val)),
                    BinaryOp::GreaterEq => Ok(builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, left_val, right_val)),
                    BinaryOp::LessEq => Ok(builder.ins().icmp(IntCC::SignedLessThanOrEqual, left_val, right_val)),
                    BinaryOp::Eq => Ok(builder.ins().icmp(IntCC::Equal, left_val, right_val)),
                    BinaryOp::NotEq => Ok(builder.ins().icmp(IntCC::NotEqual, left_val, right_val)),
                    BinaryOp::And => {
                        // Logical AND: both must be non-zero
                        let zero = builder.ins().iconst(types::I64, 0);
                        let left_bool = builder.ins().icmp(IntCC::NotEqual, left_val, zero);
                        let right_bool = builder.ins().icmp(IntCC::NotEqual, right_val, zero);
                        Ok(builder.ins().band(left_bool, right_bool))
                    }
                    BinaryOp::Or => {
                        // Logical OR: at least one must be non-zero
                        let zero = builder.ins().iconst(types::I64, 0);
                        let left_bool = builder.ins().icmp(IntCC::NotEqual, left_val, zero);
                        let right_bool = builder.ins().icmp(IntCC::NotEqual, right_val, zero);
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
                        Ok(builder.ins().icmp(IntCC::Equal, val, zero))
                    }
                }
            }
            
            // --- Ternary expression ---
            caria_parser::ast::Expression::TernaryExpr { condition, true_expr, false_expr } => {
                let cond = self.compile_expression(condition, ctx, builder)?;
                let true_val = self.compile_expression(true_expr, ctx, builder)?;
                let false_val = self.compile_expression(false_expr, ctx, builder)?;
                Ok(builder.ins().select(cond, true_val, false_val))
            }
            
            // --- If expression ---
            caria_parser::ast::Expression::IfExpr(if_expr) => {
                let cond = self.compile_expression(&if_expr.condition, ctx, builder)?;
                
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                
                // Add a block parameter for the result value
                builder.append_block_param(merge_block, types::I64);
                
                builder.ins().brif(cond, then_block, &[], else_block, &[]);
                
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
            
            // --- Array/Map literals (simplified) ---
            caria_parser::ast::Expression::ArrayLiteral(_) => {
                // For MVP: return 0 (no real array support yet)
                Ok(builder.ins().iconst(types::I64, 0))
            }
            
            caria_parser::ast::Expression::MapLiteral(_) => {
                // For MVP: return 0 (no real map support yet)
                Ok(builder.ins().iconst(types::I64, 0))
            }
            
            // --- Function calls ---
            caria_parser::ast::Expression::CallExpr { callee: _, args: _ } => {
                // For MVP: function calls not yet supported
                Err("Function calls not yet implemented".into())
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
                    let id = match &mut self.module {
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

                let local_data = match &mut self.module {
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
