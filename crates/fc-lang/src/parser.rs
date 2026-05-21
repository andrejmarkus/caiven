use crate::ast::*;
use crate::error::{LangError, Result};
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn line(&self) -> usize {
        self.tokens.get(self.pos).map(|t| t.line).unwrap_or(0)
    }

    fn peek(&self) -> &TokenKind {
        self.tokens.get(self.pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn peek2(&self) -> &TokenKind {
        self.tokens.get(self.pos + 1).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn advance(&mut self) -> &TokenKind {
        let t = &self.tokens[self.pos].kind;
        self.pos += 1;
        t
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), TokenKind::Newline | TokenKind::Semicolon) {
            self.advance();
        }
    }

    fn expect(&mut self, expected: TokenKind) -> Result<()> {
        self.skip_newlines();
        let line = self.line();
        if self.peek() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(LangError::UnexpectedToken {
                line,
                got: self.peek().to_string(),
                expected: expected.to_string(),
            })
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        self.skip_newlines();
        let line = self.line();
        match self.peek().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(LangError::UnexpectedToken {
                line,
                got: other.to_string(),
                expected: "identifier".to_string(),
            }),
        }
    }

    // Parse a u32 literal (number constant)
    fn parse_literal(&mut self) -> Result<u32> {
        self.skip_newlines();
        let line = self.line();
        match self.peek().clone() {
            TokenKind::Number(n) => { self.advance(); Ok(n) }
            other => Err(LangError::UnexpectedToken {
                line,
                got: other.to_string(),
                expected: "number literal".to_string(),
            }),
        }
    }

    pub fn parse_file(&mut self) -> Result<SourceFile> {
        let mut consts = Vec::new();
        let mut globals = Vec::new();
        let mut init_block = None;
        let mut loop_block = None;
        let mut functions = Vec::new();

        loop {
            self.skip_newlines();
            match self.peek().clone() {
                TokenKind::Eof => break,
                TokenKind::Const => {
                    self.advance();
                    let line = self.line();
                    let name = self.expect_ident()?;
                    self.expect(TokenKind::Eq)?;
                    let value = self.parse_literal()?;
                    consts.push(ConstDecl { name, value, line });
                }
                TokenKind::Let => {
                    self.advance();
                    let line = self.line();
                    let name = self.expect_ident()?;
                    let init = if matches!(self.peek(), TokenKind::Eq) {
                        self.advance();
                        self.parse_expr()?
                    } else {
                        Expr::Number(0, line)
                    };
                    globals.push(LetDecl { name, init, line });
                }
                TokenKind::Fn | TokenKind::Function => {
                    self.advance();
                    let line = self.line();
                    let name = self.expect_ident()?;
                    let params = self.parse_param_list()?;
                    let body = self.parse_block()?;
                    self.expect(TokenKind::End)?;
                    functions.push(FnDecl { name, params, body, line });
                }
                TokenKind::Ident(ref name) if name == "init" => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    let body = self.parse_block()?;
                    init_block = Some(body);
                }
                TokenKind::Ident(ref name) if name == "loop" => {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    let body = self.parse_block()?;
                    loop_block = Some(body);
                }
                other => {
                    let line = self.line();
                    return Err(LangError::UnexpectedToken {
                        line,
                        got: other.to_string(),
                        expected: "const / let / fn / function / init / loop".to_string(),
                    });
                }
            }
        }

        Ok(SourceFile { consts, globals, init_block, loop_block, functions })
    }

    fn parse_param_list(&mut self) -> Result<Vec<String>> {
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        self.skip_newlines();
        if !matches!(self.peek(), TokenKind::RParen) {
            params.push(self.expect_ident()?);
            while matches!(self.peek(), TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
                if matches!(self.peek(), TokenKind::RParen) { break; }
                params.push(self.expect_ident()?);
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(params)
    }

    fn is_block_end(&self) -> bool {
        match self.peek() {
            TokenKind::End | TokenKind::Else | TokenKind::Elseif
            | TokenKind::Until | TokenKind::Eof => true,
            TokenKind::Ident(name) if name == "init" || name == "loop" => true,
            _ => false,
        }
    }

    fn parse_block(&mut self) -> Result<Block> {
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.is_block_end() {
                break;
            }
            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        self.skip_newlines();
        let line = self.line();
        match self.peek().clone() {
            TokenKind::Local => {
                self.advance();
                let mut names = Vec::new();
                names.push(self.expect_ident()?);
                while matches!(self.peek(), TokenKind::Comma) {
                    self.advance();
                    self.skip_newlines();
                    names.push(self.expect_ident()?);
                }
                let mut inits = Vec::new();
                if matches!(self.peek(), TokenKind::Eq) {
                    self.advance();
                    inits.push(self.parse_expr()?);
                    while matches!(self.peek(), TokenKind::Comma) {
                        self.advance();
                        inits.push(self.parse_expr()?);
                    }
                }
                Ok(Stmt::Local { names, inits, line })
            }
            TokenKind::If => {
                self.advance();
                let cond = self.parse_expr()?;
                self.expect(TokenKind::Then)?;
                let then_block = self.parse_block()?;
                let mut elseif_clauses = Vec::new();
                let mut else_block = None;
                loop {
                    self.skip_newlines();
                    match self.peek().clone() {
                        TokenKind::Elseif => {
                            self.advance();
                            let ec = self.parse_expr()?;
                            self.expect(TokenKind::Then)?;
                            let eb = self.parse_block()?;
                            elseif_clauses.push((ec, eb));
                        }
                        TokenKind::Else => {
                            self.advance();
                            else_block = Some(self.parse_block()?);
                            break;
                        }
                        _ => break,
                    }
                }
                self.expect(TokenKind::End)?;
                Ok(Stmt::If { cond, then_block, elseif_clauses, else_block, line })
            }
            TokenKind::While => {
                self.advance();
                let cond = self.parse_expr()?;
                self.expect(TokenKind::Do)?;
                let body = self.parse_block()?;
                self.expect(TokenKind::End)?;
                Ok(Stmt::While { cond, body, line })
            }
            TokenKind::Repeat => {
                self.advance();
                let body = self.parse_block()?;
                self.expect(TokenKind::Until)?;
                let cond = self.parse_expr()?;
                Ok(Stmt::Repeat { body, cond, line })
            }
            TokenKind::For => {
                self.advance();
                let var = self.expect_ident()?;
                self.expect(TokenKind::Eq)?;
                let start = self.parse_expr()?;
                self.expect(TokenKind::Comma)?;
                let stop = self.parse_expr()?;
                let step = if matches!(self.peek(), TokenKind::Comma) {
                    self.advance();
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect(TokenKind::Do)?;
                let body = self.parse_block()?;
                self.expect(TokenKind::End)?;
                Ok(Stmt::NumericFor { var, start, stop, step, body, line })
            }
            TokenKind::Do => {
                self.advance();
                let body = self.parse_block()?;
                self.expect(TokenKind::End)?;
                Ok(Stmt::Do { body, line })
            }
            TokenKind::Return => {
                self.advance();
                let mut values = Vec::new();
                self.skip_newlines();
                if !self.is_block_end() && !matches!(self.peek(), TokenKind::Newline | TokenKind::Semicolon | TokenKind::Eof) {
                    values.push(self.parse_expr()?);
                    while matches!(self.peek(), TokenKind::Comma) {
                        self.advance();
                        values.push(self.parse_expr()?);
                    }
                }
                Ok(Stmt::Return { values, line })
            }
            TokenKind::Break => {
                self.advance();
                Ok(Stmt::Break { line })
            }
            TokenKind::Ident(_) => {
                // Could be: plain assign, compound assign, call, or chained accessor (t.f, t[k], t:m())
                let name = self.expect_ident()?;
                // Check for postfix chain: . [ : before deciding on simple assign vs table op
                if matches!(self.peek(), TokenKind::Dot | TokenKind::LBracket | TokenKind::Colon) {
                    // Build base expr then follow chain like parse_primary does
                    let mut base: Expr = Expr::Var(name, line);
                    // Walk all but last postfix so we can distinguish LHS from call
                    loop {
                        match self.peek().clone() {
                            TokenKind::Dot => {
                                self.advance();
                                let field = self.expect_ident()?;
                                if matches!(self.peek(), TokenKind::Eq) {
                                    self.advance();
                                    let value = self.parse_expr()?;
                                    return Ok(Stmt::SetField { table: base, name: field, value, line });
                                }
                                if matches!(self.peek(), TokenKind::PlusEq | TokenKind::MinusEq) {
                                    let op = if matches!(self.peek(), TokenKind::PlusEq) { BinOp::Add } else { BinOp::Sub };
                                    self.advance();
                                    let rhs = self.parse_expr()?;
                                    let read = Expr::Field { table: Box::new(base.clone()), name: field.clone(), line };
                                    let value = Expr::BinOp { op, left: Box::new(read), right: Box::new(rhs), line };
                                    return Ok(Stmt::SetField { table: base, name: field, value, line });
                                }
                                base = Expr::Field { table: Box::new(base), name: field, line };
                            }
                            TokenKind::LBracket => {
                                self.advance();
                                let key = self.parse_expr()?;
                                self.expect(TokenKind::RBracket)?;
                                if matches!(self.peek(), TokenKind::Eq) {
                                    self.advance();
                                    let value = self.parse_expr()?;
                                    return Ok(Stmt::SetIndex { table: base, key, value, line });
                                }
                                if matches!(self.peek(), TokenKind::PlusEq | TokenKind::MinusEq) {
                                    let op = if matches!(self.peek(), TokenKind::PlusEq) { BinOp::Add } else { BinOp::Sub };
                                    self.advance();
                                    let rhs = self.parse_expr()?;
                                    let read = Expr::Index { table: Box::new(base.clone()), key: Box::new(key.clone()), line };
                                    let value = Expr::BinOp { op, left: Box::new(read), right: Box::new(rhs), line };
                                    return Ok(Stmt::SetIndex { table: base, key, value, line });
                                }
                                base = Expr::Index { table: Box::new(base), key: Box::new(key), line };
                            }
                            TokenKind::Colon => {
                                // Method call: base:name(args) → call(base.name, base, args)
                                self.advance();
                                let method = self.expect_ident()?;
                                let func_expr = Expr::Field { table: Box::new(base.clone()), name: method, line };
                                let mut args = vec![base.clone()];
                                let extra = self.parse_call_args()?;
                                args.extend(extra);
                                base = Expr::Call { func: Box::new(func_expr), args, line };
                                // After method call, may continue chain or end as ExprStmt
                                if !matches!(self.peek(), TokenKind::Dot | TokenKind::LBracket | TokenKind::Colon | TokenKind::LParen) {
                                    return Ok(Stmt::ExprStmt { expr: base, line });
                                }
                            }
                            TokenKind::LParen => {
                                let args = self.parse_call_args()?;
                                base = Expr::Call { func: Box::new(base), args, line };
                                if !matches!(self.peek(), TokenKind::Dot | TokenKind::LBracket | TokenKind::Colon | TokenKind::LParen) {
                                    return Ok(Stmt::ExprStmt { expr: base, line });
                                }
                            }
                            _ => {
                                return Err(LangError::UnexpectedToken {
                                    line,
                                    got: self.peek().to_string(),
                                    expected: "= / ( / . / [ / :".to_string(),
                                });
                            }
                        }
                    }
                } else {
                    match self.peek().clone() {
                        TokenKind::Eq => {
                            self.advance();
                            let value = self.parse_expr()?;
                            Ok(Stmt::Assign { target: name, value, line })
                        }
                        TokenKind::PlusEq => {
                            self.advance();
                            let rhs = self.parse_expr()?;
                            let value = Expr::BinOp {
                                op: BinOp::Add,
                                left: Box::new(Expr::Var(name.clone(), line)),
                                right: Box::new(rhs),
                                line,
                            };
                            Ok(Stmt::Assign { target: name, value, line })
                        }
                        TokenKind::MinusEq => {
                            self.advance();
                            let rhs = self.parse_expr()?;
                            let value = Expr::BinOp {
                                op: BinOp::Sub,
                                left: Box::new(Expr::Var(name.clone(), line)),
                                right: Box::new(rhs),
                                line,
                            };
                            Ok(Stmt::Assign { target: name, value, line })
                        }
                        TokenKind::LParen => {
                            let args = self.parse_call_args()?;
                            Ok(Stmt::ExprStmt {
                                expr: Expr::Call {
                                    func: Box::new(Expr::Var(name, line)),
                                    args,
                                    line,
                                },
                                line,
                            })
                        }
                        other => {
                            Err(LangError::UnexpectedToken {
                                line,
                                got: other.to_string(),
                                expected: "= / += / -= / (".to_string(),
                            })
                        }
                    }
                }
            }
            other => {
                Err(LangError::UnexpectedToken {
                    line,
                    got: other.to_string(),
                    expected: "statement".to_string(),
                })
            }
        }
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>> {
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        self.skip_newlines();
        if !matches!(self.peek(), TokenKind::RParen) {
            args.push(self.parse_expr()?);
            while matches!(self.peek(), TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
                if matches!(self.peek(), TokenKind::RParen) { break; }
                args.push(self.parse_expr()?);
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(args)
    }

    // Lua 5.1 precedence (low to high):
    // or → and → comparisons → .. → +- → */%  → unary → ^ → primary
    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        loop {
            self.skip_newlines();
            if matches!(self.peek(), TokenKind::Or) {
                let line = self.line();
                self.advance();
                let right = self.parse_and()?;
                left = Expr::BinOp { op: BinOp::Or, left: Box::new(left), right: Box::new(right), line };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_comparison()?;
        loop {
            self.skip_newlines();
            if matches!(self.peek(), TokenKind::And) {
                let line = self.line();
                self.advance();
                let right = self.parse_comparison()?;
                left = Expr::BinOp { op: BinOp::And, left: Box::new(left), right: Box::new(right), line };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let left = self.parse_concat()?;
        let line = self.line();
        let op = match self.peek() {
            TokenKind::EqEq => BinOp::Eq,
            TokenKind::NotEq | TokenKind::TildeEq => BinOp::NotEq,
            TokenKind::Lt => BinOp::Lt,
            TokenKind::Gt => BinOp::Gt,
            TokenKind::LtEq => BinOp::LtEq,
            TokenKind::GtEq => BinOp::GtEq,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.parse_concat()?;
        Ok(Expr::BinOp { op, left: Box::new(left), right: Box::new(right), line })
    }

    fn parse_concat(&mut self) -> Result<Expr> {
        let left = self.parse_additive()?;
        if matches!(self.peek(), TokenKind::DotDot) {
            let line = self.line();
            self.advance();
            // right-associative
            let right = self.parse_concat()?;
            Ok(Expr::BinOp { op: BinOp::Concat, left: Box::new(left), right: Box::new(right), line })
        } else {
            Ok(left)
        }
    }

    fn parse_additive(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let line = self.line();
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let line = self.line();
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        let line = self.line();
        match self.peek().clone() {
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnOp { op: UnOp::Neg, expr: Box::new(expr), line })
            }
            TokenKind::Not => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnOp { op: UnOp::Not, expr: Box::new(expr), line })
            }
            TokenKind::Hash => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnOp { op: UnOp::Len, expr: Box::new(expr), line })
            }
            _ => self.parse_power(),
        }
    }

    fn parse_power(&mut self) -> Result<Expr> {
        let base = self.parse_primary()?;
        if matches!(self.peek(), TokenKind::Caret) {
            let line = self.line();
            self.advance();
            // right-associative
            let exp = self.parse_unary()?;
            Ok(Expr::BinOp { op: BinOp::Pow, left: Box::new(base), right: Box::new(exp), line })
        } else {
            Ok(base)
        }
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        self.skip_newlines();
        let line = self.line();
        match self.peek().clone() {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Number(n, line))
            }
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Expr::Str(s, line))
            }
            TokenKind::Nil => {
                self.advance();
                Ok(Expr::Nil(line))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::True(line))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::False(line))
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            TokenKind::LBrace => {
                self.parse_table_ctor()
            }
            TokenKind::Fn | TokenKind::Function => {
                self.advance();
                let params = self.parse_param_list()?;
                let body = self.parse_block()?;
                self.expect(TokenKind::End)?;
                Ok(Expr::Func { params, body, line })
            }
            TokenKind::Ident(name) => {
                self.advance();
                // Could be a call or just a var
                let mut expr = Expr::Var(name, line);
                loop {
                    match self.peek().clone() {
                        TokenKind::LParen => {
                            let args = self.parse_call_args()?;
                            let l = expr.line();
                            expr = Expr::Call { func: Box::new(expr), args, line: l };
                        }
                        TokenKind::Dot => {
                            self.advance();
                            let field_name = self.expect_ident()?;
                            let l = expr.line();
                            expr = Expr::Field { table: Box::new(expr), name: field_name, line: l };
                        }
                        TokenKind::LBracket => {
                            self.advance();
                            let key = self.parse_expr()?;
                            self.expect(TokenKind::RBracket)?;
                            let l = expr.line();
                            expr = Expr::Index { table: Box::new(expr), key: Box::new(key), line: l };
                        }
                        _ => break,
                    }
                }
                Ok(expr)
            }
            other => Err(LangError::UnexpectedToken {
                line,
                got: other.to_string(),
                expected: "expression".to_string(),
            }),
        }
    }

    fn parse_table_ctor(&mut self) -> Result<Expr> {
        let line = self.line();
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), TokenKind::RBrace) { break; }
            let field = match self.peek().clone() {
                TokenKind::LBracket => {
                    self.advance();
                    let key = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    self.expect(TokenKind::Eq)?;
                    let value = self.parse_expr()?;
                    TableField::IndexField { key, value }
                }
                TokenKind::Ident(name) if matches!(self.peek2(), TokenKind::Eq) => {
                    self.advance(); // consume ident
                    self.advance(); // consume '='
                    let value = self.parse_expr()?;
                    TableField::NameField { name, value }
                }
                _ => {
                    let value = self.parse_expr()?;
                    TableField::ValueField { value }
                }
            };
            fields.push(field);
            self.skip_newlines();
            match self.peek() {
                TokenKind::Comma | TokenKind::Semicolon => { self.advance(); }
                _ => {}
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::Table { fields, line })
    }
}
