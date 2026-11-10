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
                let old_env = self.env.clone();
                for param in &rune.params {
                    self.env.insert(param.name.clone(), ("Any".to_string(), false));
                }
                let res = match &rune.body {
                    RuneBody::Block(stmts) => {
                        let mut r = Ok(());
                        for body_stmt in stmts {
                            if let Err(e) = self.check_statement(body_stmt) {
                                r = Err(e);
                                break;
                            }
                        }
                        r
                    }
                    RuneBody::Inline(expr) => {
                        self.infer_expression(expr).map(|_| ())
                    }
                };
                self.env = old_env;
                res?;
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
                let old_env = self.env.clone();
                self.env.insert(f.var.clone(), ("int".to_string(), false));
                let mut res = Ok(());
                for s in &f.body {
                    if let Err(e) = self.check_statement(s) {
                        res = Err(e);
                        break;
                    }
                }
                self.env = old_env;
                res?;
            }
            Statement::ForInStmt(f) => {
                self.infer_expression(&f.collection)?;
                let old_env = self.env.clone();
                self.env.insert(f.item_var.clone(), ("Any".to_string(), false));
                if let Some(idx) = &f.index_var {
                    self.env.insert(idx.clone(), ("int".to_string(), false));
                }
                let mut res = Ok(());
                for s in &f.body {
                    if let Err(e) = self.check_statement(s) {
                        res = Err(e);
                        break;
                    }
                }
                self.env = old_env;
                res?;
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
            _ => {}
        }
        Ok(())
    }

    fn infer_expression(&mut self, expr: &Expression) -> Result<(String, bool), TypeError> {
        match expr {
            Expression::Literal(Literal::Int(_)) => Ok(("int".into(), false)),
            Expression::Literal(Literal::Float(_)) => Ok(("dec".into(), false)),
            Expression::Literal(Literal::Str(_)) => Ok(("txt".into(), false)),
            Expression::Literal(Literal::Bool(_)) => Ok(("flag".into(), false)),
            Expression::Literal(Literal::Nil) => Ok(("Any".into(), true)),
            Expression::Literal(Literal::Unit(_, _)) => Ok(("Duration".into(), false)),

            Expression::Identifier(name) => {
                if let Some(t) = self.env.get(name) {
                    Ok(t.clone())
                } else {
                    // If it starts with uppercase, it's likely a Model/Class, don't typo-correct it to a lowercase variable
                    if name.chars().next().map_or(false, |c| c.is_uppercase()) {
                        return Ok((name.clone(), false));
                    }

                    // Typo suggestion via Levenshtein distance
                    if name.len() >= 3 {
                        let mut closest = None;
                        let mut min_dist = usize::MAX;
                        for k in self.env.keys() {
                            let dist = levenshtein(name, k);
                            if dist < min_dist && dist <= 2 && dist < name.len() {
                                min_dist = dist;
                                closest = Some(k);
                            }
                        }
                        if let Some(c) = closest {
                            return Err(TypeError {
                                message: format!("Variable '{}' not found. Did you mean '{}'?", name, c)
                            });
                        }
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
            Expression::ErrorPropagate(_) => Ok(("Any".into(), false)),
            Expression::EnvExpr(_) => Ok(("txt".into(), false)),
            Expression::NullableCast { target_type, .. } => {
                let name = match target_type {
                    caria_parser::ast::TypeExpr::Named(n) => n.clone(),
                    _ => "Any".into(),
                };
                Ok((name, false))
            }
            Expression::TernaryExpr { condition, true_expr, false_expr } => {
                self.infer_expression(condition)?;
                let true_type = self.infer_expression(true_expr)?;
                let false_type = self.infer_expression(false_expr)?;
                // Return the true branch type; in a real typechecker we'd unify both branches
                Ok((true_type.0, true_type.1 || false_type.1))
            }
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
