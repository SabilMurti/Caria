// ── Calesteria V2 AST ────────────────────────────────────────────────────────
// The root node of a Calesteria program.
#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

// ── Top-level Statements ─────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
    // Include directive
    Include(String),        // include "path/file.cria"

    // Function / Rune
    RuneDecl(RuneDecl),

    // Variables
    ConstDecl(ConstDecl),   // name = value  (immutable, keyword-less)
    MutDecl(MutDecl),       // name := value (mutable)
    AssignStmt(AssignStmt), // name = value  (reassignment of existing mut)
    NullAssignStmt { name: String, value: Expression }, // name ?= value
    DestructureAssign { names: Vec<String>, value: Expression }, // a, b = value

    // Expressions used as statements
    ExprStmt(Expression),

    // Control Flow
    RetStmt(Expression),
    IfStmt(IfExpr),
    UnlessStmt(UnlessStmt),
    WhileStmt(WhileStmt),
    ForRangeStmt(ForRangeStmt),   // for i in 0..10 step 2
    ForInStmt(ForInStmt),         // for item in collection

    // Early-exit guard
    GuardStmt(GuardStmt),

    // Fallthrough for pattern matching
    FallthroughStmt,

    // HTTP Routing
    RouteDecl(RouteDecl),
    RouteGroup(RouteGroupStmt),
    ResourceRoute(ResourceRouteStmt),

    // Database / ORM
    ModelDecl(ModelDecl),

    // Async / Concurrency
    ParallelBlock(ParallelBlock),
    DeferStmt(DeferStmt),
    RetryBlock(RetryBlock),

    // Web Blocks
    ValidateBlock(ValidateBlock),
    TransactionBlock(TransactionBlock),
    QueueStmt(QueueStmt),
    StreamDecl(StreamDecl),

    // View / Template
    ViewDecl(ViewDecl),

    // Raw JS Escape
    JsBlock(JsBlock),

    // Output
    SayStmt(Expression),
    SayLevel { level: String, args: Vec<Expression> },  // say.info("msg")

    // Tests
    TestDecl(TestDecl),
}

// ── Rune (Function) ──────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub struct RuneDecl {
    pub name: String,
    pub params: Vec<RuneParam>,
    pub return_type: Option<TypeExpr>,
    pub body: RuneBody,
    pub is_public: bool,
    pub doc_comment: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RuneParam {
    pub name: String,
    pub type_expr: TypeExpr,
    pub default_value: Option<Expression>,
    pub is_variadic: bool,   // rune f(nums: int...)
}

/// Rune body: either a full block or an inline `=> expr` shorthand
#[derive(Debug, PartialEq, Clone)]
pub enum RuneBody {
    Block(Vec<Statement>),
    Inline(Box<Expression>),  // rune double(x: int) -> int => x * 2
}

// ── Type Expressions ─────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub enum TypeExpr {
    /// Primitive types: int, dec, txt, flag, nil
    Named(String),
    /// List type: [Type]
    List(Box<TypeExpr>),
    /// Map type: [Key:Value]
    Map(Box<TypeExpr>, Box<TypeExpr>),
    /// Optional / nullable type: Type?
    Optional(Box<TypeExpr>),
    /// Non-empty array type: [T]+
    NonEmptyArray(Box<TypeExpr>),
}

// ── Variable Declarations ─────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub type_hint: Option<TypeExpr>,
    pub value: Expression,
}

#[derive(Debug, PartialEq, Clone)]
pub struct MutDecl {
    pub name: String,
    pub type_hint: Option<TypeExpr>,
    pub value: Expression,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AssignStmt {
    pub name: String,
    pub value: Expression,
}

// ── Control Flow ──────────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub struct IfExpr {
    pub condition: Box<Expression>,
    pub consequence: Vec<Statement>,
    pub alternative: Option<Vec<Statement>>,  // else { } or else if
}

/// unless condition { body }  ≡  if !condition { body }
#[derive(Debug, PartialEq, Clone)]
pub struct UnlessStmt {
    pub condition: Box<Expression>,
    pub body: Vec<Statement>,
}

