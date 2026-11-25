//! Recursive descent parser. Every block closes with `amen`, and so does
//! the program.

use crate::ast::*;
use crate::diag::Diag;
use crate::lexer::{lex, StrPart, Tok};
use std::rc::Rc;

type P<T> = Result<T, Diag>;

pub struct Parser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
    pub warnings: Vec<Diag>,
    denials: Vec<usize>,
}

fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Number(n) => format!("the number {n}"),
        Tok::Decimal(d) => format!("the decimal {d}"),
        Tok::Str(_) => "a text".to_string(),
        Tok::Ident(s) => format!("`{s}`"),
        Tok::Kw(k) => format!("`{k}`"),
        Tok::Newline => "the end of the line".to_string(),
        Tok::Eof => "the end of the file".to_string(),
        Tok::LParen => "`(`".into(),
        Tok::RParen => "`)`".into(),
        Tok::LBracket => "`[`".into(),
        Tok::RBracket => "`]`".into(),
        Tok::LBrace => "`{`".into(),
        Tok::RBrace => "`}`".into(),
        Tok::Comma => "`,`".into(),
        Tok::Colon => "`:`".into(),
        Tok::Dot => "`.`".into(),
        Tok::Arrow => "`->`".into(),
        Tok::Assign => "`=`".into(),
        Tok::Plus => "`+`".into(),
        Tok::Minus => "`-`".into(),
        Tok::Star => "`*`".into(),
        Tok::Slash => "`/`".into(),
        Tok::Percent => "`%`".into(),
        Tok::Lt => "`<`".into(),
        Tok::Gt => "`>`".into(),
        Tok::Le => "`<=`".into(),
        Tok::Ge => "`>=`".into(),
    }
}

fn capitalized(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_uppercase())
}

