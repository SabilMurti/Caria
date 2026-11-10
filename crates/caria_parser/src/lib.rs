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
    allow_trailing_closure: bool,
}

pub fn token_to_ident(token: &Token) -> Option<String> {
    match token {
        Token::Ident(s) => Some(s.clone()),
        Token::Get => Some("get".into()),
        Token::Post => Some("post".into()),
        Token::Put => Some("put".into()),
        Token::Patch => Some("patch".into()),
        Token::Delete => Some("delete".into()),
        Token::Transaction => Some("transaction".into()),
        Token::Queue => Some("queue".into()),
        Token::Cache => Some("cache".into()),
        Token::Stream => Some("stream".into()),
        Token::Socket => Some("socket".into()),
        Token::Validate => Some("validate".into()),
        Token::Model => Some("model".into()),
        Token::View => Some("view".into()),
        Token::Style => Some("style".into()),
        Token::Client => Some("client".into()),
        Token::Retry => Some("retry".into()),
        Token::Timeout => Some("timeout".into()),
        Token::Step => Some("step".into()),
        Token::Times => Some("times".into()),
        Token::Only => Some("only".into()),
        Token::Except => Some("except".into()),
        Token::Env => Some("env".into()),
        Token::RouteGroup => Some("route".into()),
        Token::ResourceRoute => Some("resource".into()),
        _ => None,
    }
}

impl<'a> Parser<'a> {
    pub fn new(lexer: CariaLexer<'a>) -> Self {
        Self {
            lexer: lexer.peekable(),
            allow_trailing_closure: true,
        }
    }

    pub fn parse_program(&mut self) -> Result<ast::Program, ParseError> {
        let mut statements = Vec::new();
        while self.lexer.peek().is_some() {
            statements.push(self.parse_statement()?);
        }
        Ok(ast::Program { statements })
    }

    // ── Statement Dispatcher ─────────────────────────────────────────────────

