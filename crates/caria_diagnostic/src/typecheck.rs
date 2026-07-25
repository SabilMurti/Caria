use caria_parser::ast::{Program, Statement, Expression, Literal};
use std::collections::HashMap;

#[derive(Debug)]
pub struct TypeError {
    pub message: String,
}

pub struct TypeChecker {
    // Map of variable name -> (type, is_nullable)
    env: HashMap<String, (String, bool)>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: HashMap::new(),
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), TypeError> {
        for stmt in &program.statements {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    fn check_statement(&mut self, stmt: &Statement) -> Result<(), TypeError> {
        match stmt {
            Statement::LetStmt(let_stmt) => {
                let inferred_type = self.infer_expression(&let_stmt.value)?;
                // Naive type inference: store variable
                self.env.insert(let_stmt.name.clone(), inferred_type);
            }
            Statement::MutStmt(mut_stmt) => {
                let inferred_type = self.infer_expression(&mut_stmt.value)?;
                self.env.insert(mut_stmt.name.clone(), inferred_type);
            }
            Statement::FnDecl(fn_decl) => {
                // If single-expression function (return type missing but body has RetStmt)
                // then we infer return type.
                for body_stmt in &fn_decl.body {
                    self.check_statement(body_stmt)?;
                }
            }
            Statement::IfExpr(if_expr) => {
                self.infer_expression(&if_expr.condition)?;
                for s in &if_expr.consequence {
                    self.check_statement(s)?;
                }
                if let Some(alt) = &if_expr.alternative {
                    for s in alt {
                        self.check_statement(s)?;
                    }
                }
            }
            Statement::WhileStmt(w) => {
                self.infer_expression(&w.condition)?;
                for s in &w.body {
                    self.check_statement(s)?;
                }
            }
            Statement::ForStmt(f) => {
                for s in &f.body {
                    self.check_statement(s)?;
                }
            }
            Statement::ForInStmt(f) => {
                for s in &f.body {
                    self.check_statement(s)?;
                }
            }
            Statement::ExprStmt(e) => {
                self.infer_expression(e)?;
            }
            Statement::RetStmt(e) => {
                self.infer_expression(e)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn infer_expression(&mut self, expr: &Expression) -> Result<(String, bool), TypeError> {
        match expr {
            Expression::Literal(Literal::Int(_)) => Ok(("Int".into(), false)),
            Expression::Literal(Literal::Float(_)) => Ok(("Float".into(), false)),
            Expression::Literal(Literal::Str(_)) => Ok(("Str".into(), false)),
            Expression::Literal(Literal::Bool(_)) => Ok(("Bool".into(), false)),
            Expression::Literal(Literal::Nil) => Ok(("Any".into(), true)),
            
            Expression::Identifier(name) => {
                if let Some(t) = self.env.get(name) {
                    Ok(t.clone())
                } else {
                    Ok(("Any".into(), false)) // Unbound var fallback
                }
            }
            
            Expression::MemberAccess { object, member: _, is_safe } => {
                let (obj_type, is_nullable) = self.infer_expression(object)?;
                
                // Compiler error when using T? without null check
                if is_nullable && !is_safe {
                    return Err(TypeError {
                        message: format!("Cannot access member of nullable type '{}?' without safe navigation operator '?.'", obj_type)
                    });
                }
                
                Ok(("Any".into(), *is_safe))
            }
            
            Expression::BinaryExpr { left, right, .. } => {
                self.infer_expression(left)?;
                self.infer_expression(right)?;
                Ok(("Any".into(), false))
            }
            
            Expression::CallExpr { callee, args } => {
                self.infer_expression(callee)?;
                for arg in args {
                    self.infer_expression(arg)?;
                }
                Ok(("Any".into(), false))
            }
            
            Expression::MatchExpr { expr, .. } => {
                self.infer_expression(expr)?;
                Ok(("Any".into(), false))
            }
            
            Expression::ClosureExpr { .. } => Ok(("Function".into(), false)),
            Expression::ArrayLiteral(_) => Ok(("List".into(), false)),
            Expression::MapLiteral(_) => Ok(("Map".into(), false)),
            Expression::ErrorPropagate(_) => Ok(("Any".into(), false)),
            Expression::StructInst { .. } => Ok(("Struct".into(), false)),
            Expression::IfExpr(_) => Ok(("Any".into(), false)),
        }
    }
}