/// guard condition else { fallback }
#[derive(Debug, PartialEq, Clone)]
pub struct GuardStmt {
    pub condition: Box<Expression>,
    pub fallback: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct WhileStmt {
    pub condition: Box<Expression>,
    pub body: Vec<Statement>,
}

/// for i in start..end [step N]
#[derive(Debug, PartialEq, Clone)]
pub struct ForRangeStmt {
    pub var: String,
    pub start: Box<Expression>,
    pub end: Box<Expression>,
    pub step: Option<Box<Expression>>,
    pub body: Vec<Statement>,
    pub else_body: Option<Vec<Statement>>,  // for ... { } else { }
}

/// for item [, index] in collection
#[derive(Debug, PartialEq, Clone)]
pub struct ForInStmt {
    pub item_var: String,
    pub index_var: Option<String>,
    pub collection: Box<Expression>,
    pub body: Vec<Statement>,
    pub else_body: Option<Vec<Statement>>,  // for ... in ... { } else { }
}

// ── HTTP Routing ──────────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RouteDecl {
    pub method: HttpMethod,
    pub path: String,       // "/users/{id}"
    pub body: Vec<Statement>,
    pub decorators: Vec<RouteDecorator>,
    pub with_clauses: Vec<WithClause>,  // with rate_limit(100), cors("url")
}

