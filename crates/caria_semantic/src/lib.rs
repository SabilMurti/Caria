pub mod scope;
pub mod checker;

use caria_parser::ast::Program;
use checker::TypeChecker;

#[derive(Debug, Clone)]
pub enum SemanticError {
    UndefinedVariable(String),
    TypeMismatch(String, String),
    InvalidScope,
}

pub fn analyze(program: &Program) -> Result<(), Vec<SemanticError>> {
    let mut checker = TypeChecker::new();
    checker.check_program(program)
}
