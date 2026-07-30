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

            // HTTP Routing
            Some((Ok(Token::Get), _)) => self.parse_route_decl(ast::HttpMethod::Get),
            Some((Ok(Token::Post), _)) => self.parse_route_decl(ast::HttpMethod::Post),
            Some((Ok(Token::Put), _)) => self.parse_route_decl(ast::HttpMethod::Put),
            Some((Ok(Token::Patch), _)) => self.parse_route_decl(ast::HttpMethod::Patch),
            Some((Ok(Token::Delete), _)) => self.parse_route_decl(ast::HttpMethod::Delete),
            Some((Ok(Token::RouteGroup), _)) => self.parse_route_group(),
            Some((Ok(Token::ResourceRoute), _)) => self.parse_resource_route(),

            // Modules & Code Organization
            Some((Ok(Token::Summon), _)) => {
                self.lexer.next(); // consume summon
                
                let mut path = String::new();
                loop {
                    match self.lexer.next() {
                        Some((Ok(Token::Ident(p)), _)) => path.push_str(&p),
                        Some((Err(_), span)) => return Err(ParseError::UnexpectedToken(format!("Lexer error at {}..{}", span.start, span.end))),
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

            // Model / ORM
            Some((Ok(Token::Model), _)) => {
                self.parse_model_decl(doc_comment).map(ast::Statement::ModelDecl)
            }

            // View / Template
            Some((Ok(Token::View), _)) => {
                self.parse_view_decl().map(ast::Statement::ViewDecl)
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

            // Web Blocks
            Some((Ok(Token::Validate), _)) => {
                self.parse_validate_block().map(ast::Statement::ValidateBlock)
            }
            Some((Ok(Token::Transaction), _)) => {
                self.parse_transaction_block().map(ast::Statement::TransactionBlock)
            }
            Some((Ok(Token::Queue), _)) => {
                self.parse_queue_stmt().map(ast::Statement::QueueStmt)
            }
            Some((Ok(Token::Stream), _)) => {
                self.parse_stream_decl().map(ast::Statement::StreamDecl)
            }
            Some((Ok(Token::Js), _)) => {
                self.parse_js_block().map(ast::Statement::JsBlock)
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
                                Some((Ok(Token::Ident(n)), _)) => names.push(n),
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
            Some((Ok(Token::Ident(n)), _)) => n,
            Some((Ok(Token::Get), _)) => "get".to_string(),
            Some((Ok(Token::Post), _)) => "post".to_string(),
            Some((Ok(Token::Put), _)) => "put".to_string(),
            Some((Ok(Token::Patch), _)) => "patch".to_string(),
            Some((Ok(Token::Delete), _)) => "delete".to_string(),
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
                    Some((Ok(Token::Ident(n)), _)) => n,
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
        let condition = Box::new(self.parse_expression(0)?);
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
        let condition = Box::new(self.parse_expression(0)?);
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
        let condition = Box::new(self.parse_expression(0)?);
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
            Some((Ok(Token::Ident(n)), _)) => n,
            _ => return Err(ParseError::UnexpectedToken("Expected variable name in for loop".into())),
        };

        // Optional second variable: for item, index in ...
        let mut var2 = None;
        if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
            self.lexer.next();
            match self.lexer.next() {
                Some((Ok(Token::Ident(n)), _)) => var2 = Some(n),
                _ => return Err(ParseError::UnexpectedToken("Expected index variable".into())),
            }
        }

        self.expect(Token::In)?;
        let start_expr = self.parse_expression(0)?;

        // Range: for i in start..end [step N]
        if let Some((Ok(Token::Range), _)) = self.lexer.peek() {
            self.lexer.next(); // consume ..
            let end_expr = self.parse_expression(0)?;

            // Optional step
            let step = if let Some((Ok(Token::Step), _)) = self.lexer.peek() {
                self.lexer.next(); // consume step
                Some(Box::new(self.parse_expression(0)?))
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

    fn parse_route_decl(&mut self, method: ast::HttpMethod) -> Result<ast::Statement, ParseError> {
        self.lexer.next(); // consume get/post/put/patch/delete

        let path = match self.lexer.next() {
            Some((Ok(Token::Str(p)), _)) => p,
            Some((Ok(Token::Ident(i)), _)) if i == "health_check" => {
                return Err(ParseError::UnexpectedToken("Expected route path string".into()));
            }
            _ => return Err(ParseError::UnexpectedToken("Expected route path".into())),
        };

        // health check shorthand: get "/health" health_check
        if let Some((Ok(Token::Ident(n)), _)) = self.lexer.peek() {
            if n == "health_check" {
                self.lexer.next(); // consume health_check
                return Ok(ast::Statement::RouteDecl(ast::RouteDecl {
                    method,
                    path,
                    body: vec![ast::Statement::RetStmt(ast::Expression::Literal(ast::Literal::Str("ok".to_string())))],
                    decorators: vec![],
                    with_clauses: vec![],
                }));
            }
        }

        let mut decorators = Vec::new();
        while let Some((Ok(Token::With), _)) = self.lexer.peek() {
            self.lexer.next(); // consume with
            let name = match self.lexer.next() {
                Some((Ok(Token::Ident(n)), _)) => n,
                _ => return Err(ParseError::UnexpectedToken("Expected decorator name after with".into())),
            };
            let mut args = Vec::new();
            if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
                self.lexer.next();
                while let Some((Ok(tok), _)) = self.lexer.peek() {
                    if tok == &Token::RParen { break; }
                    args.push(self.parse_expression(0)?);
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::RParen)?;
            }
            decorators.push(ast::RouteDecorator { name, args });
        }

        let mut with_clauses = Vec::new();
        if let Some((Ok(Token::With), _)) = self.lexer.peek() {
            self.lexer.next();
            loop {
                let name = match self.lexer.next() {
                    Some((Ok(Token::Ident(n)), _)) => n,
                    _ => return Err(ParseError::UnexpectedToken("Expected decorator name in with clause".into())),
                };
                let mut args = Vec::new();
                if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
                    self.lexer.next();
                    while let Some((Ok(tok), _)) = self.lexer.peek() {
                        if tok == &Token::RParen { break; }
                        args.push(self.parse_expression(0)?);
                        if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                            self.lexer.next();
                        }
                    }
                    self.expect(Token::RParen)?;
                }
                with_clauses.push(ast::WithClause { name, args });
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                    self.lexer.next();
                } else {
                    break;
                }
            }
        }

        let body = if let Some((Ok(Token::Arrow), _)) = self.lexer.peek() {
            self.lexer.next(); // consume ->
            let expr = self.parse_expression(0)?;
            vec![ast::Statement::RetStmt(expr)]
        } else {
            self.expect(Token::LBrace)?;
            let mut body = Vec::new();
            while let Some((Ok(tok), _)) = self.lexer.peek() {
                if tok == &Token::RBrace { break; }
                body.push(self.parse_statement()?);
            }
            self.expect(Token::RBrace)?;
            body
        };

        Ok(ast::Statement::RouteDecl(ast::RouteDecl { method, path, body, decorators, with_clauses }))
    }

    fn parse_route_group(&mut self) -> Result<ast::Statement, ParseError> {
        self.expect(Token::RouteGroup)?;

        let prefix = match self.lexer.next() {
            Some((Ok(Token::Str(p)), _)) => p,
            _ => return Err(ParseError::UnexpectedToken("Expected prefix path string after route".into())),
        };

        // Optional decorators: with auth(), with cors(...)
        let mut decorators = Vec::new();
        while let Some((Ok(Token::With), _)) = self.lexer.peek() {
            self.lexer.next(); // consume with
            let name = match self.lexer.next() {
                Some((Ok(Token::Ident(n)), _)) => n,
                _ => return Err(ParseError::UnexpectedToken("Expected decorator name after with".into())),
            };
            let mut args = Vec::new();
            if let Some((Ok(Token::LParen), _)) = self.lexer.peek() {
                self.lexer.next();
                while let Some((Ok(tok), _)) = self.lexer.peek() {
                    if tok == &Token::RParen { break; }
                    args.push(self.parse_expression(0)?);
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::RParen)?;
            }
            decorators.push(ast::RouteDecorator { name, args });
        }

        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        Ok(ast::Statement::RouteGroup(ast::RouteGroupStmt {
            prefix,
            decorators,
            body,
        }))
    }

    fn parse_resource_route(&mut self) -> Result<ast::Statement, ParseError> {
        self.expect(Token::ResourceRoute)?;

        let path = match self.lexer.next() {
            Some((Ok(Token::Str(p)), _)) => p,
            _ => return Err(ParseError::UnexpectedToken("Expected resource path string".into())),
        };

        self.expect(Token::Arrow)?;

        let controller = match self.lexer.next() {
            Some((Ok(Token::Ident(c)), _)) => c,
            Some((Ok(Token::Get), _)) => "get".to_string(),
            Some((Ok(Token::Post), _)) => "post".to_string(),
            Some((Ok(Token::Put), _)) => "put".to_string(),
            Some((Ok(Token::Patch), _)) => "patch".to_string(),
            Some((Ok(Token::Delete), _)) => "delete".to_string(),
            _ => return Err(ParseError::UnexpectedToken("Expected controller identifier after ->".into())),
        };

        let mut only = None;
        let mut except = None;

        if let Some((Ok(Token::Only), _)) = self.lexer.peek() {
            self.lexer.next(); // consume only
            self.expect(Token::LBracket)?;
            let mut actions = Vec::new();
            while let Some((Ok(tok), _)) = self.lexer.peek() {
                if tok == &Token::RBracket { break; }
                let action = match self.lexer.next() {
                    Some((Ok(Token::Ident(a)), _)) => a,
                    _ => return Err(ParseError::UnexpectedToken("Expected action identifier in only list".into())),
                };
                actions.push(action);
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                    self.lexer.next();
                }
            }
            self.expect(Token::RBracket)?;
            only = Some(actions);
        } else if let Some((Ok(Token::Except), _)) = self.lexer.peek() {
            self.lexer.next(); // consume except
            self.expect(Token::LBracket)?;
            let mut actions = Vec::new();
            while let Some((Ok(tok), _)) = self.lexer.peek() {
                if tok == &Token::RBracket { break; }
                let action = match self.lexer.next() {
                    Some((Ok(Token::Ident(a)), _)) => a,
                    _ => return Err(ParseError::UnexpectedToken("Expected action identifier in except list".into())),
                };
                actions.push(action);
                if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                    self.lexer.next();
                }
            }
            self.expect(Token::RBracket)?;
            except = Some(actions);
        }

        Ok(ast::Statement::ResourceRoute(ast::ResourceRouteStmt {
            path,
            controller,
            only,
            except,
        }))
    }

    // ── Model / ORM ──────────────────────────────────────────────────────────

    fn parse_model_decl(&mut self, doc_comment: Option<String>) -> Result<ast::ModelDecl, ParseError> {
        self.expect(Token::Model)?;

        let name = match self.lexer.next() {
            Some((Ok(Token::Ident(n)), _)) => n,
            _ => return Err(ParseError::UnexpectedToken("Expected model name".into())),
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

        self.expect(Token::LBrace)?;
        let mut fields = Vec::new();

        let mut methods = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }

            let is_static = if let Token::Ident(id) = token { id.as_str() == "static" } else { false };
            let is_rune = token == &Token::Rune;
            println!("DEBUG: parsing model member, token: {:?}, is_static: {}, is_rune: {}", token, is_static, is_rune);

            if is_rune || is_static {
                let mut is_public = false; 
                if is_static {
                    self.lexer.next();
                }
                let mut rune_decl = self.parse_rune_decl(None)?;
                
                if !is_static {
                    rune_decl.params.insert(0, ast::RuneParam {
                        name: "self".to_string(),
                        type_expr: ast::TypeExpr::Named(name.clone()),
                        default_value: None,
                        is_variadic: false,
                    });
                }
                
                methods.push(rune_decl);
                continue;
            }

            let field_name = match self.lexer.next() {
                Some((Ok(Token::Ident(n)), _)) => n,
                _ => return Err(ParseError::UnexpectedToken("Expected field name in model".into())),
            };

            self.expect(Token::Colon)?;
            let type_expr = self.parse_type_expr()?;

            // Tags: [pk, auto, unique, index, ...]
            let mut tags = Vec::new();
            if let Some((Ok(Token::LBracket), _)) = self.lexer.peek() {
                self.lexer.next(); // consume [
                while let Some((Ok(tok), _)) = self.lexer.peek() {
                    if tok == &Token::RBracket { break; }
                    let tag = match self.lexer.next() {
                        Some((Ok(Token::Ident(t)), _)) => t,
                        _ => break,
                    };
                    tags.push(tag);
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::RBracket)?;
            }

            // Default value: field = expr
            let default_value = if let Some((Ok(Token::Eq), _)) = self.lexer.peek() {
                self.lexer.next();
                Some(self.parse_expression(0)?)
            } else {
                None
            };

            fields.push(ast::ModelField { name: field_name, type_expr, tags, default_value });

            // Optional comma or newline separator
            if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                self.lexer.next();
            }
        }

        self.expect(Token::RBrace)?;
        Ok(ast::ModelDecl { name, fields, methods, is_public: false, doc_comment })
    }

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

    fn parse_validate_block(&mut self) -> Result<ast::ValidateBlock, ParseError> {
        self.expect(Token::Validate)?;
        self.expect(Token::LParen)?;
        let target = Box::new(self.parse_expression(0)?);
        self.expect(Token::RParen)?;
        self.expect(Token::As)?;
        self.expect(Token::LBrace)?;

        let mut fields = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            let name = match self.lexer.next() {
                Some((Ok(Token::Ident(n)), _)) => n,
                _ => break,
            };
            self.expect(Token::Colon)?;
            let type_expr = self.parse_type_expr()?;

            let mut rules = Vec::new();
            if let Some((Ok(Token::LBracket), _)) = self.lexer.peek() {
                self.lexer.next();
                while let Some((Ok(tok), _)) = self.lexer.peek() {
                    if tok == &Token::RBracket { break; }
                    let rule_name = match self.lexer.next() {
                        Some((Ok(Token::Ident(r)), _)) => r,
                        _ => break,
                    };
                    let rule = match rule_name.as_str() {
                        "required" => ast::ValidateRule::Required,
                        "email" => ast::ValidateRule::Email,
                        "url" => ast::ValidateRule::Url,
                        "min" => {
                            self.expect(Token::Colon)?;
                            ast::ValidateRule::Min(self.parse_expression(0)?)
                        }
                        "max" => {
                            self.expect(Token::Colon)?;
                            ast::ValidateRule::Max(self.parse_expression(0)?)
                        }
                        "min_len" => {
                            self.expect(Token::Colon)?;
                            ast::ValidateRule::MinLen(self.parse_expression(0)?)
                        }
                        "max_len" => {
                            self.expect(Token::Colon)?;
                            ast::ValidateRule::MaxLen(self.parse_expression(0)?)
                        }
                        other => ast::ValidateRule::Custom(other.to_string()),
                    };
                    rules.push(rule);
                    if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                        self.lexer.next();
                    }
                }
                self.expect(Token::RBracket)?;
            }
            fields.push(ast::ValidateField { name, type_expr, rules });
            if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                self.lexer.next();
            }
        }
        self.expect(Token::RBrace)?;
        Ok(ast::ValidateBlock { target, fields })
    }

    fn parse_transaction_block(&mut self) -> Result<ast::TransactionBlock, ParseError> {
        self.expect(Token::Transaction)?;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(ast::TransactionBlock { body })
    }

    fn parse_queue_stmt(&mut self) -> Result<ast::QueueStmt, ParseError> {
        self.expect(Token::Queue)?;
        let call = Box::new(self.parse_expression(0)?);
        Ok(ast::QueueStmt { call })
    }

    fn parse_stream_decl(&mut self) -> Result<ast::StreamDecl, ParseError> {
        self.expect(Token::Stream)?;
        let path = match self.lexer.next() {
            Some((Ok(Token::Str(p)), _)) => p,
            _ => return Err(ParseError::UnexpectedToken("Expected path string after stream".into())),
        };
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while let Some((Ok(token), _)) = self.lexer.peek() {
            if token == &Token::RBrace { break; }
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(ast::StreamDecl { path, body })
    }

    fn parse_js_block(&mut self) -> Result<ast::JsBlock, ParseError> {
        self.expect(Token::Js)?;
        self.expect(Token::LBrace)?;
        // Collect raw tokens as string until matching }
        let mut raw = String::new();
        let mut depth = 1u32;
        loop {
            match self.lexer.next() {
                Some((Ok(Token::LBrace), _)) => { depth += 1; raw.push('{'); }
                Some((Ok(Token::RBrace), _)) => {
                    depth -= 1;
                    if depth == 0 { break; }
                    raw.push('}');
                }
                Some((Ok(Token::Ident(s)), _)) => { raw.push_str(&s); raw.push(' '); }
                Some((Ok(Token::Str(s)), _)) => { raw.push('"'); raw.push_str(&s); raw.push('"'); }
                Some((Ok(Token::Int(n)), _)) => { raw.push_str(&n.to_string()); }
                Some((Ok(Token::Float(f)), _)) => { raw.push_str(&f.to_string()); }
                Some((Ok(Token::Dot), _)) => { raw.push('.'); }
                Some((Ok(Token::LParen), _)) => { raw.push('('); }
                Some((Ok(Token::RParen), _)) => { raw.push(')'); }
                Some((Ok(Token::Comma), _)) => { raw.push_str(", "); }
                Some((Ok(Token::Semi), _)) => { raw.push_str(";\n"); }
                Some((Ok(_), _)) => { raw.push(' '); }
                None => return Err(ParseError::UnexpectedEOF("Unclosed js block".into())),
                Some((Err(_), _)) => {}
            }
        }
        Ok(ast::JsBlock { raw: raw.trim().to_string() })
    }

    // ── View / HTML Template ──────────────────────────────────────────────────

    fn parse_view_decl(&mut self) -> Result<ast::ViewDecl, ParseError> {
        self.expect(Token::View)?;
        let name = match self.lexer.next() {
            Some((Ok(Token::Ident(n)), _)) => n,
            _ => return Err(ParseError::UnexpectedToken("Expected view name".into())),
        };
        let params = self.parse_rune_params()?;
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
                    self.lexer.next();
                    // Closing tag </tag>
                    if let Some((Ok(Token::Slash), _)) = self.lexer.peek() {
                        self.lexer.next();
                        self.lexer.next(); // consume tag name
                        self.expect(Token::Greater)?;
                        break;
                    }
                    let tag = match self.lexer.next() {
                        Some((Ok(Token::Ident(t)), _)) => t,
                        _ => return Err(ParseError::UnexpectedToken("Expected tag name".into())),
                    };
                    let mut attrs = Vec::new();
                    let mut self_closing = false;
                    while let Some((Ok(tok), _)) = self.lexer.peek() {
                        match tok {
                            Token::Greater => { self.lexer.next(); break; }
                            Token::Slash => {
                                self.lexer.next();
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
                                    self.lexer.next();
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
                    self.lexer.next();
                    if let Some((Ok(Token::For), _)) = self.lexer.peek() {
                        self.lexer.next();
                        let item = match self.lexer.next() {
                            Some((Ok(Token::Ident(n)), _)) => n,
                            _ => return Err(ParseError::UnexpectedToken("Expected item variable".into())),
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
                _ => { self.lexer.next(); }
            }
        }
        Ok(children)
    }

    // ── Expression Parsing (Pratt Parser) ────────────────────────────────────

    fn parse_expression(&mut self, min_bp: u8) -> Result<ast::Expression, ParseError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            // Check for postfix operators
            match self.lexer.peek() {
                Some((Ok(Token::Question), _)) => {
                    self.lexer.next();
                    lhs = ast::Expression::ErrorPropagate(Box::new(lhs));
                    continue;
                }
                Some((Ok(Token::Dot), _)) | Some((Ok(Token::SafeChain), _)) => {
                    let is_safe = matches!(self.lexer.peek(), Some((Ok(Token::SafeChain), _)));
                    self.lexer.next(); // consume . or ?.
                    let member = match self.lexer.next() {
                        Some((Ok(Token::Ident(n)), _)) => n,
                        Some((Ok(Token::Get), _)) => "get".to_string(),
                        Some((Ok(Token::Post), _)) => "post".to_string(),
                        Some((Ok(Token::Put), _)) => "put".to_string(),
                        Some((Ok(Token::Patch), _)) => "patch".to_string(),
                        Some((Ok(Token::Delete), _)) => "delete".to_string(),
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

                        // Trailing closure check for method calls (Disabled to prevent eating `if` blocks)
                        /*
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
                        */

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
                        Some((Ok(Token::Ident(n)), _)) => params.push(n),
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
                let expr = self.parse_expression(0)?;
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

            // view("welcome")
            Some((Ok(Token::View), _)) => {
                self.lexer.next();
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
                Ok(ast::Expression::CallExpr {
                    callee: Box::new(ast::Expression::Identifier("view".to_string())),
                    args,
                })
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

            // Identifier → may be function call
            Some((Ok(Token::Ident(_)), _)) => {
                let name = match self.lexer.next() {
                    Some((Ok(Token::Ident(n)), _)) => n,
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

                    return Ok(ast::Expression::CallExpr {
                        callee: Box::new(ast::Expression::Identifier(name)),
                        args,
                    });
                }

                // Struct instantiation: TypeName { field: val, ... }
                if let Some((Ok(Token::LBrace), _)) = self.lexer.peek() {
                    // Only parse as struct if name starts with uppercase
                    if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        self.lexer.next();
                        let mut fields = Vec::new();
                        while let Some((Ok(tok), _)) = self.lexer.peek() {
                            if tok == &Token::RBrace { break; }
                            let field_name = match self.lexer.next() {
                                Some((Ok(Token::Ident(n)), _)) => n,
                                _ => break,
                            };
                            self.expect(Token::Colon)?;
                            let val = self.parse_expression(0)?;
                            fields.push((field_name, val));
                            if let Some((Ok(Token::Comma), _)) = self.lexer.peek() {
                                self.lexer.next();
                            }
                        }
                        self.expect(Token::RBrace)?;
                        return Ok(ast::Expression::StructInst { name, fields });
                    }
                }

                Ok(ast::Expression::Identifier(name))
            }

            Some((tok, _)) => Err(ParseError::UnexpectedToken(
                format!("Unexpected token in expression: {:?}", tok)
            )),
            None => Err(ParseError::UnexpectedEOF("Expected expression".into())),
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
            Some((Ok(token), _)) => Err(ParseError::UnexpectedToken(format!(
                "Expected {:?}, got {:?}", expected, token
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
    fn test_parse_model() {
        let lexer = CariaLexer::new("model User {\n    id: int [pk, auto]\n    name: txt\n}");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::ModelDecl(m) => {
                assert_eq!(m.name, "User");
                assert_eq!(m.fields.len(), 2);
                assert_eq!(m.fields[0].name, "id");
                assert!(m.fields[0].tags.contains(&"pk".to_string()));
            }
            _ => panic!("Expected ModelDecl"),
        }
    }

    #[test]
    fn test_parse_route_get() {
        let lexer = CariaLexer::new("get \"/users\" { ret nil }");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::RouteDecl(r) => {
                assert_eq!(r.method, ast::HttpMethod::Get);
                assert_eq!(r.path, "/users");
            }
            _ => panic!("Expected RouteDecl"),
        }
    }

    #[test]
    fn test_parse_include() {
        let lexer = CariaLexer::new("include \"routes/web.cria\"");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::Include(path) => {
                assert_eq!(path, "routes/web.cria");
            }
            _ => panic!("Expected Include statement"),
        }
    }

    #[test]
    fn test_parse_route_group() {
        let lexer = CariaLexer::new("route \"/api/v1\" with auth() {\n get \"/users\" -> handler\n}");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::RouteGroup(g) => {
                assert_eq!(g.prefix, "/api/v1");
                assert_eq!(g.decorators.len(), 1);
                assert_eq!(g.decorators[0].name, "auth");
                assert_eq!(g.body.len(), 1);
            }
            _ => panic!("Expected RouteGroup statement"),
        }
    }

    #[test]
    fn test_parse_resource_route() {
        let lexer = CariaLexer::new("resource \"/posts\" -> PostController only [index, show]");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::ResourceRoute(r) => {
                assert_eq!(r.path, "/posts");
                assert_eq!(r.controller, "PostController");
                assert_eq!(r.only, Some(vec!["index".to_string(), "show".to_string()]));
                assert_eq!(r.except, None);
            }
            _ => panic!("Expected ResourceRoute statement"),
        }
    }

    #[test]
    fn test_parse_route_arrow() {
        let lexer = CariaLexer::new("get \"/\" -> view(\"welcome\")");
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program().unwrap();
        match &program.statements[0] {
            ast::Statement::RouteDecl(r) => {
                assert_eq!(r.path, "/");
                assert_eq!(r.body.len(), 1);
            }
            _ => panic!("Expected RouteDecl statement"),
        }
    }
}