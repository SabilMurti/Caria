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
            Some((Ok(Token::While), _)) => self.parse_while_stmt().map(ast::Statement::WhileStmt),
            Some((Ok(Token::For), _)) => self.parse_for_stmt(),
            Some((Ok(Token::Struct), _)) => self.parse_struct_decl().map(ast::Statement::StructDecl),
            Some((Ok(Token::Enum), _)) => self.parse_enum_decl().map(ast::Statement::EnumDecl),
            Some((Ok(Token::Mod), _)) => self.parse_mod_decl().map(ast::Statement::ModDecl),
            Some((Ok(Token::Use), _)) => self.parse_use_decl().map(ast::Statement::UseDecl),
            Some((Ok(Token::Route), _)) => self.parse_route_decl().map(ast::Statement::RouteDecl),
            Some((Ok(Token::View), _)) => self.parse_view_decl().map(ast::Statement::ViewDecl),
            Some((Ok(Token::Serve), _)) => self.parse_serve_stmt().map(ast::Statement::ServeStmt),
            Some((Ok(Token::Ret), _)) => {
                self.lexer.next(); // consume ret
                self.parse_expression(0).map(ast::Statement::RetStmt)
            }
            Some((Ok(Token::Pub), _)) => {
                self.lexer.next(); // Consume pub
                match self.lexer.peek() {
                    Some((Ok(Token::Struct), _)) => self.parse_struct_decl().map(ast::Statement::StructDecl),
                    Some((Ok(Token::Enum), _)) => self.parse_enum_decl().map(ast::Statement::EnumDecl),
                    _ => return Err(ParseError::UnexpectedToken("Expected struct or enum after pub".into())),
                }
            }
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
        let mut params = Vec::new();
        let mut return_type = None;
        let mut body = Vec::new();
        
        if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
            self.lexer.next();
            while let Some((Ok(token), _)) = self.lexer.peek() {
                if token == &Token::RParen {
                    break;
                }
                
                let param_name = match self.lexer.next() {
                    Some((Ok(Token::Ident(n)), _)) => n,
                    _ => return Err(ParseError::UnexpectedToken("Expected parameter name".into())),
                };
                
                self.expect(Token::Colon)?;
                
                let type_name = match self.lexer.next() {
                    Some((Ok(Token::Ident(t)), _)) => t,
                    _ => return Err(ParseError::UnexpectedToken("Expected type name".into())),
                };
                let mut default_value = None;
                if let Some((Ok(Token::Eq), _)) = self.lexer.peek() {
                    self.lexer.next(); // consume '='
                    default_value = Some(self.parse_expression(0)?);
                }
                
                params.push(ast::FnParam { name: param_name, type_name, default_value });
                
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                    self.lexer.next();
                }
            }
            
            self.expect(Token::RParen)?;
        }
        
        if let Some((Ok(Token::Arrow), _)) = self.lexer.peek() {
            self.lexer.next(); // consume ->
            match self.lexer.next() {
                Some((Ok(Token::Ident(t)), _)) => return_type = Some(t),
                _ => return Err(ParseError::UnexpectedToken("Expected return type".into())),
            }
        }

        if let Some((Ok(Token::Eq), _)) = self.lexer.peek() {
            self.lexer.next(); // consume '='
            let expr = self.parse_expression(0)?;
            body.push(ast::Statement::RetStmt(expr));
        } else {
            self.expect(Token::LBrace)?;

            while let Some((Ok(token), _)) = self.lexer.peek() {
                if token == &Token::RBrace {
                    break;
                }
                body.push(self.parse_statement()?);
            }

            self.expect(Token::RBrace)?;
        }
        
        Ok(ast::FnDecl { name, params, return_type, body })
    }

    fn parse_while_stmt(&mut self) -> Result<ast::WhileStmt, ParseError> {
        self.expect(Token::While)?;
        let condition = Box::new(self.parse_expression(0)?);
        
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        
        Ok(ast::WhileStmt { condition, body })
    }

    fn parse_for_stmt(&mut self) -> Result<ast::Statement, ParseError> {
        self.expect(Token::For)?;
        
        let var1 = match self.lexer.next() {
            Some((Ok(Token::Ident(name)), _)) => name,
            _ => return Err(ParseError::UnexpectedToken("Expected variable name in for loop".into())),
        };

        let mut var2 = None;
        if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
            self.lexer.next(); // consume ','
            match self.lexer.next() {
                Some((Ok(Token::Ident(name)), _)) => var2 = Some(name),
                _ => return Err(ParseError::UnexpectedToken("Expected index variable name".into())),
            }
        }
        
        self.expect(Token::In)?;
        let first_expr = self.parse_expression(0)?;
        
        let mut is_range = false;
        let mut end_expr = None;
        
        if let Some((Ok(Token::Range), _)) = self.lexer.peek() {
            self.lexer.next(); // consume '..'
            is_range = true;
            end_expr = Some(self.parse_expression(0)?);
        }
        
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        
        if is_range {
            if var2.is_some() {
                return Err(ParseError::UnexpectedToken("Range loop cannot have an index variable".into()));
            }
            Ok(ast::Statement::ForStmt(ast::ForStmt { var: var1, start: Box::new(first_expr), end: Box::new(end_expr.unwrap()), body }))
        } else {
            Ok(ast::Statement::ForInStmt(ast::ForInStmt { item_var: var1, index_var: var2, collection: Box::new(first_expr), body }))
        }
    }

    fn parse_struct_decl(&mut self) -> Result<ast::StructDecl, ParseError> {
        self.expect(Token::Struct)?;
        
        let name = match self.lexer.next() {
            Some((Ok(Token::Ident(name)), _)) => name,
            _ => return Err(ParseError::UnexpectedToken("Expected struct name".into())),
        };
        
        self.expect(Token::LBrace)?;
        
        let mut fields = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            
            let field_name = match self.lexer.next() {
                Some((Ok(Token::Ident(n)), _)) => n,
                _ => return Err(ParseError::UnexpectedToken("Expected field name".into())),
            };
            
            self.expect(Token::Colon)?;
            
            let type_name = match self.lexer.next() {
                Some((Ok(Token::Ident(t)), _)) => t,
                _ => return Err(ParseError::UnexpectedToken("Expected type name".into())),
            };
            
            fields.push(ast::StructField { name: field_name, type_name });
            
            // Optional comma
            if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                self.lexer.next();
            }
        }
        self.expect(Token::RBrace)?;
        
        Ok(ast::StructDecl { name, fields })
    }
    fn parse_enum_decl(&mut self) -> Result<ast::EnumDecl, ParseError> {
        self.expect(Token::Enum)?;
        
        let name = match self.lexer.next() {
            Some((Ok(Token::Ident(name)), _)) => name,
            _ => return Err(ParseError::UnexpectedToken("Expected enum name".into())),
        };
        
        self.expect(Token::LBrace)?;
        
        let mut variants = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            
            let variant_name = match self.lexer.next() {
                Some((Ok(Token::Ident(n)), _)) => n,
                _ => return Err(ParseError::UnexpectedToken("Expected variant name".into())),
            };
            
            let mut data = None;
            
            // Check if variant has data: EnumVariant(x: Int)
            if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
                self.lexer.next(); // consume '('
                let mut fields = Vec::new();
                while let Some((Ok(tok), _)) = self.lexer.peek() {
                    if tok == &Token::RParen { break; }
                    
                    let field_name = match self.lexer.next() {
                        Some((Ok(Token::Ident(n)), _)) => n,
                        _ => return Err(ParseError::UnexpectedToken("Expected field name in enum variant".into())),
                    };
                    
                    self.expect(Token::Colon)?;
                    
                    let type_name = match self.lexer.next() {
                        Some((Ok(Token::Ident(t)), _)) => t,
                        _ => return Err(ParseError::UnexpectedToken("Expected type name in enum variant".into())),
                    };
                    
                    fields.push(ast::StructField { name: field_name, type_name });
                    
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::RParen)?;
                data = Some(fields);
            }
            
            variants.push(ast::EnumVariant { name: variant_name, data });
            
            // Optional comma
            if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                self.lexer.next();
            }
        }
        self.expect(Token::RBrace)?;
        
        Ok(ast::EnumDecl { name, variants })
    }

    fn parse_mod_decl(&mut self) -> Result<ast::ModDecl, ParseError> {
        self.expect(Token::Mod)?;
        let mut parts = Vec::new();
        
        let name = match self.lexer.next() {
            Some((Ok(Token::Ident(name)), _)) => name,
            _ => return Err(ParseError::UnexpectedToken("Expected module name".into())),
        };
        parts.push(name);
        
        while let Some((Ok(Token::Dot), _)) = self.lexer.peek() {
            self.lexer.next();
            match self.lexer.next() {
                Some((Ok(Token::Ident(n)), _)) => parts.push(n),
                _ => return Err(ParseError::UnexpectedToken("Expected identifier after dot in module path".into())),
            }
        }
        
        Ok(ast::ModDecl { name: parts.join(".") })
    }

    fn parse_use_decl(&mut self) -> Result<ast::UseDecl, ParseError> {
        self.expect(Token::Use)?;
        
        let mut is_npm = false;
        let module = match self.lexer.peek() {
            Some((Ok(Token::Ident(n)), _)) if n == "npm" => {
                is_npm = true;
                self.lexer.next();
                match self.lexer.next() {
                    Some((Ok(Token::Str(s)), _)) => s,
                    _ => return Err(ParseError::UnexpectedToken("Expected package string after npm".into())),
                }
            }
            _ => {
                let mut parts = Vec::new();
                let first = match self.lexer.next() {
                    Some((Ok(Token::Ident(name)), _)) => name,
                    _ => return Err(ParseError::UnexpectedToken("Expected module path".into())),
                };
                parts.push(first);
                
                while let Some((Ok(Token::Dot), _)) = self.lexer.peek() {
                    self.lexer.next();
                    match self.lexer.next() {
                        Some((Ok(Token::Ident(n)), _)) => parts.push(n),
                        _ => return Err(ParseError::UnexpectedToken("Expected identifier after dot".into())),
                    }
                }
                parts.join(".")
            }
        };
        
        // Two forms:
        //   use npm "pkg" as alias          → symbols: ["alias"], is_npm: true
        //   use npm "pkg" { Sym1, Sym2 }    → symbols: ["Sym1","Sym2"], is_npm: true
        //   use module.path { Sym1, Sym2 }  → existing form
        if is_npm {
            if let Some((Ok(Token::As), _)) = self.lexer.peek() {
                self.lexer.next(); // consume 'as'
                let alias = match self.lexer.next() {
                    Some((Ok(Token::Ident(n)), _)) => n,
                    _ => return Err(ParseError::UnexpectedToken("Expected alias name after 'as'".into())),
                };
                return Ok(ast::UseDecl { module, is_npm, alias_import: true, symbols: vec![alias] });
            }
        }

        self.expect(Token::LBrace)?;
        
        let mut symbols = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            
            let sym = match self.lexer.next() {
                Some((Ok(Token::Ident(n)), _)) => n,
                _ => return Err(ParseError::UnexpectedToken("Expected symbol name".into())),
            };
            symbols.push(sym);
            
            if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                self.lexer.next();
            }
        }
        
        self.expect(Token::RBrace)?;
        
        Ok(ast::UseDecl { module, is_npm, alias_import: false, symbols })
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


    fn parse_pattern(&mut self) -> Result<ast::Pattern, ParseError> {
        match self.lexer.next() {
            Some((Ok(Token::Underscore), _)) => Ok(ast::Pattern::Wildcard),
            Some((Ok(Token::Int(n)), _)) => Ok(ast::Pattern::Literal(ast::Literal::Int(n))),
            Some((Ok(Token::Float(n)), _)) => Ok(ast::Pattern::Literal(ast::Literal::Float(n))),
            Some((Ok(Token::Str(s)), _)) => Ok(ast::Pattern::Literal(ast::Literal::Str(s))),
            Some((Ok(Token::Nil), _)) => Ok(ast::Pattern::Literal(ast::Literal::Nil)),
            Some((Ok(token_val), _)) if token_val == Token::LParen || token_val == Token::LBracket => {
                let is_array = token_val == Token::LBracket;
                let mut tuple = Vec::new();
                while let Some((Ok(token), _)) = self.lexer.peek() {
                    if (is_array && token == &Token::RBracket) || (!is_array && token == &Token::RParen) { break; }
                    tuple.push(self.parse_pattern()?);
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                if is_array {
                    self.expect(Token::RBracket)?;
                } else {
                    self.expect(Token::RParen)?;
                }
                Ok(ast::Pattern::Tuple(tuple))
            }
            got => Err(ParseError::UnexpectedToken(format!("Expected pattern, got {:?}", got)))
        }
    }

    fn parse_expression(&mut self, min_bp: u8) -> Result<ast::Expression, ParseError> {
        let mut left = match self.lexer.next() {
            Some((Ok(Token::Bar), _)) => {
                let mut params = Vec::new();
                while let Some((Ok(token), _)) = self.lexer.peek() {
                    if token == &Token::Bar { break; }
                    match self.lexer.next() {
                        Some((Ok(Token::Ident(name)), _)) => params.push(name),
                        _ => return Err(ParseError::UnexpectedToken("Expected parameter name in closure".into())),
                    }
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::Bar)?;
                
                let mut body = Vec::new();
                if let Some((Ok(Token::LBrace), _)) = self.lexer.peek() {
                    self.lexer.next();
                    while let Some((Ok(token), _)) = self.lexer.peek() {
                        if token == &Token::RBrace { break; }
                        body.push(self.parse_statement()?);
                    }
                    self.expect(Token::RBrace)?;
                } else {
                    body.push(ast::Statement::RetStmt(self.parse_expression(0)?));
                }
                ast::Expression::ClosureExpr { params, body }
            },
            Some((Ok(Token::Int(val)), _)) => ast::Expression::Literal(ast::Literal::Int(val)),
            Some((Ok(Token::Float(val)), _)) => ast::Expression::Literal(ast::Literal::Float(val)),
            Some((Ok(Token::Str(val)), _)) => ast::Expression::Literal(ast::Literal::Str(val)),
            Some((Ok(Token::Nil), _)) => ast::Expression::Literal(ast::Literal::Nil),
            Some((Ok(Token::Ident(name)), _)) => ast::Expression::Identifier(name),
            Some((Ok(Token::Match), _)) => ast::Expression::MatchExpr {
                expr: Box::new(self.parse_expression(0)?),
                arms: {
                    self.expect(Token::LBrace)?;
                    let mut arms = Vec::new();
                    while let Some((Ok(token), _)) = self.lexer.peek() {
                        if token == &Token::RBrace { break; }
                        let pat = self.parse_pattern()?;
                        self.expect(Token::Arrow)?;
                        let expr = self.parse_expression(0)?;
                        arms.push((pat, expr));
                        if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                            self.lexer.next();
                        }
                    }
                    self.expect(Token::RBrace)?;
                    arms
                }
            },
            Some((Ok(Token::LBracket), _)) => {
                let mut elements = Vec::new();
                while let Some((Ok(token), _)) = self.lexer.peek() {
                    if token == &Token::RBracket { break; }
                    elements.push(self.parse_expression(0)?);
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::RBracket)?;
                ast::Expression::ArrayLiteral(elements)
            },
            Some((Ok(Token::LBrace), _)) => {
                let mut pairs = Vec::new();
                while let Some((Ok(token), _)) = self.lexer.peek() {
                    if token == &Token::RBrace { break; }
                    let key = self.parse_expression(0)?;
                    self.expect(Token::Colon)?;
                    let value = self.parse_expression(0)?;
                    pairs.push((key, value));
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::RBrace)?;
                ast::Expression::MapLiteral(pairs)
            },
            Some((Ok(Token::Query), _)) => {
                // query<T>("SQL") — optional generic type hint
                let type_hint = if let Some((Ok(Token::Less), _)) = self.lexer.peek() {
                    self.lexer.next(); // consume '<'
                    let t = match self.lexer.next() {
                        Some((Ok(Token::Ident(name)), _)) => name,
                        _ => return Err(ParseError::UnexpectedToken("Expected type in query<T>".into())),
                    };
                    self.expect(Token::Greater)?;
                    Some(t)
                } else {
                    None
                };
                self.expect(Token::LParen)?;
                let sql = match self.lexer.next() {
                    Some((Ok(Token::Str(s)), _)) => s,
                    _ => return Err(ParseError::UnexpectedToken("query() expects a string literal SQL".into())),
                };
                self.expect(Token::RParen)?;
                ast::Expression::QueryExpr { type_hint, sql }
            },
            Some((Ok(Token::State), _)) => {
                self.expect(Token::LParen)?;
                let initial = Box::new(self.parse_expression(0)?);
                self.expect(Token::RParen)?;
                ast::Expression::StateExpr { initial }
            },
            got => return Err(ParseError::UnexpectedToken(format!("Expected expression, got {:?}", got))),
        };
        // Call Expression
        if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
            self.lexer.next(); // consume '('
            let mut args = Vec::new();
            while let Some((Ok(token), _)) = self.lexer.peek() {
                if token == &Token::RParen { break; }
                args.push(self.parse_expression(0)?);
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                    self.lexer.next();
                }
            }
            self.expect(Token::RParen)?;
            left = ast::Expression::CallExpr {
                callee: Box::new(left),
                args,
            };
        }

        // Struct instantiation

        // Member access (Dot operator)
        while let Some((Ok(token), _)) = self.lexer.peek() {
            let is_safe = token == &Token::SafeChain;
            if token == &Token::Dot || is_safe {
                self.lexer.next(); // consume '.' or '?.'
                let member = match self.lexer.next() {
                    Some((Ok(Token::Ident(n)), _)) => n,
                    _ => return Err(ParseError::UnexpectedToken("Expected property name".into())),
                };
                left = ast::Expression::MemberAccess {
                    object: Box::new(left),
                    member,
                    is_safe,
                };
            } else if token == &Token::Question {
                self.lexer.next(); // consume '?'
                left = ast::Expression::ErrorPropagate(Box::new(left));
            } else {
                break;
            }
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
                Some((Ok(Token::Coalesce), _)) => ast::BinaryOp::Coalesce,
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
            ast::BinaryOp::Coalesce => (1, 2), // Lowest precedence logic
            ast::BinaryOp::Or => (3, 4),
            ast::BinaryOp::And => (5, 6),
            ast::BinaryOp::Eq | ast::BinaryOp::NotEq | 
            ast::BinaryOp::Less | ast::BinaryOp::Greater | 
            ast::BinaryOp::LessEq | ast::BinaryOp::GreaterEq => (7, 8),
            ast::BinaryOp::Add | ast::BinaryOp::Subtract => (9, 10),
            ast::BinaryOp::Multiply | ast::BinaryOp::Divide | ast::BinaryOp::Modulo => (11, 12),
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


    fn parse_route_decl(&mut self) -> Result<ast::RouteDecl, ParseError> {
        self.expect(Token::Route)?;
        
        // HTTP method: GET, POST, PUT, DELETE, PATCH (lexed as Ident)
        let method = match self.lexer.next() {
            Some((Ok(Token::Ident(m)), _)) => m.to_uppercase(),
            _ => return Err(ParseError::UnexpectedToken("Expected HTTP method (GET, POST, etc.)".into())),
        };
        
        // Path: string literal "/api/todos"
        let path = match self.lexer.next() {
            Some((Ok(Token::Str(p)), _)) => p,
            _ => return Err(ParseError::UnexpectedToken("Expected path string after HTTP method".into())),
        };
        
        // Arrow: ->
        self.expect(Token::Arrow)?;
        
        // Handler: anonymous fn or fn keyword
        // Parse as `fn(params) { body }` — consuming fn keyword if present
        if let Some((Ok(Token::Fn), _)) = self.lexer.peek() {
            self.lexer.next(); // consume fn
        }
        
        let mut params = Vec::new();
        if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
            self.lexer.next();
            while let Some((Ok(token), _)) = self.lexer.peek() {
                if token == &Token::RParen { break; }
                let param_name = match self.lexer.next() {
                    Some((Ok(Token::Ident(n)), _)) => n,
                    _ => return Err(ParseError::UnexpectedToken("Expected param name in route handler".into())),
                };
                // type annotation is optional for route handlers
                let type_name = if let Some((Ok(Token::Colon), _)) = self.lexer.peek() {
                    self.lexer.next();
                    match self.lexer.next() {
                        Some((Ok(Token::Ident(t)), _)) => t,
                        _ => return Err(ParseError::UnexpectedToken("Expected type".into())),
                    }
                } else {
                    "Any".into()
                };
                params.push(ast::FnParam { name: param_name, type_name, default_value: None });
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() { self.lexer.next(); }
            }
            self.expect(Token::RParen)?;
        }
        
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        
        Ok(ast::RouteDecl {
            method,
            path,
            handler: ast::FnDecl { name: "_route_handler".into(), params, return_type: None, body },
        })
    }

    fn parse_serve_stmt(&mut self) -> Result<ast::ServeStmt, ParseError> {
        self.expect(Token::Serve)?;
        let port = match self.lexer.next() {
            Some((Ok(Token::Int(n)), _)) => n as u16,
            _ => return Err(ParseError::UnexpectedToken("Expected port number after serve".into())),
        };
        Ok(ast::ServeStmt { port })
    }

    fn parse_view_decl(&mut self) -> Result<ast::ViewDecl, ParseError> {
        self.expect(Token::View)?;
        let name = match self.lexer.next() {
            Some((Ok(Token::Ident(n)), _)) => n,
            _ => return Err(ParseError::UnexpectedToken("Expected view name".into())),
        };
        
        let mut params = Vec::new();
        if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
            self.lexer.next();
            while let Some((Ok(token), _)) = self.lexer.peek() {
                if token == &Token::RParen { break; }
                let param_name = match self.lexer.next() {
                    Some((Ok(Token::Ident(n)), _)) => n,
                    _ => return Err(ParseError::UnexpectedToken("Expected param name in view".into())),
                };
                let type_name = if let Some((Ok(Token::Colon), _)) = self.lexer.peek() {
                    self.lexer.next();
                    match self.lexer.next() {
                        Some((Ok(Token::Ident(t)), _)) => t,
                        _ => "Any".into(),
                    }
                } else { "Any".into() };
                params.push(ast::FnParam { name: param_name, type_name, default_value: None });
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() { self.lexer.next(); }
            }
            self.expect(Token::RParen)?;
        }
        
        self.expect(Token::LBrace)?;
        let body = self.parse_html_children()?;
        self.expect(Token::RBrace)?;
        
        Ok(ast::ViewDecl { name, params, body })
    }

    fn parse_html_children(&mut self) -> Result<Vec<ast::HtmlNode>, ParseError> {
        let mut children = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            match token {
                Token::RBrace => break,
                Token::Less => {
                    self.lexer.next(); // consume '<'
                    // Check for closing tag </tag>
                    if let Some((Ok(Token::Slash), _)) = self.lexer.peek() {
                        // closing tag — stop collecting children
                        self.lexer.next(); // consume '/'
                        // consume tag name
                        self.lexer.next();
                        self.expect(Token::Greater)?;
                        break;
                    }
                    let tag = match self.lexer.next() {
                        Some((Ok(Token::Ident(t)), _)) => t,
                        _ => return Err(ParseError::UnexpectedToken("Expected tag name".into())),
                    };
                    let mut attrs = Vec::new();
                    let mut self_closing = false;
                    // parse attrs
                    while let Some((Ok(tok), _)) = self.lexer.peek() {
                        match tok {
                            Token::Greater => { self.lexer.next(); break; }
                            Token::Slash => {
                                self.lexer.next(); // '/'
                                self.expect(Token::Greater)?;
                                self_closing = true;
                                break;
                            }
                            Token::Ident(_) => {
                                let attr_name = match self.lexer.next() {
                                    Some((Ok(Token::Ident(n)), _)) => n,
                                    _ => break,
                                };
                                if let Some((Ok(Token::Eq), _)) = self.lexer.peek() {
                                    self.lexer.next(); // '='
                                    let val = if let Some((Ok(Token::LBrace), _)) = self.lexer.peek() {
                                        self.lexer.next();
                                        let expr = self.parse_expression(0)?;
                                        self.expect(Token::RBrace)?;
                                        ast::HtmlAttrValue::Expr(expr)
                                    } else {
                                        match self.lexer.next() {
                                            Some((Ok(Token::Str(s)), _)) => ast::HtmlAttrValue::Static(s),
                                            _ => return Err(ParseError::UnexpectedToken("Expected attribute value".into())),
                                        }
                                    };
                                    attrs.push((attr_name, val));
                                }
                            }
                            _ => { self.lexer.next(); }
                        }
                    }
                    let child_nodes = if self_closing { vec![] } else { self.parse_html_children()? };
                    children.push(ast::HtmlNode::Element { tag, attrs, children: child_nodes, self_closing });
                }
                Token::LBrace => {
                    self.lexer.next(); // '{'
                    // check for control flow: for, if
                    if let Some((Ok(Token::For), _)) = self.lexer.peek() {
                        self.lexer.next();
                        let item = match self.lexer.next() {
                            Some((Ok(Token::Ident(n)), _)) => n,
                            _ => return Err(ParseError::UnexpectedToken("Expected item in for".into())),
                        };
                        self.expect(Token::In)?;
                        let collection = Box::new(self.parse_expression(0)?);
                        self.expect(Token::LBrace)?;
                        let inner = self.parse_html_children()?;
                        self.expect(Token::RBrace)?;
                        self.expect(Token::RBrace)?;
                        children.push(ast::HtmlNode::ForNode { item, collection, children: inner });
                    } else if let Some((Ok(Token::If), _)) = self.lexer.peek() {
                        self.lexer.next();
                        let condition = Box::new(self.parse_expression(0)?);
                        self.expect(Token::LBrace)?;
                        let then = self.parse_html_children()?;
                        self.expect(Token::RBrace)?;
                        let else_ = if let Some((Ok(Token::Else), _)) = self.lexer.peek() {
                            self.lexer.next();
                            self.expect(Token::LBrace)?;
                            let e = self.parse_html_children()?;
                            self.expect(Token::RBrace)?;
                            Some(e)
                        } else { None };
                        self.expect(Token::RBrace)?;
                        children.push(ast::HtmlNode::IfNode { condition, then, else_ });
                    } else {
                        let expr = self.parse_expression(0)?;
                        self.expect(Token::RBrace)?;
                        children.push(ast::HtmlNode::Expr(expr));
                    }
                }
                Token::Str(s) => {
                    let s = s.clone();
                    self.lexer.next();
                    children.push(ast::HtmlNode::Text(s));
                }
                _ => { self.lexer.next(); } // skip unknown tokens in HTML context
            }
        }
        Ok(children)
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