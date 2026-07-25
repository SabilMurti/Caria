use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    // Keywords
    #[token("fn")] Fn,
    #[token("let")] Let,
    #[token("mut")] Mut,
    #[token("ret")] Ret,
    #[token("pub")] Pub,
    #[token("mod")] Mod,
    #[token("use")] Use,
    #[token("if")] If,
    #[token("else")] Else,
    #[token("match")] Match,
    #[token("for")] For,
    #[token("while")] While,
    #[token("in")] In,
    #[token("break")] Break,
    #[token("struct")] Struct,
    #[token("enum")] Enum,
    #[token("trait")] Trait,
    #[token("impl")] Impl,
    #[token("self")] SelfKw,
    #[token("as")] As,
    #[token("is")] Is,
    #[token("route")] Route,
    #[token("view")] View,
    #[token("query")] Query,
    #[token("serve")] Serve,
    #[token("state")] State,
    #[token("nil")] Nil,
    #[token("true")] True,
    #[token("false")] False,
    #[token("and")] And,
    #[token("or")] Or,
    #[token("not")] Not,

    // Operators & Punctuation
    #[token("+")] Plus,
    #[token("-")] Minus,
    #[token("*")] Star,
    #[token("/")] Slash,
    #[token("%")] Percent,
    
    #[token("==")] EqEq,
    #[token("!=")] NotEq,
    #[token("<")] Less,
    #[token(">")] Greater,
    #[token("<=")] LessEq,
    #[token(">=")] GreaterEq,
    
    #[token("=")] Eq,
    #[token("+=")] PlusEq,
    #[token("-=")] MinusEq,
    #[token("*=")] StarEq,
    #[token("/=")] SlashEq,

    #[token("->")] Arrow,
    #[token("=>")] FatArrow,
    #[token("|>")] Pipe,
    #[token("..")] Range,
    #[token("?.")] SafeChain,
    #[token("??")] Coalesce,
    #[token("?")] Question,
    #[token("!")] Bang,
    #[token("_", priority = 3)] Underscore,
    #[token(".")] Dot,
    #[token(",")] Comma,
    #[token(":")] Colon,

    #[token("(")] LParen,
    #[token(")")] RParen,
    #[token("{")] LBrace,
    #[token("}")] RBrace,
    #[token("[")] LBracket,
    #[token("]")] RBracket,

    // Literals
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Int(i64),

    // Identifiers (e.g. variable names)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // Strings
    #[regex(r#""(?:[^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string() // Strip quotes
    })]
    Str(String),

    // Ignored
    #[regex(r"//[^\n]*", logos::skip, allow_greedy = true)]
    Comment,

    #[regex(r"[ \t\n\f]+", logos::skip)]
    Whitespace,
}
