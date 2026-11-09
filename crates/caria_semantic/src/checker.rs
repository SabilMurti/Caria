use caria_parser::ast::{
    Program, Statement, Expression, Literal, BinaryOp, UnaryOp, RuneDecl, RuneBody
};
use crate::scope::{Scope, TypeInfo};
use crate::SemanticError;

pub struct TypeChecker {
    scope: Scope,
    current_return_type: Option<TypeInfo>,
    errors: Vec<SemanticError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            scope: Scope::new(),
            current_return_type: None,
            errors: Vec::new(),
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        // First pass: register all top-level functions in scope
        for stmt in &program.statements {
            if let Statement::RuneDecl(decl) = stmt {
                let ret_ty = decl.return_type.as_ref()
                    .map(TypeInfo::from_ast_type)
                    .unwrap_or(TypeInfo::Void);
                let param_types: Vec<TypeInfo> = decl.params.iter()
                    .map(|p| TypeInfo::from_ast_type(&p.type_expr))
                    .collect();
                self.scope.insert(
                    decl.name.clone(),
                    TypeInfo::Function {
                        params: param_types,
                        ret: Box::new(ret_ty),
                    },
                    false,
                );
            }
        }

        // Second pass: full statement and expression analysis
        for stmt in &program.statements {
            self.check_statement(stmt);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    pub fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::ConstDecl(decl) => {
                let val_ty = self.check_expression(&decl.value);
                if let Some(hint) = &decl.type_hint {
                    let expected = TypeInfo::from_ast_type(hint);
                    if !expected.is_compatible_with(&val_ty) {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: expected.display_name(),
                            found: val_ty.display_name(),
                            context: format!("const '{}' declaration", decl.name),
                        });
                    }
                    self.scope.insert(decl.name.clone(), expected, false);
                } else {
                    self.scope.insert(decl.name.clone(), val_ty, false);
                }
            }
            Statement::MutDecl(decl) => {
                let val_ty = self.check_expression(&decl.value);
                if let Some(hint) = &decl.type_hint {
                    let expected = TypeInfo::from_ast_type(hint);
                    if !expected.is_compatible_with(&val_ty) {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: expected.display_name(),
                            found: val_ty.display_name(),
                            context: format!("mutable ':=' '{}' declaration", decl.name),
                        });
                    }
                    self.scope.insert(decl.name.clone(), expected, true);
                } else {
                    self.scope.insert(decl.name.clone(), val_ty, true);
                }
            }
            Statement::AssignStmt(assign) => {
                let val_ty = self.check_expression(&assign.value);
                if let Some((existing_ty, is_mutable)) = self.scope.lookup(&assign.name) {
                    if !is_mutable {
                        self.errors.push(SemanticError::CannotAssignToImmutable(assign.name.clone()));
                    } else if !existing_ty.is_compatible_with(&val_ty) {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: existing_ty.display_name(),
                            found: val_ty.display_name(),
                            context: format!("assignment to variable '{}'", assign.name),
                        });
                    }
                } else {
                    self.errors.push(SemanticError::UndefinedVariable(assign.name.clone()));
                }
            }
            Statement::NullAssignStmt { name, value } => {
                let val_ty = self.check_expression(value);
                if let Some((existing_ty, is_mutable)) = self.scope.lookup(name) {
                    if !is_mutable {
                        self.errors.push(SemanticError::CannotAssignToImmutable(name.clone()));
                    } else if !existing_ty.is_compatible_with(&val_ty) {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: existing_ty.display_name(),
                            found: val_ty.display_name(),
                            context: format!("null-coalescing assignment to '{}'", name),
                        });
                    }
                } else {
                    self.errors.push(SemanticError::UndefinedVariable(name.clone()));
                }
            }
            Statement::RuneDecl(decl) => {
                self.check_rune(decl);
            }
            Statement::RetStmt(expr) => {
                let ret_ty = self.check_expression(expr);
                if let Some(expected_ret) = &self.current_return_type {
                    if !expected_ret.is_compatible_with(&ret_ty) {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: expected_ret.display_name(),
                            found: ret_ty.display_name(),
                            context: "return statement".to_string(),
                        });
                    }
                }
            }
            Statement::IfStmt(if_expr) => {
                let cond_ty = self.check_expression(&if_expr.condition);
                if !cond_ty.is_compatible_with(&TypeInfo::Bool) && cond_ty != TypeInfo::Unknown {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: "flag".to_string(),
                        found: cond_ty.display_name(),
                        context: "if condition".to_string(),
                    });
                }
                for s in &if_expr.consequence {
                    self.check_statement(s);
                }
                if let Some(alt) = &if_expr.alternative {
                    for s in alt {
                        self.check_statement(s);
                    }
                }
            }
            Statement::UnlessStmt(unless) => {
                let cond_ty = self.check_expression(&unless.condition);
                if !cond_ty.is_compatible_with(&TypeInfo::Bool) && cond_ty != TypeInfo::Unknown {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: "flag".to_string(),
                        found: cond_ty.display_name(),
                        context: "unless condition".to_string(),
                    });
                }
                for s in &unless.body {
                    self.check_statement(s);
                }
            }
            Statement::WhileStmt(while_stmt) => {
                let cond_ty = self.check_expression(&while_stmt.condition);
                if !cond_ty.is_compatible_with(&TypeInfo::Bool) && cond_ty != TypeInfo::Unknown {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: "flag".to_string(),
                        found: cond_ty.display_name(),
                        context: "while condition".to_string(),
                    });
                }
                for s in &while_stmt.body {
                    self.check_statement(s);
                }
            }
            Statement::ForRangeStmt(range) => {
                let start_ty = self.check_expression(&range.start);
                let end_ty = self.check_expression(&range.end);
                if !start_ty.is_compatible_with(&TypeInfo::Int) {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: "int".to_string(),
                        found: start_ty.display_name(),
                        context: "for-range start bound".to_string(),
                    });
                }
                if !end_ty.is_compatible_with(&TypeInfo::Int) {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: "int".to_string(),
                        found: end_ty.display_name(),
                        context: "for-range end bound".to_string(),
                    });
                }
                self.enter_scope();
                self.scope.insert(range.var.clone(), TypeInfo::Int, false);
                for s in &range.body {
                    self.check_statement(s);
                }
                self.exit_scope();
            }
            Statement::ForInStmt(for_in) => {
                let col_ty = self.check_expression(&for_in.collection);
                let item_ty = match col_ty {
                    TypeInfo::List(inner) => *inner,
                    TypeInfo::String => TypeInfo::String,
                    _ => TypeInfo::Unknown,
                };
                self.enter_scope();
                self.scope.insert(for_in.item_var.clone(), item_ty, false);
                if let Some(idx_var) = &for_in.index_var {
                    self.scope.insert(idx_var.clone(), TypeInfo::Int, false);
                }
                for s in &for_in.body {
                    self.check_statement(s);
                }
                self.exit_scope();
            }
            Statement::GuardStmt(guard) => {
                let cond_ty = self.check_expression(&guard.condition);
                if !cond_ty.is_compatible_with(&TypeInfo::Bool) && cond_ty != TypeInfo::Unknown {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: "flag".to_string(),
                        found: cond_ty.display_name(),
                        context: "guard condition".to_string(),
                    });
                }
                for s in &guard.fallback {
                    self.check_statement(s);
                }
            }
            Statement::SayStmt(expr) => {
                self.check_expression(expr);
            }
            Statement::SayLevel { args, .. } => {
                for arg in args {
                    self.check_expression(arg);
                }
            }
            Statement::ExprStmt(expr) => {
                self.check_expression(expr);
            }
            Statement::ParallelBlock(p) => {
                for s in &p.body {
                    self.check_statement(s);
                }
            }
            Statement::DeferStmt(d) => {
                self.check_expression(&d.expr);
            }
            Statement::RetryBlock(r) => {
                self.check_expression(&r.count);
                for s in &r.body {
                    self.check_statement(s);
                }
            }
            Statement::TestDecl(t) => {
                self.enter_scope();
                for s in &t.body {
                    self.check_statement(s);
                }
                self.exit_scope();
            }
            Statement::DestructureAssign { names, value } => {
                let val_ty = self.check_expression(value);
                let elem_ty = match val_ty {
                    TypeInfo::List(inner) => *inner,
                    _ => TypeInfo::Unknown,
                };
                for name in names {
                    self.scope.insert(name.clone(), elem_ty.clone(), false);
                }
            }
            Statement::Summon(_) | Statement::FallthroughStmt => {}
        }
    }

    fn check_rune(&mut self, decl: &RuneDecl) {
        let expected_ret = decl.return_type.as_ref()
            .map(TypeInfo::from_ast_type)
            .unwrap_or(TypeInfo::Void);

        let prev_ret = self.current_return_type.replace(expected_ret);
        self.enter_scope();

        for param in &decl.params {
            let param_ty = TypeInfo::from_ast_type(&param.type_expr);
            self.scope.insert(param.name.clone(), param_ty, false);
        }

        match &decl.body {
            RuneBody::Block(stmts) => {
                for s in stmts {
                    self.check_statement(s);
                }
            }
            RuneBody::Inline(expr) => {
                let expr_ty = self.check_expression(expr);
                if let Some(expected) = &self.current_return_type {
                    if !expected.is_compatible_with(&expr_ty) {
                        self.errors.push(SemanticError::TypeMismatch {
                            expected: expected.display_name(),
                            found: expr_ty.display_name(),
                            context: format!("inline rune '{}' body", decl.name),
                        });
                    }
                }
            }
        }

        self.exit_scope();
        self.current_return_type = prev_ret;
    }

    pub fn check_expression(&mut self, expr: &Expression) -> TypeInfo {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Int(_) => TypeInfo::Int,
                Literal::Float(_) => TypeInfo::Float,
                Literal::Str(_) => TypeInfo::String,
                Literal::Bool(_) => TypeInfo::Bool,
                Literal::Nil => TypeInfo::Nil,
                Literal::Unit(_, _) => TypeInfo::Int,
            },
            Expression::Identifier(name) => {
                if let Some((ty, _)) = self.scope.lookup(name) {
                    ty
                } else {
                    self.errors.push(SemanticError::UndefinedVariable(name.clone()));
                    TypeInfo::Unknown
                }
            }
            Expression::BinaryExpr { left, op, right } => {
                let l_ty = self.check_expression(left);
                let r_ty = self.check_expression(right);
                match op {
                    BinaryOp::Add => {
                        if l_ty == TypeInfo::String || r_ty == TypeInfo::String {
                            TypeInfo::String
                        } else if l_ty == TypeInfo::Float || r_ty == TypeInfo::Float {
                            TypeInfo::Float
                        } else {
                            TypeInfo::Int
                        }
                    }
                    BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                        if l_ty == TypeInfo::Float || r_ty == TypeInfo::Float {
                            TypeInfo::Float
                        } else {
                            TypeInfo::Int
                        }
                    }
                    BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEq | BinaryOp::GreaterEq => {
                        TypeInfo::Bool
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        TypeInfo::Bool
                    }
                    BinaryOp::Coalesce => {
                        if l_ty != TypeInfo::Nil && l_ty != TypeInfo::Unknown {
                            l_ty
                        } else {
                            r_ty
                        }
                    }
                    BinaryOp::Is => TypeInfo::Bool,
                }
            }
            Expression::UnaryExpr { op, operand } => {
                let op_ty = self.check_expression(operand);
                match op {
                    UnaryOp::Not => TypeInfo::Bool,
                    UnaryOp::Neg => {
                        if op_ty == TypeInfo::Float {
                            TypeInfo::Float
                        } else {
                            TypeInfo::Int
                        }
                    }
                }
            }
            Expression::CallExpr { callee, args } => {
                let arg_types: Vec<TypeInfo> = args.iter().map(|a| self.check_expression(a)).collect();
                if let Expression::Identifier(name) = callee.as_ref() {
                    // Check built-in functions
                    match name.as_str() {
                        "say" | "print" => return TypeInfo::Int,
                        "len" => return TypeInfo::Int,
                        "str" => return TypeInfo::String,
                        "int" => return TypeInfo::Int,
                        "dec" => return TypeInfo::Float,
                        _ => {}
                    }
                }

                let callee_ty = self.check_expression(callee);
                match callee_ty {
                    TypeInfo::Function { params, ret } => {
                        if params.len() != arg_types.len() {
                            self.errors.push(SemanticError::ArgumentCountMismatch {
                                expected: params.len(),
                                found: arg_types.len(),
                                callee: format!("{:?}", callee),
                            });
                        } else {
                            for (p, a) in params.iter().zip(arg_types.iter()) {
                                if !p.is_compatible_with(a) {
                                    self.errors.push(SemanticError::TypeMismatch {
                                        expected: p.display_name(),
                                        found: a.display_name(),
                                        context: "function call argument".to_string(),
                                    });
                                }
                            }
                        }
                        *ret
                    }
                    TypeInfo::Unknown => TypeInfo::Unknown,
                    other => {
                        self.errors.push(SemanticError::CannotCallNonFunction(other.display_name()));
                        TypeInfo::Unknown
                    }
                }
            }
            Expression::MemberAccess { object, member, .. } => {
                let obj_ty = self.check_expression(object);
                match obj_ty {
                    TypeInfo::String => match member.as_str() {
                        "len" | "count" | "index_of" | "last_index_of" => TypeInfo::Int,
                        "trim" | "lower" | "upper" | "trim_left" | "trim_right" | "substring" | "replace" => {
                            TypeInfo::String
                        }
                        "contains" | "is_empty" => TypeInfo::Bool,
                        "split" => TypeInfo::List(Box::new(TypeInfo::String)),
                        _ => {
                            self.errors.push(SemanticError::UnknownMember {
                                member: member.clone(),
                                on_type: "txt".to_string(),
                            });
                            TypeInfo::Unknown
                        }
                    },
                    TypeInfo::List(inner) => match member.as_str() {
                        "len" | "count" => TypeInfo::Int,
                        "first" | "last" | "pop" => *inner,
                        "is_empty" => TypeInfo::Bool,
                        _ => TypeInfo::Unknown,
                    },
                    TypeInfo::Map(k, v) => match member.as_str() {
                        "len" | "count" => TypeInfo::Int,
                        "keys" => TypeInfo::List(k),
                        "values" => TypeInfo::List(v),
                        "is_empty" => TypeInfo::Bool,
                        _ => TypeInfo::Unknown,
                    },
                    _ => TypeInfo::Unknown,
                }
            }
            Expression::IndexExpr { object, index } => {
                let obj_ty = self.check_expression(object);
                let _idx_ty = self.check_expression(index);
                match obj_ty {
                    TypeInfo::List(inner) => *inner,
                    TypeInfo::Map(_, v) => *v,
                    TypeInfo::String => TypeInfo::String,
                    _ => TypeInfo::Unknown,
                }
            }
            Expression::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    TypeInfo::List(Box::new(TypeInfo::Unknown))
                } else {
                    let first_ty = self.check_expression(&elements[0]);
                    for elem in &elements[1..] {
                        let elem_ty = self.check_expression(elem);
                        if !first_ty.is_compatible_with(&elem_ty) {
                            self.errors.push(SemanticError::TypeMismatch {
                                expected: first_ty.display_name(),
                                found: elem_ty.display_name(),
                                context: "array element type consistency".to_string(),
                            });
                        }
                    }
                    TypeInfo::List(Box::new(first_ty))
                }
            }
            Expression::MapLiteral(pairs) => {
                if pairs.is_empty() {
                    TypeInfo::Map(Box::new(TypeInfo::Unknown), Box::new(TypeInfo::Unknown))
                } else {
                    let k_ty = self.check_expression(&pairs[0].0);
                    let v_ty = self.check_expression(&pairs[0].1);
                    TypeInfo::Map(Box::new(k_ty), Box::new(v_ty))
                }
            }
            Expression::PipeExpr { left, right } => {
                let _l_ty = self.check_expression(left);
                self.check_expression(right)
            }
            Expression::TernaryExpr { condition, true_expr, false_expr } => {
                let cond_ty = self.check_expression(condition);
                if !cond_ty.is_compatible_with(&TypeInfo::Bool) && cond_ty != TypeInfo::Unknown {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: "flag".to_string(),
                        found: cond_ty.display_name(),
                        context: "ternary condition".to_string(),
                    });
                }
                let t_ty = self.check_expression(true_expr);
                let _f_ty = self.check_expression(false_expr);
                t_ty
            }
            Expression::IfExpr(if_expr) => {
                let cond_ty = self.check_expression(&if_expr.condition);
                if !cond_ty.is_compatible_with(&TypeInfo::Bool) && cond_ty != TypeInfo::Unknown {
                    self.errors.push(SemanticError::TypeMismatch {
                        expected: "flag".to_string(),
                        found: cond_ty.display_name(),
                        context: "if-expression condition".to_string(),
                    });
                }
                TypeInfo::Unknown
            }
            Expression::WhenExpr { subject, arms } => {
                let _subj_ty = self.check_expression(subject);
                let mut arm_ty = TypeInfo::Unknown;
                for arm in arms {
                    arm_ty = self.check_expression(&arm.body);
                }
                arm_ty
            }
            Expression::CastExpr { target_type, .. } | Expression::NullableCast { target_type, .. } => {
                TypeInfo::from_ast_type(target_type)
            }
            Expression::EnvExpr(_) => TypeInfo::String,
            Expression::RangeExpr { start, end } => {
                self.check_expression(start);
                self.check_expression(end);
                TypeInfo::Range
            }
            Expression::ErrorPropagate(inner) => {
                self.check_expression(inner)
            }
            Expression::NilAssignExpr { target, default } => {
                let def_ty = self.check_expression(default);
                if let Some((existing_ty, _)) = self.scope.lookup(target) {
                    existing_ty
                } else {
                    def_ty
                }
            }
            Expression::ClosureExpr { params, body } => {
                self.enter_scope();
                for p in params {
                    self.scope.insert(p.clone(), TypeInfo::Unknown, false);
                }
                for s in body {
                    self.check_statement(s);
                }
                self.exit_scope();
                TypeInfo::Function {
                    params: vec![TypeInfo::Unknown; params.len()],
                    ret: Box::new(TypeInfo::Unknown),
                }
            }
        }
    }

    fn enter_scope(&mut self) {
        let current = std::mem::replace(&mut self.scope, Scope::new());
        self.scope = Scope::new_child(current);
    }

    fn exit_scope(&mut self) {
        if let Some(parent) = self.scope.pop_parent() {
            self.scope = parent;
        }
    }
}