/// Inline middleware: `with rate_limit(100), cors("url")` after route path
#[derive(Debug, PartialEq, Clone)]
pub struct WithClause {
    pub name: String,
    pub args: Vec<Expression>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct RouteGroupStmt {
    pub prefix: String,
    pub decorators: Vec<RouteDecorator>,
    pub body: Vec<Statement>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ResourceRouteStmt {
    pub path: String,
    pub controller: String,
    pub only: Option<Vec<String>>,
    pub except: Option<Vec<String>>,
}

/// Route-level decorators: @rate_limit, @auth, @cors, etc.
#[derive(Debug, PartialEq, Clone)]
pub struct RouteDecorator {
    pub name: String,
    pub args: Vec<Expression>,
}

// ── Model / ORM ───────────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub struct ModelDecl {
    pub name: String,
    pub fields: Vec<ModelField>,
    pub is_public: bool,
    pub doc_comment: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModelField {
    pub name: String,
    pub type_expr: TypeExpr,
    pub tags: Vec<String>,              // [pk, auto, unique, index, encrypted, auto_now, ...]
    pub default_value: Option<Expression>,
}

// ── Async / Concurrency ───────────────────────────────────────────────────────
/// parallel { ... }  — concurrent statement execution
#[derive(Debug, PartialEq, Clone)]
pub struct ParallelBlock {
    pub body: Vec<Statement>,
}

/// defer expr  — run at end of current scope
#[derive(Debug, PartialEq, Clone)]
pub struct DeferStmt {
    pub expr: Box<Expression>,
}

/// retry N times [with timeout(Xs)] { ... }
#[derive(Debug, PartialEq, Clone)]
pub struct RetryBlock {
    pub count: Box<Expression>,
    pub timeout_secs: Option<Box<Expression>>,
    pub body: Vec<Statement>,
}

// ── Web Blocks ────────────────────────────────────────────────────────────────
/// validate(expr) as { field: Type [tags] }
#[derive(Debug, PartialEq, Clone)]
pub struct ValidateBlock {
    pub target: Box<Expression>,
    pub fields: Vec<ValidateField>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ValidateField {
    pub name: String,
    pub type_expr: TypeExpr,
    pub rules: Vec<ValidateRule>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ValidateRule {
    Required,
    Email,
    Url,
    Min(Expression),
    Max(Expression),
    MinLen(Expression),
    MaxLen(Expression),
    Pattern(String),
    Custom(String),
}

/// transaction { ... }
#[derive(Debug, PartialEq, Clone)]
pub struct TransactionBlock {
    pub body: Vec<Statement>,
}

/// queue job_name(args)
#[derive(Debug, PartialEq, Clone)]
pub struct QueueStmt {
    pub call: Box<Expression>,
}

/// stream "/path" { ... }
#[derive(Debug, PartialEq, Clone)]
pub struct StreamDecl {
    pub path: String,
    pub body: Vec<Statement>,
}

/// js { raw_code }  — raw JavaScript escape hatch
#[derive(Debug, PartialEq, Clone)]
pub struct JsBlock {
    pub raw: String,
}

// ── View / HTML Template ──────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub struct ViewDecl {
    pub name: String,
    pub params: Vec<RuneParam>,
    pub body: Vec<HtmlNode>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum HtmlNode {
    Element {
        tag: String,
        attrs: Vec<(String, HtmlAttrValue)>,
        children: Vec<HtmlNode>,
        self_closing: bool,
    },
    Text(String),
    Expr(Expression),       // {expr} interpolation
    IfNode {
        condition: Box<Expression>,
        then: Vec<HtmlNode>,
        else_: Option<Vec<HtmlNode>>,
    },
    ForNode {
        item: String,
        collection: Box<Expression>,
        children: Vec<HtmlNode>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum HtmlAttrValue {
    Static(String),
    Expr(Expression),
}

// ── Test Block ────────────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub struct TestDecl {
    pub name: String,
    pub body: Vec<Statement>,
}

// ── Expressions ───────────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),

    // Function call
    CallExpr {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },

    // Member access: obj.field or obj?.field (nil-safe)
    MemberAccess {
        object: Box<Expression>,
        member: String,
        is_safe: bool,          // true for ?.
    },

    // Index access: arr[i]
    IndexExpr {
        object: Box<Expression>,
        index: Box<Expression>,
    },

    // Binary expression
    BinaryExpr {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },

    // Unary expression
    UnaryExpr {
        op: UnaryOp,
        operand: Box<Expression>,
    },

    // Pipe: a |> b |> c
    PipeExpr {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    // Nil-coalescing assign: x ?= default
    NilAssignExpr {
        target: String,
        default: Box<Expression>,
    },

    // when expr { arm => expr, else => expr }
    WhenExpr {
        subject: Box<Expression>,
        arms: Vec<WhenArm>,
    },

    // if expr as expression (ternary-style)
    IfExpr(Box<IfExpr>),

    // Closure: { |params| body } or trailing { it ... }
    ClosureExpr {
        params: Vec<String>,
        body: Vec<Statement>,
    },

    // Collection literals
    ArrayLiteral(Vec<Expression>),
    MapLiteral(Vec<(Expression, Expression)>),

    // Struct instantiation: TypeName { field: val }
    StructInst {
        name: String,
        fields: Vec<(String, Expression)>,
    },

    // Error propagation: expr?
    ErrorPropagate(Box<Expression>),

    // Type cast: expr as Type
    CastExpr {
        value: Box<Expression>,
        target_type: TypeExpr,
        default: Option<Box<Expression>>,  // expr as int or 0
    },

    // env("KEY") — read environment variable
    EnvExpr(String),

    // Range expression: start..end
    RangeExpr {
        start: Box<Expression>,
        end: Box<Expression>,
    },

    // Nullable type cast: expr as? Type  — returns nil on failure
    NullableCast {
        expr: Box<Expression>,
        target_type: TypeExpr,
    },
}

// ── When Arms ─────────────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub struct WhenArm {
    pub pattern: WhenPattern,
    pub body: Box<Expression>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum WhenPattern {
    Literal(Literal),
    Range(Literal, Literal),
    Wildcard,                // else
    Identifier(String),
}

// ── Literals ──────────────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
    Unit(i64, String),  // 5s, 10mb, 7d — value + unit string
}

// ── Binary Operators ──────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Eq,
    NotEq,
    Less,
    Greater,
    LessEq,
    GreaterEq,

    // Logical
    And,
    Or,

    // Special
    Coalesce,   // ??
    Is,         // identity / type check
}

// ── Unary Operators ───────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOp {
    Not,    // not / !
    Neg,    // -
}

// ── Pattern (used in when arms for destructuring) ─────────────────────────────
#[derive(Debug, PartialEq, Clone)]
pub enum Pattern {
    Literal(Literal),
    Wildcard,
    Tuple(Vec<Pattern>),
    Identifier(String),
}