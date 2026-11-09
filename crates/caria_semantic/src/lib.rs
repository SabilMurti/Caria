pub mod scope;
pub mod checker;

use caria_parser::ast::Program;
use checker::TypeChecker;

#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    UndefinedVariable(String),
    TypeMismatch {
        expected: String,
        found: String,
        context: String,
    },
    CannotCallNonFunction(String),
    ArgumentCountMismatch {
        expected: usize,
        found: usize,
        callee: String,
    },
    InvalidOperation {
        op: String,
        left: String,
        right: String,
    },
    UnknownMember {
        member: String,
        on_type: String,
    },
    CannotAssignToImmutable(String),
    InvalidScope,
}

pub fn analyze(program: &Program) -> Result<(), Vec<SemanticError>> {
    let mut checker = TypeChecker::new();
    checker.check_program(program)
}
