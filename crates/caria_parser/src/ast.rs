#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
    FnDecl(FnDecl),
    LetStmt(LetStmt),
    MutStmt(MutStmt),
    AssignStmt(AssignStmt),
    ExprStmt(Expression),
    IfExpr(IfExpr),
    WhileStmt(WhileStmt),
    StructDecl(StructDecl),
    EnumDecl(EnumDecl),
}

#[derive(Debug, PartialEq, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub data: Option<Vec<StructField>>, // Similar to struct fields
}

#[derive(Debug, PartialEq, Clone)]
pub struct WhileStmt {
    pub condition: Box<Expression>,
    pub body: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct StructField {
    pub name: String,
    pub type_name: String, // Simplified type representation for now
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfExpr {
    pub condition: Box<Expression>,
    pub consequence: Vec<Statement>,
    pub alternative: Option<Vec<Statement>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MutStmt {
    pub name: String,
    pub value: Expression,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AssignStmt {
    pub name: String,
    pub value: Expression,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<FnParam>,
    pub return_type: Option<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnParam {
    pub name: String,
    pub type_name: String,
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
    IfExpr(IfExpr),
    StructInst {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    MemberAccess {
        object: Box<Expression>,
        member: String,
        is_safe: bool, // true for ?.
    },
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
    Nil,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Eq,
    NotEq,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    And,
    Or,
    Coalesce, // ??
}