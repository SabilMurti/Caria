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
        match self {
            BackendModule::JIT(m) => m.define_function(func_id, ctx).map_err(|e| e.to_string()),
            BackendModule::Object(m) => m.define_function(func_id, ctx).map_err(|e| e.to_string()),
        }
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

pub struct CariaCompiler {
    module: Option<BackendModule>,
    fn_builder_ctx: FunctionBuilderContext,
    ctx: cranelift_codegen::Context,
    // Map function names to their FuncId and signatures
    functions: HashMap<String, (FuncId, Signature)>,
    // String literal counter
    str_count: usize,
    // Native runtime helpers
    runtime_helpers: HashMap<String, (FuncId, Signature)>,
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
        runtime_helpers.insert("get_env_int".to_string(), (id_env, sig_env));

        Ok(Self {
            module: Some(BackendModule::JIT(module)),
            fn_builder_ctx: FunctionBuilderContext::new(),
            ctx: cranelift_codegen::Context::new(),
            functions: HashMap::new(),
            str_count: 0,
            runtime_helpers,
        })
    }

    pub fn new_object() -> Result<Self, String> {
        let isa_builder = isa::lookup(target_lexicon::Triple::host()).map_err(|e| e.to_string())?;
        let mut flags_builder = settings::builder();
        flags_builder.set("is_pic", "true").unwrap();
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
        runtime_helpers.insert("get_env_int".to_string(), (id_env, sig_env));

        Ok(Self {
            module: Some(BackendModule::Object(module)),
            fn_builder_ctx: FunctionBuilderContext::new(),
            ctx: cranelift_codegen::Context::new(),
            functions: HashMap::new(),
            str_count: 0,
            runtime_helpers,
        })
    }

    pub fn compile_to_jit(&mut self, program: &Program) -> Result<*const u8, String> {
        self.compile_all(program)?;
        
        if let Some(BackendModule::JIT(ref mut m)) = self.module {
            m.finalize_definitions().map_err(|e| e.to_string())?;
            if let Some(&(func_id, _)) = self.functions.get("main") {
                let code_ptr = m.get_finalized_function(func_id);
                Ok(code_ptr)
            } else {
                Err("No main function defined in Calesteria code".to_string())
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

    fn compile_all(&mut self, program: &Program) -> Result<(), String> {
        let mod_ref = self.module.as_mut().ok_or("Module not initialized")?;

        // Phase 1: Declare all functions
        for stmt in &program.statements {
            if let Statement::RuneDecl(rune) = stmt {
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
        }

        // Phase 2: Compile bodies
        for stmt in &program.statements {
            if let Statement::RuneDecl(rune) = stmt {
                self.compile_rune(rune)?;
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
            next_var_index: &mut next_var_index,
            module: self.module.as_mut().ok_or("Module not initialized")?,
            functions: &self.functions,
            runtime_helpers: &self.runtime_helpers,
            str_count: &mut self.str_count,
            string_literals: HashMap::new(),
        };

        match &rune.body {
            RuneBody::Block(stmts) => {
                let mut returned = false;
                for stmt in stmts {
                    helper.compile_statement(stmt, &mut returned)?;
                }
                
                if rune.name == "main" && !returned {
                    let zero = helper.builder.ins().iconst(types::I32, 0);
                    helper.builder.ins().return_(&[zero]);
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
}

// ── Function-level Emitter helper ───────────────────────────────────────────

struct FunctionEmitter<'a> {
    builder: FunctionBuilder<'a>,
    variables: HashMap<String, Variable>,
    next_var_index: &'a mut usize,
    module: &'a mut BackendModule,
    functions: &'a HashMap<String, (FuncId, Signature)>,
    runtime_helpers: &'a HashMap<String, (FuncId, Signature)>,
    str_count: &'a mut usize,
    string_literals: HashMap<String, DataId>,
}

impl<'a> FunctionEmitter<'a> {
    fn compile_statement(&mut self, stmt: &Statement, returned: &mut bool) -> Result<(), String> {
        if *returned {
            return Ok(());
        }

        match stmt {
            Statement::ConstDecl(decl) => {
                let val = self.compile_expr(&decl.value)?;
                let var = Variable::from_u32(*self.next_var_index as u32);
                *self.next_var_index += 1;
                
                let val_type = self.builder.func.dfg.value_type(val);
                self.builder.declare_var(var, val_type);
                self.builder.def_var(var, val);
                self.variables.insert(decl.name.clone(), var);
            }
            Statement::MutDecl(decl) => {
                let val = self.compile_expr(&decl.value)?;
                let var = Variable::from_u32(*self.next_var_index as u32);
                *self.next_var_index += 1;
                
                let val_type = self.builder.func.dfg.value_type(val);
                self.builder.declare_var(var, val_type);
                self.builder.def_var(var, val);
                self.variables.insert(decl.name.clone(), var);
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
                let val = self.compile_expr(expr)?;
                let val_type = self.builder.func.dfg.value_type(val);

                if val_type == types::I64 {
                    let helper_id = self.runtime_helpers.get("caria_say_int").unwrap().0;
                    let local_ref = self.module.declare_func_in_func(helper_id, &mut self.builder.func);
                    self.builder.ins().call(local_ref, &[val]);
                } else {
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
                    BinaryOp::Add => Ok(self.builder.ins().iadd(lhs, rhs)),
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

                            // Compile match block
                            self.builder.switch_to_block(match_block);
                            let val = self.compile_expr(&arm.body)?;
                            self.builder.ins().jump(merge_block, &[val]);

                            // Set compiler focus to next block for subsequent arm branches
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
