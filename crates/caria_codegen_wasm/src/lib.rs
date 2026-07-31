use caria_parser::ast::{
    Expression, RuneDecl, RuneBody, TypeExpr, Literal, Program, Statement, BinaryOp,
};
use std::collections::HashMap;
use wasm_encoder::{
    BlockType, CodeSection, ExportKind, ExportSection, Function,
    FunctionSection, ImportSection, MemorySection, MemoryType, Module,
    TypeSection, ValType, Instruction, DataSection, ConstExpr,
};

pub struct WasmCodegen {
    module: Module,
    types: TypeSection,
    functions: FunctionSection,
    exports: ExportSection,
    codes: CodeSection,
    data: DataSection,
    
    // Map function name to index
    func_indices: HashMap<String, u32>,
    func_return_types: HashMap<String, String>,
    next_func_index: u32,
    
    // String literal offsets and lengths
    strings: Vec<String>,
}

fn get_type_str(te: &TypeExpr) -> &str {
    match te {
        TypeExpr::Named(s) => s.as_str(),
        _ => "int",
    }
}

fn get_opt_type_str(te: &Option<TypeExpr>) -> &str {
    match te {
        Some(t) => get_type_str(t),
        None => "Void",
    }
}

impl WasmCodegen {
    pub fn new() -> Self {
        Self {
            module: Module::new(),
            types: TypeSection::new(),
            functions: FunctionSection::new(),
            exports: ExportSection::new(),
            codes: CodeSection::new(),
            data: DataSection::new(),
            func_indices: HashMap::new(),
            func_return_types: HashMap::new(),
            next_func_index: 0,
            strings: Vec::new(),
        }
    }

    pub fn generate(mut self, program: &Program) -> Vec<u8> {
        // Collect all string literals first to write to DataSection
        self.collect_strings(program);

        // 1. Declare print & DOM helper import type signatures:
        // print_int: index 0: [I64] -> []
        self.types.function(vec![ValType::I64], vec![]);
        // print_float: index 1: [F64] -> []
        self.types.function(vec![ValType::F64], vec![]);
        // print_bool: index 2: [I32] -> []
        self.types.function(vec![ValType::I32], vec![]);
        // print_str: index 3: [I32, I32] -> []
        self.types.function(vec![ValType::I32, ValType::I32], vec![]);
        // dom_set_text: index 4: [I32, I32, I32, I32] -> [] (id_ptr, id_len, val_ptr, val_len)
        self.types.function(vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32], vec![]);
        // dom_get_int: index 5: [I32, I32] -> [I64] (id_ptr, id_len) -> i64
        self.types.function(vec![ValType::I32, ValType::I32], vec![ValType::I64]);
        
        self.next_func_index = 6; // imports are indices 0..5

        let mut imports = ImportSection::new();
        imports.import("env", "print_int", wasm_encoder::EntityType::Function(0));
        imports.import("env", "print_float", wasm_encoder::EntityType::Function(1));
        imports.import("env", "print_bool", wasm_encoder::EntityType::Function(2));
        imports.import("env", "print_str", wasm_encoder::EntityType::Function(3));
        imports.import("env", "dom_set_text", wasm_encoder::EntityType::Function(4));
        imports.import("env", "dom_get_int", wasm_encoder::EntityType::Function(5));

        // 2. Scan for all functions in Calesteria AST and assign their type indexes and function indexes
        let mut fn_decls = Vec::new();
        for stmt in &program.statements {
            if let Statement::RuneDecl(decl) = stmt {
                fn_decls.push(decl);
                
                // Construct WASM type for this function
                let mut params = Vec::new();
                for p in &decl.params {
                    params.push(self.map_type(get_type_str(&p.type_expr)));
                }
                
                let ret_ty_str = get_opt_type_str(&decl.return_type);
                let results = if ret_ty_str == "Nil" || ret_ty_str == "Void" {
                    vec![]
                } else {
                    vec![self.map_type(ret_ty_str)]
                };
                
                let type_idx = self.next_func_index;
                self.types.function(params, results);
                
                self.func_indices.insert(decl.name.clone(), self.next_func_index);
                self.func_return_types.insert(decl.name.clone(), ret_ty_str.to_string());
                
                self.functions.function(type_idx);
                
                if decl.is_public || decl.name == "main" || decl.name.starts_with("on_") || decl.name.starts_with("calc") {
                    self.exports.export(&decl.name, ExportKind::Func, self.next_func_index);
                }
                self.next_func_index += 1;
            }
        }

