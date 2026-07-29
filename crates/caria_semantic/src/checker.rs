use caria_parser::ast::{Program, Statement, Expression};
use crate::scope::{Scope, TypeInfo};
use crate::SemanticError;

pub struct TypeChecker {
    scope: Scope,
    errors: Vec<SemanticError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            scope: Scope::new(),
            errors: Vec::new(),
        }
    }

    pub fn check_program(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        for stmt in &program.statements {
            self.check_statement(stmt);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::MutDecl(decl) => {
                self.scope.insert(decl.name.clone(), TypeInfo::Unknown);
                self.check_expression(&decl.value);
            }
            Statement::ConstDecl(decl) => {
                self.scope.insert(decl.name.clone(), TypeInfo::Unknown);
                self.check_expression(&decl.value);
            }
            Statement::RuneDecl(decl) => {
                self.scope.insert(decl.name.clone(), TypeInfo::Void); // Stub
            }
            // Add other statement types as needed...
            _ => {}
        }
    }

    fn check_expression(&mut self, expr: &Expression) -> TypeInfo {
        match expr {
            Expression::Literal(_) => TypeInfo::Unknown, // Stub
            Expression::Identifier(name) => {
                if let Some(ty) = self.scope.lookup(name) {
                    ty
                } else {
                    self.errors.push(SemanticError::UndefinedVariable(name.clone()));
                    TypeInfo::Unknown
                }
            }
            _ => TypeInfo::Unknown,
        }
    }
}
