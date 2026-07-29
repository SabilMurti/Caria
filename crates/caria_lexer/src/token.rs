use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    // ── V2 Keywords ────────────────────────────────────────────────────────────

    // Core
    #[token("rune")]        Rune,           // function declaration (replaces fn)
    #[token("ret")]         Ret,            // return value
    #[token("say")]         Say,            // print to stdout
    #[token("env")]         Env,            // read environment variable

    // Control Flow
    #[token("if")]          If,
    #[token("else")]        Else,
    #[token("unless")]      Unless,         // inverted if (unless x { })
    #[token("when")]        When,           // Kotlin-style pattern match
    #[token("fallthrough")] Fallthrough,    // explicit fallthrough for when
    #[token("guard")]       Guard,          // Swift-style early-exit guard
    #[token("while")]       While,
    #[token("for")]         For,
    #[token("in")]          In,
    #[token("step")]        Step,           // range step: for i in 0..10 step 2

    // HTTP Routing (native keywords)
    #[token("get")]         Get,
    #[token("post")]        Post,
    #[token("put")]         Put,
    #[token("patch")]       Patch,
    #[token("delete")]      Delete,
    #[token("route")]       RouteGroup,
    #[token("resource")]    ResourceRoute,
    #[token("only")]        Only,
    #[token("except")]      Except,

    // Modules & Code Organization
    #[token("include")]     Include,

    // Database / ORM
    #[token("model")]       Model,

    // Async / Concurrency
    #[token("parallel")]    Parallel,
    #[token("defer")]       Defer,
    #[token("retry")]       Retry,
    #[token("times")]       Times,
    #[token("timeout")]     Timeout,
    #[token("with")]        With,

    // Web Blocks
    #[token("validate")]    Validate,
    #[token("stream")]      Stream,
    #[token("socket")]      Socket,
    #[token("transaction")] Transaction,
    #[token("queue")]       Queue,
    #[token("cache")]       Cache,
    #[token("js")]          Js,             // escape hatch to raw JS block

    // View / Template
    #[token("view")]        View,
    #[token("style")]       Style,
    #[token("client")]      Client,

    // Primitive Type Keywords (used in type annotations)
    #[token("int")]         IntType,        // 64-bit integer
    #[token("dec")]         DecType,        // exact decimal (no float errors)
    #[token("txt")]         TxtType,        // UTF-8 string
    #[token("flag")]        FlagType,       // boolean

    // Boolean / Nil Literals
    #[token("true")]        True,
    #[token("false")]       False,
    #[token("nil")]         Nil,

    // Logical Operators (word form)
    #[token("and")]         And,
    #[token("or")]          Or,
    #[token("not")]         Not,
    #[token("is")]          Is,
    #[token("as?")]         SafeAs,
    #[token("as")]          As,

    // ── Operators & Punctuation ──────────────────────────────────────────────

    // Arithmetic
    #[token("+")]   Plus,
    #[token("-")]   Minus,
    #[token("*")]   Star,
    #[token("/")]   Slash,
    #[token("%")]   Percent,

    // Comparison
    #[token("==")]  EqEq,
    #[token("!=")]  NotEq,
    #[token("<")]   Less,
    #[token(">")]   Greater,
    #[token("<=")]  LessEq,
    #[token(">=")]  GreaterEq,

    // Assignment Operators (multi-char first to avoid prefix conflicts)
    #[token(":=")]  Walrus,         // mutable variable declaration
    #[token("+=")]  PlusEq,
    #[token("-=")]  MinusEq,
    #[token("*=")]  StarEq,
    #[token("/=")]  SlashEq,
    #[token("?=")]  NilAssign,      // assign only if nil
    #[token("=")]   Eq,             // constant assignment / reassignment

    // Special Operators (multi-char first)
    #[token("...")]  Ellipsis,       // varargs parameter: rune f(x: int...)
    #[token("->")]  Arrow,          // return type annotation
    #[token("=>")]  FatArrow,       // inline rune body / when arm
    #[token("|>")]  Pipe,           // pipe operator
    #[token("?.")]  SafeChain,      // nil-safe member access
    #[token("??")]  Coalesce,       // null coalescing
    #[token("..")]  Range,          // range operator

    // Single-char Symbols
    #[token("@")]   At,             // decorator prefix
    #[token("!")]   Bang,
    #[token("?")]   Question,
    #[token(".")]   Dot,
    #[token(",")]   Comma,
    #[token(":")]   Colon,
    #[token(";")]   Semi,
    #[token("_", priority = 3)] Underscore,
    #[token("|")]   Bar,

    // Brackets
    #[token("(")]   LParen,
    #[token(")")]   RParen,
    #[token("{")]   LBrace,
    #[token("}")]   RBrace,
    #[token("[")]   LBracket,
    #[token("]")]   RBracket,

    // ── Literals ─────────────────────────────────────────────────────────────

    // Float / Decimal literal (must be before Int to avoid partial match)
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    // Unit literal: 5s, 10mb, 7d, 100ms, 1h, 1kb, 1gb (must be before Int)
    #[regex(r"[0-9]+(ms|kb|mb|gb|s|m|h|d)", |lex| {
        let s = lex.slice();
        let unit_start = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
        if let Ok(value) = s[..unit_start].parse::<i64>() {
            let unit = s[unit_start..].to_string();
            Some((value, unit))
        } else {
            None
        }
    }, priority = 3)]
    UnitLiteral((i64, String)),

    // Integer literal
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Int(i64),

    // String literal (double-quoted, supports escape sequences)
    #[regex(r#""(?:[^"\\]|\\.)*""#, |lex| {
        let s = &lex.slice()[1..lex.slice().len()-1];
        let mut unescaped = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => unescaped.push('\n'),
                    Some('r') => unescaped.push('\r'),
                    Some('t') => unescaped.push('\t'),
                    Some('"') => unescaped.push('"'),
                    Some('\\') => unescaped.push('\\'),
                    Some(other) => {
                        unescaped.push('\\');
                        unescaped.push(other);
                    }
                    None => unescaped.push('\\'),
                }
            } else {
                unescaped.push(c);
            }
        }
        Some(unescaped)
    })]
    Str(String),

    // Identifiers (must come after all keyword tokens)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // ── Comments ─────────────────────────────────────────────────────────────

    // Doc comments (/// ...) — preserved for tooling
    #[regex(r"///[^\n]*", |lex| lex.slice()[3..].trim().to_string(), allow_greedy = true)]
    DocComment(String),

    // Single-line comments — skipped
    #[regex(r"//[^\n]*", logos::skip, allow_greedy = true)]
    Comment,

    // Whitespace — skipped
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Whitespace,
}