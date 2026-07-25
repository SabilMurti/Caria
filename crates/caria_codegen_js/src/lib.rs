use caria_parser::ast::{AssignStmt, Expression, FnDecl, IfExpr, LetStmt, Literal, MutStmt, Program, Statement, StructDecl, WhileStmt};

pub struct JsCodegen {
    output: String,
    indent_level: usize,
}

impl JsCodegen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
        }
    }

    pub fn generate(&mut self, program: &Program) -> String {
        for stmt in &program.statements {
            self.emit_statement(stmt);
            self.output.push('\n');
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
                    self.output.push_str("import { ");
                    self.output.push_str(&decl.symbols.join(", "));
                    self.output.push_str(&format!(" }} from '{}';\n", decl.module));
                } else {
                    self.output.push_str("import { ");
                    self.output.push_str(&decl.symbols.join(", "));
                    self.output.push_str(&format!(" }} from './{}.js';\n", decl.module.replace(".", "/")));
                }
            }
            Statement::ExprStmt(expr) => {
                self.emit_indent();
                self.emit_expression(expr);
                self.output.push(';');
            }
        }
    }

    fn emit_fn_decl(&mut self, decl: &FnDecl) {
        self.emit_indent();
        self.output.push_str(&format!("function {}() {{\n", decl.name));
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
        self.output.push_str(&format!("const {} = ", stmt.name));
        self.emit_expression(&stmt.value);
        self.output.push(';');
    }

    fn emit_mut_stmt(&mut self, stmt: &MutStmt) {
        self.emit_indent();
        self.output.push_str(&format!("let {} = ", stmt.name));
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
                    self.emit_expression(key);
                    self.output.push_str(": ");
                    self.emit_expression(value);
                }
                self.output.push_str(" }");
            }
            Expression::CallExpr { callee, args } => {
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
                            self.output.push_str("return ");
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
                                self.output.push_str("return ");
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