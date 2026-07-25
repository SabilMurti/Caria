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
            Some((Ok(Token::Mut), _)) => self.parse_mut_stmt().map(ast::Statement::MutStmt),
            Some((Ok(Token::If), _)) => self.parse_if_expr().map(ast::Statement::IfExpr),
            _ => {
                // Could be ExprStmt or AssignStmt
                // We handle AssignStmt hackily here for simplicity right now
                let expr = self.parse_expression(0)?;
                if let Some((Ok(Token::Eq), _)) = self.lexer.peek() {
                    if let ast::Expression::Identifier(name) = expr {
                        self.lexer.next(); // consume '='
                        let value = self.parse_expression(0)?;
                        return Ok(ast::Statement::AssignStmt(ast::AssignStmt { name, value }));
                    }
                }
                Ok(ast::Statement::ExprStmt(expr))
            }
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

        let value = self.parse_expression(0)?;
        Ok(ast::LetStmt { name, value })
    }
    fn parse_mut_stmt(&mut self) -> Result<ast::MutStmt, ParseError> {
        self.expect(Token::Mut)?;
        let name = match self.lexer.next() {
            Some((Ok(Token::Ident(name)), _)) => name,
            _ => return Err(ParseError::UnexpectedToken("Expected variable name".into())),
        };
        self.expect(Token::Eq)?;
        let value = self.parse_expression(0)?;
        Ok(ast::MutStmt { name, value })
    }

    fn parse_if_expr(&mut self) -> Result<ast::IfExpr, ParseError> {
        self.expect(Token::If)?;
        let condition = Box::new(self.parse_expression(0)?);
        
        self.expect(Token::LBrace)?;
        let mut consequence = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            consequence.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        let mut alternative = None;
        if let Some((Ok(Token::Else), _)) = self.lexer.peek() {
            self.lexer.next(); // consume 'else'
            
            if let Some((Ok(Token::If), _)) = self.lexer.peek() {
                // else if
                alternative = Some(vec![ast::Statement::IfExpr(self.parse_if_expr()?)]);
            } else {
                self.expect(Token::LBrace)?;
                let mut alt_stmts = Vec::new();
                while let Some((Ok(token), _)) = self.lexer.peek() {
                    if token == &Token::RBrace { break; }
                    alt_stmts.push(self.parse_statement()?);
                }
                self.expect(Token::RBrace)?;
                alternative = Some(alt_stmts);
            }
        }

        Ok(ast::IfExpr { condition, consequence, alternative })
    }


    fn parse_expression(&mut self, min_bp: u8) -> Result<ast::Expression, ParseError> {
        let mut left = match self.lexer.next() {
            Some((Ok(Token::Int(val)), _)) => ast::Expression::Literal(ast::Literal::Int(val)),
            Some((Ok(Token::Float(val)), _)) => ast::Expression::Literal(ast::Literal::Float(val)),
            Some((Ok(Token::Str(val)), _)) => ast::Expression::Literal(ast::Literal::Str(val)),
            Some((Ok(Token::True), _)) => ast::Expression::Literal(ast::Literal::Bool(true)),
            Some((Ok(Token::False), _)) => ast::Expression::Literal(ast::Literal::Bool(false)),
            Some((Ok(Token::Ident(name)), _)) => ast::Expression::Identifier(name),
            _ => return Err(ParseError::UnexpectedToken("Expected expression".into())),
        };

        // Simple check for CallExpr (e.g. print(x))
        if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
            self.lexer.next(); // consume '('
            let mut args = Vec::new();
            if let Some((Ok(token), _)) = self.lexer.peek() {
                if token != &Token::RParen {
                    args.push(self.parse_expression(0)?);
                }
            }
            self.expect(Token::RParen)?;
            left = ast::Expression::CallExpr {
                callee: Box::new(left),
                args,
            };
        }
        
        loop {
            let op = match self.lexer.peek() {
                Some((Ok(Token::Plus), _)) => ast::BinaryOp::Add,
                Some((Ok(Token::Minus), _)) => ast::BinaryOp::Subtract,
                Some((Ok(Token::Star), _)) => ast::BinaryOp::Multiply,
                Some((Ok(Token::Slash), _)) => ast::BinaryOp::Divide,
                Some((Ok(Token::Percent), _)) => ast::BinaryOp::Modulo,
                Some((Ok(Token::EqEq), _)) => ast::BinaryOp::Eq,
                Some((Ok(Token::NotEq), _)) => ast::BinaryOp::NotEq,
                Some((Ok(Token::Less), _)) => ast::BinaryOp::Less,
                Some((Ok(Token::Greater), _)) => ast::BinaryOp::Greater,
                Some((Ok(Token::LessEq), _)) => ast::BinaryOp::LessEq,
                Some((Ok(Token::GreaterEq), _)) => ast::BinaryOp::GreaterEq,
                Some((Ok(Token::And), _)) => ast::BinaryOp::And,
                Some((Ok(Token::Or), _)) => ast::BinaryOp::Or,
                _ => break,
            };
            
            let (l_bp, r_bp) = self.infix_binding_power(&op);
            if l_bp < min_bp {
                break;
            }
            
            self.lexer.next(); // Consume op
            
            let right = self.parse_expression(r_bp)?;
            left = ast::Expression::BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }

    fn infix_binding_power(&self, op: &ast::BinaryOp) -> (u8, u8) {
        match op {
            ast::BinaryOp::Or => (1, 2),
            ast::BinaryOp::And => (3, 4),
            ast::BinaryOp::Eq | ast::BinaryOp::NotEq | 
            ast::BinaryOp::Less | ast::BinaryOp::Greater | 
            ast::BinaryOp::LessEq | ast::BinaryOp::GreaterEq => (5, 6),
            ast::BinaryOp::Add | ast::BinaryOp::Subtract => (7, 8),
            ast::BinaryOp::Multiply | ast::BinaryOp::Divide | ast::BinaryOp::Modulo => (9, 10),
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