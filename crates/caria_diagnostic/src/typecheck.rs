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
            // V2: ConstDecl (immutable, no keyword)
            Statement::ConstDecl(decl) => {
                let inferred_type = self.infer_expression(&decl.value)?;
                self.env.insert(decl.name.clone(), inferred_type);
            }
            // V2: MutDecl (mutable, := operator)
            Statement::MutDecl(decl) => {
                let inferred_type = self.infer_expression(&decl.value)?;
                self.env.insert(decl.name.clone(), inferred_type);
            }
            Statement::AssignStmt(assign) => {
                let val_type = self.infer_expression(&assign.value)?;
                if let Some(target_type) = self.env.get(&assign.name) {
                    if target_type.0 != "Any" && val_type.0 != "Any" && target_type.0 != val_type.0 {
                        return Err(TypeError {
                            message: format!(
                                "Type mismatch: Cannot assign value of type '{}' to variable '{}' of type '{}'.",
                                val_type.0, assign.name, target_type.0
                            )
                        });
                    }
                }
            }
            // V2: RuneDecl (replaces FnDecl)
            Statement::RuneDecl(rune) => {
                use caria_parser::ast::RuneBody;
                match &rune.body {
                    RuneBody::Block(stmts) => {
                        for body_stmt in stmts {
                            self.check_statement(body_stmt)?;
                        }
                    }
                    RuneBody::Inline(expr) => {
                        self.infer_expression(expr)?;
                    }
                }
            }
            Statement::IfStmt(if_expr) => {
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
            Statement::UnlessStmt(u) => {
                self.infer_expression(&u.condition)?;
                for s in &u.body {
                    self.check_statement(s)?;
                }
            }
            Statement::GuardStmt(g) => {
                self.infer_expression(&g.condition)?;
                for s in &g.fallback {
                    self.check_statement(s)?;
                }
            }
            Statement::WhileStmt(w) => {
                self.infer_expression(&w.condition)?;
                for s in &w.body {
                    self.check_statement(s)?;
                }
            }
            Statement::ForRangeStmt(f) => {
                self.infer_expression(&f.start)?;
                self.infer_expression(&f.end)?;
                for s in &f.body {
                    self.check_statement(s)?;
                }
            }
            Statement::ForInStmt(f) => {
                self.infer_expression(&f.collection)?;
                for s in &f.body {
                    self.check_statement(s)?;
                }
            }
            Statement::RouteDecl(r) => {
                for s in &r.body {
                    self.check_statement(s)?;
                }
            }
            Statement::ModelDecl(_) => {
                // Models are schema declarations — type checked separately at schema-level
            }
            Statement::ParallelBlock(p) => {
                for s in &p.body {
                    self.check_statement(s)?;
                }
            }
            Statement::RetryBlock(r) => {
                for s in &r.body {
                    self.check_statement(s)?;
                }
            }
            Statement::TransactionBlock(t) => {
                for s in &t.body {
                    self.check_statement(s)?;
                }
            }
            Statement::StreamDecl(s) => {
                for stmt in &s.body {
                    self.check_statement(stmt)?;
                }
            }
            Statement::ViewDecl(v) => {
                let _ = v; // HTML views checked at template level
            }
            Statement::ExprStmt(e) => {
                self.infer_expression(e)?;
            }
            Statement::RetStmt(e) => {
                self.infer_expression(e)?;
            }
            Statement::SayStmt(e) => {
                self.infer_expression(e)?;
            }
            Statement::DeferStmt(d) => {
                self.infer_expression(&d.expr)?;
            }
            Statement::QueueStmt(q) => {
                self.infer_expression(&q.call)?;
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
                    // Typo suggestion via Levenshtein distance
                    let mut closest = None;
                    let mut min_dist = usize::MAX;
                    for k in self.env.keys() {
                        let dist = levenshtein(name, k);
                        if dist < min_dist && dist <= 2 {
                            min_dist = dist;
                            closest = Some(k);
                        }
                    }
                    if let Some(c) = closest {
                        return Err(TypeError {
                            message: format!("Variable '{}' not found. Did you mean '{}'?", name, c)
                        });
                    }
                    Ok(("Any".into(), false))
                }
            }

            Expression::MemberAccess { object, member: _, is_safe } => {
                let (obj_type, is_nullable) = self.infer_expression(object)?;
                if is_nullable && !is_safe {
                    return Err(TypeError {
                        message: format!(
                            "Cannot access member of nullable type '{}?' without safe navigation operator '?.'",
                            obj_type
                        )
                    });
                }
                Ok(("Any".into(), *is_safe))
            }

            Expression::BinaryExpr { left, right, op } => {
                let left_type = self.infer_expression(left)?;
                let right_type = self.infer_expression(right)?;
                if left_type.0 != "Any" && right_type.0 != "Any" && left_type.0 != right_type.0 {
                    return Err(TypeError {
                        message: format!(
                            "Type mismatch in binary operation '{:?}': Left type '{}', right type '{}'.",
                            op, left_type.0, right_type.0
                        )
                    });
                }
                Ok((left_type.0, left_type.1 || right_type.1))
            }

            Expression::UnaryExpr { operand, .. } => {
                self.infer_expression(operand)
            }

            Expression::PipeExpr { left, right } => {
                self.infer_expression(left)?;
                self.infer_expression(right)
            }

            Expression::CallExpr { callee, args } => {
                self.infer_expression(callee)?;
                for arg in args {
                    self.infer_expression(arg)?;
                }
                Ok(("Any".into(), false))
            }

            Expression::WhenExpr { subject, arms } => {
                self.infer_expression(subject)?;
                for arm in arms {
                    self.infer_expression(&arm.body)?;
                }
                Ok(("Any".into(), false))
            }

            Expression::IfExpr(if_expr) => {
                self.infer_expression(&if_expr.condition)?;
                Ok(("Any".into(), false))
            }

            Expression::CastExpr { value, .. } => {
                self.infer_expression(value)?;
                Ok(("Any".into(), false))
            }

            Expression::IndexExpr { object, index } => {
                self.infer_expression(object)?;
                self.infer_expression(index)?;
                Ok(("Any".into(), false))
            }

            Expression::NilAssignExpr { default, .. } => {
                self.infer_expression(default)
            }

            Expression::RangeExpr { start, end } => {
                self.infer_expression(start)?;
                self.infer_expression(end)?;
                Ok(("Range".into(), false))
            }

            Expression::ClosureExpr { .. } => Ok(("Function".into(), false)),
            Expression::ArrayLiteral(_) => Ok(("List".into(), false)),
            Expression::MapLiteral(_) => Ok(("Map".into(), false)),
            Expression::StructInst { .. } => Ok(("Struct".into(), false)),
            Expression::ErrorPropagate(_) => Ok(("Any".into(), false)),
            Expression::EnvExpr(_) => Ok(("Str".into(), false)),
        }
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let mut cache = vec![0; b.len() + 1];
    for i in 1..=b.len() {
        cache[i] = i;
    }
    for (i, ca) in a.chars().enumerate() {
        let mut prev = i;
        cache[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let old = cache[j + 1];
            cache[j + 1] = std::cmp::min(
                std::cmp::min(cache[j + 1] + 1, cache[j] + 1),
                prev + if ca == cb { 0 } else { 1 },
            );
            prev = old;
        }
    }
    cache[b.len()]
}
