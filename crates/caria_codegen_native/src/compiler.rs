use caria_parser::ast::{
    self, Program, Statement, Expression, Literal, BinaryOp, UnaryOp, RuneDecl, RuneBody, TypeExpr
};
use cranelift_codegen::ir::{
    self, types, AbiParam, Signature, Function, Value, InstBuilder
};
use cranelift_codegen::isa::{self, TargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{Module, Linkage, FuncId, DataId, DataDescription};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;

pub enum BackendModule {
    JIT(JITModule),
    Object(ObjectModule),
}

impl BackendModule {
    pub fn declare_function(
        &mut self,
        name: &str,
        linkage: Linkage,
        signature: &Signature,
    ) -> Result<FuncId, String> {
        match self {
            BackendModule::JIT(m) => m.declare_function(name, linkage, signature).map_err(|e| e.to_string()),
            BackendModule::Object(m) => m.declare_function(name, linkage, signature).map_err(|e| e.to_string()),
        }
    }

    pub fn define_function(
        &mut self,
        func_id: FuncId,
        ctx: &mut cranelift_codegen::Context,
    ) -> Result<(), String> {
        let res = match self {
            BackendModule::JIT(m) => m.define_function(func_id, ctx),
            BackendModule::Object(m) => m.define_function(func_id, ctx),
        };
        if let Err(ref e) = res {
            eprintln!("Cranelift define_function failed for function {}: {}", func_id, e);
            eprintln!("Function IR:\n{}", ctx.func);
        }
        res.map_err(|e| e.to_string())
    }

    pub fn declare_data(
        &mut self,
        name: &str,
        linkage: Linkage,
        writable: bool,
        tls: bool,
    ) -> Result<DataId, String> {
        match self {
            BackendModule::JIT(m) => m.declare_data(name, linkage, writable, tls).map_err(|e| e.to_string()),
            BackendModule::Object(m) => m.declare_data(name, linkage, writable, tls).map_err(|e| e.to_string()),
        }
    }

    pub fn define_data(&mut self, data_id: DataId, desc: &DataDescription) -> Result<(), String> {
        match self {
            BackendModule::JIT(m) => m.define_data(data_id, desc).map_err(|e| e.to_string()),
            BackendModule::Object(m) => m.define_data(data_id, desc).map_err(|e| e.to_string()),
        }
    }

    pub fn declare_func_in_func(&mut self, func_id: FuncId, func: &mut Function) -> ir::FuncRef {
        match self {
            BackendModule::JIT(ref mut m) => m.declare_func_in_func(func_id, func),
            BackendModule::Object(ref mut m) => m.declare_func_in_func(func_id, func),
        }
    }

    pub fn declare_data_in_func(&mut self, data_id: DataId, func: &mut Function) -> ir::GlobalValue {
        match self {
            BackendModule::JIT(ref mut m) => m.declare_data_in_func(data_id, func),
            BackendModule::Object(ref mut m) => m.declare_data_in_func(data_id, func),
        }
    }

    pub fn make_signature(&self) -> Signature {
        match self {
            BackendModule::JIT(m) => m.make_signature(),
            BackendModule::Object(m) => m.make_signature(),
        }
    }

    pub fn isa(&self) -> &dyn TargetIsa {
        match self {
            BackendModule::JIT(m) => m.isa(),
            BackendModule::Object(m) => m.isa(),
        }
    }
}

pub struct RouteRegistration {
    method: String,
    path: String,
    func_id: FuncId,
}

pub struct CariaCompiler {
    module: Option<BackendModule>,
    fn_builder_ctx: FunctionBuilderContext,
    ctx: cranelift_codegen::Context,
    // Map function/route names to their FuncId and signatures
    functions: HashMap<String, (FuncId, Signature)>,
    // String literal counter
    str_count: usize,
    // Native runtime helpers
    runtime_helpers: HashMap<String, (FuncId, Signature)>,
    // Registered HTTP routes
    routes: Vec<RouteRegistration>,
}

fn translate_type(type_expr: &TypeExpr) -> ir::Type {
    match type_expr {
        TypeExpr::Named(name) => match name.as_str() {
            "int" => types::I64,
            "flag" => types::I8,
            "txt" => types::I64,
            _ => types::I64,
        },
        _ => types::I64,
    }
}

impl CariaCompiler {
    pub fn new_jit() -> Result<Self, String> {
        let isa_builder = isa::lookup(target_lexicon::Triple::host()).map_err(|e| e.to_string())?;
        let mut flags_builder = settings::builder();
        flags_builder.set("use_colocated_libcalls", "false").unwrap();
        flags_builder.set("is_pic", "false").unwrap();
        let isa = isa_builder.finish(settings::Flags::new(flags_builder)).map_err(|e| e.to_string())?;
        
        let mut jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        
        // Register symbols to make them available in memory during execution
        jit_builder.symbol("caria_say_int", crate::runtime::caria_say_int as *const u8);
        jit_builder.symbol("caria_say_txt", crate::runtime::caria_say_txt as *const u8);
        jit_builder.symbol("caria_get_env_int", crate::runtime::caria_get_env_int as *const u8);
        jit_builder.symbol("caria_get_env_txt", crate::runtime::caria_get_env_txt as *const u8);
        jit_builder.symbol("caria_str_concat", crate::runtime::caria_str_concat as *const u8);
        jit_builder.symbol("caria_arg_count", crate::runtime::caria_arg_count as *const u8);
        jit_builder.symbol("caria_arg_get", crate::runtime::caria_arg_get as *const u8);
        jit_builder.symbol("caria_write_file", crate::runtime::caria_write_file as *const u8);
        jit_builder.symbol("caria_create_dir", crate::runtime::caria_create_dir as *const u8);
        jit_builder.symbol("caria_run_cmd", crate::runtime::caria_run_cmd as *const u8);

        let mut module = JITModule::new(jit_builder);

        // Declare signatures of runtime helpers
        let mut runtime_helpers = HashMap::new();
        
        let mut sig_int = module.make_signature();
        sig_int.params.push(AbiParam::new(types::I64));
        let id_int = module
            .declare_function("caria_say_int", Linkage::Import, &sig_int)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("caria_say_int".to_string(), (id_int, sig_int.clone()));

        let mut sig_txt = module.make_signature();
        sig_txt.params.push(AbiParam::new(types::I64)); // pointer to str bytes
        sig_txt.params.push(AbiParam::new(types::I64)); // length of str
        let id_txt = module
            .declare_function("caria_say_txt", Linkage::Import, &sig_txt)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("caria_say_txt".to_string(), (id_txt, sig_txt));

        let mut sig_env = module.make_signature();
        sig_env.params.push(AbiParam::new(types::I64));
        sig_env.returns.push(AbiParam::new(types::I64));
        let id_env = module
            .declare_function("caria_get_env_int", Linkage::Import, &sig_env)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("get_env_int".to_string(), (id_env, sig_env.clone()));
        runtime_helpers.insert("caria_get_env_int".to_string(), (id_env, sig_env));

        let mut sig_env_txt = module.make_signature();
        sig_env_txt.params.push(AbiParam::new(types::I64));
        sig_env_txt.returns.push(AbiParam::new(types::I64));
        let id_env_txt = module
            .declare_function("caria_get_env_txt", Linkage::Import, &sig_env_txt)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("caria_get_env_txt".to_string(), (id_env_txt, sig_env_txt));

        let mut sig_time = module.make_signature();
        sig_time.returns.push(AbiParam::new(types::I64));
        let id_time = module
            .declare_function("caria_time_now_ms", Linkage::Import, &sig_time)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("time_now_ms".to_string(), (id_time, sig_time.clone()));
        runtime_helpers.insert("time_ms".to_string(), (id_time, sig_time));



        let mut sig_concat = module.make_signature();
        sig_concat.params.push(AbiParam::new(types::I64));
        sig_concat.params.push(AbiParam::new(types::I64));
        sig_concat.returns.push(AbiParam::new(types::I64));
        let id_concat = module
            .declare_function("caria_str_concat", Linkage::Import, &sig_concat)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("caria_str_concat".to_string(), (id_concat, sig_concat));

        let mut sig_args_cnt = module.make_signature();
        sig_args_cnt.returns.push(AbiParam::new(types::I64));
        let id_args_cnt = module
            .declare_function("caria_arg_count", Linkage::Import, &sig_args_cnt)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("args_count".to_string(), (id_args_cnt, sig_args_cnt));

        let mut sig_arg = module.make_signature();
        sig_arg.params.push(AbiParam::new(types::I64));
        sig_arg.returns.push(AbiParam::new(types::I64));
        let id_arg = module
            .declare_function("caria_arg_get", Linkage::Import, &sig_arg)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("arg".to_string(), (id_arg, sig_arg));

        let mut sig_write = module.make_signature();
        sig_write.params.push(AbiParam::new(types::I64));
        sig_write.params.push(AbiParam::new(types::I64));
        let id_write = module
            .declare_function("caria_write_file", Linkage::Import, &sig_write)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("write_file".to_string(), (id_write, sig_write));

        let mut sig_mkdir = module.make_signature();
        sig_mkdir.params.push(AbiParam::new(types::I64));
        let id_mkdir = module
            .declare_function("caria_create_dir", Linkage::Import, &sig_mkdir)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("create_dir".to_string(), (id_mkdir, sig_mkdir));

        let mut sig_run_cmd = module.make_signature();
        sig_run_cmd.params.push(AbiParam::new(types::I64));
        sig_run_cmd.returns.push(AbiParam::new(types::I64));
        let id_run_cmd = module
            .declare_function("caria_run_cmd", Linkage::Import, &sig_run_cmd)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("run_cmd".to_string(), (id_run_cmd, sig_run_cmd));

        let mut sig_view = module.make_signature();
        sig_view.params.push(AbiParam::new(types::I64));
        sig_view.returns.push(AbiParam::new(types::I64));
        let id_view = module
            .declare_function("caria_view", Linkage::Import, &sig_view)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("view".to_string(), (id_view, sig_view));


        Ok(Self {
            module: Some(BackendModule::JIT(module)),
            fn_builder_ctx: FunctionBuilderContext::new(),
            ctx: cranelift_codegen::Context::new(),
            functions: HashMap::new(),
            str_count: 0,
            runtime_helpers,
            routes: Vec::new(),
        })
    }

    pub fn new_object() -> Result<Self, String> {
        let isa_builder = isa::lookup(target_lexicon::Triple::host()).map_err(|e| e.to_string())?;
        let mut flags_builder = settings::builder();
        flags_builder.set("is_pic", "false").unwrap();
        let flags = settings::Flags::new(flags_builder);
        let isa = isa_builder.finish(flags).map_err(|e| e.to_string())?;
        
        let obj_builder = ObjectBuilder::new(
            isa,
            "caria_module",
            cranelift_module::default_libcall_names(),
        ).map_err(|e| e.to_string())?;

        let mut module = ObjectModule::new(obj_builder);

        let mut runtime_helpers = HashMap::new();
        
        let mut sig_int = module.make_signature();
        sig_int.params.push(AbiParam::new(types::I64));
        let id_int = module
            .declare_function("caria_say_int", Linkage::Import, &sig_int)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("caria_say_int".to_string(), (id_int, sig_int.clone()));

        let mut sig_txt = module.make_signature();
        sig_txt.params.push(AbiParam::new(types::I64));
        sig_txt.params.push(AbiParam::new(types::I64));
        let id_txt = module
            .declare_function("caria_say_txt", Linkage::Import, &sig_txt)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("caria_say_txt".to_string(), (id_txt, sig_txt));

        let mut sig_env = module.make_signature();
        sig_env.params.push(AbiParam::new(types::I64));
        sig_env.returns.push(AbiParam::new(types::I64));
        let id_env = module
            .declare_function("caria_get_env_int", Linkage::Import, &sig_env)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("get_env_int".to_string(), (id_env, sig_env.clone()));
        runtime_helpers.insert("caria_get_env_int".to_string(), (id_env, sig_env));

        let mut sig_env_txt = module.make_signature();
        sig_env_txt.params.push(AbiParam::new(types::I64));
        sig_env_txt.returns.push(AbiParam::new(types::I64));
        let id_env_txt = module
            .declare_function("caria_get_env_txt", Linkage::Import, &sig_env_txt)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("caria_get_env_txt".to_string(), (id_env_txt, sig_env_txt));

        let mut sig_time = module.make_signature();
        sig_time.returns.push(AbiParam::new(types::I64));
        let id_time = module
            .declare_function("caria_time_now_ms", Linkage::Import, &sig_time)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("time_now_ms".to_string(), (id_time, sig_time.clone()));
        runtime_helpers.insert("time_ms".to_string(), (id_time, sig_time));

        let mut sig_reg = module.make_signature();
        sig_reg.params.push(AbiParam::new(types::I64));
        sig_reg.params.push(AbiParam::new(types::I64));
        sig_reg.params.push(AbiParam::new(types::I64));
        let id_reg = module
            .declare_function("caria_register_route", Linkage::Import, &sig_reg)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("caria_register_route".to_string(), (id_reg, sig_reg));

        let mut sig_start = module.make_signature();
        sig_start.params.push(AbiParam::new(types::I64));
        let id_start = module
            .declare_function("caria_start_server", Linkage::Import, &sig_start)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("caria_start_server".to_string(), (id_start, sig_start));

        let mut sig_concat = module.make_signature();
        sig_concat.params.push(AbiParam::new(types::I64));
        sig_concat.params.push(AbiParam::new(types::I64));
        sig_concat.returns.push(AbiParam::new(types::I64));
        let id_concat = module
            .declare_function("caria_str_concat", Linkage::Import, &sig_concat)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("caria_str_concat".to_string(), (id_concat, sig_concat));

        let mut sig_args_cnt = module.make_signature();
        sig_args_cnt.returns.push(AbiParam::new(types::I64));
        let id_args_cnt = module
            .declare_function("caria_arg_count", Linkage::Import, &sig_args_cnt)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("args_count".to_string(), (id_args_cnt, sig_args_cnt));

        let mut sig_arg = module.make_signature();
        sig_arg.params.push(AbiParam::new(types::I64));
        sig_arg.returns.push(AbiParam::new(types::I64));
        let id_arg = module
            .declare_function("caria_arg_get", Linkage::Import, &sig_arg)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("arg".to_string(), (id_arg, sig_arg));

        let mut sig_write = module.make_signature();
        sig_write.params.push(AbiParam::new(types::I64));
        sig_write.params.push(AbiParam::new(types::I64));
        let id_write = module
            .declare_function("caria_write_file", Linkage::Import, &sig_write)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("write_file".to_string(), (id_write, sig_write));

        let mut sig_mkdir = module.make_signature();
        sig_mkdir.params.push(AbiParam::new(types::I64));
        let id_mkdir = module
            .declare_function("caria_create_dir", Linkage::Import, &sig_mkdir)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("create_dir".to_string(), (id_mkdir, sig_mkdir));

        let mut sig_run_cmd = module.make_signature();
        sig_run_cmd.params.push(AbiParam::new(types::I64));
        sig_run_cmd.returns.push(AbiParam::new(types::I64));
        let id_run_cmd = module
            .declare_function("caria_run_cmd", Linkage::Import, &sig_run_cmd)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("run_cmd".to_string(), (id_run_cmd, sig_run_cmd));

        let mut sig_view = module.make_signature();
        sig_view.params.push(AbiParam::new(types::I64));
        sig_view.returns.push(AbiParam::new(types::I64));
        let id_view = module
            .declare_function("caria_view", Linkage::Import, &sig_view)
            .map_err(|e| e.to_string())?;
        runtime_helpers.insert("view".to_string(), (id_view, sig_view));


        Ok(Self {
            module: Some(BackendModule::Object(module)),
            fn_builder_ctx: FunctionBuilderContext::new(),
            ctx: cranelift_codegen::Context::new(),
            functions: HashMap::new(),
            str_count: 0,
            runtime_helpers,
            routes: Vec::new(),
        })
    }

    pub fn compile_to_jit(&mut self, program: &Program) -> Result<*const u8, String> {
        self.compile_and_get_function(program, "main")
    }

    pub fn compile_and_get_function(&mut self, program: &Program, name: &str) -> Result<*const u8, String> {
        self.compile_all(program)?;
        
        if let Some(BackendModule::JIT(ref mut m)) = self.module {
            m.finalize_definitions().map_err(|e| e.to_string())?;
            if let Some(&(func_id, _)) = self.functions.get(name) {
                let code_ptr = m.get_finalized_function(func_id);
                Ok(code_ptr)
            } else {
                Err(format!("No function named '{}' defined in the code", name))
            }
        } else {
            Err("JIT compiler module not initialized or already finished".to_string())
        }
    }

    pub fn compile_to_object(&mut self, program: &Program) -> Result<Vec<u8>, String> {
        self.compile_all(program)?;
        
        if let Some(BackendModule::Object(m)) = self.module.take() {
            let product = m.finish();
            let bytes = product.emit().map_err(|e| e.to_string())?;
            Ok(bytes)
        } else {
            Err("Object compiler module not initialized or already finished".to_string())
        }
    }

    /// Flatten RouteGroup (prefix injection) and ResourceRoute (7 CRUD routes) into plain
    /// RouteDecl nodes. Returns a new Statement list ready for the two-phase compile.
    fn flatten_statements(&self, stmts: &[Statement], prefix: &str) -> Vec<Statement> {
        let mut out = Vec::new();
        for stmt in stmts {
            match stmt {
                // Pass-through: already flat
                Statement::RouteDecl(route) => {
                    let mut r = route.clone();
                    r.path = format!("{}{}", prefix, route.path);
                    out.push(Statement::RouteDecl(r));
                }
                // Route group: recurse with combined prefix
                Statement::RouteGroup(group) => {
                    let new_prefix = format!("{}{}", prefix, group.prefix);
                    let nested = self.flatten_statements(&group.body, &new_prefix);
                    out.extend(nested);
                }
                // Resource route: expand into up to 7 CRUD routes
                Statement::ResourceRoute(res) => {
                    let base = format!("{}{}", prefix, res.path);
                    let ctrl = &res.controller;
                    // All 7 REST conventions:
                    let all_actions: &[(&str, &str, &str)] = &[
                        ("GET",    &base,               "index"),
                        ("POST",   &base,               "store"),
                        ("GET",    &format!("{base}/{{id}}"),  "show"),
                        ("PUT",    &format!("{base}/{{id}}"),  "update"),
                        ("PATCH",  &format!("{base}/{{id}}"),  "patch"),
                        ("DELETE", &format!("{base}/{{id}}"),  "destroy"),
                        ("GET",    &format!("{base}/create"),  "create"),
                    ];
                    for (method_s, path, action) in all_actions {
                        // Apply `only` / `except` filters
                        if let Some(ref only) = res.only {
                            if !only.iter().any(|a| a == action) { continue; }
                        }
                        if let Some(ref except) = res.except {
                            if except.iter().any(|a| a == action) { continue; }
                        }
                        let http_method = match *method_s {
                            "GET"    => ast::HttpMethod::Get,
                            "POST"   => ast::HttpMethod::Post,
                            "PUT"    => ast::HttpMethod::Put,
                            "PATCH"  => ast::HttpMethod::Patch,
                            "DELETE" => ast::HttpMethod::Delete,
                            _ => unreachable!(),
                        };
                        // Body: call ctrl.action()
                        let call_expr = ast::Expression::CallExpr {
                            callee: Box::new(ast::Expression::MemberAccess {
                                object: Box::new(ast::Expression::Identifier(ctrl.clone())),
                                member: action.to_string(),
                                is_safe: false,
                            }),
                            args: vec![],
                        };
                        out.push(Statement::RouteDecl(ast::RouteDecl {
                            method: http_method,
                            path: path.to_string(),
                            body: vec![Statement::RetStmt(call_expr)],
                            decorators: vec![],
                            with_clauses: vec![],
                        }));
                    }
                }
                // Everything else passes through unchanged
                other => out.push(other.clone()),
            }
        }
        out
    }

    fn compile_all(&mut self, program: &Program) -> Result<(), String> {
        // Pre-pass: flatten RouteGroup and ResourceRoute into plain RouteDecls
        // Must happen before the mutable borrow of self.module
        let flat = self.flatten_statements(&program.statements, "");

        let mod_ref = self.module.as_mut().ok_or("Module not initialized")?;

        // Phase 1: Declare all functions (runes & routes)
        for stmt in &flat {
            match stmt {
                Statement::RuneDecl(rune) => {
                    let mut sig = mod_ref.make_signature();
                    for param in &rune.params {
                        sig.params.push(AbiParam::new(translate_type(&param.type_expr)));
                    }
                    if let Some(ref ret) = rune.return_type {
                        sig.returns.push(AbiParam::new(translate_type(ret)));
                    } else if rune.name == "main" {
                        sig.returns.push(AbiParam::new(types::I32));
                    }
                    let linkage = if rune.name == "main" {
                        Linkage::Export
                    } else {
                        Linkage::Local
                    };
                    let func_id = mod_ref.declare_function(&rune.name, linkage, &sig)?;
                    self.functions.insert(rune.name.clone(), (func_id, sig));
                }
                Statement::RouteDecl(route) => {
                    let method_str = match route.method {
                        ast::HttpMethod::Get => "GET",
                        ast::HttpMethod::Post => "POST",
                        ast::HttpMethod::Put => "PUT",
                        ast::HttpMethod::Patch => "PATCH",
                        ast::HttpMethod::Delete => "DELETE",
                    };
                    let route_fn_name = format!(
                        "__route_{}_{}",
                        method_str.to_lowercase(),
                        route.path.replace("/", "_").replace("-", "_").replace(".", "_")
                    );
                    let mut sig = mod_ref.make_signature();
                    sig.returns.push(AbiParam::new(types::I64)); // returns string ptr (txt)
                    let func_id = mod_ref.declare_function(&route_fn_name, Linkage::Local, &sig)?;
                    self.functions.insert(route_fn_name.clone(), (func_id, sig));
                    self.routes.push(RouteRegistration {
                        method: method_str.to_string(),
                        path: route.path.clone(),
                        func_id,
                    });
                }
                _ => {}
            }
        }

        // Phase 2: Compile bodies
        for stmt in &flat {
            match stmt {
                Statement::RuneDecl(rune) => {
                    self.compile_rune(rune)?;
                }
                Statement::RouteDecl(route) => {
                    let method_str = match route.method {
                        ast::HttpMethod::Get => "GET",
                        ast::HttpMethod::Post => "POST",
                        ast::HttpMethod::Put => "PUT",
                        ast::HttpMethod::Patch => "PATCH",
                        ast::HttpMethod::Delete => "DELETE",
                    };
                    let route_fn_name = format!(
                        "__route_{}_{}",
                        method_str.to_lowercase(),
                        route.path.replace("/", "_").replace("-", "_").replace(".", "_")
                    );
                    self.compile_route(&route_fn_name, &route.body)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn compile_rune(&mut self, rune: &RuneDecl) -> Result<(), String> {
        let &(func_id, ref sig) = self.functions.get(&rune.name).unwrap();
        self.ctx.func.signature = sig.clone();
        self.ctx.func.name = ir::UserFuncName::user(0, func_id.as_u32());

        // Pre-extract parameter types to satisfy the borrow checker
        let param_types: Vec<ir::Type> = rune.params.iter().map(|p| translate_type(&p.type_expr)).collect();

        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.fn_builder_ctx);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let mut variables = HashMap::new();
        
        for (i, param) in rune.params.iter().enumerate() {
            let var = Variable::from_u32(i as u32);
            builder.declare_var(var, param_types[i]);
            let val = builder.block_params(entry_block)[i];
            builder.def_var(var, val);
            variables.insert(param.name.clone(), var);
        }

        let mut next_var_index = rune.params.len();
        let mut helper = FunctionEmitter {
            builder,
            variables,
            variable_types: HashMap::new(),
            next_var_index: &mut next_var_index,
            module: self.module.as_mut().ok_or("Module not initialized")?,
            functions: &self.functions,
            runtime_helpers: &self.runtime_helpers,
            str_count: &mut self.str_count,
            string_literals: HashMap::new(),
            routes: &self.routes,
        };

        match &rune.body {
            RuneBody::Block(stmts) => {
                // If this is main, register routes and start server at the entry of the function!
                if rune.name == "main" {
                    if !helper.routes.is_empty() {
                        for route in helper.routes {
                            let method_gv = helper.get_or_create_string_literal(&route.method)?;
                            let path_gv = helper.get_or_create_string_literal(&route.path)?;
                            
                            let method_local = helper.module.declare_data_in_func(method_gv, &mut helper.builder.func);
                            let path_local = helper.module.declare_data_in_func(path_gv, &mut helper.builder.func);
                            
                            let method_ptr = helper.builder.ins().global_value(types::I64, method_local);
                            let path_ptr = helper.builder.ins().global_value(types::I64, path_local);
                            
                            let local_func = helper.module.declare_func_in_func(route.func_id, &mut helper.builder.func);
                            let func_ptr = helper.builder.ins().func_addr(types::I64, local_func);
                            
                            let register_id = helper.runtime_helpers.get("caria_register_route").unwrap().0;
                            let local_ref = helper.module.declare_func_in_func(register_id, &mut helper.builder.func);
                            helper.builder.ins().call(local_ref, &[method_ptr, path_ptr, func_ptr]);
                        }
                        
                        let start_id = helper.runtime_helpers.get("caria_start_server").unwrap().0;
                        let local_ref = helper.module.declare_func_in_func(start_id, &mut helper.builder.func);
                        let port_val = helper.builder.ins().iconst(types::I64, 8080);
                        helper.builder.ins().call(local_ref, &[port_val]);
                    }
                }

                let mut returned = false;
                for stmt in stmts {
                    helper.compile_statement(stmt, &mut returned)?;
                }

                if !returned {
                    if sig.returns.is_empty() {
                        helper.builder.ins().return_(&[]);
                    } else if rune.name == "main" {
                        let zero = helper.builder.ins().iconst(types::I32, 0);
                        helper.builder.ins().return_(&[zero]);
                    } else {
                        let zero = helper.builder.ins().iconst(types::I64, 0);
                        helper.builder.ins().return_(&[zero]);
                    }
                }
            }
            RuneBody::Inline(expr) => {
                let val = helper.compile_expr(expr)?;
                helper.builder.ins().return_(&[val]);
            }
        }

        helper.builder.finalize();
        let mod_ref = self.module.as_mut().ok_or("Module not initialized")?;
        mod_ref.define_function(func_id, &mut self.ctx)?;
        self.ctx.clear();

        Ok(())
    }

    fn compile_route(&mut self, name: &str, body: &[Statement]) -> Result<(), String> {
        let &(func_id, ref sig) = self.functions.get(name).unwrap();
        self.ctx.func.signature = sig.clone();
        self.ctx.func.name = ir::UserFuncName::user(0, func_id.as_u32());

        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.fn_builder_ctx);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let variables = HashMap::new();
        let mut next_var_index = 0;
        
        let mut helper = FunctionEmitter {
            builder,
            variables,
            variable_types: HashMap::new(),
            next_var_index: &mut next_var_index,
            module: self.module.as_mut().ok_or("Module not initialized")?,
            functions: &self.functions,
            runtime_helpers: &self.runtime_helpers,
            str_count: &mut self.str_count,
            string_literals: HashMap::new(),
            routes: &self.routes,
        };

        let mut returned = false;
        for stmt in body {
            helper.compile_statement(stmt, &mut returned)?;
        }
        
        // Default return 0 if not returned
        if !returned {
            let zero = helper.builder.ins().iconst(types::I64, 0);
            helper.builder.ins().return_(&[zero]);
        }

        helper.builder.finalize();
        let mod_ref = self.module.as_mut().ok_or("Module not initialized")?;
        mod_ref.define_function(func_id, &mut self.ctx)?;
        self.ctx.clear();

        Ok(())
    }
}

// ── Function-level Emitter helper ───────────────────────────────────────────

struct FunctionEmitter<'a> {
    builder: FunctionBuilder<'a>,
    variables: HashMap<String, Variable>,
    variable_types: HashMap<String, String>,
    next_var_index: &'a mut usize,
    module: &'a mut BackendModule,
    functions: &'a HashMap<String, (FuncId, Signature)>,
    runtime_helpers: &'a HashMap<String, (FuncId, Signature)>,
    str_count: &'a mut usize,
    string_literals: HashMap<String, DataId>,
    routes: &'a [RouteRegistration],
}

impl<'a> FunctionEmitter<'a> {
    fn infer_type(&self, expr: &Expression) -> &'static str {
        match expr {
            Expression::Literal(Literal::Str(_)) => "Str",
            Expression::Literal(Literal::Int(_)) => "Int",
            Expression::Literal(Literal::Bool(_)) => "Bool",
            Expression::EnvExpr(_) => "Str",
            Expression::Identifier(name) => {
                if let Some(t) = self.variable_types.get(name) {
                    if t == "Str" {
                        return "Str";
                    }
                }
                "Any"
            }
            Expression::BinaryExpr { left, .. } => {
                self.infer_type(left)
            }
            Expression::CallExpr { callee, .. } => {
                if let Expression::Identifier(name) = &**callee {
                    if name == "view" || name == "arg" || name == "caria_arg_get" {
                        return "Str";
                    }
                }
                "Any"
            }
            _ => "Any",
        }
    }
    fn compile_statement(&mut self, stmt: &Statement, returned: &mut bool) -> Result<(), String> {
        if *returned {
            return Ok(());
        }

        match stmt {
            Statement::ConstDecl(decl) => {
                let val = self.compile_expr(&decl.value)?;
                let inferred = self.infer_type(&decl.value).to_string();
                self.variable_types.insert(decl.name.clone(), inferred);
                if let Some(&var) = self.variables.get(&decl.name) {
                    self.builder.def_var(var, val);
                } else {
                    let var = Variable::from_u32(*self.next_var_index as u32);
                    *self.next_var_index += 1;
                    let val_type = self.builder.func.dfg.value_type(val);
                    self.builder.declare_var(var, val_type);
                    self.builder.def_var(var, val);
                    self.variables.insert(decl.name.clone(), var);
                }
            }
            Statement::MutDecl(decl) => {
                let val = self.compile_expr(&decl.value)?;
                let inferred = self.infer_type(&decl.value).to_string();
                self.variable_types.insert(decl.name.clone(), inferred);
                if let Some(&var) = self.variables.get(&decl.name) {
                    self.builder.def_var(var, val);
                } else {
                    let var = Variable::from_u32(*self.next_var_index as u32);
                    *self.next_var_index += 1;
                    let val_type = self.builder.func.dfg.value_type(val);
                    self.builder.declare_var(var, val_type);
                    self.builder.def_var(var, val);
                    self.variables.insert(decl.name.clone(), var);
                }
            }
            Statement::AssignStmt(assign) => {
                let val = self.compile_expr(&assign.value)?;
                if let Some(&var) = self.variables.get(&assign.name) {
                    self.builder.def_var(var, val);
                } else {
                    return Err(format!("Assigning to undeclared variable: {}", assign.name));
                }
            }
            Statement::RetStmt(expr) => {
                let val = self.compile_expr(expr)?;
                self.builder.ins().return_(&[val]);
                *returned = true;
            }
            Statement::SayStmt(expr) => {
                let is_str = self.infer_type(expr) == "Str";
                if is_str {
                    let ptr = self.compile_expr(expr)?;
                    let len_val = if let Expression::Literal(Literal::Str(s)) = expr {
                        self.builder.ins().iconst(types::I64, s.len() as i64)
                    } else {
                        self.builder.ins().iconst(types::I64, -1)
                    };
                    let helper_id = self.runtime_helpers.get("caria_say_txt").unwrap().0;
                    let local_ref = self.module.declare_func_in_func(helper_id, &mut self.builder.func);
                    self.builder.ins().call(local_ref, &[ptr, len_val]);
                } else {
                    let val = self.compile_expr(expr)?;
                    let helper_id = self.runtime_helpers.get("caria_say_int").unwrap().0;
                    let local_ref = self.module.declare_func_in_func(helper_id, &mut self.builder.func);
                    self.builder.ins().call(local_ref, &[val]);
                }
            }
            Statement::IfStmt(if_stmt) => {
                let cond_val = self.compile_expr(&if_stmt.condition)?;
                
                let then_block = self.builder.create_block();
                let else_block = self.builder.create_block();
                let merge_block = self.builder.create_block();

                self.builder.ins().brif(cond_val, then_block, &[], else_block, &[]);

                self.builder.switch_to_block(then_block);
                self.builder.seal_block(then_block);
                let mut then_returned = false;
                for s in &if_stmt.consequence {
                    self.compile_statement(s, &mut then_returned)?;
                }
                if !then_returned {
                    self.builder.ins().jump(merge_block, &[]);
                }

                self.builder.switch_to_block(else_block);
                self.builder.seal_block(else_block);
                let mut else_returned = false;
                if let Some(alternative) = &if_stmt.alternative {
                    for s in alternative {
                        self.compile_statement(s, &mut else_returned)?;
                    }
                }
                if !else_returned {
                    self.builder.ins().jump(merge_block, &[]);
                }

                self.builder.switch_to_block(merge_block);
                self.builder.seal_block(merge_block);

                if then_returned && else_returned {
                    *returned = true;
                }
            }
            Statement::WhileStmt(w) => {
                let header_block = self.builder.create_block();
                let body_block = self.builder.create_block();
                let exit_block = self.builder.create_block();

                self.builder.ins().jump(header_block, &[]);

                self.builder.switch_to_block(header_block);
                let cond_val = self.compile_expr(&w.condition)?;
                self.builder.ins().brif(cond_val, body_block, &[], exit_block, &[]);

                self.builder.switch_to_block(body_block);
                self.builder.seal_block(body_block);
                let mut body_returned = false;
                for s in &w.body {
                    self.compile_statement(s, &mut body_returned)?;
                }
                if !body_returned {
                    self.builder.ins().jump(header_block, &[]);
                }

                self.builder.seal_block(header_block);

                self.builder.switch_to_block(exit_block);
                self.builder.seal_block(exit_block);
            }
            Statement::ExprStmt(expr) => {
                let _ = self.compile_expr(expr)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expression) -> Result<Value, String> {
        match expr {
            Expression::Literal(Literal::Int(val)) => {
                Ok(self.builder.ins().iconst(types::I64, *val))
            }
            Expression::Literal(Literal::Bool(val)) => {
                let bit = if *val { 1 } else { 0 };
                Ok(self.builder.ins().iconst(types::I8, bit))
            }
            Expression::Literal(Literal::Str(s)) => {
                let data_id = self.get_or_create_string_literal(s)?;
                let local_gv = self.module.declare_data_in_func(data_id, &mut self.builder.func);
                let ptr = self.builder.ins().global_value(types::I64, local_gv);
                Ok(ptr)
            }
            Expression::Identifier(name) => {
                if let Some(&var) = self.variables.get(name) {
                    Ok(self.builder.use_var(var))
                } else {
                    Err(format!("Access to undeclared identifier: {}", name))
                }
            }
            Expression::BinaryExpr { left, op, right } => {
                let lhs = self.compile_expr(left)?;
                let rhs = self.compile_expr(right)?;
                match op {
                    BinaryOp::Add => {
                        let left_type = self.infer_type(left);
                        let right_type = self.infer_type(right);
                        if left_type == "Str" || right_type == "Str" {
                            let helper_id = self.runtime_helpers.get("caria_str_concat").unwrap().0;
                            let local_ref = self.module.declare_func_in_func(helper_id, &mut self.builder.func);
                            let inst = self.builder.ins().call(local_ref, &[lhs, rhs]);
                            Ok(self.builder.inst_results(inst)[0])
                        } else {
                            Ok(self.builder.ins().iadd(lhs, rhs))
                        }
                    }
                    BinaryOp::Subtract => Ok(self.builder.ins().isub(lhs, rhs)),
                    BinaryOp::Multiply => Ok(self.builder.ins().imul(lhs, rhs)),
                    BinaryOp::Divide => Ok(self.builder.ins().sdiv(lhs, rhs)),
                    BinaryOp::Eq => {
                        let flags = self.builder.ins().icmp(ir::condcodes::IntCC::Equal, lhs, rhs);
                        Ok(flags)
                    }
                    BinaryOp::Less => {
                        let flags = self.builder.ins().icmp(ir::condcodes::IntCC::SignedLessThan, lhs, rhs);
                        Ok(flags)
                    }
                    BinaryOp::Greater => {
                        let flags = self.builder.ins().icmp(ir::condcodes::IntCC::SignedGreaterThan, lhs, rhs);
                        Ok(flags)
                    }
                    BinaryOp::And => Ok(self.builder.ins().band(lhs, rhs)),
                    BinaryOp::Or => Ok(self.builder.ins().bor(lhs, rhs)),
                    _ => Err(format!("Unsupported binary operator: {:?}", op)),
                }
            }
            Expression::UnaryExpr { op, operand } => {
                let val = self.compile_expr(operand)?;
                match op {
                    UnaryOp::Neg => Ok(self.builder.ins().ineg(val)),
                    UnaryOp::Not => {
                        let one = self.builder.ins().iconst(types::I8, 1);
                        Ok(self.builder.ins().bxor(val, one))
                    }
                }
            }
            Expression::CallExpr { callee, args } => {
                if let Expression::Identifier(name) = &**callee {


                    if let Some(&(func_id, ref sig)) = self.functions.get(name) {
                        let mut compiled_args = Vec::new();
                        for arg in args {
                            compiled_args.push(self.compile_expr(arg)?);
                        }
                        let local_ref = self.module.declare_func_in_func(func_id, &mut self.builder.func);
                        let inst = self.builder.ins().call(local_ref, &compiled_args);
                        if !sig.returns.is_empty() {
                            Ok(self.builder.inst_results(inst)[0])
                        } else {
                            Ok(self.builder.ins().iconst(types::I64, 0))
                        }
                    } else if let Some(&(func_id, ref sig)) = self.runtime_helpers.get(name) {
                        let mut compiled_args = Vec::new();
                        for arg in args {
                            compiled_args.push(self.compile_expr(arg)?);
                        }
                        let local_ref = self.module.declare_func_in_func(func_id, &mut self.builder.func);
                        let inst = self.builder.ins().call(local_ref, &compiled_args);
                        if !sig.returns.is_empty() {
                            Ok(self.builder.inst_results(inst)[0])
                        } else {
                            Ok(self.builder.ins().iconst(types::I64, 0))
                        }
                    } else {
                        Err(format!("Undefined function call: {}", name))
                    }
                } else {
                    Err("Dynamic function pointers not supported natively yet".to_string())
                }
            }
            Expression::WhenExpr { subject, arms } => {
                let subject_val = self.compile_expr(subject)?;
                let merge_block = self.builder.create_block();
                self.builder.append_block_param(merge_block, types::I64);

                let mut wildcard_emitted = false;

                for arm in arms {
                    if wildcard_emitted {
                        break;
                    }

                    let match_block = self.builder.create_block();

                    match &arm.pattern {
                        ast::WhenPattern::Literal(Literal::Int(n)) => {
                            let next_arm_block = self.builder.create_block();
                            let match_val = self.builder.ins().iconst(types::I64, *n);
                            let matched = self.builder.ins().icmp(ir::condcodes::IntCC::Equal, subject_val, match_val);
                            self.builder.ins().brif(matched, match_block, &[], next_arm_block, &[]);
                            self.builder.seal_block(match_block);
                            self.builder.seal_block(next_arm_block);

                            self.builder.switch_to_block(match_block);
                            let val = self.compile_expr(&arm.body)?;
                            self.builder.ins().jump(merge_block, &[val]);

                            self.builder.switch_to_block(next_arm_block);
                        }
                        ast::WhenPattern::Wildcard => {
                            self.builder.ins().jump(match_block, &[]);
                            self.builder.seal_block(match_block);

                            self.builder.switch_to_block(match_block);
                            let val = self.compile_expr(&arm.body)?;
                            self.builder.ins().jump(merge_block, &[val]);

                            wildcard_emitted = true;
                        }
                        _ => {
                            let next_arm_block = self.builder.create_block();
                            self.builder.ins().jump(next_arm_block, &[]);
                            self.builder.seal_block(next_arm_block);
                            self.builder.switch_to_block(next_arm_block);
                        }
                    }
                }

                if !wildcard_emitted {
                    let default_val = self.builder.ins().iconst(types::I64, 0);
                    self.builder.ins().jump(merge_block, &[default_val]);
                }

                self.builder.switch_to_block(merge_block);
                self.builder.seal_block(merge_block);

                let res = self.builder.block_params(merge_block)[0];
                Ok(res)
            }
            Expression::EnvExpr(key) => {
                let key_gv = self.get_or_create_string_literal(key)?;
                let key_local = self.module.declare_data_in_func(key_gv, &mut self.builder.func);
                let key_ptr = self.builder.ins().global_value(types::I64, key_local);
                
                let helper_id = self.runtime_helpers.get("caria_get_env_txt").unwrap().0;
                let local_ref = self.module.declare_func_in_func(helper_id, &mut self.builder.func);
                let inst = self.builder.ins().call(local_ref, &[key_ptr]);
                Ok(self.builder.inst_results(inst)[0])
            }
            _ => {
                Ok(self.builder.ins().iconst(types::I64, 0))
            }
        }
    }

    fn get_or_create_string_literal(&mut self, s: &str) -> Result<DataId, String> {
        if let Some(&id) = self.string_literals.get(s) {
            return Ok(id);
        }

        let name = format!("__str_{}", *self.str_count);
        *self.str_count += 1;

        let data_id = self.module
            .declare_data(&name, Linkage::Local, false, false)
            .map_err(|e| e.to_string())?;

        let mut desc = DataDescription::new();
        let mut bytes = s.as_bytes().to_vec();
        bytes.push(0); // Null-terminate for standard C-helpers (getenv, strlen, etc.)
        desc.define(bytes.into_boxed_slice());
        self.module.define_data(data_id, &desc)?;

        self.string_literals.insert(s.to_string(), data_id);
        Ok(data_id)
    }
}
