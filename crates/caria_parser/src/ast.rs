#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
    FnDecl(FnDecl),
    LetStmt(LetStmt),
    ExprStmt(Expression),
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnDecl {
    pub name: String,
    pub body: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LetStmt {
    pub name: String,
    pub value: Expression,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    BinaryExpr {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    CallExpr {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}