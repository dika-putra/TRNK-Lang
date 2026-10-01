//! TRU Language Parser — recursive-descent, produces an AST.

use crate::tru_lang::lexer::{Lexer, Token, TokenKind, Span};
use crate::tru_lang::ast::*;

// ─── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub span: Span,
}

impl ParseError {
    fn new(msg: impl Into<String>, span: Span) -> Self {
        Self { msg: msg.into(), span }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ParseError] {} at {}:{}", self.msg, self.span.line, self.span.col)
    }
}

type PResult<T> = Result<T, ParseError>;

// ─── Parser ───────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(source: &str) -> PResult<Program> {
        let mut lex = Lexer::new(source);
        let tokens = lex.tokenize().map_err(|e| {
            ParseError::new(e.to_string(), Span { line: 0, col: 0 })
        })?;
        let mut p = Parser::new(tokens);
        p.parse_program()
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos.min(self.tokens.len() - 1)].kind
    }

    fn peek_tok(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn span(&self) -> Span {
        self.peek_tok().span.clone()
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() - 1 { self.pos += 1; }
        tok
    }

    fn expect(&mut self, kind: &TokenKind) -> PResult<Span> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(kind) {
            let s = self.span();
            self.advance();
            Ok(s)
        } else {
            Err(ParseError::new(
                format!("expected {:?}, got {:?}", kind, self.peek()),
                self.span(),
            ))
        }
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(kind) {
            self.advance();
            true
        } else { false }
    }

    fn peek_at(&self, n: usize) -> &TokenKind {
        &self.tokens[self.pos.saturating_add(n).min(self.tokens.len() - 1)].kind
    }

    /// `Ident {` is only a struct literal if the braces open a field list
    /// (`field:`), never a block. Otherwise `while i < n { ... }` would parse
    /// `n { ... }` as a literal.
    fn at_struct_literal(&self) -> bool {
        matches!(self.peek(), TokenKind::LBrace)
            && matches!(self.peek_at(1), TokenKind::Ident(_))
            && matches!(self.peek_at(2), TokenKind::Colon)
    }

    fn eat_semicolons(&mut self) {
        while self.eat(&TokenKind::Semicolon) {}
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    // ── program ──────────────────────────────────────────────────────────────

    fn parse_program(&mut self) -> PResult<Program> {
        let mut stmts = Vec::new();
        while !self.is_eof() {
            self.eat_semicolons();
            if self.is_eof() { break; }
            stmts.push(self.parse_stmt()?);
        }
        Ok(Program { stmts })
    }

    // ── statements ───────────────────────────────────────────────────────────

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        let span = self.span();
        match self.peek().clone() {
            TokenKind::Let            => self.parse_let(),
            TokenKind::Fn             => self.parse_fn(false),
            TokenKind::Pub            => self.parse_pub_item(),
            TokenKind::Struct         => self.parse_struct(false),
            TokenKind::Enum           => self.parse_enum(false),
            TokenKind::Impl           => self.parse_impl(),
            TokenKind::Trait          => self.parse_trait(false),
            TokenKind::Return         => self.parse_return(),
            TokenKind::If             => { let s = self.parse_if_stmt()?; Ok(Stmt::If(s)) }
            TokenKind::While          => self.parse_while(),
            TokenKind::For            => self.parse_for(),
            TokenKind::Break          => { self.advance(); self.eat_semicolons(); Ok(Stmt::Break(span)) }
            TokenKind::Continue       => { self.advance(); self.eat_semicolons(); Ok(Stmt::Continue(span)) }
            TokenKind::LBrace         => self.parse_block_stmt(),
            TokenKind::Use            => self.parse_use(),
            TokenKind::Mod            => self.parse_mod(),
            _                         => {
                let e = self.parse_expr()?;
                self.eat_semicolons();
                Ok(Stmt::Expr(e))
            }
        }
    }

    fn parse_let(&mut self) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // consume `let`
        let mutable = self.eat(&TokenKind::Mut);
        let name = self.expect_ident()?;
        let ty = if self.eat(&TokenKind::Colon) { Some(self.parse_type()?) } else { None };
        let value = if self.eat(&TokenKind::Eq) { Some(self.parse_expr()?) } else { None };
        self.eat_semicolons();
        Ok(Stmt::Let(LetStmt { name, mutable, ty, value, span }))
    }

    fn parse_pub_item(&mut self) -> PResult<Stmt> {
        self.advance(); // consume `pub`
        match self.peek().clone() {
            TokenKind::Fn     => self.parse_fn(true),
            TokenKind::Struct => self.parse_struct(true),
            TokenKind::Enum   => self.parse_enum(true),
            TokenKind::Trait  => self.parse_trait(true),
            _ => Err(ParseError::new("expected fn/struct/enum/trait after pub", self.span())),
        }
    }

    fn parse_fn(&mut self, is_pub: bool) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // consume `fn`
        let name = self.expect_ident()?;
        let params = self.parse_params()?;
        let ret_ty = if self.eat(&TokenKind::Arrow) { Some(self.parse_type()?) } else { None };
        let body = self.parse_block()?;
        Ok(Stmt::Fn(FnDecl { name, params, ret_ty, body, is_pub, span }))
    }

    fn parse_params(&mut self) -> PResult<Vec<Param>> {
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
            let mutable = self.eat(&TokenKind::Mut);
            let name = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type()?;
            params.push(Param { name, ty, mutable });
            if !self.eat(&TokenKind::Comma) { break; }
        }
        self.expect(&TokenKind::RParen)?;
        Ok(params)
    }

    fn parse_struct(&mut self, is_pub: bool) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // `struct`
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            let field_pub = self.eat(&TokenKind::Pub);
            let fname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let fty = self.parse_type()?;
            fields.push(StructField { name: fname, ty: fty, is_pub: field_pub });
            if !self.eat(&TokenKind::Comma) { break; }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::Struct(StructDecl { name, fields, is_pub, span }))
    }

    fn parse_enum(&mut self, is_pub: bool) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // `enum`
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            let vname = self.expect_ident()?;
            let fields = if self.eat(&TokenKind::LParen) {
                let mut tys = Vec::new();
                while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
                    tys.push(self.parse_type()?);
                    if !self.eat(&TokenKind::Comma) { break; }
                }
                self.expect(&TokenKind::RParen)?;
                tys
            } else { Vec::new() };
            variants.push(EnumVariant { name: vname, fields });
            if !self.eat(&TokenKind::Comma) { break; }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::Enum(EnumDecl { name, variants, is_pub, span }))
    }

    fn parse_impl(&mut self) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // `impl`
        let ty = self.expect_ident()?;
        // optional: `for TraitName`  (not parsing full generics here)
        let trait_name = None;
        self.expect(&TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            self.eat_semicolons();
            if matches!(self.peek(), TokenKind::RBrace) { break; }
            let is_pub = self.eat(&TokenKind::Pub);
            if matches!(self.peek(), TokenKind::Fn) {
                self.advance();
                let mspan = self.span();
                let name = self.expect_ident()?;
                let params = self.parse_params()?;
                let ret_ty = if self.eat(&TokenKind::Arrow) { Some(self.parse_type()?) } else { None };
                let body = self.parse_block()?;
                methods.push(FnDecl { name, params, ret_ty, body, is_pub, span: mspan });
            } else {
                break;
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::Impl(ImplBlock { ty, trait_name, methods, span }))
    }

    fn parse_trait(&mut self, is_pub: bool) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // `trait`
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            self.eat_semicolons();
            if matches!(self.peek(), TokenKind::RBrace) { break; }
            if matches!(self.peek(), TokenKind::Fn) {
                self.advance();
                let mname = self.expect_ident()?;
                let params = self.parse_params()?;
                let ret_ty = if self.eat(&TokenKind::Arrow) { Some(self.parse_type()?) } else { None };
                self.eat_semicolons();
                methods.push(FnSignature { name: mname, params, ret_ty });
            } else { break; }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::Trait(TraitDecl { name, methods, is_pub, span }))
    }

    fn parse_return(&mut self) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // `return`
        let val = if !matches!(self.peek(), TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof) {
            Some(self.parse_expr()?)
        } else { None };
        self.eat_semicolons();
        Ok(Stmt::Return(val, span))
    }

    fn parse_if_stmt(&mut self) -> PResult<IfStmt> {
        let span = self.span();
        self.advance(); // `if`
        let cond = self.parse_expr()?;
        let then_block = self.parse_block()?;
        let else_block = if self.eat(&TokenKind::Else) {
            if matches!(self.peek(), TokenKind::If) {
                let nested = self.parse_if_stmt()?;
                Some(vec![Stmt::If(nested)])
            } else {
                Some(self.parse_block()?)
            }
        } else { None };
        Ok(IfStmt { cond, then_block, else_block, span })
    }

    fn parse_while(&mut self) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // `while`
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::While(WhileStmt { cond, body, span }))
    }

    fn parse_for(&mut self) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // `for`
        let var = self.expect_ident()?;
        self.expect(&TokenKind::In)?;
        let iter = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::For(ForStmt { var, iter, body, span }))
    }

    fn parse_block_stmt(&mut self) -> PResult<Stmt> {
        let span = self.span();
        let stmts = self.parse_block()?;
        Ok(Stmt::Block(stmts, span))
    }

    fn parse_use(&mut self) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // `use`
        let mut segments = Vec::new();
        segments.push(self.expect_ident()?);
        while self.eat(&TokenKind::ColonColon) {
            segments.push(self.expect_ident()?);
        }
        let alias = if matches!(self.peek(), TokenKind::Ident(_)) {
            // `use foo as bar` — treat next token as alias if it's an ident after `as`
            None // simplified: no `as` keyword yet
        } else { None };
        self.eat_semicolons();
        Ok(Stmt::Use(UsePath { segments, alias }, span))
    }

    fn parse_mod(&mut self) -> PResult<Stmt> {
        let span = self.span();
        self.advance(); // `mod`
        let name = self.expect_ident()?;
        let body = self.parse_block()?;
        Ok(Stmt::Mod(name, body, span))
    }

    fn parse_block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            self.eat_semicolons();
            if matches!(self.peek(), TokenKind::RBrace) { break; }
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(stmts)
    }

    /// Block in expression position: a trailing expression statement becomes the
    /// block's value, so `if c { a } else { b }` works as an expression.
    fn parse_block_expr(&mut self) -> PResult<Expr> {
        let span = self.span();
        self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        let mut tail: Option<Box<Expr>> = None;
        while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
            self.eat_semicolons();
            if matches!(self.peek(), TokenKind::RBrace) { break; }
            // `if` is a statement keyword, but it also produces a value. Parse it
            // as an expression so a trailing `if` can be the block's result.
            if matches!(self.peek(), TokenKind::If) {
                let e = self.parse_expr()?;
                if matches!(self.peek(), TokenKind::RBrace) {
                    tail = Some(Box::new(e));
                    break;
                }
                stmts.push(Stmt::Expr(e));
                continue;
            }
            let s = self.parse_stmt()?;
            if matches!(self.peek(), TokenKind::RBrace) {
                if let Stmt::Expr(e) = s {
                    tail = Some(Box::new(e));
                    break;
                }
            }
            stmts.push(s);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Expr::Block(stmts, tail, span))
    }

    // ── types ─────────────────────────────────────────────────────────────────

    fn parse_type(&mut self) -> PResult<TypeExpr> {
        match self.peek().clone() {
            TokenKind::Ampersand => {
                self.advance();
                let mutable = self.eat(&TokenKind::Mut);
                let inner = self.parse_type()?;
                Ok(TypeExpr::Ref(Box::new(inner), mutable))
            }
            TokenKind::LBracket => {
                self.advance();
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(TypeExpr::Slice(Box::new(inner)))
            }
            TokenKind::LParen => {
                self.advance();
                if self.eat(&TokenKind::RParen) {
                    return Ok(TypeExpr::Unit);
                }
                let mut tys = vec![self.parse_type()?];
                while self.eat(&TokenKind::Comma) {
                    tys.push(self.parse_type()?);
                }
                self.expect(&TokenKind::RParen)?;
                Ok(TypeExpr::Tuple(tys))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                // generic args
                if self.eat(&TokenKind::Lt) {
                    let mut args = vec![self.parse_type()?];
                    while self.eat(&TokenKind::Comma) {
                        args.push(self.parse_type()?);
                    }
                    self.expect(&TokenKind::Gt)?;
                    Ok(TypeExpr::Generic(name, args))
                } else {
                    Ok(TypeExpr::Simple(name))
                }
            }
            _ => Err(ParseError::new(format!("expected type, got {:?}", self.peek()), self.span())),
        }
    }

    // ── expressions ───────────────────────────────────────────────────────────

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> PResult<Expr> {
        let lhs = self.parse_range()?;
        let span = self.span();
        let op = match self.peek() {
            TokenKind::Eq       => Some(AssignOp::Assign),
            TokenKind::PlusEq   => Some(AssignOp::AddAssign),
            TokenKind::MinusEq  => Some(AssignOp::SubAssign),
            TokenKind::StarEq   => Some(AssignOp::MulAssign),
            TokenKind::SlashEq  => Some(AssignOp::DivAssign),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let rhs = self.parse_assignment()?;
            return Ok(Expr::Assign(Box::new(lhs), op, Box::new(rhs), span));
        }
        Ok(lhs)
    }

    fn parse_range(&mut self) -> PResult<Expr> {
        let lhs = self.parse_or()?;
        let span = self.span();
        if self.eat(&TokenKind::DotDot) {
            let inclusive = self.eat(&TokenKind::Eq);
            let rhs = self.parse_or()?;
            return Ok(Expr::Range(Box::new(lhs), Box::new(rhs), inclusive, span));
        }
        Ok(lhs)
    }

    fn parse_or(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_and()?;
        while matches!(self.peek(), TokenKind::Or) {
            let span = self.span(); self.advance();
            let rhs = self.parse_and()?;
            lhs = Expr::Binary(Box::new(lhs), BinOp::Or, Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_equality()?;
        while matches!(self.peek(), TokenKind::And) {
            let span = self.span(); self.advance();
            let rhs = self.parse_equality()?;
            lhs = Expr::Binary(Box::new(lhs), BinOp::And, Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_equality(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_comparison()?;
        loop {
            let span = self.span();
            let op = match self.peek() {
                TokenKind::EqEq   => BinOp::Eq,
                TokenKind::BangEq => BinOp::Ne,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_comparison()?;
            lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_comparison(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_bitwise()?;
        loop {
            let span = self.span();
            let op = match self.peek() {
                TokenKind::Lt   => BinOp::Lt,
                TokenKind::LtEq => BinOp::Le,
                TokenKind::Gt   => BinOp::Gt,
                TokenKind::GtEq => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_bitwise()?;
            lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_bitwise(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_shift()?;
        loop {
            let span = self.span();
            let op = match self.peek() {
                TokenKind::Ampersand => BinOp::BitAnd,
                TokenKind::Pipe      => BinOp::BitOr,
                TokenKind::Caret     => BinOp::BitXor,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_shift()?;
            lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_shift(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_additive()?;
        loop {
            let span = self.span();
            let op = match self.peek() {
                TokenKind::Shl => BinOp::Shl,
                TokenKind::Shr => BinOp::Shr,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_additive()?;
            lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_additive(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let span = self.span();
            let op = match self.peek() {
                TokenKind::Plus  => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_unary()?;
        loop {
            let span = self.span();
            let op = match self.peek() {
                TokenKind::Star    => BinOp::Mul,
                TokenKind::Slash   => BinOp::Div,
                TokenKind::Percent => BinOp::Rem,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_unary()?;
            lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs), span);
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        let span = self.span();
        match self.peek().clone() {
            TokenKind::Bang  => { self.advance(); let e = self.parse_unary()?; Ok(Expr::Unary(UnOp::Not, Box::new(e), span)) }
            TokenKind::Minus => { self.advance(); let e = self.parse_unary()?; Ok(Expr::Unary(UnOp::Neg, Box::new(e), span)) }
            TokenKind::Ampersand => { self.advance(); let e = self.parse_unary()?; Ok(Expr::Unary(UnOp::Ref, Box::new(e), span)) }
            TokenKind::Star  => { self.advance(); let e = self.parse_unary()?; Ok(Expr::Unary(UnOp::Deref, Box::new(e), span)) }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            let span = self.span();
            match self.peek().clone() {
                // function call
                TokenKind::LParen => {
                    self.advance();
                    let args = self.parse_call_args()?;
                    self.expect(&TokenKind::RParen)?;
                    expr = Expr::Call(Box::new(expr), args, span);
                }
                // index
                TokenKind::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(&TokenKind::RBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(idx), span);
                }
                // field / method
                TokenKind::Dot => {
                    self.advance();
                    let name = self.expect_ident()?;
                    if matches!(self.peek(), TokenKind::LParen) {
                        self.advance();
                        let args = self.parse_call_args()?;
                        self.expect(&TokenKind::RParen)?;
                        expr = Expr::MethodCall(Box::new(expr), name, args, span);
                    } else {
                        expr = Expr::Field(Box::new(expr), name, span);
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_call_args(&mut self) -> PResult<Vec<Expr>> {
        let mut args = Vec::new();
        while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
            args.push(self.parse_expr()?);
            if !self.eat(&TokenKind::Comma) { break; }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let span = self.span();
        match self.peek().clone() {
            TokenKind::Int(n)   => { self.advance(); Ok(Expr::Int(n, span)) }
            TokenKind::Float(f) => { self.advance(); Ok(Expr::Float(f, span)) }
            TokenKind::Str(s)   => { self.advance(); Ok(Expr::Str(s, span)) }
            TokenKind::True     => { self.advance(); Ok(Expr::Bool(true, span)) }
            TokenKind::False    => { self.advance(); Ok(Expr::Bool(false, span)) }
            TokenKind::Nil      => { self.advance(); Ok(Expr::Nil(span)) }

            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                // struct literal: Foo { field: val, ... }
                if self.at_struct_literal() {
                    self.advance();
                    let mut fields = Vec::new();
                    while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
                        let fname = self.expect_ident()?;
                        self.expect(&TokenKind::Colon)?;
                        let fval = self.parse_expr()?;
                        fields.push((fname, fval));
                        if !self.eat(&TokenKind::Comma) { break; }
                    }
                    self.expect(&TokenKind::RBrace)?;
                    return Ok(Expr::StructInit(name, fields, span));
                }
                // path expression like Foo::Bar
                if matches!(self.peek(), TokenKind::ColonColon) {
                    self.advance();
                    let variant = self.expect_ident()?;
                    let args = if matches!(self.peek(), TokenKind::LParen) {
                        self.advance();
                        let a = self.parse_call_args()?;
                        self.expect(&TokenKind::RParen)?;
                        a
                    } else { Vec::new() };
                    return Ok(Expr::EnumVariant(name, variant, args, span));
                }
                Ok(Expr::Ident(name, span))
            }

            TokenKind::LParen => {
                self.advance();
                if self.eat(&TokenKind::RParen) {
                    return Ok(Expr::Tuple(vec![], span));
                }
                let first = self.parse_expr()?;
                if self.eat(&TokenKind::Comma) {
                    let mut items = vec![first];
                    while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
                        items.push(self.parse_expr()?);
                        if !self.eat(&TokenKind::Comma) { break; }
                    }
                    self.expect(&TokenKind::RParen)?;
                    return Ok(Expr::Tuple(items, span));
                }
                self.expect(&TokenKind::RParen)?;
                Ok(first)
            }

            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !matches!(self.peek(), TokenKind::RBracket | TokenKind::Eof) {
                    items.push(self.parse_expr()?);
                    if !self.eat(&TokenKind::Comma) { break; }
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr::Array(items, span))
            }

            TokenKind::If => {
                self.advance();
                let cond = self.parse_expr()?;
                let then_val = self.parse_block_expr()?;
                self.expect(&TokenKind::Else)?;
                let else_val = self.parse_block_expr()?;
                Ok(Expr::If(Box::new(IfExpr { cond, then_val, else_val, span })))
            }

            TokenKind::Match => {
                self.advance();
                let subject = self.parse_expr()?;
                self.expect(&TokenKind::LBrace)?;
                let mut arms = Vec::new();
                while !matches!(self.peek(), TokenKind::RBrace | TokenKind::Eof) {
                    let pattern = self.parse_pattern()?;
                    let guard = if matches!(self.peek(), TokenKind::If) {
                        self.advance();
                        Some(self.parse_expr()?)
                    } else { None };
                    self.expect(&TokenKind::FatArrow)?;
                    let body = self.parse_expr()?;
                    arms.push(MatchArm { pattern, guard, body });
                    self.eat(&TokenKind::Comma);
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(Expr::Match(Box::new(MatchExpr { subject, arms, span })))
            }

            TokenKind::LBrace => self.parse_block_expr(),

            TokenKind::Print | TokenKind::Println => {
                // treat as a call to builtin function
                let name = if matches!(self.peek(), TokenKind::Println) { "println" } else { "print" };
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let args = self.parse_call_args()?;
                self.expect(&TokenKind::RParen)?;
                Ok(Expr::Call(Box::new(Expr::Ident(name.to_string(), span.clone())), args, span))
            }

            _ => Err(ParseError::new(format!("unexpected token {:?}", self.peek()), span)),
        }
    }

    fn parse_pattern(&mut self) -> PResult<Pattern> {
        match self.peek().clone() {
            TokenKind::Int(n)  => { self.advance(); Ok(Pattern::Int(n)) }
            TokenKind::Float(f)=> { self.advance(); Ok(Pattern::Float(f)) }
            TokenKind::Str(s)  => { self.advance(); Ok(Pattern::Str(s)) }
            TokenKind::True    => { self.advance(); Ok(Pattern::Bool(true)) }
            TokenKind::False   => { self.advance(); Ok(Pattern::Bool(false)) }
            TokenKind::Nil     => { self.advance(); Ok(Pattern::Nil) }
            TokenKind::Ident(n) if n == "_" => { self.advance(); Ok(Pattern::Wildcard) }
            TokenKind::Ident(n) => {
                let name = n.clone(); self.advance();
                if matches!(self.peek(), TokenKind::ColonColon) {
                    self.advance();
                    let variant = self.expect_ident()?;
                    let fields = if matches!(self.peek(), TokenKind::LParen) {
                        self.advance();
                        let mut ps = Vec::new();
                        while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
                            ps.push(self.parse_pattern()?);
                            if !self.eat(&TokenKind::Comma) { break; }
                        }
                        self.expect(&TokenKind::RParen)?;
                        ps
                    } else { Vec::new() };
                    Ok(Pattern::EnumVariant(name, variant, fields))
                } else {
                    Ok(Pattern::Ident(name))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let mut ps = Vec::new();
                while !matches!(self.peek(), TokenKind::RParen | TokenKind::Eof) {
                    ps.push(self.parse_pattern()?);
                    if !self.eat(&TokenKind::Comma) { break; }
                }
                self.expect(&TokenKind::RParen)?;
                Ok(Pattern::Tuple(ps))
            }
            _ => Ok(Pattern::Wildcard),
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn expect_ident(&mut self) -> PResult<String> {
        match self.peek().clone() {
            TokenKind::Ident(name) => { self.advance(); Ok(name) }
            TokenKind::Self_       => { self.advance(); Ok("self".to_string()) }
            _ => Err(ParseError::new(format!("expected identifier, got {:?}", self.peek()), self.span())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_let() {
        let prog = Parser::parse("let x = 42;").unwrap();
        assert_eq!(prog.stmts.len(), 1);
        assert!(matches!(prog.stmts[0], Stmt::Let(_)));
    }

    #[test]
    fn test_parse_fn() {
        let src = "fn add(a: i64, b: i64) -> i64 { return a + b; }";
        let prog = Parser::parse(src).unwrap();
        assert!(matches!(prog.stmts[0], Stmt::Fn(_)));
    }

    #[test]
    fn test_parse_struct() {
        let src = "struct Point { x: f64, y: f64 }";
        let prog = Parser::parse(src).unwrap();
        assert!(matches!(prog.stmts[0], Stmt::Struct(_)));
    }

    #[test]
    fn test_parse_if_else() {
        let src = "if x > 0 { println(\"pos\"); } else { println(\"neg\"); }";
        let prog = Parser::parse(src).unwrap();
        assert!(matches!(prog.stmts[0], Stmt::If(_)));
    }

    #[test]
    fn test_condition_ending_in_ident_is_not_struct_literal() {
        let src = "while i < n { i += 1; }";
        let prog = Parser::parse(src).unwrap();
        match &prog.stmts[0] {
            Stmt::While(w) => assert!(!matches!(w.cond, Expr::StructInit(..))),
            other => panic!("expected while, got {:?}", other),
        }
    }

    #[test]
    fn test_struct_literal_still_parses() {
        let src = "let p = Point { x: 1, y: 2 };";
        let prog = Parser::parse(src).unwrap();
        match &prog.stmts[0] {
            Stmt::Let(l) => assert!(matches!(l.value, Some(Expr::StructInit(..)))),
            other => panic!("expected let, got {:?}", other),
        }
    }

    #[test]
    fn test_trailing_if_is_block_value() {
        // A trailing `if` inside a block must yield its value, even when nested.
        let prog = Parser::parse("let x = if a { 1 } else { if b { 2 } else { 3 } };").unwrap();
        match &prog.stmts[0] {
            Stmt::Let(l) => assert!(matches!(l.value, Some(Expr::If(_)))),
            other => panic!("expected let, got {:?}", other),
        }
    }
}