impl Parser {
    pub fn new(toks: Vec<(Tok, usize)>) -> Self {
        Parser { toks, pos: 0, warnings: vec![], denials: vec![] }
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.pos].0
    }

    fn peek_at(&self, offset: usize) -> &Tok {
        &self.toks[(self.pos + offset).min(self.toks.len() - 1)].0
    }

    fn line(&self) -> usize {
        self.toks[self.pos].1
    }

    fn advance(&mut self) -> Tok {
        let tok = self.toks[self.pos].0.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn is_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Tok::Kw(k) if *k == kw)
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.is_kw(kw) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn eat(&mut self, tok: &Tok) -> bool {
        if self.peek() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    fn unexpected(&self, wanted: &str) -> Diag {
        Diag::error(self.line(), format!("expected {wanted}, found {}", describe(self.peek())))
    }

    fn expect_kw(&mut self, kw: &str) -> P<()> {
        if self.eat_kw(kw) {
            Ok(())
        } else {
            Err(self.unexpected(&format!("`{kw}`")))
        }
    }

    fn expect(&mut self, tok: Tok) -> P<()> {
        if self.eat(&tok) {
            Ok(())
        } else {
            Err(self.unexpected(&describe(&tok)))
        }
    }

    fn ident(&mut self, what: &str) -> P<String> {
        match self.peek().clone() {
            Tok::Ident(name) => {
                self.advance();
                Ok(name)
            }
            Tok::Kw(k) => Err(Diag::error(self.line(), format!("expected {what}, found `{k}`"))
                .note(format!("`{k}` is a word of the language and cannot be used as a name"))),
            _ => Err(self.unexpected(what)),
        }
    }

    fn skip_newlines(&mut self) {
        while self.peek() == &Tok::Newline {
            self.advance();
        }
    }

    fn end_stmt(&mut self) -> P<()> {
        match self.peek() {
            Tok::Newline => {
                self.advance();
                Ok(())
            }
            Tok::Eof => Ok(()),
            _ => Err(self.unexpected("the end of the line")),
        }
    }

    fn expect_amen(&mut self, what: &str, opened: usize) -> P<()> {
        if self.eat_kw("amen") {
            Ok(())
        } else {
            Err(Diag::error(self.line(), format!("expected `amen`, found {}", describe(self.peek())))
                .note(format!("the `{what}` opened on line {opened} must close with `amen`")))
        }
    }

    // ---- program ----

    pub fn program(&mut self) -> P<Vec<Stmt>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.is_kw("amen") {
                self.advance();
                self.skip_newlines();
                if self.peek() != &Tok::Eof {
                    return Err(Diag::error(self.line(), "nothing may follow the final `amen`")
                        .note("an `amen` here closes the whole program; perhaps a block above has one `amen` too many"));
                }
                return Ok(stmts);
            }
            if self.peek() == &Tok::Eof {
                return Err(Diag::error(self.line(), "a program must end with `amen`")
                    .note("hint: add `amen` as the last line"));
            }
            stmts.push(self.stmt(true)?);
        }
    }

    /// Statements up to, but not including, the word that closes the block.
    fn block(&mut self, in_arm: bool) -> P<Vec<Stmt>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.is_kw("amen") || self.is_kw("otherwise") || self.is_kw("forgive") || self.peek() == &Tok::Eof {
                return Ok(stmts);
            }
            if in_arm && self.at_arm_start() {
                return Ok(stmts);
            }
            stmts.push(self.stmt(false)?);
        }
    }

    fn at_arm_start(&self) -> bool {
        let named = match self.peek() {
            Tok::Ident(name) => capitalized(name),
            Tok::Kw("Jesus") | Tok::Kw("False") => true,
            _ => false,
        };
        if !named {
            return false;
        }
        let mut i = 1;
        if self.peek_at(i) == &Tok::LParen {
            i += 1;
            loop {
                match self.peek_at(i) {
                    Tok::Ident(_) | Tok::Comma => i += 1,
                    Tok::RParen => {
                        i += 1;
                        break;
                    }
                    _ => return false,
                }
            }
        }
        self.peek_at(i) == &Tok::Colon
    }

    fn top_only(&self, top: bool, what: &str) -> P<()> {
        if top {
            Ok(())
        } else {
            Err(Diag::error(self.line(), format!("a {what} belongs at the top of the program, not inside a block")))
        }
    }

    fn stmt(&mut self, top: bool) -> P<Stmt> {
        let line = self.line();
        let kind = match self.peek().clone() {
            Tok::Kw("let") => {
                self.advance();
                let mortal = self.eat_kw("mortal");
                let name = self.ident("a name")?;
                let ty = if self.eat(&Tok::Colon) { Some(self.type_expr()?) } else { None };
                if self.peek() != &Tok::Assign {
                    return Err(self.unexpected("`=`").note("Gesù has no null: every `let` must hold a value"));
                }
                self.advance();
                let init = self.expr()?;
                self.end_stmt()?;
                StmtKind::Let { name, mortal, ty, init }
            }
            Tok::Kw("law") => {
                self.top_only(top, "law")?;
                self.advance();
                let name = self.ident("a name")?;
                self.expect(Tok::Assign)?;
                let init = self.expr()?;
                self.end_stmt()?;
                StmtKind::Law { name, init }
            }
            Tok::Kw("parable") => {
                self.top_only(top, "parable")?;
                StmtKind::Parable(Rc::new(self.parable()?))
            }
            Tok::Kw("kind") => {
                self.top_only(top, "kind")?;
                self.kind()?
            }
            Tok::Kw("pagan") => {
                self.top_only(top, "pagan")?;
                self.advance();
                let lang = self.ident("`python` or `lua`")?;
                let path = match self.advance() {
                    Tok::Str(parts) => match parts.as_slice() {
                        [StrPart::Lit(s)] => s.clone(),
                        _ => return Err(Diag::error(line, "a pagan path must be plain text")),
                    },
                    _ => return Err(Diag::error(line, "expected the pagan's name in quotes")
                        .note("example: pagan python \"statistics\" as stats")),
                };
                self.expect_kw("as")?;
                let name = self.ident("a name")?;
                self.end_stmt()?;
                StmtKind::Pagan { lang, path, name }
            }
            Tok::Kw("preach") => {
                self.advance();
                let e = self.expr()?;
                self.end_stmt()?;
                StmtKind::Preach(e)
            }
            Tok::Kw("give") => {
                self.advance();
                let e = if matches!(self.peek(), Tok::Newline | Tok::Eof) { None } else { Some(self.expr()?) };
                self.end_stmt()?;
                StmtKind::Give(e)
            }
            Tok::Kw("sin") => {
                self.advance();
                let e = self.expr()?;
                self.end_stmt()?;
                StmtKind::Sin(e)
            }
            Tok::Kw("if") => return self.if_stmt(),
            Tok::Kw("while") => {
                self.advance();
                let cond = self.expr()?;
                self.expect(Tok::Newline)?;
                let body = self.block(false)?;
                self.expect_amen("while", line)?;
                self.end_stmt()?;
                StmtKind::While { cond, body }
            }
            Tok::Kw("repeat") => {
                self.advance();
                let count = self.expr()?;
                self.expect_kw("times")?;
                self.expect(Tok::Newline)?;
                let body = self.block(false)?;
                self.expect_amen("repeat", line)?;
                self.end_stmt()?;
                StmtKind::Repeat { count, body }
            }
            Tok::Kw("for") => {
                self.advance();
                self.expect_kw("each")?;
                let first = self.ident("a name")?;
                let second = if self.eat(&Tok::Comma) { Some(self.ident("a name")?) } else { None };
                self.expect_kw("in")?;
                let iter = self.expr()?;
                self.expect(Tok::Newline)?;
                let body = self.block(false)?;
                self.expect_amen("for each", line)?;
                self.end_stmt()?;
                StmtKind::ForEach { first, second, iter, body }
            }
            Tok::Kw("break") => {
                self.advance();
                self.end_stmt()?;
                StmtKind::Break
            }
            Tok::Kw("continue") => {
                self.advance();
                self.end_stmt()?;
                StmtKind::Continue
            }
            Tok::Kw("repent") => {
                self.advance();
                self.end_stmt()?;
                StmtKind::Repent
            }
            Tok::Kw("tempt") => {
                self.advance();
                self.expect(Tok::Newline)?;
                let body = self.block(false)?;
                if !self.eat_kw("forgive") {
                    return Err(Diag::error(self.line(), format!("expected `forgive`, found {}", describe(self.peek())))
                        .note(format!("the `tempt` opened on line {line} needs a `forgive`")));
                }
                let name = self.ident("a name for the trespass")?;
                self.expect(Tok::Newline)?;
                let handler = self.block(false)?;
                self.expect_amen("tempt", line)?;
                self.end_stmt()?;
                StmtKind::Tempt { body, name, handler }
            }
            Tok::Kw("discern") => {
                self.advance();
                let subject = self.expr()?;
                self.expect(Tok::Newline)?;
                let mut arms = Vec::new();
                loop {
                    self.skip_newlines();
                    if self.is_kw("amen") {
                        break;
                    }
                    if !self.at_arm_start() {
                        return Err(self.unexpected("a case such as `Found(x):` or `amen`"));
                    }
                    let pattern = self.pattern()?;
                    self.expect(Tok::Colon)?;
                    let body = if self.eat(&Tok::Newline) { self.block(true)? } else { vec![self.stmt(false)?] };
                    arms.push(Arm { pattern, body });
                }
                self.expect_amen("discern", line)?;
                self.end_stmt()?;
                StmtKind::Discern { subject, arms }
            }
            Tok::Kw("amen") | Tok::Kw("otherwise") | Tok::Kw("forgive") => {
                return Err(self.unexpected("a statement"));
            }
            _ => {
                let e = self.expr()?;
                if self.eat(&Tok::Assign) {
                    if !matches!(e.kind, ExprKind::Var(_) | ExprKind::Index(..) | ExprKind::Field(..)) {
                        return Err(Diag::error(line, "only a name, an element or a field can be assigned to"));
                    }
                    let value = self.expr()?;
                    self.end_stmt()?;
                    StmtKind::Assign { target: e, value }
                } else {
                    self.end_stmt()?;
                    StmtKind::Expr(e)
                }
            }
        };
        Ok(Stmt { kind, line })
    }

    fn if_stmt(&mut self) -> P<Stmt> {
        let line = self.line();
        self.expect_kw("if")?;
        let cond = self.expr()?;
        self.expect(Tok::Newline)?;
        let then = self.block(false)?;
        let otherwise = if self.eat_kw("otherwise") {
            if self.is_kw("if") {
                // `otherwise if` shares the single closing `amen`.
                Some(vec![self.if_stmt()?])
            } else {
                self.expect(Tok::Newline)?;
                let block = self.block(false)?;
                self.expect_amen("if", line)?;
                self.end_stmt()?;
                Some(block)
            }
        } else {
            self.expect_amen("if", line)?;
            self.end_stmt()?;
            None
        };
        Ok(Stmt { kind: StmtKind::If { cond, then, otherwise }, line })
    }

    fn pattern(&mut self) -> P<Pattern> {
        let line = self.line();
        let name = match self.advance() {
            Tok::Ident(n) => n,
            Tok::Kw(k) => k.to_string(),
            _ => return Err(Diag::error(line, "expected a case")),
        };
        let mut binds = Vec::new();
        if self.eat(&Tok::LParen) {
            loop {
                binds.push(self.ident("a name")?);
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
            self.expect(Tok::RParen)?;
        }
        Ok(Pattern { name, binds, line })
    }

    fn parable(&mut self) -> P<Parable> {
        let line = self.line();
        self.expect_kw("parable")?;
        let name = self.ident("the parable's name")?;
        self.expect(Tok::LParen)?;
        let mut params = Vec::new();
        if self.peek() != &Tok::RParen {
            loop {
                let pline = self.line();
                let pname = self.ident("a parameter name")?;
                if self.peek() != &Tok::Colon {
                    return Err(self.unexpected("`:` and a type").note(format!("every parameter declares its type, as in `{pname}: Number`")));
                }
                self.advance();
                let mode = if self.eat_kw("lend") {
                    Mode::Lend
                } else if self.eat_kw("entrust") {
                    Mode::Entrust
                } else {
                    Mode::Own
                };
                let ty = self.type_expr()?;
                params.push(Param { name: pname, ty, mode, line: pline });
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
        }
        self.expect(Tok::RParen)?;
        let ret = if self.eat(&Tok::Arrow) { Some(self.type_expr()?) } else { None };
        let may_sin = if self.eat_kw("may") {
            self.expect_kw("sin")?;
            true
        } else {
            false
        };
        self.expect(Tok::Newline)?;
        let body = self.block(false)?;
        self.expect_amen("parable", line)?;
        self.end_stmt()?;
        Ok(Parable { name, params, ret, may_sin, body, line })
    }

    fn kind(&mut self) -> P<StmtKind> {
        let line = self.line();
        self.expect_kw("kind")?;
        let name = self.ident("the kind's name")?;
        if !capitalized(&name) {
            return Err(Diag::error(line, format!("a kind's name begins with a capital letter: `{name}`")));
        }
        self.expect(Tok::Newline)?;
        self.skip_newlines();
        let is_record = matches!(self.peek(), Tok::Ident(_)) && self.peek_at(1) == &Tok::Colon;
        let body = if is_record {
            let mut fields = Vec::new();
            loop {
                self.skip_newlines();
                if self.is_kw("amen") {
                    break;
                }
                let field = self.ident("a field name")?;
                self.expect(Tok::Colon)?;
                let ty = self.type_expr()?;
                self.end_stmt()?;
                fields.push((field, ty));
            }
            KindBody::Record(fields)
        } else {
            let mut variants = Vec::new();
            loop {
                self.skip_newlines();
                if self.is_kw("amen") {
                    break;
                }
                let vline = self.line();
                let variant = self.ident("a case name")?;
                if !capitalized(&variant) {
                    return Err(Diag::error(vline, format!("a case begins with a capital letter: `{variant}`")));
                }
                let mut fields = Vec::new();
                if self.eat(&Tok::LParen) {
                    loop {
                        fields.push(self.type_expr()?);
                        if !self.eat(&Tok::Comma) {
                            break;
                        }
                    }
                    self.expect(Tok::RParen)?;
                }
                self.end_stmt()?;
                variants.push((variant, fields));
            }
            if variants.is_empty() {
                return Err(Diag::error(line, format!("the kind `{name}` is empty")));
            }
            KindBody::Choice(variants)
        };
        self.expect_amen("kind", line)?;
        self.end_stmt()?;
        Ok(StmtKind::Kind { name, body })
    }

    fn type_expr(&mut self) -> P<TypeExpr> {
        let line = self.line();
        match self.advance() {
            Tok::Ident(name) if name == "Maybe" => Ok(TypeExpr::Maybe(Box::new(self.type_expr()?))),
            Tok::Ident(name) => Ok(TypeExpr::Named(name, line)),
            Tok::LBracket => {
                let inner = self.type_expr()?;
                self.expect(Tok::RBracket)?;
                Ok(TypeExpr::List(Box::new(inner)))
            }
            Tok::LBrace => {
                let key = self.type_expr()?;
                self.expect(Tok::Colon)?;
                let value = self.type_expr()?;
                self.expect(Tok::RBrace)?;
                Ok(TypeExpr::Map(Box::new(key), Box::new(value)))
            }
            other => Err(Diag::error(line, format!("expected a type, found {}", describe(&other)))
                .note("types: Number, Decimal, Text, Truth, [T], {K: V}, Maybe T, Pagan, or a kind")),
        }
    }

    // ---- expressions ----

    pub fn expr(&mut self) -> P<Expr> {
        self.or_expr()
    }

    fn binary(line: usize, op: BinOp, l: Expr, r: Expr) -> Expr {
        Expr { kind: ExprKind::Binary(op, Box::new(l), Box::new(r)), line }
    }

    fn or_expr(&mut self) -> P<Expr> {
        let mut left = self.and_expr()?;
        while self.is_kw("or") {
            let line = self.line();
            self.advance();
            let right = self.and_expr()?;
            left = Self::binary(line, BinOp::Or, left, right);
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> P<Expr> {
        let mut left = self.deny_expr()?;
        while self.is_kw("and") {
            let line = self.line();
            self.advance();
            let right = self.deny_expr()?;
            left = Self::binary(line, BinOp::And, left, right);
        }
        Ok(left)
    }

    fn deny_expr(&mut self) -> P<Expr> {
        if self.is_kw("deny") {
            let line = self.line();
            self.advance();
            let inner = self.deny_expr()?;
            if matches!(inner.kind, ExprKind::Jesus) {
                self.denials.push(line);
                if self.denials.len() == 3 {
                    let lines: Vec<String> = self.denials.iter().map(|l| l.to_string()).collect();
                    self.warnings.push(
                        Diag::warning(line, "the rooster crows")
                            .note(format!("Jesus has been denied three times (lines {})", lines.join(", "))),
                    );
                }
            }
            return Ok(Expr { kind: ExprKind::Deny(Box::new(inner)), line });
        }
        self.cmp_expr()
    }

    fn cmp_expr(&mut self) -> P<Expr> {
        let left = self.add_expr()?;
        let line = self.line();
        let op = match self.peek() {
            Tok::Lt => BinOp::Lt,
            Tok::Gt => BinOp::Gt,
            Tok::Le => BinOp::Le,
            Tok::Ge => BinOp::Ge,
            Tok::Kw("is") => BinOp::Is,
            _ => return Ok(left),
        };
        self.advance();
        let op = if op == BinOp::Is && self.eat_kw("not") { BinOp::IsNot } else { op };
        let right = self.add_expr()?;
        Ok(Self::binary(line, op, left, right))
    }

    fn add_expr(&mut self) -> P<Expr> {
        let mut left = self.mul_expr()?;
        loop {
            let line = self.line();
            let op = match self.peek() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.mul_expr()?;
            left = Self::binary(line, op, left, right);
        }
    }

    fn mul_expr(&mut self) -> P<Expr> {
        let mut left = self.unary()?;
        loop {
            let line = self.line();
            let op = match self.peek() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::Percent => BinOp::Rem,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.unary()?;
            left = Self::binary(line, op, left, right);
        }
    }

    fn unary(&mut self) -> P<Expr> {
        let line = self.line();
        if self.eat(&Tok::Minus) {
            let inner = self.unary()?;
            return Ok(Expr { kind: ExprKind::Neg(Box::new(inner)), line });
        }
        if self.eat_kw("baptize") {
            let inner = self.postfix()?;
            if !self.eat_kw("as") {
                return Err(self.unexpected("`as` and a type").note("example: baptize raw as Number"));
            }
            let ty = self.type_expr()?;
            return Ok(Expr { kind: ExprKind::Baptize(Box::new(inner), ty), line });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> P<Expr> {
        let mut e = self.primary()?;
        loop {
            let line = self.line();
            match self.peek() {
                Tok::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != &Tok::RParen {
                        loop {
                            let mode = if self.eat_kw("lend") {
                                Mode::Lend
                            } else if self.eat_kw("entrust") {
                                Mode::Entrust
                            } else {
                                Mode::Own
                            };
                            let name = match (self.peek().clone(), self.peek_at(1)) {
                                (Tok::Ident(n), Tok::Colon) => {
                                    self.advance();
                                    self.advance();
                                    Some(n)
                                }
                                _ => None,
                            };
                            let expr = self.expr()?;
                            args.push(Arg { name, mode, expr });
                            if !self.eat(&Tok::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(Tok::RParen)?;
                    e = Expr { kind: ExprKind::Call(Box::new(e), args), line };
                }
                Tok::LBracket => {
                    self.advance();
                    let index = self.expr()?;
                    self.expect(Tok::RBracket)?;
                    e = Expr { kind: ExprKind::Index(Box::new(e), Box::new(index)), line };
                }
                Tok::Dot => {
                    self.advance();
                    let name = self.ident("a field name")?;
                    e = Expr { kind: ExprKind::Field(Box::new(e), name), line };
                }
                _ => return Ok(e),
            }
        }
    }

    fn primary(&mut self) -> P<Expr> {
        let line = self.line();
        let kind = match self.advance() {
            Tok::Number(n) => ExprKind::Number(n),
            Tok::Decimal(d) => ExprKind::Decimal(d),
            Tok::Str(parts) => {
                let mut out = Vec::new();
                for part in parts {
                    match part {
                        StrPart::Lit(s) => out.push(TextPart::Lit(s)),
                        StrPart::Code(code) => {
                            let mut sub = Parser::new(lex(&code, line)?);
                            let e = sub.expr()?;
                            if !matches!(sub.peek(), Tok::Newline | Tok::Eof) {
                                return Err(Diag::error(line, format!("could not read `{{{code}}}` inside the text"))
                                    .note("hint: write `\\{` for a plain brace"));
                            }
                            out.push(TextPart::Code(e));
                        }
                    }
                }
                ExprKind::Text(out)
            }
            Tok::Kw("Jesus") => ExprKind::Jesus,
            Tok::Kw("False") => ExprKind::False,
            Tok::Ident(name) => ExprKind::Var(name),
            Tok::LParen => {
                let e = self.expr()?;
                self.expect(Tok::RParen)?;
                return Ok(e);
            }
            Tok::LBracket => {
                let mut items = Vec::new();
                if self.peek() != &Tok::RBracket {
                    loop {
                        items.push(self.expr()?);
                        if !self.eat(&Tok::Comma) || self.peek() == &Tok::RBracket {
                            break;
                        }
                    }
                }
                self.expect(Tok::RBracket)?;
                ExprKind::List(items)
            }
            Tok::LBrace => {
                let mut pairs = Vec::new();
                if self.peek() != &Tok::RBrace {
                    loop {
                        let key = self.expr()?;
                        self.expect(Tok::Colon)?;
                        let value = self.expr()?;
                        pairs.push((key, value));
                        if !self.eat(&Tok::Comma) || self.peek() == &Tok::RBrace {
                            break;
                        }
                    }
                }
                self.expect(Tok::RBrace)?;
                ExprKind::Map(pairs)
            }
            Tok::Kw("lend") | Tok::Kw("entrust") => {
                return Err(Diag::error(line, "a loan cannot be stored or given back")
                    .note("`lend` and `entrust` are only allowed on an argument of a call"));
            }
            other => return Err(Diag::error(line, format!("expected a value, found {}", describe(&other)))),
        };
        Ok(Expr { kind, line })
    }
}