        // Add TypeSection to module
        self.module.section(&self.types);

        // Add ImportSection to module
        self.module.section(&imports);
        
        // Add FunctionSection to module
        self.module.section(&self.functions);

        // 3. Define Memory Section
        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            shared: false,
            memory64: false,
        });
        self.module.section(&memories);
        self.exports.export("memory", ExportKind::Memory, 0);

        // Add ExportSection to module
        self.module.section(&self.exports);

        // 4. Construct string data section data
        let mut current_offset = 16;
        let mut string_map = HashMap::new();
        for s in &self.strings {
            let bytes = s.as_bytes();
            let offset_expr = ConstExpr::i32_const(current_offset);
            self.data.active(0, &offset_expr, bytes.iter().copied());
            string_map.insert(s.clone(), (current_offset, bytes.len() as i32));
            current_offset += bytes.len() as i32;
        }

        // 5. Compile function bodies into CodeSection
        for decl in fn_decls {
            let compiled_fn = self.compile_fn(decl, &string_map);
            self.codes.function(&compiled_fn);
        }
        
        // Add CodeSection first!
        self.module.section(&self.codes);
        
        // Add DataSection after!
        self.module.section(&self.data);

        self.module.finish()
    }

    fn collect_strings(&mut self, program: &Program) {
        for stmt in &program.statements {
            self.collect_strings_stmt(stmt);
        }
    }

    fn collect_strings_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::ConstDecl(c_stmt) => self.collect_strings_expr(&c_stmt.value),
            Statement::MutDecl(mut_stmt) => self.collect_strings_expr(&mut_stmt.value),
            Statement::AssignStmt(assign) => self.collect_strings_expr(&assign.value),
            Statement::ExprStmt(expr) => self.collect_strings_expr(expr),
            Statement::RetStmt(expr) => self.collect_strings_expr(expr),
            Statement::SayStmt(expr) => self.collect_strings_expr(expr),
            Statement::RuneDecl(decl) => {
                match &decl.body {
                    RuneBody::Block(stmts) => {
                        for s in stmts {
                            self.collect_strings_stmt(s);
                        }
                    }
                    RuneBody::Inline(expr) => {
                        self.collect_strings_expr(expr);
                    }
                }
            }
            Statement::IfStmt(if_stmt) => {
                self.collect_strings_expr(&if_stmt.condition);
                for s in &if_stmt.consequence {
                    self.collect_strings_stmt(s);
                }
                if let Some(alt) = &if_stmt.alternative {
                    for s in alt {
                        self.collect_strings_stmt(s);
                    }
                }
            }
            Statement::WhileStmt(w) => {
                self.collect_strings_expr(&w.condition);
                for s in &w.body {
                    self.collect_strings_stmt(s);
                }
            }
            _ => {}
        }
    }

    fn collect_strings_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::Literal(Literal::Str(s)) => {
                if !self.strings.contains(s) {
                    self.strings.push(s.clone());
                }
            }
            Expression::BinaryExpr { left, right, .. } => {
                self.collect_strings_expr(left);
                self.collect_strings_expr(right);
            }
            Expression::CallExpr { callee, args } => {
                self.collect_strings_expr(callee);
                for arg in args {
                    self.collect_strings_expr(arg);
                }
            }
            _ => {}
        }
    }

    fn map_type(&self, type_name: &str) -> ValType {
        match type_name {
            "int" | "int" => ValType::I64,
            "dec" | "dec" => ValType::F64,
            "flag" | "flag" => ValType::I32,
            "txt" | "txt" => ValType::I32,
            _ => ValType::I64,
        }
    }

    fn compile_fn(&self, decl: &RuneDecl, string_map: &HashMap<String, (i32, i32)>) -> Function {
        let mut locals_map = HashMap::new();
        let mut locals_list = Vec::new();
        let mut next_local_index = decl.params.len() as u32;

        let stmts_vec = match &decl.body {
            RuneBody::Block(stmts) => stmts.clone(),
            RuneBody::Inline(expr) => vec![Statement::RetStmt(*expr.clone())],
        };

        self.collect_locals(&stmts_vec, &mut locals_map, &mut locals_list, &mut next_local_index, decl);

        let mut wasm_locals = Vec::new();
        for ty in &locals_list {
            wasm_locals.push((1, *ty));
        }
        
        let mut f = Function::new(wasm_locals);
        
        for stmt in &stmts_vec {
            self.compile_stmt(stmt, &mut f, &locals_map, decl, string_map);
        }
        
        f.instruction(&Instruction::End);
        f
    }

    fn collect_locals(
        &self,
        body: &[Statement],
        locals_map: &mut HashMap<String, (u32, String)>,
        locals_list: &mut Vec<ValType>,
        next_index: &mut u32,
        decl: &RuneDecl,
    ) {
        for stmt in body {
            match stmt {
                Statement::ConstDecl(c_stmt) => {
                    let type_name = self.infer_expr_type_name(&c_stmt.value, locals_map, decl);
                    let ty = self.map_type(&type_name);
                    if !locals_map.contains_key(&c_stmt.name) {
                        locals_map.insert(c_stmt.name.clone(), (*next_index, type_name));
                        locals_list.push(ty);
                        *next_index += 1;
                    }
                }
                Statement::MutDecl(mut_stmt) => {
                    let type_name = self.infer_expr_type_name(&mut_stmt.value, locals_map, decl);
                    let ty = self.map_type(&type_name);
                    if !locals_map.contains_key(&mut_stmt.name) {
                        locals_map.insert(mut_stmt.name.clone(), (*next_index, type_name));
                        locals_list.push(ty);
                        *next_index += 1;
                    }
                }
                Statement::IfStmt(if_stmt) => {
                    self.collect_locals(&if_stmt.consequence, locals_map, locals_list, next_index, decl);
                    if let Some(alt) = &if_stmt.alternative {
                        self.collect_locals(alt, locals_map, locals_list, next_index, decl);
                    }
                }
                Statement::WhileStmt(w) => {
                    self.collect_locals(&w.body, locals_map, locals_list, next_index, decl);
                }
                _ => {}
            }
        }
    }

    fn infer_expr_type_name(
        &self,
        expr: &Expression,
        locals_map: &HashMap<String, (u32, String)>,
        decl: &RuneDecl,
    ) -> String {
        match expr {
            Expression::Literal(Literal::Int(_)) => "int".to_string(),
            Expression::Literal(Literal::Float(_)) => "dec".to_string(),
            Expression::Literal(Literal::Bool(_)) => "flag".to_string(),
            Expression::Literal(Literal::Str(_)) => "txt".to_string(),
            Expression::Identifier(name) => {
                if let Some((_, type_name)) = locals_map.get(name) {
                    type_name.clone()
                } else if let Some(p) = decl.params.iter().find(|p| &p.name == name) {
                    get_type_str(&p.type_expr).to_string()
                } else {
                    "int".to_string()
                }
            }
            Expression::BinaryExpr { left, .. } => self.infer_expr_type_name(left, locals_map, decl),
            Expression::CallExpr { callee, .. } => {
                if let Expression::Identifier(name) = &**callee {
                    if let Some(ret_ty) = self.func_return_types.get(name) {
                        ret_ty.clone()
                    } else if name == "dom_get_int" {
                        "int".to_string()
                    } else {
                        "int".to_string()
                    }
                } else {
                    "int".to_string()
                }
            }
            _ => "int".to_string(),
        }
    }

    fn infer_expr_type(
        &self,
        expr: &Expression,
        locals_map: &HashMap<String, (u32, String)>,
        decl: &RuneDecl,
    ) -> ValType {
        let type_name = self.infer_expr_type_name(expr, locals_map, decl);
        self.map_type(&type_name)
    }

    fn compile_stmt(
        &self,
        stmt: &Statement,
        f: &mut Function,
        locals_map: &HashMap<String, (u32, String)>,
        decl: &RuneDecl,
        string_map: &HashMap<String, (i32, i32)>,
    ) {
        match stmt {
            Statement::ConstDecl(c_stmt) => {
                self.compile_expr(&c_stmt.value, f, locals_map, decl, string_map);
                let (idx, _) = locals_map.get(&c_stmt.name).unwrap();
                f.instruction(&Instruction::LocalSet(*idx));
            }
            Statement::MutDecl(mut_stmt) => {
                self.compile_expr(&mut_stmt.value, f, locals_map, decl, string_map);
                let (idx, _) = locals_map.get(&mut_stmt.name).unwrap();
                f.instruction(&Instruction::LocalSet(*idx));
            }
            Statement::AssignStmt(assign) => {
                self.compile_expr(&assign.value, f, locals_map, decl, string_map);
                if let Some((idx, _)) = locals_map.get(&assign.name) {
                    f.instruction(&Instruction::LocalSet(*idx));
                } else {
                    if let Some(idx) = decl.params.iter().position(|p| p.name == assign.name) {
                        f.instruction(&Instruction::LocalSet(idx as u32));
                    }
                }
            }
            Statement::SayStmt(expr) => {
                let ty = self.infer_expr_type(expr, locals_map, decl);
                match ty {
                    ValType::I64 => {
                        self.compile_expr(expr, f, locals_map, decl, string_map);
                        f.instruction(&Instruction::Call(0));
                    }
                    ValType::F64 => {
                        self.compile_expr(expr, f, locals_map, decl, string_map);
                        f.instruction(&Instruction::Call(1));
                    }
                    ValType::I32 => {
                        self.compile_expr(expr, f, locals_map, decl, string_map);
                        f.instruction(&Instruction::Call(3));
                    }
                    _ => {}
                }
            }
            Statement::ExprStmt(expr) => {
                if let Expression::CallExpr { callee, args } = expr {
                    if let Expression::Identifier(name) = &**callee {
                        if name == "print" && args.len() == 1 {
                            let arg = &args[0];
                            let ty = self.infer_expr_type(arg, locals_map, decl);
                            match ty {
                                ValType::I64 => {
                                    self.compile_expr(arg, f, locals_map, decl, string_map);
                                    f.instruction(&Instruction::Call(0));
                                }
                                ValType::F64 => {
                                    self.compile_expr(arg, f, locals_map, decl, string_map);
                                    f.instruction(&Instruction::Call(1));
                                }
                                ValType::I32 => {
                                    self.compile_expr(arg, f, locals_map, decl, string_map);
                                    f.instruction(&Instruction::Call(3));
                                }
                                _ => {}
                            }
                            return;
                        } else if (name == "dom_set_text" || name == "set_text") && args.len() == 2 {
                            self.compile_expr(&args[0], f, locals_map, decl, string_map);
                            self.compile_expr(&args[1], f, locals_map, decl, string_map);
                            f.instruction(&Instruction::Call(4));
                            return;
                        }
                    }
                }
                self.compile_expr(expr, f, locals_map, decl, string_map);
            }
            Statement::RetStmt(expr) => {
                self.compile_expr(expr, f, locals_map, decl, string_map);
                f.instruction(&Instruction::Return);
            }
            Statement::IfStmt(if_stmt) => {
                self.compile_expr(&if_stmt.condition, f, locals_map, decl, string_map);
                f.instruction(&Instruction::If(BlockType::Empty));
                for s in &if_stmt.consequence {
                    self.compile_stmt(s, f, locals_map, decl, string_map);
                }
                if let Some(alt) = &if_stmt.alternative {
                    f.instruction(&Instruction::Else);
                    for s in alt {
                        self.compile_stmt(s, f, locals_map, decl, string_map);
                    }
                }
                f.instruction(&Instruction::End);
            }
            Statement::WhileStmt(w) => {
                f.instruction(&Instruction::Block(BlockType::Empty));
                f.instruction(&Instruction::Loop(BlockType::Empty));
                
                self.compile_expr(&w.condition, f, locals_map, decl, string_map);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1));
                
                for s in &w.body {
                    self.compile_stmt(s, f, locals_map, decl, string_map);
                }
                
                f.instruction(&Instruction::Br(0));
                f.instruction(&Instruction::End);
                f.instruction(&Instruction::End);
            }
            _ => {}
        }
    }

    fn compile_expr(
        &self,
        expr: &Expression,
        f: &mut Function,
        locals_map: &HashMap<String, (u32, String)>,
        decl: &RuneDecl,
        string_map: &HashMap<String, (i32, i32)>,
    ) {
        match expr {
            Expression::Literal(Literal::Int(val)) => {
                f.instruction(&Instruction::I64Const(*val));
            }
            Expression::Literal(Literal::Float(val)) => {
                f.instruction(&Instruction::F64Const(*val));
            }
            Expression::Literal(Literal::Bool(val)) => {
                f.instruction(&Instruction::I32Const(if *val { 1 } else { 0 }));
            }
            Expression::Literal(Literal::Str(val)) => {
                if let Some((offset, len)) = string_map.get(val) {
                    f.instruction(&Instruction::I32Const(*offset));
                    f.instruction(&Instruction::I32Const(*len));
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            Expression::Identifier(name) => {
                if let Some((idx, _)) = locals_map.get(name) {
                    f.instruction(&Instruction::LocalGet(*idx));
                } else if let Some(idx) = decl.params.iter().position(|p| p.name == *name) {
                    f.instruction(&Instruction::LocalGet(idx as u32));
                }
            }
            Expression::BinaryExpr { left, op, right } => {
                let ty = self.infer_expr_type(left, locals_map, decl);
                self.compile_expr(left, f, locals_map, decl, string_map);
                self.compile_expr(right, f, locals_map, decl, string_map);
                
                match ty {
                    ValType::I64 => {
                        match op {
                            BinaryOp::Add => f.instruction(&Instruction::I64Add),
                            BinaryOp::Subtract => f.instruction(&Instruction::I64Sub),
                            BinaryOp::Multiply => f.instruction(&Instruction::I64Mul),
                            BinaryOp::Divide => f.instruction(&Instruction::I64DivS),
                            BinaryOp::Modulo => f.instruction(&Instruction::I64RemS),
                            BinaryOp::Eq => f.instruction(&Instruction::I64Eq),
                            BinaryOp::NotEq => f.instruction(&Instruction::I64Ne),
                            BinaryOp::Less => f.instruction(&Instruction::I64LtS),
                            BinaryOp::LessEq => f.instruction(&Instruction::I64LeS),
                            BinaryOp::Greater => f.instruction(&Instruction::I64GtS),
                            BinaryOp::GreaterEq => f.instruction(&Instruction::I64GeS),
                            _ => f.instruction(&Instruction::Nop),
                        };
                    }
                    ValType::F64 => {
                        match op {
                            BinaryOp::Add => f.instruction(&Instruction::F64Add),
                            BinaryOp::Subtract => f.instruction(&Instruction::F64Sub),
                            BinaryOp::Multiply => f.instruction(&Instruction::F64Mul),
                            BinaryOp::Divide => f.instruction(&Instruction::F64Div),
                            BinaryOp::Eq => f.instruction(&Instruction::F64Eq),
                            BinaryOp::NotEq => f.instruction(&Instruction::F64Ne),
                            BinaryOp::Less => f.instruction(&Instruction::F64Lt),
                            BinaryOp::LessEq => f.instruction(&Instruction::F64Le),
                            BinaryOp::Greater => f.instruction(&Instruction::F64Gt),
                            BinaryOp::GreaterEq => f.instruction(&Instruction::F64Ge),
                            _ => f.instruction(&Instruction::Nop),
                        };
                    }
                    ValType::I32 => {
                        match op {
                            BinaryOp::Eq => f.instruction(&Instruction::I32Eq),
                            BinaryOp::NotEq => f.instruction(&Instruction::I32Ne),
                            BinaryOp::And => f.instruction(&Instruction::I32And),
                            BinaryOp::Or => f.instruction(&Instruction::I32Or),
                            _ => f.instruction(&Instruction::Nop),
                        };
                    }
                    _ => {
                        f.instruction(&Instruction::Nop);
                    }
                }
            }
            Expression::CallExpr { callee, args } => {
                if let Expression::Identifier(name) = &**callee {
                    if (name == "dom_get_int" || name == "get_int") && args.len() == 1 {
                        self.compile_expr(&args[0], f, locals_map, decl, string_map);
                        f.instruction(&Instruction::Call(5)); // dom_get_int import index 5
                        return;
                    }

                    for arg in args {
                        self.compile_expr(arg, f, locals_map, decl, string_map);
                    }
                    if let Some(idx) = self.func_indices.get(name) {
                        f.instruction(&Instruction::Call(*idx));
                    }
                }
            }
            _ => {}
        }
    }
}
