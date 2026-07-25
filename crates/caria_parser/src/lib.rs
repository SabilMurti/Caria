pub mod ast;

use caria_lexer::{CariaLexer, Token};
use std::iter::Peekable;

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnexpectedToken(String),
    UnexpectedEOF(String),
}

pub struct Parser<'a> {
    lexer: Peekable<CariaLexer<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: CariaLexer<'a>) -> Self {
        Self {
            lexer: lexer.peekable(),
        }
    }

    pub fn parse_program(&mut self) -> Result<ast::Program, ParseError> {
        let mut statements = Vec::new();

        while self.lexer.peek().is_some() {
            statements.push(self.parse_statement()?);
        }

        Ok(ast::Program { statements })
    }

    fn parse_statement(&mut self) -> Result<ast::Statement, ParseError> {
        match self.lexer.peek() {
            Some((Ok(Token::Fn), _)) => self.parse_fn_decl().map(ast::Statement::FnDecl),
            Some((Ok(Token::Let), _)) => self.parse_let_stmt().map(ast::Statement::LetStmt),
            _ => self.parse_expression().map(ast::Statement::ExprStmt),
        }
    }

    fn parse_fn_decl(&mut self) -> Result<ast::FnDecl, ParseError> {
        self.expect(Token::Fn)?;

        let name = match self.lexer.next() {
            Some((Ok(Token::Ident(name)), _)) => name,
            _ => return Err(ParseError::UnexpectedToken("Expected function name".into())),
        };

        self.expect(Token::LParen)?;
        self.expect(Token::RParen)?; // Skipping args for now

        self.expect(Token::LBrace)?;

        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace {
                break;
            }
            body.push(self.parse_statement()?);
        }

        self.expect(Token::RBrace)?;

        Ok(ast::FnDecl { name, body })
    }

    fn parse_let_stmt(&mut self) -> Result<ast::LetStmt, ParseError> {
        self.expect(Token::Let)?;

        let name = match self.lexer.next() {
            Some((Ok(Token::Ident(name)), _)) => name,
            _ => return Err(ParseError::UnexpectedToken("Expected variable name".into())),
        };

        self.expect(Token::Eq)?;

        let value = self.parse_expression()?;

        Ok(ast::LetStmt { name, value })
    }

    fn parse_expression(&mut self) -> Result<ast::Expression, ParseError> {
        match self.lexer.next() {
            Some((Ok(Token::Int(val)), _)) => Ok(ast::Expression::Literal(ast::Literal::Int(val))),
            Some((Ok(Token::Float(val)), _)) => Ok(ast::Expression::Literal(ast::Literal::Float(val))),
            Some((Ok(Token::Str(val)), _)) => Ok(ast::Expression::Literal(ast::Literal::Str(val))),
            Some((Ok(Token::True), _)) => Ok(ast::Expression::Literal(ast::Literal::Bool(true))),
            Some((Ok(Token::False), _)) => Ok(ast::Expression::Literal(ast::Literal::Bool(false))),
            Some((Ok(Token::Ident(name)), _)) => Ok(ast::Expression::Identifier(name)),
            _ => Err(ParseError::UnexpectedToken("Expected expression".into())),
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        match self.lexer.next() {
            Some((Ok(token), _)) if token == expected => Ok(()),
            Some((Ok(token), _)) => Err(ParseError::UnexpectedToken(format!(
                "Expected {:?}, got {:?}",
                expected, token
            ))),
            Some((Err(_), _)) => Err(ParseError::UnexpectedToken("Lexer error".into())),
            None => Err(ParseError::UnexpectedEOF("Unexpected EOF".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_let_stmt() {
        let lexer = CariaLexer::new("let x = 42");
        let mut parser = Parser::new(lexer);

        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);

        match &program.statements[0] {
            ast::Statement::LetStmt(stmt) => {
                assert_eq!(stmt.name, "x");
                match &stmt.value {
                    ast::Expression::Literal(ast::Literal::Int(42)) => {}
                    _ => panic!("Expected Literal(Int(42))"),
                }
            }
            _ => panic!("Expected LetStmt"),
        }
    }

    #[test]
    fn test_parse_fn_decl() {
        let lexer = CariaLexer::new("fn main() { let x = \"hello\" }");
        let mut parser = Parser::new(lexer);

        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);

        match &program.statements[0] {
            ast::Statement::FnDecl(decl) => {
                assert_eq!(decl.name, "main");
                assert_eq!(decl.body.len(), 1);

                match &decl.body[0] {
                    ast::Statement::LetStmt(stmt) => {
                        assert_eq!(stmt.name, "x");
                        match &stmt.value {
                            ast::Expression::Literal(ast::Literal::Str(s)) => {
                                assert_eq!(s, "hello");
                            }
                            _ => panic!("Expected Literal(Str(\"hello\"))"),
                        }
                    }
                    _ => panic!("Expected LetStmt in function body"),
                }
            }
            _ => panic!("Expected FnDecl"),
        }
    }
}