    fn parse_statement(&mut self) -> Result<ast::Statement, ParseError> {
        // Collect leading doc comments
        let mut doc_comment: Option<String> = None;
        while let Some((Ok(Token::DocComment(_)), _)) = self.lexer.peek() {
            let text = match self.lexer.next() {
                Some((Ok(Token::DocComment(s)), _)) => s,
                _ => unreachable!(),
            };
            doc_comment = Some(match doc_comment {
                None => text,
                Some(prev) => format!("{}\n{}", prev, text),
            });
        }

        match self.lexer.peek() {
            // Rune (function declaration)
            Some((Ok(Token::Rune), _)) => {
                self.parse_rune_decl(doc_comment).map(ast::Statement::RuneDecl)
            }

            // Return
            Some((Ok(Token::Ret), _)) => {
                self.lexer.next();
                self.parse_expression(0).map(ast::Statement::RetStmt)
            }

            // Print / Say
            Some((Ok(Token::Say), _)) => {
                self.lexer.next();
                if let Some((Ok(Token::Dot), _)) = self.lexer.peek() {
                    self.lexer.next();
                    let level = match self.lexer.next() {
                        Some((Ok(Token::Ident(n)), _)) => n,
                        _ => return Err(ParseError::UnexpectedToken("Expected say level".into())),
                    };
                    self.expect(Token::LParen)?;
                    let mut args = Vec::new();
                    while let Some((Ok(tok), _)) = self.lexer.peek() {
                        if tok == &Token::RParen { break; }
                        args.push(self.parse_expression(0)?);
                        if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                            self.lexer.next();
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(ast::Statement::SayLevel { level, args })
                } else {
                    self.expect(Token::LParen)?;
                    let expr = self.parse_expression(0)?;
                    self.expect(Token::RParen)?;
                    Ok(ast::Statement::SayStmt(expr))
                }
            }

            // Control flow
            Some((Ok(Token::If), _)) => {
                self.parse_if_expr().map(|e| ast::Statement::IfStmt(e))
            }
            Some((Ok(Token::Unless), _)) => {
                self.parse_unless_stmt().map(ast::Statement::UnlessStmt)
            }
            Some((Ok(Token::Guard), _)) => {
                self.parse_guard_stmt().map(ast::Statement::GuardStmt)
            }
            Some((Ok(Token::Fallthrough), _)) => {
                self.lexer.next();
                Ok(ast::Statement::FallthroughStmt)
            }
            Some((Ok(Token::While), _)) => {
                self.parse_while_stmt().map(ast::Statement::WhileStmt)
            }
            Some((Ok(Token::For), _)) => self.parse_for_stmt(),


            // Modules & Code Organization
            Some((Ok(Token::Summon), _)) => {
                self.lexer.next(); // consume summon
                
                let mut path = String::new();
                loop {
                    match self.lexer.next() {
                        Some((Ok(Token::Ident(p)), _)) => path.push_str(&p),
                        // Keyword tokens are valid module names in summon paths
                        // (e.g. `summon foundation.env`, `summon http.cache`).
                        Some((Ok(Token::Env), _)) => path.push_str("env"),
                        Some((Ok(Token::Cache), _)) => path.push_str("cache"),
                        Some((Ok(Token::Style), _)) => path.push_str("style"),
                        Some((Ok(Token::Client), _)) => path.push_str("client"),
                        Some((Ok(Token::Only), _)) => path.push_str("only"),
                        Some((Ok(Token::Except), _)) => path.push_str("except"),
                        Some((Ok(Token::Parallel), _)) => path.push_str("parallel"),
                        Some((Ok(Token::Defer), _)) => path.push_str("defer"),
                        Some((Ok(Token::Retry), _)) => path.push_str("retry"),
                        Some((Ok(Token::Timeout), _)) => path.push_str("timeout"),
                        Some((Ok(Token::Step), _)) => path.push_str("step"),
                        Some((Ok(Token::Socket), _)) => path.push_str("socket"),
                        Some((Ok(Token::Times), _)) => path.push_str("times"),
                        Some((Ok(Token::With), _)) => path.push_str("with"),
                        Some((Err(_), span)) => return Err(ParseError::UnexpectedToken(format!("Lexer error at {}..{}", span.start, span.end))),
                        Some((Ok(tok), _)) => {
                            let tok_str = format!("{:?}", tok).to_lowercase();
                            path.push_str(&tok_str);
                        },
                        Some((tok, _)) => return Err(ParseError::UnexpectedToken(format!("Expected identifier in summon path, found {:?}", tok))),
                        None => return Err(ParseError::UnexpectedEOF("Expected identifier in summon path".into())),
                    }
                    
                    if let Some((Ok(Token::Dot), _)) = self.lexer.peek() {
                        self.lexer.next(); // consume dot
                        path.push('.');
                    } else {
                        break;
                    }
                }
                
                Ok(ast::Statement::Summon(path))
            }



            // Async / Concurrency
            Some((Ok(Token::Parallel), _)) => {
                self.parse_parallel_block().map(ast::Statement::ParallelBlock)
            }
            Some((Ok(Token::Defer), _)) => {
                self.parse_defer_stmt().map(ast::Statement::DeferStmt)
            }
            Some((Ok(Token::Retry), _)) => {
                self.parse_retry_block().map(ast::Statement::RetryBlock)
            }


            _ => {
                // Could be:
                // 1. `name := expr`  → MutDecl
                // 2. `name = expr`   → ConstDecl (first occurrence) or AssignStmt (reassignment)
                // 3. Any other expression
                let expr = self.parse_expression(0)?;

                // Check for destructuring assignment: a, b = expr
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                    if let ast::Expression::Identifier(first_name) = &expr {
                        let mut names = vec![first_name.clone()];
                        while let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                            self.lexer.next(); // consume ,
                            match self.lexer.next() {
                                Some((Ok(tok), _)) => match token_to_ident(&tok) {
                                    Some(n) => names.push(n),
                                    None => return Err(ParseError::UnexpectedToken("Expected identifier in destructuring".into())),
                                },
                                _ => return Err(ParseError::UnexpectedToken("Expected identifier in destructuring".into())),
                            }
                        }
                        self.expect(Token::Eq)?;
                        let value = self.parse_expression(0)?;
                        return Ok(ast::Statement::DestructureAssign { names, value });
                    }
                }

                // Check for ?= (null-coalescing assignment)
                if let Some((Ok(Token::NilAssign), _)) = self.lexer.peek() {
                    if let ast::Expression::Identifier(name) = &expr {
                        self.lexer.next(); // consume ?=
                        let value = self.parse_expression(0)?;
                        return Ok(ast::Statement::NullAssignStmt { name: name.clone(), value });
                    }
                }

                // Check for := (mutable declaration)
                if let Some((Ok(Token::Walrus), _)) = self.lexer.peek() {
                    if let ast::Expression::Identifier(name) = expr {
                        self.lexer.next(); // consume :=
                        let value = self.parse_expression(0)?;
                        return Ok(ast::Statement::MutDecl(ast::MutDecl {
                            name,
                            type_hint: None,
                            value,
                        }));
                    } else {
                        return Err(ParseError::UnexpectedToken("LHS of := must be an identifier".into()));
                    }
                }

                // Check for = (const decl or reassignment)
                if let Some((Ok(Token::Eq), _)) = self.lexer.peek() {
                    if let ast::Expression::Identifier(name) = expr {
                        self.lexer.next(); // consume =
                        // Check for when expression (special case: name = when ...)
                        let value = self.parse_expression(0)?;
                        // We treat first-time = assignment as ConstDecl
                        // The semantic analyzer will distinguish reassignment
                        return Ok(ast::Statement::ConstDecl(ast::ConstDecl {
                            name,
                            type_hint: None,
                            value,
                        }));
                    }
                }

                // Check for compound assignment operators
                if let Some((Ok(op_tok), _)) = self.lexer.peek() {
                    let compound_op = match op_tok {
                        Token::PlusEq => Some(ast::BinaryOp::Add),
                        Token::MinusEq => Some(ast::BinaryOp::Subtract),
                        Token::StarEq => Some(ast::BinaryOp::Multiply),
                        Token::SlashEq => Some(ast::BinaryOp::Divide),
                        _ => None,
                    };
                    if let Some(op) = compound_op {
                        if let ast::Expression::Identifier(name) = &expr {
                            let name = name.clone();
                            self.lexer.next(); // consume op
                            let rhs = self.parse_expression(0)?;
                            let value = ast::Expression::BinaryExpr {
                                left: Box::new(ast::Expression::Identifier(name.clone())),
                                op,
                                right: Box::new(rhs),
                            };
                            return Ok(ast::Statement::AssignStmt(ast::AssignStmt { name, value }));
                        }
                    }
                }

                Ok(ast::Statement::ExprStmt(expr))
            }
        }
    }

    // ── Rune (Function) ──────────────────────────────────────────────────────

    fn parse_rune_decl(&mut self, doc_comment: Option<String>) -> Result<ast::RuneDecl, ParseError> {
        self.expect(Token::Rune)?;

        let name = match self.lexer.next() {
            Some((Ok(tok), _)) => match token_to_ident(&tok) {
                Some(n) => n,
                None => return Err(ParseError::UnexpectedToken("Expected rune name".into())),
            },
            _ => return Err(ParseError::UnexpectedToken("Expected rune name".into())),
        };

        // Generic parameters? <T, U>
        if let Some((Ok(Token::Less), _)) = self.lexer.peek() {
            self.lexer.next(); // consume <
            while let Some((Ok(tok), _)) = self.lexer.peek() {
                if tok == &Token::Greater { break; }
                self.lexer.next(); // Just consume generic names for now
            }
            self.expect(Token::Greater)?;
        }

        let params = self.parse_rune_params()?;

        let return_type = if let Some((Ok(Token::Arrow), _)) = self.lexer.peek() {
            self.lexer.next(); // consume ->
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        // Inline body: rune name(...) -> T => expr
        if let Some((Ok(Token::FatArrow), _)) = self.lexer.peek() {
            self.lexer.next(); // consume =>
            let expr = self.parse_expression(0)?;
            return Ok(ast::RuneDecl {
                name,
                params,
                return_type,
                body: ast::RuneBody::Inline(Box::new(expr)),
                is_public: false,
                doc_comment,
            });
        }

        // Block body
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        Ok(ast::RuneDecl {
            name,
            params,
            return_type,
            body: ast::RuneBody::Block(body),
            is_public: false,
            doc_comment,
        })
    }

    fn parse_rune_params(&mut self) -> Result<Vec<ast::RuneParam>, ParseError> {
        let mut params = Vec::new();
        if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
            self.lexer.next(); // consume (
            while let Some((Ok(token), _)) = self.lexer.peek() {
                if token == &Token::RParen { break; }
                let param_name = match self.lexer.next() {
                    Some((Ok(tok), _)) => match token_to_ident(&tok) {
                        Some(n) => n,
                        None => return Err(ParseError::UnexpectedToken("Expected parameter name".into())),
                    },
                    _ => return Err(ParseError::UnexpectedToken("Expected parameter name".into())),
                };
                self.expect(Token::Colon)?;
                let type_expr = self.parse_type_expr()?;
                let is_variadic = if let Some((Ok(Token::Ellipsis), _)) = self.lexer.peek() {
                    self.lexer.next();
                    true
                } else {
                    false
                };
                let default_value = if let Some((Ok(Token::Eq), _)) = self.lexer.peek() {
                    self.lexer.next();
                    Some(self.parse_expression(0)?)
                } else {
                    None
                };
                params.push(ast::RuneParam { name: param_name, type_expr, default_value, is_variadic });
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                    self.lexer.next();
                }
            }
            self.expect(Token::RParen)?;
        }
        Ok(params)
    }

    // ── Type Expressions ─────────────────────────────────────────────────────

    fn parse_type_expr(&mut self) -> Result<ast::TypeExpr, ParseError> {
        // List type: [Type] or [Key:Value]
        if let Some((Ok(Token::LBracket), _)) = self.lexer.peek() {
            self.lexer.next(); // consume [
            let inner = self.parse_type_expr()?;
            // Map type: [Key:Value]
            if let Some((Ok(Token::Colon), _)) = self.lexer.peek() {
                self.lexer.next(); // consume :
                let value_type = self.parse_type_expr()?;
                self.expect(Token::RBracket)?;
                return Ok(ast::TypeExpr::Map(Box::new(inner), Box::new(value_type)));
            }
            self.expect(Token::RBracket)?;
            let list_type = ast::TypeExpr::List(Box::new(inner));
            // Non-empty list?
            let list_type = if let Some((Ok(Token::Plus), _)) = self.lexer.peek() {
                self.lexer.next();
                ast::TypeExpr::NonEmptyArray(Box::new(list_type))
            } else {
                list_type
            };
            // Optional list?
            if let Some((Ok(Token::Question), _)) = self.lexer.peek() {
                self.lexer.next();
                return Ok(ast::TypeExpr::Optional(Box::new(list_type)));
            }
            return Ok(list_type);
        }

        // Function type: (T, U) -> R
        if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
            self.lexer.next(); // consume (
            let mut params = Vec::new();
            while let Some((Ok(tok), _)) = self.lexer.peek() {
                if tok == &Token::RParen { break; }
                params.push(self.parse_type_expr()?);
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                    self.lexer.next();
                }
            }
            self.expect(Token::RParen)?;
            self.expect(Token::Arrow)?;
            let return_type = self.parse_type_expr()?;
            
            let func_type = ast::TypeExpr::Function(params, Box::new(return_type));
            // Optional?
            if let Some((Ok(Token::Question), _)) = self.lexer.peek() {
                self.lexer.next();
                return Ok(ast::TypeExpr::Optional(Box::new(func_type)));
            }
            return Ok(func_type);
        }

        // Named type
        let name = match self.lexer.next() {
            Some((Ok(Token::IntType), _)) => "int".to_string(),
            Some((Ok(Token::DecType), _)) => "dec".to_string(),
            Some((Ok(Token::TxtType), _)) => "txt".to_string(),
            Some((Ok(Token::FlagType), _)) => "flag".to_string(),
            Some((Ok(Token::Nil), _)) => "nil".to_string(),
            Some((Ok(Token::Ident(n)), _)) => n,
            _ => return Err(ParseError::UnexpectedToken("Expected type name".into())),
        };

        let mut named = ast::TypeExpr::Named(name.clone());

        // Generic parameters? <T, U>
        if let Some((Ok(Token::Less), _)) = self.lexer.peek() {
            self.lexer.next(); // consume <
            let mut params = Vec::new();
            while let Some((Ok(tok), _)) = self.lexer.peek() {
                if tok == &Token::Greater { break; }
                params.push(self.parse_type_expr()?);
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                    self.lexer.next();
                }
            }
            self.expect(Token::Greater)?;
            named = ast::TypeExpr::Generic(name, params);
        }

        // Optional?
        if let Some((Ok(Token::Question), _)) = self.lexer.peek() {
            self.lexer.next();
            return Ok(ast::TypeExpr::Optional(Box::new(named)));
        }
        Ok(named)
    }

    // ── Control Flow ─────────────────────────────────────────────────────────

    fn parse_if_expr(&mut self) -> Result<ast::IfExpr, ParseError> {
        self.expect(Token::If)?;
        let prev = self.allow_trailing_closure;
        self.allow_trailing_closure = false;
        let condition = Box::new(self.parse_expression(0)?);
        self.allow_trailing_closure = prev;
        self.expect(Token::LBrace)?;
        let mut consequence = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            consequence.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        let alternative = if let Some((Ok(Token::Else), _)) = self.lexer.peek() {
            self.lexer.next(); // consume else
            if let Some((Ok(Token::If), _)) = self.lexer.peek() {
                // else if chaining
                let elif = self.parse_if_expr()?;
                Some(vec![ast::Statement::IfStmt(elif)])
            } else {
                self.expect(Token::LBrace)?;
                let mut alt = Vec::new();
                while let Some((Ok(token), _)) = self.lexer.peek() {
                    if token == &Token::RBrace { break; }
                    alt.push(self.parse_statement()?);
                }
                self.expect(Token::RBrace)?;
                Some(alt)
            }
        } else {
            None
        };

        Ok(ast::IfExpr { condition, consequence, alternative })
    }

    fn parse_unless_stmt(&mut self) -> Result<ast::UnlessStmt, ParseError> {
        self.expect(Token::Unless)?;
        let prev = self.allow_trailing_closure;
        self.allow_trailing_closure = false;
        let condition = Box::new(self.parse_expression(0)?);
        self.allow_trailing_closure = prev;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(ast::UnlessStmt { condition, body })
    }

    fn parse_guard_stmt(&mut self) -> Result<ast::GuardStmt, ParseError> {
        self.expect(Token::Guard)?;
        let prev = self.allow_trailing_closure;
        self.allow_trailing_closure = false;
        let condition = Box::new(self.parse_expression(0)?);
        self.allow_trailing_closure = prev;
        self.expect(Token::Else)?;
        self.expect(Token::LBrace)?;
        let mut fallback = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            fallback.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(ast::GuardStmt { condition, fallback })
    }

    fn parse_while_stmt(&mut self) -> Result<ast::WhileStmt, ParseError> {
        self.expect(Token::While)?;
        let prev = self.allow_trailing_closure;
        self.allow_trailing_closure = false;
        let condition = Box::new(self.parse_expression(0)?);
        self.allow_trailing_closure = prev;
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
            Some((Ok(tok), _)) => match token_to_ident(&tok) {
                Some(n) => n,
                None => return Err(ParseError::UnexpectedToken("Expected variable name in for loop".into())),
            },
            _ => return Err(ParseError::UnexpectedToken("Expected variable name in for loop".into())),
        };

        // Optional second variable: for item, index in ...
        let mut var2 = None;
        if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
            self.lexer.next();
            match self.lexer.next() {
                Some((Ok(tok), _)) => match token_to_ident(&tok) {
                    Some(n) => var2 = Some(n),
                    None => return Err(ParseError::UnexpectedToken("Expected index variable".into())),
                },
                _ => return Err(ParseError::UnexpectedToken("Expected index variable".into())),
            }
        }

        self.expect(Token::In)?;
        let prev = self.allow_trailing_closure;
        self.allow_trailing_closure = false;
        let start_expr = self.parse_expression(0)?;
        self.allow_trailing_closure = prev;

        // Range: for i in start..end [step N]
        if let Some((Ok(Token::Range), _)) = self.lexer.peek() {
            self.lexer.next(); // consume ..
            let prev = self.allow_trailing_closure;
            self.allow_trailing_closure = false;
            let end_expr = self.parse_expression(0)?;
            self.allow_trailing_closure = prev;

            // Optional step
            let step = if let Some((Ok(Token::Step), _)) = self.lexer.peek() {
                self.lexer.next(); // consume step
                let prev = self.allow_trailing_closure;
                self.allow_trailing_closure = false;
                let s = self.parse_expression(0)?;
                self.allow_trailing_closure = prev;
                Some(Box::new(s))
            } else {
                None
            };

            self.expect(Token::LBrace)?;
            let mut body = Vec::new();
            while let Some((Ok(tok), _)) = self.lexer.peek() {
                if tok == &Token::RBrace { break; }
                body.push(self.parse_statement()?);
            }
            self.expect(Token::RBrace)?;

            let else_body = if let Some((Ok(Token::Else), _)) = self.lexer.peek() {
                self.lexer.next();
                self.expect(Token::LBrace)?;
                let mut e_body = Vec::new();
                while let Some((Ok(tok), _)) = self.lexer.peek() {
                    if tok == &Token::RBrace { break; }
                    e_body.push(self.parse_statement()?);
                }
                self.expect(Token::RBrace)?;
                Some(e_body)
            } else {
                None
            };

            return Ok(ast::Statement::ForRangeStmt(ast::ForRangeStmt {
                var: var1,
                start: Box::new(start_expr),
                end: Box::new(end_expr),
                step,
                body,
                else_body,
            }));
        }

        // for-in over collection
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(tok), _)) = self.lexer.peek() {
            if tok == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        let else_body = if let Some((Ok(Token::Else), _)) = self.lexer.peek() {
            self.lexer.next();
            self.expect(Token::LBrace)?;
            let mut e_body = Vec::new();
            while let Some((Ok(tok), _)) = self.lexer.peek() {
                if tok == &Token::RBrace { break; }
                e_body.push(self.parse_statement()?);
            }
            self.expect(Token::RBrace)?;
            Some(e_body)
        } else {
            None
        };

        Ok(ast::Statement::ForInStmt(ast::ForInStmt {
            item_var: var1,
            index_var: var2,
            collection: Box::new(start_expr),
            body,
            else_body,
        }))
    }

    // ── HTTP Routing ─────────────────────────────────────────────────────────




    // ── Model / ORM ──────────────────────────────────────────────────────────


    // ── Async / Concurrency ───────────────────────────────────────────────────

    fn parse_parallel_block(&mut self) -> Result<ast::ParallelBlock, ParseError> {
        self.expect(Token::Parallel)?;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(ast::ParallelBlock { body })
    }

    fn parse_defer_stmt(&mut self) -> Result<ast::DeferStmt, ParseError> {
        self.expect(Token::Defer)?;
        let expr = self.parse_expression(0)?;
        Ok(ast::DeferStmt { expr: Box::new(expr) })
    }

    fn parse_retry_block(&mut self) -> Result<ast::RetryBlock, ParseError> {
        self.expect(Token::Retry)?;
        let count = Box::new(self.parse_expression(0)?);
        self.expect(Token::Times)?;

        let timeout_secs = if let Some((Ok(Token::With), _)) = self.lexer.peek() {
            self.lexer.next(); // consume with
            self.expect(Token::Timeout)?;
            self.expect(Token::LParen)?;
            let t = self.parse_expression(0)?;
            self.expect(Token::RParen)?;
            Some(Box::new(t))
        } else {
            None
        };

        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        Ok(ast::RetryBlock { count, timeout_secs, body })
    }

    // ── Web Blocks ────────────────────────────────────────────────────────────






    // ── View / HTML Template ──────────────────────────────────────────────────



    // ── Expression Parsing (Pratt Parser) ────────────────────────────────────

    fn parse_expression(&mut self, min_bp: u8) -> Result<ast::Expression, ParseError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            // Check for postfix operators
            match self.lexer.peek() {
                Some((Ok(Token::Question), _)) => {
                    // Distinguish between:
                    // - Error propagation: `expr?` (nothing after or non-expression token)
                    // - Ternary: `cond ? true_expr : false_expr` (expression after ?)
                    
                    // Simple heuristic: if we're at min_bp 0 (top level) and there's
                    // something expression-like after ?, try ternary. Otherwise, error propagation.
                    
                    // Ternary has very low precedence (should only parse at statement/assignment level)
                    // Ternary operator must only be recognized at min_bp <= 1 to avoid consuming ? inside
                    // binary expressions like `x > 0 ? ...` where the RHS of > is parsed with min_bp=8
                    let ternary_bp = 1u8;
                    
                    if min_bp > ternary_bp {
                        // We're in a higher precedence context (e.g., inside a binary operator's RHS),
                        // The ? should NOT be consumed here - let the outer expression handle it
                        // Just break out of the loop without consuming the token
                        break;
                    }
                    
                    // Peek ahead: is there something after `?` that looks like an expression?
                    self.lexer.next(); // consume ?
                    
                    // Check what's next
                    match self.lexer.peek() {
                        // If we see nothing, or statement-ending tokens, it's error propagation
                        None | Some((Ok(Token::RParen | Token::RBrace | Token::RBracket | 
                                         Token::Comma | Token::Semi), _)) => {
                            lhs = ast::Expression::ErrorPropagate(Box::new(lhs));
                            continue;
                        }
                        // Otherwise, try to parse as ternary
                        _ => {}
                    }
                    
                    // Parse the true branch with high precedence to ensure we consume only
                    // the immediate expression and stop before the `:` colon
                    // Use a precedence higher than all binary operators (which top out at 13-14)
                    let true_expr = self.parse_expression(15)?;
                    
                    // Check if colon follows
                    if matches!(self.lexer.peek(), Some((Ok(Token::Colon), _))) {
                        // It's a ternary expression
                        self.lexer.next(); // consume :
                        let false_expr = self.parse_expression(ternary_bp)?;
                        
                        lhs = ast::Expression::TernaryExpr {
                            condition: Box::new(lhs),
                            true_expr: Box::new(true_expr),
                            false_expr: Box::new(false_expr),
                        };
                        continue;
                    } else {
                        // No colon - this is a syntax error
                        return Err(ParseError::UnexpectedToken(
                            "Expected ':' after '?' in ternary expression".into()
                        ));
                    }
                }
                Some((Ok(Token::Dot), _)) | Some((Ok(Token::SafeChain), _)) => {
                    let is_safe = matches!(self.lexer.peek(), Some((Ok(Token::SafeChain), _)));
                    self.lexer.next(); // consume . or ?.
                    let member = match self.lexer.next() {
                        Some((Ok(tok), _)) => match token_to_ident(&tok) {
                            Some(n) => n,
                            None => return Err(ParseError::UnexpectedToken("Expected member name after .".into())),
                        },
                        _ => return Err(ParseError::UnexpectedToken("Expected member name after .".into())),
                    };
                    // Method call?
                    if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
                        self.lexer.next();
                        let mut args = Vec::new();
                        while let Some((Ok(tok), _)) = self.lexer.peek() {
                            if tok == &Token::RParen { break; }
                            args.push(self.parse_expression(0)?);
                            if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                                self.lexer.next();
                            }
                        }
                        self.expect(Token::RParen)?;

                        let callee = Box::new(ast::Expression::MemberAccess {
                            object: Box::new(lhs),
                            member,
                            is_safe,
                        });
                        lhs = ast::Expression::CallExpr { callee, args };
                    } else {
                        lhs = ast::Expression::MemberAccess {
                            object: Box::new(lhs),
                            member,
                            is_safe,
                        };
                    }
                    continue;
                }
                Some((Ok(Token::LBracket), _)) => {
                    self.lexer.next();
                    let index = self.parse_expression(0)?;
                    self.expect(Token::RBracket)?;
                    lhs = ast::Expression::IndexExpr {
                        object: Box::new(lhs),
                        index: Box::new(index),
                    };
                    continue;
                }
                Some((Ok(Token::SafeAs), _)) => {
                    self.lexer.next(); // consume as?
                    let target_type = self.parse_type_expr()?;
                    lhs = ast::Expression::NullableCast {
                        expr: Box::new(lhs),
                        target_type,
                    };
                    continue;
                }
                Some((Ok(Token::As), _)) => {
                    self.lexer.next(); // consume as
                    let target_type = self.parse_type_expr()?;
                    let default = if let Some((Ok(Token::Or), _)) = self.lexer.peek() {
                        self.lexer.next();
                        Some(Box::new(self.parse_expression(0)?))
                    } else {
                        None
                    };
                    lhs = ast::Expression::CastExpr {
                        value: Box::new(lhs),
                        target_type,
                        default,
                    };
                    continue;
                }
                _ => {}
            }

            // Binary operators
            let op = match self.lexer.peek() {
                Some((Ok(Token::Pipe), _)) => ast::BinaryOp::Or, // will handle pipe separately
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
                Some((Ok(Token::Is), _)) => ast::BinaryOp::Is,
                _ => break,
            };

            // Pipe operator: special handling
            if matches!(self.lexer.peek(), Some((Ok(Token::Pipe), _))) {
                let (l_bp, r_bp) = (15u8, 16u8);
                if l_bp < min_bp { break; }
                self.lexer.next();
                let rhs = self.parse_expression(r_bp)?;
                lhs = ast::Expression::PipeExpr {
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                };
                continue;
            }

            let (l_bp, r_bp) = self.infix_binding_power(&op);
            if l_bp < min_bp { break; }
            self.lexer.next(); // consume the operator
            let rhs = self.parse_expression(r_bp)?;
            lhs = ast::Expression::BinaryExpr {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<ast::Expression, ParseError> {
        match self.lexer.peek() {
            // Literals
            Some((Ok(Token::Int(_)), _)) => {
                if let Some((Ok(Token::Int(n)), _)) = self.lexer.next() {
                    Ok(ast::Expression::Literal(ast::Literal::Int(n)))
                } else { unreachable!() }
            }
            Some((Ok(Token::Float(_)), _)) => {
                if let Some((Ok(Token::Float(f)), _)) = self.lexer.next() {
                    Ok(ast::Expression::Literal(ast::Literal::Float(f)))
                } else { unreachable!() }
            }
            Some((Ok(Token::Str(_)), _)) => {
                if let Some((Ok(Token::Str(s)), _)) = self.lexer.next() {
                    Ok(ast::Expression::Literal(ast::Literal::Str(s)))
                } else { unreachable!() }
            }
            Some((Ok(Token::True), _)) => {
                self.lexer.next();
                Ok(ast::Expression::Literal(ast::Literal::Bool(true)))
            }
            Some((Ok(Token::False), _)) => {
                self.lexer.next();
                Ok(ast::Expression::Literal(ast::Literal::Bool(false)))
            }
            Some((Ok(Token::Nil), _)) => {
                self.lexer.next();
                Ok(ast::Expression::Literal(ast::Literal::Nil))
            }
            Some((Ok(Token::UnitLiteral(_)), _)) => {
                if let Some((Ok(Token::UnitLiteral((v, u))), _)) = self.lexer.next() {
                    Ok(ast::Expression::Literal(ast::Literal::Unit(v, u)))
                } else { unreachable!() }
            }

            // Unary negation
            Some((Ok(Token::Minus), _)) => {
                self.lexer.next();
                let operand = self.parse_expression(20)?;
                Ok(ast::Expression::UnaryExpr { op: ast::UnaryOp::Neg, operand: Box::new(operand) })
            }

            // Unary not
            Some((Ok(Token::Not), _)) | Some((Ok(Token::Bang), _)) => {
                self.lexer.next();
                let operand = self.parse_expression(20)?;
                Ok(ast::Expression::UnaryExpr { op: ast::UnaryOp::Not, operand: Box::new(operand) })
            }
            
            // Closure: rune(req) => expr OR rune(req) { ... }
            Some((Ok(Token::Rune), _)) => {
                self.lexer.next();
                self.expect(Token::LParen)?;
                let mut params = Vec::new();
                while let Some((Ok(tok), _)) = self.lexer.peek() {
                    if tok == &Token::RParen { break; }
                    match self.lexer.next() {
                        Some((Ok(tok), _)) => match token_to_ident(&tok) {
                            Some(n) => params.push(n),
                            None => return Err(ParseError::UnexpectedToken("Expected parameter name in closure".into())),
                        },
                        _ => return Err(ParseError::UnexpectedToken("Expected parameter name in closure".into())),
                    }
                    // optionally parse type: : Type
                    if let Some((Ok(Token::Colon), _)) = self.lexer.peek() {
                        self.lexer.next();
                        let _type = self.parse_type_expr()?;
                    }
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::RParen)?;

                // optional return type
                if let Some((Ok(Token::Arrow), _)) = self.lexer.peek() {
                    self.lexer.next();
                    let _ret_type = self.parse_type_expr()?;
                }

                // Inline body: => expr
                if let Some((Ok(Token::FatArrow), _)) = self.lexer.peek() {
                    self.lexer.next();
                    let expr = self.parse_expression(0)?;
                    return Ok(ast::Expression::ClosureExpr {
                        params,
                        body: vec![ast::Statement::RetStmt(expr)],
                    });
                }

                // Block body
                self.expect(Token::LBrace)?;
                let mut body = Vec::new();
                while let Some((Ok(token), _)) = self.lexer.peek() {
                    if token == &Token::RBrace { break; }
                    body.push(self.parse_statement()?);
                }
                self.expect(Token::RBrace)?;
                Ok(ast::Expression::ClosureExpr { params, body })
            }

            // Grouped: (expr)
            Some((Ok(Token::LParen), _)) => {
                self.lexer.next();
                let prev = self.allow_trailing_closure;
                self.allow_trailing_closure = true;
                let expr = self.parse_expression(0)?;
                self.allow_trailing_closure = prev;
                self.expect(Token::RParen)?;
                Ok(expr)
            }

            // Array literal: [a, b, c]
            Some((Ok(Token::LBracket), _)) => {
                self.lexer.next();
                let mut items = Vec::new();
                while let Some((Ok(tok), _)) = self.lexer.peek() {
                    if tok == &Token::RBracket { break; }
                    items.push(self.parse_expression(0)?);
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(ast::Expression::ArrayLiteral(items))
            }

            // Map literal or block: { key: val, ... }
            Some((Ok(Token::LBrace), _)) => {
                self.lexer.next();
                let mut pairs = Vec::new();
                while let Some((Ok(tok), _)) = self.lexer.peek() {
                    if tok == &Token::RBrace { break; }
                    let key = self.parse_expression(0)?;
                    self.expect(Token::Colon)?;
                    let val = self.parse_expression(0)?;
                    pairs.push((key, val));
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::RBrace)?;
                Ok(ast::Expression::MapLiteral(pairs))
            }

            // when expression
            Some((Ok(Token::When), _)) => {
                self.parse_when_expr()
            }

            // if expression
            Some((Ok(Token::If), _)) => {
                self.parse_if_expr().map(|e| ast::Expression::IfExpr(Box::new(e)))
            }

            // env("KEY")
            Some((Ok(Token::Env), _)) => {
                self.lexer.next();
                self.expect(Token::LParen)?;
                let key = match self.lexer.next() {
                    Some((Ok(Token::Str(s)), _)) => s,
                    _ => return Err(ParseError::UnexpectedToken("Expected env key string".into())),
                };
                self.expect(Token::RParen)?;
                Ok(ast::Expression::EnvExpr(key))
            }

            // self / this keywords
            Some((Ok(Token::SelfKeyword), _)) => {
                self.lexer.next();
                Ok(ast::Expression::Identifier("self".to_string()))
            }
            Some((Ok(Token::ThisKeyword), _)) => {
                self.lexer.next();
                Ok(ast::Expression::Identifier("this".to_string()))
            }

            // Identifier or contextual keyword → may be function call
            Some((Ok(tok), _)) if token_to_ident(tok).is_some() => {
                let name = match self.lexer.next() {
                    Some((Ok(t), _)) => token_to_ident(&t).unwrap(),
                    _ => unreachable!(),
                };

                // Function call: name(...)
                if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
                    self.lexer.next();
                    let mut args = Vec::new();
                    while let Some((Ok(tok), _)) = self.lexer.peek() {
                        if tok == &Token::RParen { break; }
                        args.push(self.parse_expression(0)?);
                        if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                            self.lexer.next();
                        }
                    }
                    self.expect(Token::RParen)?;

                    // Trailing closure check for direct function calls
                    if self.allow_trailing_closure {
                        if let Some((Ok(Token::LBrace), _)) = self.lexer.peek() {
                            self.lexer.next();
                            let mut closure_body = Vec::new();
                            while let Some((Ok(tok), _)) = self.lexer.peek() {
                                if tok == &Token::RBrace { break; }
                                closure_body.push(self.parse_statement()?);
                            }
                            self.expect(Token::RBrace)?;
                            args.push(ast::Expression::ClosureExpr {
                                params: vec!["it".to_string()],
                                body: closure_body,
                            });
                        }
                    }

                    return Ok(ast::Expression::CallExpr {
                        callee: Box::new(ast::Expression::Identifier(name)),
                        args,
                    });
                }

                // Just return the identifier
                Ok(ast::Expression::Identifier(name))
            }

            _ => Err(ParseError::UnexpectedToken("Unexpected token in expression".into())),
        }
    }

    fn parse_when_expr(&mut self) -> Result<ast::Expression, ParseError> {
        self.expect(Token::When)?;
        let subject = Box::new(self.parse_expression(0)?);
        self.expect(Token::LBrace)?;

        let mut arms = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }

            let pattern = match self.lexer.peek() {
                Some((Ok(Token::Else), _)) => {
                    self.lexer.next();
                    ast::WhenPattern::Wildcard
                }
                Some((Ok(Token::Ident(n)), _)) => {
                    let n = n.clone();
                    self.lexer.next();
                    ast::WhenPattern::Identifier(n)
                }
                Some((Ok(Token::Int(_)), _)) => {
                    if let Some((Ok(Token::Int(n)), _)) = self.lexer.next() {
                        ast::WhenPattern::Literal(ast::Literal::Int(n))
                    } else { unreachable!() }
                }
                Some((Ok(Token::Str(_)), _)) => {
                    if let Some((Ok(Token::Str(s)), _)) = self.lexer.next() {
                        ast::WhenPattern::Literal(ast::Literal::Str(s))
                    } else { unreachable!() }
                }
                _ => {
                    self.lexer.next();
                    ast::WhenPattern::Wildcard
                }
            };

            self.expect(Token::FatArrow)?;
            let body = Box::new(self.parse_expression(0)?);
            arms.push(ast::WhenArm { pattern, body });

            // Optional comma between arms
            if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                self.lexer.next();
            }
        }

        self.expect(Token::RBrace)?;
        Ok(ast::Expression::WhenExpr { subject, arms })
    }

    fn infix_binding_power(&self, op: &ast::BinaryOp) -> (u8, u8) {
        match op {
            ast::BinaryOp::Or => (3, 4),
            ast::BinaryOp::And => (5, 6),
            ast::BinaryOp::Eq | ast::BinaryOp::NotEq |
            ast::BinaryOp::Less | ast::BinaryOp::Greater |
            ast::BinaryOp::LessEq | ast::BinaryOp::GreaterEq |
            ast::BinaryOp::Is => (7, 8),
            ast::BinaryOp::Coalesce => (9, 10),
            ast::BinaryOp::Add | ast::BinaryOp::Subtract => (11, 12),
            ast::BinaryOp::Multiply | ast::BinaryOp::Divide | ast::BinaryOp::Modulo => (13, 14),
        }
    }

    // ── Helper ────────────────────────────────────────────────────────────────

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        match self.lexer.next() {
            Some((Ok(token), _)) if token == expected => Ok(()),
            Some((Ok(token), span)) => Err(ParseError::UnexpectedToken(format!(
                "Expected {:?}, got {:?} at span {}..{}", expected, token, span.start, span.end
            ))),
            Some((Err(_), span)) => Err(ParseError::UnexpectedToken(format!("Lexer error at {}..{}", span.start, span.end))),
            None => Err(ParseError::UnexpectedEOF(format!("Expected {:?}, got EOF", expected))),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_const_decl() {
        let lexer = CariaLexer::new("name = \"Sabil\"");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            ast::Statement::ConstDecl(d) => {
                assert_eq!(d.name, "name");
                match &d.value {
                    ast::Expression::Literal(ast::Literal::Str(s)) => assert_eq!(s, "Sabil"),
                    _ => panic!("Expected Str literal"),
                }
            }
            _ => panic!("Expected ConstDecl"),
        }
    }

    #[test]
    fn test_parse_mut_decl() {
        let lexer = CariaLexer::new("count := 0");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            ast::Statement::MutDecl(d) => {
                assert_eq!(d.name, "count");
                match &d.value {
                    ast::Expression::Literal(ast::Literal::Int(0)) => {}
                    _ => panic!("Expected Int(0)"),
                }
            }
            _ => panic!("Expected MutDecl"),
        }
    }

    #[test]
    fn test_parse_rune_decl() {
        let lexer = CariaLexer::new("rune greet(name: txt) {\n    ret name\n}");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            ast::Statement::RuneDecl(d) => {
                assert_eq!(d.name, "greet");
                assert_eq!(d.params.len(), 1);
                assert_eq!(d.params[0].name, "name");
            }
            _ => panic!("Expected RuneDecl"),
        }
    }

    #[test]
    fn test_parse_rune_inline() {
        let lexer = CariaLexer::new("rune double(x: int) -> int => x");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::RuneDecl(d) => {
                assert!(matches!(d.body, ast::RuneBody::Inline(_)));
            }
            _ => panic!("Expected RuneDecl"),
        }
    }

    #[test]
    fn test_parse_say() {
        let lexer = CariaLexer::new("say(\"hello\")");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::SayStmt(_) => {}
            _ => panic!("Expected SayStmt"),
        }
    }

    #[test]
    fn test_parse_if_unless() {
        let lexer = CariaLexer::new("unless active { ret nil }");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::UnlessStmt(_) => {}
            _ => panic!("Expected UnlessStmt"),
        }
    }

    #[test]
    fn test_parse_when_expr() {
        let lexer = CariaLexer::new("msg = when code { 200 => \"ok\", else => \"err\" }");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::ConstDecl(d) => {
                match &d.value {
                    ast::Expression::WhenExpr { arms, .. } => {
                        assert_eq!(arms.len(), 2);
                    }
                    _ => panic!("Expected WhenExpr"),
                }
            }
            _ => panic!("Expected ConstDecl"),
        }
    }

    #[test]
    fn test_parse_for_range() {
        let lexer = CariaLexer::new("for i in 0..10 step 2 { say(i) }");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::ForRangeStmt(f) => {
                assert_eq!(f.var, "i");
                assert!(f.step.is_some());
            }
            _ => panic!("Expected ForRangeStmt"),
        }
    }

    #[test]
    fn test_parse_summon() {
        let lexer = CariaLexer::new("summon App.Models.User");
        let mut parser = Parser::new(lexer);
        let prog = parser.parse_program().unwrap();

        match &prog.statements[0] {
            ast::Statement::Summon(path) => {
                assert_eq!(path, "App.Models.User");
            }
            _ => panic!("Expected Summon statement"),
        }
    }

    #[test]
    fn test_parse_control_flow_nested_calls() {
        let code = r#"
rune test_hmac_computation() -> int {
    mac := hmac("secret_key_123", "hello_world")
    if mac == "" {
        ret 1
    }
    if !constant_time_eq(mac, mac) {
        ret 2
    }
    if constant_time_eq(mac, "tampered_mac") {
        ret 3
    }
    ret 0
}
"#;
        let lexer = CariaLexer::new(code);
        let mut parser = Parser::new(lexer);
        let prog = parser.parse_program().expect("Failed to parse control flow nested calls");
        assert_eq!(prog.statements.len(), 1);
    }

}
