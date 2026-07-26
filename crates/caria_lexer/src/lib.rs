pub mod token;
pub use token::Token;

use logos::Logos;

#[derive(Clone)]
pub struct CariaLexer<'a> {
    inner: logos::Lexer<'a, Token>,
}

impl<'a> CariaLexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: Token::lexer(source),
        }
    }
}

impl<'a> Iterator for CariaLexer<'a> {
    type Item = (Result<Token, ()>, std::ops::Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|token| (token, self.inner.span()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        // V2 keywords: rune (function), := (mutable), get (HTTP route)
        let mut lexer = CariaLexer::new("rune get");

        assert_eq!(lexer.next().unwrap().0, Ok(Token::Rune));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Get));
        assert!(lexer.next().is_none());
    }

    #[test]
    fn test_v2_control_keywords() {
        let mut lexer = CariaLexer::new("if else unless when guard while for in step");

        assert_eq!(lexer.next().unwrap().0, Ok(Token::If));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Else));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Unless));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::When));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Guard));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::While));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::For));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::In));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Step));
        assert!(lexer.next().is_none());
    }

    #[test]
    fn test_v2_routing_keywords() {
        let mut lexer = CariaLexer::new("get post put patch delete");

        assert_eq!(lexer.next().unwrap().0, Ok(Token::Get));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Post));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Put));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Patch));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Delete));
        assert!(lexer.next().is_none());
    }

    #[test]
    fn test_v2_type_keywords() {
        let mut lexer = CariaLexer::new("int dec txt flag");

        assert_eq!(lexer.next().unwrap().0, Ok(Token::IntType));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::DecType));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::TxtType));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::FlagType));
        assert!(lexer.next().is_none());
    }

    #[test]
    fn test_literals() {
        let mut lexer = CariaLexer::new("42 3.14 \"hello world\" true");

        assert_eq!(lexer.next().unwrap().0, Ok(Token::Int(42)));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Float(3.14)));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Str("hello world".to_string())));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::True));
        assert!(lexer.next().is_none());
    }

    #[test]
    fn test_operators() {
        let mut lexer = CariaLexer::new("|> ?. ?? .. ->");

        assert_eq!(lexer.next().unwrap().0, Ok(Token::Pipe));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::SafeChain));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Coalesce));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Range));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Arrow));
        assert!(lexer.next().is_none());
    }

    #[test]
    fn test_v2_assignment_operators() {
        // := is Walrus (mutable declare), ?= is NilAssign
        let mut lexer = CariaLexer::new(":= ?= =>");

        assert_eq!(lexer.next().unwrap().0, Ok(Token::Walrus));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::NilAssign));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::FatArrow));
        assert!(lexer.next().is_none());
    }

    #[test]
    fn test_comments_and_whitespace() {
        let mut lexer = CariaLexer::new("
            // This is a comment
            count := 10 // another comment
        ");

        assert_eq!(lexer.next().unwrap().0, Ok(Token::Ident("count".to_string())));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Walrus));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Int(10)));
        assert!(lexer.next().is_none());
    }

    #[test]
    fn test_unclosed_string() {
        let mut lexer = CariaLexer::new("\"unclosed string");
        assert_eq!(lexer.next().unwrap().0, Err(()));
    }

    #[test]
    fn test_v2_web_block_keywords() {
        let mut lexer = CariaLexer::new("model validate parallel defer retry transaction queue cache stream js");

        assert_eq!(lexer.next().unwrap().0, Ok(Token::Model));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Validate));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Parallel));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Defer));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Retry));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Transaction));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Queue));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Cache));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Stream));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Js));
        assert!(lexer.next().is_none());
    }
}