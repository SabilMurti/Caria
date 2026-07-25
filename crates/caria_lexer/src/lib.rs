pub mod token;
pub use token::Token;

use logos::Logos;

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
        let mut lexer = CariaLexer::new("fn let mut route");
        
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Fn));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Let));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Mut));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Route));
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
    fn test_comments_and_whitespace() {
        let mut lexer = CariaLexer::new("
            // This is a comment
            let x = 10 // another comment
        ");
        
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Let));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Ident("x".to_string())));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Eq));
        assert_eq!(lexer.next().unwrap().0, Ok(Token::Int(10)));
        assert!(lexer.next().is_none());
    }

    #[test]
    fn test_unclosed_string() {
        let mut lexer = CariaLexer::new("\"unclosed string");
        assert_eq!(lexer.next().unwrap().0, Err(()));
    }
}