use caria_parser::ast::{AssignStmt, Expression, FnDecl, IfExpr, LetStmt, Literal, MutStmt, Program, Statement, StructDecl, WhileStmt, ForStmt, ForInStmt, RouteDecl, ViewDecl, ServeStmt, HtmlNode, HtmlAttrValue};

pub struct JsCodegen {
    output: String,
    indent_level: usize,
    app_initialized: bool,
    needs_hono_serve: bool,
}
impl JsCodegen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            app_initialized: false,
            needs_hono_serve: false,
        }
    }

    pub fn generate(&mut self, program: &Program) -> String {
        for stmt in &program.statements {
            // Check if this is a ServeStmt or contains one (for import injection)
            if matches!(stmt, Statement::ServeStmt(_)) {
                self.needs_hono_serve = true;
            }
        }
        for stmt in &program.statements {
            self.emit_statement(stmt);
            self.output.push('\n');
        }
        if self.needs_hono_serve {
            self.output.insert_str(0, "import { serve } from '@hono/node-server';\n");
        }
        self.output.clone()
    }


    fn emit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::FnDecl(decl) => self.emit_fn_decl(decl),
            Statement::LetStmt(stmt) => self.emit_let_stmt(stmt),
            Statement::MutStmt(stmt) => self.emit_mut_stmt(stmt),
            Statement::AssignStmt(stmt) => self.emit_assign_stmt(stmt),
            Statement::IfExpr(expr) => self.emit_if_expr(expr),
            Statement::WhileStmt(stmt) => self.emit_while_stmt(stmt),
            Statement::ForStmt(stmt) => self.emit_for_stmt(stmt),
            Statement::ForInStmt(stmt) => self.emit_for_in_stmt(stmt),
            Statement::StructDecl(decl) => self.emit_struct_decl(decl),
            Statement::EnumDecl(_) => {
                // Enums in JS for Phase 2: just generate empty class or comments
                // Phase 3 will generate proper tagged union factories
            }
            Statement::ModDecl(_) => {
                // JS modules don't map 1:1 to Calesteria modules
                // Handled implicitly via file system
            }
            Statement::UseDecl(decl) => {
                self.emit_indent();
                if decl.is_npm {
                    // Hono exports named, not default
                    // Hono exports named `Hono` (capital H), not `hono`
                    if decl.module == "hono" {
                        self.output.push_str(&format!("import {{ Hono }} from '{}';\n", decl.module));
                    } else if decl.alias_import {
                        // use npm "pkg" as alias → import alias from 'pkg'
                        self.output.push_str(&format!("import {} from '{}';\n", decl.symbols[0], decl.module));
                    } else {
                        // use npm "pkg" { Sym1, Sym2 } → import { Sym1, Sym2 } from 'pkg'
                        self.output.push_str("import { ");
                        self.output.push_str(&decl.symbols.join(", "));
                        self.output.push_str(&format!(" }} from '{}';\n", decl.module));
                    }
                } else {
                    // Local module
                    self.output.push_str("import { ");
                    self.output.push_str(&decl.symbols.join(", "));
                    self.output.push_str(&format!(" }} from './{}.js';\n", decl.module.replace(".", "/")));
                }
            }
            Statement::ExprStmt(expr) => {
                self.emit_indent();
                self.output.push_str("await ");
                self.emit_expression(expr);
                self.output.push(';');
            }
            Statement::RetStmt(expr) => {
                self.emit_indent();
                self.output.push_str("return await ");
                self.emit_expression(expr);
                self.output.push(';');
            }
            Statement::RouteDecl(decl) => self.emit_route_decl(decl),
            Statement::ViewDecl(decl) => self.emit_view_decl(decl),
            Statement::ServeStmt(stmt) => self.emit_serve_stmt(stmt),
        }
    }

    fn emit_fn_decl(&mut self, decl: &FnDecl) {
        self.emit_indent();
        self.output.push_str(&format!("async function {}(", decl.name));
        for (i, param) in decl.params.iter().enumerate() {
            if i > 0 { self.output.push_str(", "); }
            self.output.push_str(&param.name);
            if let Some(default_val) = &param.default_value {
                self.output.push_str(" = ");
                self.emit_expression(default_val);
            }
        }
        self.output.push_str(") {\n");
        self.indent_level += 1;

        for stmt in &decl.body {
            self.emit_statement(stmt);
            self.output.push('\n');
        }
        self.indent_level -= 1;
        self.emit_indent();
        self.output.push('}');
    }
    fn emit_let_stmt(&mut self, stmt: &LetStmt) {
        self.emit_indent();
        self.output.push_str(&format!("const {} = await ", stmt.name));
        self.emit_expression(&stmt.value);
        self.output.push(';');
    }
    fn emit_mut_stmt(&mut self, stmt: &MutStmt) {
        self.emit_indent();
        self.output.push_str(&format!("let {} = await ", stmt.name));
        self.emit_expression(&stmt.value);
        self.output.push(';');
    }

    fn emit_assign_stmt(&mut self, stmt: &AssignStmt) {
        self.emit_indent();
        self.output.push_str(&format!("{} = ", stmt.name));
        self.emit_expression(&stmt.value);
        self.output.push(';');
    }

    fn emit_if_expr(&mut self, expr: &IfExpr) {
        self.emit_indent();
        self.output.push_str("if (");
        self.emit_expression(&expr.condition);
        self.output.push_str(") {\n");
        
        self.indent_level += 1;
        for stmt in &expr.consequence {
            self.emit_statement(stmt);
            self.output.push('\n');
        }
        self.indent_level -= 1;
        self.emit_indent();
        self.output.push('}');

        if let Some(alternative) = &expr.alternative {
            // Check if alternative is a single IfExpr (else if)
            if alternative.len() == 1 {
                if let Statement::IfExpr(else_if) = &alternative[0] {
                    self.output.push_str(" else ");
                    // Temporarily remove indent for the else-if part
                    let old_indent = self.indent_level;
                    self.indent_level = 0; 
                    self.emit_if_expr(else_if);
                    self.indent_level = old_indent;
                    return;
                }
            }
            
            self.output.push_str(" else {\n");
            self.indent_level += 1;
            for stmt in alternative {
                self.emit_statement(stmt);
                self.output.push('\n');
            }
            self.indent_level -= 1;
            self.emit_indent();
            self.output.push('}');
        }
    }
    fn emit_while_stmt(&mut self, stmt: &WhileStmt) {
        self.emit_indent();
        self.output.push_str("while (");
        self.emit_expression(&stmt.condition);
        self.output.push_str(") {\n");
        
        self.indent_level += 1;
        for body_stmt in &stmt.body {
            self.emit_statement(body_stmt);
            self.output.push('\n');
        }
        self.indent_level -= 1;
        
        self.emit_indent();
        self.output.push('}');
    }

    fn emit_for_stmt(&mut self, stmt: &ForStmt) {
        self.emit_indent();
        self.output.push_str(&format!("for (let {} = ", stmt.var));
        self.emit_expression(&stmt.start);
        self.output.push_str(&format!("; {} < ", stmt.var));
        self.emit_expression(&stmt.end);
        self.output.push_str(&format!("; {}++) {{\n", stmt.var));
        
        self.indent_level += 1;
        for body_stmt in &stmt.body {
            self.emit_statement(body_stmt);
            self.output.push('\n');
        }
        self.indent_level -= 1;
        
        self.emit_indent();
        self.output.push('}');
    }

    fn emit_for_in_stmt(&mut self, stmt: &ForInStmt) {
        self.emit_indent();
        if let Some(index_var) = &stmt.index_var {
            // Try arrays first (entries returns iterator of [index, value]), fallback to Object.entries for objects
            self.output.push_str(&format!("for (const [{}, {}] of (Array.isArray(", index_var, stmt.item_var));
            self.emit_expression(&stmt.collection);
            self.output.push_str(") ? ");
            self.emit_expression(&stmt.collection);
            self.output.push_str(".entries() : Object.entries(");
            self.emit_expression(&stmt.collection);
            self.output.push_str("))) {\n");
        } else {
            self.output.push_str(&format!("for (const {} of ", stmt.item_var));
            self.emit_expression(&stmt.collection);
            self.output.push_str(") {\n");
        }
        
        self.indent_level += 1;
        for body_stmt in &stmt.body {
            self.emit_statement(body_stmt);
            self.output.push('\n');
        }
        self.indent_level -= 1;
        
        self.emit_indent();
        self.output.push('}');
    }

    fn emit_struct_decl(&mut self, decl: &StructDecl) {
        self.emit_indent();
        self.output.push_str(&format!("class {} {{\n", decl.name));
        self.indent_level += 1;
        
        self.emit_indent();
        self.output.push_str("constructor({ ");
        for (i, field) in decl.fields.iter().enumerate() {
            if i > 0 { self.output.push_str(", "); }
            self.output.push_str(&field.name);
        }
        self.output.push_str(" }) {\n");
        
        self.indent_level += 1;
        for field in &decl.fields {
            self.emit_indent();
            self.output.push_str(&format!("this.{} = {};\n", field.name, field.name));
        }
        self.indent_level -= 1;
        
        self.emit_indent();
        self.output.push_str("}\n");
        
        self.indent_level -= 1;
        self.emit_indent();
        self.output.push('}');
    }


    fn emit_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Literal(Literal::Int(val)) => self.output.push_str(&val.to_string()),
            Expression::ClosureExpr { params, body } => {
                self.output.push_str("async (");
                self.output.push_str(&params.join(", "));
                self.output.push_str(") => {\n");
                self.indent_level += 1;
                for stmt in body {
                    self.emit_statement(stmt);
                    self.output.push('\n');
                }
                self.indent_level -= 1;
                self.emit_indent();
                self.output.push('}');
            }
            Expression::Literal(Literal::Float(val)) => self.output.push_str(&val.to_string()),
            Expression::Literal(Literal::Str(val)) => {
                self.output.push_str(&format!("`{}`", val));
            }
            Expression::Literal(Literal::Bool(val)) => {
                self.output.push_str(if *val { "true" } else { "false" })
            }
            Expression::Literal(Literal::Nil) => {
                self.output.push_str("null");
            }
            Expression::Identifier(name) => {
                if name == "print" {
                    self.output.push_str("console.log");
                } else {
                    self.output.push_str(name);
                }
            }
            Expression::StructInst { name, fields } => {
                self.output.push_str(&format!("new {}({{ ", name));
                for (i, (field_name, value)) in fields.iter().enumerate() {
                    if i > 0 { self.output.push_str(", "); }
                    self.output.push_str(field_name);
                    self.output.push_str(": ");
                    self.emit_expression(value);
                }
                self.output.push_str(" })");
            }
            Expression::MemberAccess { object, member, is_safe } => {
                self.emit_expression(object);
                if *is_safe {
                    self.output.push_str("?.");
                } else {
                    self.output.push('.');
                }
                self.output.push_str(member);
            }
            Expression::ErrorPropagate(expr) => {
                // Phase 2 naive representation: await execution
                // A real result unwrap would require an unwrap macro or error bubble wrapper
                self.output.push_str("await ");
                self.emit_expression(expr);
            }
            Expression::ArrayLiteral(elements) => {
                self.output.push('[');
                for (i, el) in elements.iter().enumerate() {
                    if i > 0 { self.output.push_str(", "); }
                    self.emit_expression(el);
                }
                self.output.push(']');
            }
            Expression::MapLiteral(pairs) => {
                self.output.push_str("{ ");
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 { self.output.push_str(", "); }
                    if let Expression::Literal(Literal::Str(s)) = key {
                        self.output.push_str(&format!("\"{}\"", s));
                    } else {
                        self.output.push('[');
                        self.emit_expression(key);
                        self.output.push(']');
                    }
                    self.output.push_str(": ");
                    self.emit_expression(value);
                }
                self.output.push_str(" }");
            }
            Expression::MatchExpr { expr, arms } => {
                self.output.push_str("(function() {\n");
                self.indent_level += 1;
                self.emit_indent();
                
                self.output.push_str("const __m = ");
                self.emit_expression(expr);
                self.output.push_str(";\n");
                
                for (pat, result_expr) in arms {
                    self.emit_indent();
                    match pat {
                        caria_parser::ast::Pattern::Wildcard => {
                            self.output.push_str("return await ");
                            self.emit_expression(result_expr);
                            self.output.push_str(";\n");
                        }
                        caria_parser::ast::Pattern::Literal(lit) => {
                            self.output.push_str("if (__m === ");
                            self.emit_expression(&Expression::Literal(lit.clone()));
                            self.output.push_str(") return await ");
                            self.emit_expression(result_expr);
                            self.output.push_str(";\n");
                        }
                        caria_parser::ast::Pattern::Tuple(patterns) => {
                            self.output.push_str("if (");
                            for (i, p) in patterns.iter().enumerate() {
                                if i > 0 { self.output.push_str(" && "); }
                                match p {
                                    caria_parser::ast::Pattern::Wildcard => {
                                        self.output.push_str("true");
                                    }
                                    caria_parser::ast::Pattern::Literal(lit) => {
                                        self.output.push_str(&format!("__m[{}] === ", i));
                                        self.emit_expression(&Expression::Literal(lit.clone()));
                                    }
                                    _ => self.output.push_str("true")
                                }
                            }
                            self.output.push_str(") return await ");
                            self.emit_expression(result_expr);
                            self.output.push_str(";\n");
                        }
                    }
                }
                
                self.emit_indent();
                self.output.push_str("throw new Error('Non-exhaustive match');\n");
                self.indent_level -= 1;
                self.emit_indent();
                self.output.push_str("})()");
            }
            Expression::CallExpr { callee, args } => {
                if let Expression::MemberAccess { object, member, is_safe } = &**callee {
                    if member == "len" && args.is_empty() {
                        self.emit_expression(object);
                        if *is_safe {
                            self.output.push_str("?.length");
                        } else {
                            self.output.push_str(".length");
                        }
                        return;
                    } else if member == "get" && args.len() == 1 {
                        self.emit_expression(object);
                        if *is_safe { self.output.push_str("?."); }
                        self.output.push('[');
                        self.emit_expression(&args[0]);
                        self.output.push(']');
                        return;
                    } else if member == "set" && args.len() == 2 {
                        self.output.push('(');
                        self.emit_expression(object);
                        self.output.push('[');
                        self.emit_expression(&args[0]);
                        self.output.push_str("] = ");
                        self.emit_expression(&args[1]);
                        self.output.push(')');
                        return;
                    } else if (member == "keys" || member == "values") && args.is_empty() {
                        if member == "keys" {
                            self.output.push_str("Object.keys(");
                        } else {
                            self.output.push_str("Object.values(");
                        }
                        self.emit_expression(object);
                        self.output.push(')');
                        return;
                    }
                }
                self.emit_expression(callee);
                self.output.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_expression(arg);
                }
                self.output.push(')');
            }
            Expression::BinaryExpr { left, op, right } => {
                self.emit_expression(left);
                let op_str = match op {
                    caria_parser::ast::BinaryOp::Add => " + ",
                    caria_parser::ast::BinaryOp::Subtract => " - ",
                    caria_parser::ast::BinaryOp::Multiply => " * ",
                    caria_parser::ast::BinaryOp::Divide => " / ",
                    caria_parser::ast::BinaryOp::Modulo => " % ",
                    caria_parser::ast::BinaryOp::Eq => " === ",
                    caria_parser::ast::BinaryOp::NotEq => " !== ",
                    caria_parser::ast::BinaryOp::Less => " < ",
                    caria_parser::ast::BinaryOp::Greater => " > ",
                    caria_parser::ast::BinaryOp::LessEq => " <= ",
                    caria_parser::ast::BinaryOp::GreaterEq => " >= ",
                    caria_parser::ast::BinaryOp::And => " && ",
                    caria_parser::ast::BinaryOp::Or => " || ",
                    caria_parser::ast::BinaryOp::Coalesce => " ?? ",
                };
                self.output.push_str(op_str);
                self.emit_expression(right);
            }
            Expression::IfExpr(expr) => {
                self.output.push_str("(() => {\n");
                self.emit_indent();
                self.output.push_str("if (");
                self.emit_expression(&expr.condition);
                self.output.push_str(") {\n");
                
                self.indent_level += 1;
                for (i, stmt) in expr.consequence.iter().enumerate() {
                    self.emit_indent();
                    if i == expr.consequence.len() - 1 {
                        if let Statement::ExprStmt(e) = stmt {
                            self.output.push_str("return await ");
                            self.emit_expression(e);
                            self.output.push_str(";\n");
                            continue;
                        }
                    }
                    self.emit_statement(stmt);
                    self.output.push('\n');
                }
                self.indent_level -= 1;
                self.emit_indent();
                self.output.push('}');

                if let Some(alternative) = &expr.alternative {
                    self.output.push_str(" else {\n");
                    self.indent_level += 1;
                    for (i, stmt) in alternative.iter().enumerate() {
                        self.emit_indent();
                        if i == alternative.len() - 1 {
                            if let Statement::ExprStmt(e) = stmt {
                                self.output.push_str("return await ");
                                self.emit_expression(e);
                                self.output.push_str(";\n");
                                continue;
                            }
                        }
                        self.emit_statement(stmt);
                        self.output.push('\n');
                    }
                    self.indent_level -= 1;
                    self.emit_indent();
                    self.output.push('}');
                }
                self.output.push('\n');
                self.indent_level -= 1;
                self.emit_indent();
            }
            Expression::QueryExpr { sql, params, .. } => {
                self.output.push_str(&format!("db.prepare(`{}`).all(", sql));
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { self.output.push_str(", "); }
                    self.emit_expression(p);
                }
                self.output.push(')');
            }
            Expression::StateExpr { initial } => {
                self.output.push_str("(function() { let __state = ");
                self.emit_expression(initial);
                self.output.push_str("; return { get: () => __state, set: (v) => { __state = v; } }; })()");
            }
        }
    }


    fn emit_route_decl(&mut self, decl: &RouteDecl) {
        if !self.app_initialized {
            self.output.push_str("const app = new Hono();\n");
            self.app_initialized = true;
        }
        
        // Generate: app.get("/path", async (req) => { ... })
        let method = decl.method.to_lowercase();
        let params_str: Vec<String> = decl.handler.params.iter().map(|p| p.name.clone()).collect();
        let params_joined = if params_str.is_empty() { "c".into() } else { params_str.join(", ") };
        self.output.push_str(&format!("app.{}('{}', async ({}) => {{\n", method, decl.path, params_joined));
        self.indent_level += 1;
        for stmt in &decl.handler.body {
            self.emit_statement(stmt);
            self.output.push('\n');
        }
        self.indent_level -= 1;
        self.output.push_str("});\n");
    }
    fn emit_serve_stmt(&mut self, stmt: &ServeStmt) {
        // For Hono + @hono/node-server
        self.output.push_str(&format!(
            "\nserve({{ fetch: app.fetch, port: {} }});\n",
            stmt.port
        ));
    }

    fn emit_view_decl(&mut self, decl: &ViewDecl) {
        let params_str: Vec<String> = decl.params.iter().map(|p| p.name.clone()).collect();
        let params_joined = if params_str.is_empty() { "{}".into() } else { format!("{{ {} }}", params_str.join(", ")) };
        self.output.push_str(&format!("function {}({}) {{\n", decl.name, params_joined));
        self.indent_level += 1;
        self.emit_indent();
        self.output.push_str("return `\n");
        for node in &decl.body {
            self.emit_html_node(node);
        }
        self.output.push_str("`;\n");
        self.indent_level -= 1;
        self.output.push_str("}\n");
    }

    fn emit_html_node(&mut self, node: &HtmlNode) {
        match node {
            HtmlNode::Element { tag, attrs, children, self_closing } => {
                self.output.push_str(&format!("<{}", tag));
                for (name, val) in attrs {
                    match val {
                        HtmlAttrValue::Static(s) => self.output.push_str(&format!(" {}=\"{}\"", name, s)),
                        HtmlAttrValue::Expr(expr) => {
                            self.output.push_str(&format!(" {}=\"", name));
                            // Can't easily emit expression inside template string here
                            // We use ${ } interpolation
                            self.output.push_str("${");
                            self.emit_expression(expr);
                            self.output.push_str("}\"");
                        }
                    }
                }
                if *self_closing {
                    self.output.push_str(" />");
                } else {
                    self.output.push('>');
                    for child in children {
                        self.emit_html_node(child);
                    }
                    self.output.push_str(&format!("</{}>", tag));
                }
            }
            HtmlNode::Text(s) => {
                self.output.push_str(s);
            }
            HtmlNode::Expr(expr) => {
                self.output.push_str("${");
                self.emit_expression(expr);
                self.output.push('}');
            }
            HtmlNode::ForNode { item, collection, children } => {
                self.output.push_str("${(");
                self.emit_expression(collection);
                self.output.push_str(&format!(").map(({}) => `", item));
                for child in children {
                    self.emit_html_node(child);
                }
                self.output.push_str("`).join('')}");
            }
            HtmlNode::IfNode { condition, then, else_ } => {
                self.output.push_str("${(");
                self.emit_expression(condition);
                self.output.push_str(") ? `");
                for child in then {
                    self.emit_html_node(child);
                }
                self.output.push_str("` : `");
                if let Some(else_nodes) = else_ {
                    for child in else_nodes {
                        self.emit_html_node(child);
                    }
                }
                self.output.push_str("`}");
            }
        }
    }
    fn emit_indent(&mut self) {
        for _ in 0..self.indent_level {
            self.output.push_str("    ");
        }
    }
}

mod tests {
    use super::*;
    use caria_parser::ast::*;

    #[test]
    fn test_codegen_let() {
        let prog = Program {
            statements: vec![Statement::LetStmt(LetStmt {
                name: "x".to_string(),
                value: Expression::Literal(Literal::Int(42)),
            })],
        };

        let mut codegen = JsCodegen::new();
        let js = codegen.generate(&prog);
        assert_eq!(js.trim(), "const x = 42;");
    }

    #[test]
    fn test_codegen_if_else() {
        let mut codegen = JsCodegen::new();
        let prog = Program {
            statements: vec![Statement::IfExpr(IfExpr {
                condition: Box::new(Expression::Literal(Literal::Bool(true))),
                consequence: vec![Statement::ExprStmt(Expression::Identifier("x".to_string()))],
                alternative: None,
            })],
        };
        let js = codegen.generate(&prog);
        assert_eq!(js.trim(), "if (true) {\n    x;\n}");
    }
}