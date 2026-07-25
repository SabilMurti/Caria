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
    ForStmt(ForStmt),
    ForInStmt(ForInStmt),
    StructDecl(StructDecl),
    EnumDecl(EnumDecl),
    ModDecl(ModDecl),
    UseDecl(UseDecl),
    RetStmt(Expression),
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModDecl {
    pub name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct UseDecl {
    pub module: String,
    pub is_npm: bool,
    pub symbols: Vec<String>,
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
pub struct ForStmt {
    pub var: String,
    pub start: Box<Expression>,
    pub end: Box<Expression>,
    pub body: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ForInStmt {
    pub item_var: String,
    pub index_var: Option<String>,
    pub collection: Box<Expression>,
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
pub struct LetStmt {
    pub name: String,
    pub value: Expression,
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
    pub default_value: Option<Expression>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    ClosureExpr {
        params: Vec<String>,
        body: Vec<Statement>,
    },
    IfExpr(IfExpr),
    StructInst {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    ArrayLiteral(Vec<Expression>),
    MapLiteral(Vec<(Expression, Expression)>),
    ErrorPropagate(Box<Expression>), // x?
    MemberAccess {
        object: Box<Expression>,
        member: String,
        is_safe: bool, // true for ?.
    },
    MatchExpr {
        expr: Box<Expression>,
        arms: Vec<(Pattern, Expression)>,
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

#[derive(Debug, PartialEq, Clone)]
pub enum Pattern {
    Literal(Literal),
    Wildcard,
    Tuple(Vec<Pattern>),
}