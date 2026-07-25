use caria_parser::ast::{Expression, FnDecl, LetStmt, Literal, Program, Statement};

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

    fn emit_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Literal(Literal::Int(val)) => self.output.push_str(&val.to_string()),
            Expression::Literal(Literal::Float(val)) => self.output.push_str(&val.to_string()),
            Expression::Literal(Literal::Str(val)) => {
                // In a real implementation we would handle escaping.
                // For Phase 1, we just emit template literals to handle potential interpolation later.
                self.output.push_str(&format!("`{}`", val));
            }
            Expression::Literal(Literal::Bool(val)) => {
                self.output.push_str(if *val { "true" } else { "false" })
            }
            Expression::Identifier(name) => {
                // Hardcode standard library translation for Phase 1 `print` -> `console.log`
                if name == "print" {
                    self.output.push_str("console.log");
                } else {
                    self.output.push_str(name);
                }
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
                };
                self.output.push_str(op_str);
                self.emit_expression(right);
            }
        }
    }

    fn emit_indent(&mut self) {
        for _ in 0..self.indent_level {
            self.output.push_str("    ");
        }
    }
}

#[cfg(test)]
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
    fn test_codegen_fn() {
        let prog = Program {
            statements: vec![Statement::FnDecl(FnDecl {
                name: "main".to_string(),
                body: vec![Statement::LetStmt(LetStmt {
                    name: "msg".to_string(),
                    value: Expression::Literal(Literal::Str("hello".to_string())),
                })],
            })],
        };

        let mut codegen = JsCodegen::new();
        let js = codegen.generate(&prog);
        let expected = "function main() {\n    const msg = `hello`;\n}";
        assert_eq!(js.trim(), expected);
    }
}