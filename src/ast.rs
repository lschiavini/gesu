//! Syntax tree produced by the parser.

use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum TypeExpr {
    Named(String, usize),
    List(Box<TypeExpr>),
    Map(Box<TypeExpr>, Box<TypeExpr>),
    Maybe(Box<TypeExpr>),
}

/// How a value crosses into a parable: given away, lent, or entrusted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Own,
    Lend,
    Entrust,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub mode: Mode,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub enum TextPart {
    Lit(String),
    Code(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Lt,
    Gt,
    Le,
    Ge,
    Is,
    IsNot,
    And,
    Or,
}

#[derive(Clone, Debug)]
pub struct Arg {
    pub name: Option<String>,
    pub mode: Mode,
    pub expr: Expr,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Number(i64),
    Decimal(f64),
    Text(Vec<TextPart>),
    Jesus,
    False,
    Var(String),
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Neg(Box<Expr>),
    Deny(Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Arg>),
    Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, String),
    Baptize(Box<Expr>, TypeExpr),
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub name: String,
    pub binds: Vec<String>,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct Arm {
    pub pattern: Pattern,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub enum KindBody {
    Record(Vec<(String, TypeExpr)>),
    Choice(Vec<(String, Vec<TypeExpr>)>),
}

#[derive(Clone, Debug)]
pub struct Parable {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub may_sin: bool,
    pub body: Vec<Stmt>,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Let { name: String, mortal: bool, ty: Option<TypeExpr>, init: Expr },
    Law { name: String, init: Expr },
    Assign { target: Expr, value: Expr },
    Preach(Expr),
    Expr(Expr),
    Give(Option<Expr>),
    Sin(Expr),
    If { cond: Expr, then: Vec<Stmt>, otherwise: Option<Vec<Stmt>> },
    While { cond: Expr, body: Vec<Stmt> },
    Repeat { count: Expr, body: Vec<Stmt> },
    ForEach { first: String, second: Option<String>, iter: Expr, body: Vec<Stmt> },
    Break,
    Continue,
    Tempt { body: Vec<Stmt>, name: String, handler: Vec<Stmt> },
    Discern { subject: Expr, arms: Vec<Arm> },
    Parable(Rc<Parable>),
    Kind { name: String, body: KindBody },
    Pagan { lang: String, path: String, name: String },
}
