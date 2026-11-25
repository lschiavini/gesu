//! The examination of conscience: types, ownership, declared sins and
//! buried talents are all checked here, before a single line runs.

use crate::ast::*;
use crate::diag::Diag;
use crate::types::*;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
enum BKind {
    Owned,
    Lent,
    Entrusted,
    Law,
}

struct Binding {
    name: String,
    ty: Type,
    mortal: bool,
    kind: BKind,
    /// Line of the move and who received the value.
    moved: Option<(usize, String)>,
    used: bool,
    line: usize,
    /// Above zero while a `for each` walks through it.
    frozen: u32,
    /// Report it as a buried talent when never used.
    watch: bool,
    /// For a law, its value as written.
    written: Option<String>,
}

#[derive(Clone)]
struct ParamSig {
    ty: Type,
    mode: Mode,
}

#[derive(Clone)]
struct Sig {
    params: Vec<ParamSig>,
    ret: Type,
    may_sin: bool,
}

struct Cur {
    name: String,
    ret: Type,
    may_sin: bool,
    sinned: bool,
}

#[derive(Clone, Copy)]
enum Loc {
    Local(usize, usize),
    Global(usize),
}

type Snap = Vec<Vec<Option<(usize, String)>>>;

pub const BUILTINS: &[&str] = &[
    "length", "push", "pop", "get", "lookup", "contains", "remove", "keys", "copy", "range", "sort", "reverse",
    "split", "join", "trim", "upper", "lower", "replace", "chars", "text", "number", "decimal", "floor", "round",
    "abs", "min", "max", "read_file", "write_file", "read_line", "args", "parse_json", "to_json",
];

pub struct Checker {
    pub diags: Vec<Diag>,
    pub kinds: Kinds,
    kind_is_record: HashMap<String, bool>,
    variants: HashMap<String, (String, Vec<Type>)>,
    parables: HashMap<String, Sig>,
    globals: Vec<Binding>,
    scopes: Vec<Vec<Binding>>,
    cur: Option<Cur>,
    tempt_depth: u32,
    loop_depth: u32,
    sin_events: u32,
}

fn root(e: &Expr) -> Option<&str> {
    match &e.kind {
        ExprKind::Var(name) => Some(name),
        ExprKind::Index(base, _) | ExprKind::Field(base, _) => root(base),
        _ => None,
    }
}

fn mentions(e: &Expr, name: &str) -> bool {
    match &e.kind {
        ExprKind::Var(n) => n == name,
        ExprKind::Text(parts) => parts.iter().any(|p| matches!(p, TextPart::Code(x) if mentions(x, name))),
        ExprKind::List(items) => items.iter().any(|x| mentions(x, name)),
        ExprKind::Map(pairs) => pairs.iter().any(|(k, v)| mentions(k, name) || mentions(v, name)),
        ExprKind::Neg(x) | ExprKind::Deny(x) | ExprKind::Baptize(x, _) | ExprKind::Field(x, _) => mentions(x, name),
        ExprKind::Binary(_, l, r) | ExprKind::Index(l, r) => mentions(l, name) || mentions(r, name),
        ExprKind::Call(callee, args) => mentions(callee, name) || args.iter().any(|a| mentions(&a.expr, name)),
        _ => false,
    }
}

/// Does this block always leave: by `give` or `sin`, and when `loops` is
/// set, also by `break` or `continue`?
fn ends(stmts: &[Stmt], loops: bool) -> bool {
    stmts.iter().any(|s| match &s.kind {
        StmtKind::Give(_) | StmtKind::Sin(_) => true,
        StmtKind::Break | StmtKind::Continue => loops,
        StmtKind::If { then, otherwise: Some(other), .. } => ends(then, loops) && ends(other, loops),
        StmtKind::Discern { arms, .. } => !arms.is_empty() && arms.iter().all(|a| ends(&a.body, loops)),
        StmtKind::Tempt { body, handler, .. } => ends(body, loops) && ends(handler, loops),
        _ => false,
    })
}

fn literal_text(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Number(n) => Some(n.to_string()),
        ExprKind::Decimal(d) => Some(d.to_string()),
        ExprKind::Jesus => Some("Jesus".into()),
        ExprKind::False => Some("False".into()),
        ExprKind::Text(parts) => match parts.as_slice() {
            [TextPart::Lit(s)] => Some(format!("\"{s}\"")),
            _ => None,
        },
        _ => None,
    }
}

impl Checker {
    pub fn new() -> Self {
        Checker {
            diags: vec![],
            kinds: HashMap::new(),
            kind_is_record: HashMap::new(),
            variants: HashMap::new(),
            parables: HashMap::new(),
            globals: vec![],
            scopes: vec![],
            cur: None,
            tempt_depth: 0,
            loop_depth: 0,
            sin_events: 0,
        }
    }

    fn push(&mut self, d: Diag) {
        if !self.diags.contains(&d) {
            self.diags.push(d);
        }
    }

    fn err(&mut self, line: usize, title: impl Into<String>) {
        self.push(Diag::error(line, title));
    }

    // ---- names ----

    fn find(&self, name: &str) -> Option<Loc> {
        if name == "_" {
            return None;
        }
        for (si, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(bi) = scope.iter().rposition(|b| b.name == name) {
                return Some(Loc::Local(si, bi));
            }
        }
        self.globals.iter().position(|b| b.name == name).map(Loc::Global)
    }

    fn b(&self, loc: Loc) -> &Binding {
        match loc {
            Loc::Local(s, i) => &self.scopes[s][i],
            Loc::Global(i) => &self.globals[i],
        }
    }

    fn bm(&mut self, loc: Loc) -> &mut Binding {
        match loc {
            Loc::Local(s, i) => &mut self.scopes[s][i],
            Loc::Global(i) => &mut self.globals[i],
        }
    }

    fn taken(&self, name: &str) -> Option<&'static str> {
        if self.find(name).is_some() {
            Some("a value with this name is already in sight")
        } else if self.parables.contains_key(name) {
            Some("a parable has this name")
        } else if self.kinds.contains_key(name) || self.kind_is_record.contains_key(name) {
            Some("a kind has this name")
        } else if self.variants.contains_key(name) || name == "Found" || name == "Lost" {
            Some("a case has this name")
        } else if BUILTINS.contains(&name) {
            Some("a built-in parable has this name")
        } else {
            None
        }
    }

    fn binding(name: &str, ty: Type, mortal: bool, kind: BKind, line: usize, watch: bool) -> Binding {
        Binding { name: name.to_string(), ty, mortal, kind, moved: None, used: false, line, frozen: 0, watch, written: None }
    }

    fn declare(&mut self, name: &str, ty: Type, mortal: bool, kind: BKind, line: usize, watch: bool) {
        if name != "_" {
            if let Some(why) = self.taken(name) {
                self.push(
                    Diag::error(line, format!("`{name}` already exists"))
                        .note(why)
                        .note("Gesù has no shadowing: choose another name"),
                );
            }
        }
        let b = Self::binding(name, ty, mortal, kind, line, watch);
        self.scopes.last_mut().expect("a scope").push(b);
    }

    fn pop_scope(&mut self) {
        let scope = self.scopes.pop().expect("a scope");
        for b in scope {
            if b.watch && !b.used && !b.name.starts_with('_') {
                self.push(
                    Diag::error(b.line, "you buried your talent")
                        .note(format!("`{}` is never used", b.name))
                        .note(format!("hint: use it, remove it, or name it `_{}`", b.name)),
                );
            }
        }
    }

    // ---- ownership state ----

    fn snap(&self) -> Snap {
        self.scopes.iter().map(|s| s.iter().map(|b| b.moved.clone()).collect()).collect()
    }

    fn restore(&mut self, snap: &Snap) {
        for (scope, saved) in self.scopes.iter_mut().zip(snap) {
            for (b, moved) in scope.iter_mut().zip(saved) {
                b.moved = moved.clone();
            }
        }
    }

    fn merge(&mut self, snap: &Snap) {
        for (scope, saved) in self.scopes.iter_mut().zip(snap) {
            for (b, moved) in scope.iter_mut().zip(saved) {
                if b.moved.is_none() {
                    b.moved = moved.clone();
                }
            }
        }
    }

    /// Joins the branches of an `if`, `discern` or `tempt`: a value given
    /// away in any branch that carries on is gone afterwards.
    fn join(&mut self, before: &Snap, branches: &[(Snap, bool)]) {
        let live: Vec<&Snap> = branches.iter().filter(|(_, left)| !left).map(|(s, _)| s).collect();
        match live.split_first() {
            None => self.restore(before),
            Some((first, rest)) => {
                self.restore(first);
                for s in rest {
                    self.merge(s);
                }
            }
        }
    }

    fn is_copy(&self, t: &Type) -> bool {
        self.copy_inner(t, &mut vec![])
    }

    fn copy_inner(&self, t: &Type, seen: &mut Vec<String>) -> bool {
        match t {
            Type::List(_) | Type::Map(..) | Type::Record(_) => false,
            Type::Maybe(inner) => self.copy_inner(inner, seen),
            Type::Choice(name) => {
                if seen.contains(name) {
                    return true;
                }
                seen.push(name.clone());
                match self.kinds.get(name) {
                    Some(KindDef::Choice(variants)) => {
                        let fields: Vec<Type> = variants.iter().flat_map(|(_, f)| f.clone()).collect();
                        fields.iter().all(|f| self.copy_inner(f, seen))
                    }
                    _ => true,
                }
            }
            _ => true,
        }
    }

    fn resolve(&mut self, te: &TypeExpr) -> Type {
        let known = &self.kind_is_record;
        match resolve(te, &|name| known.get(name).copied()) {
            Ok(t) => t,
            Err((line, name)) => {
                let mut d = Diag::error(line, format!("unknown type `{name}`"));
                d = match name.as_str() {
                    "Bool" | "Boolean" | "bool" => d.note("hint: the type of `Jesus` and `False` is `Truth`"),
                    "String" | "str" | "string" => d.note("hint: text has the type `Text`"),
                    "Int" | "int" | "Integer" | "i64" => d.note("hint: whole numbers have the type `Number`"),
                    "Float" | "float" | "f64" => d.note("hint: fractions have the type `Decimal`"),
                    _ => d.note("types: Number, Decimal, Text, Truth, [T], {K: V}, Maybe T, Pagan, or a kind"),
                };
                self.push(d);
                Type::Error
            }
        }
    }

    /// Records that a sin can happen here and checks it is confessed.
    fn note_sin(&mut self, line: usize, why: &str) {
        self.sin_events += 1;
        if self.tempt_depth > 0 {
            return;
        }
        let Some(cur) = &mut self.cur else { return };
        if cur.may_sin {
            cur.sinned = true;
            return;
        }
        let name = cur.name.clone();
        let title = format!("the parable `{name}` may sin here and does not confess it");
        if self.diags.iter().any(|d| d.line == line && d.title == title) {
            return;
        }
        self.push(
            Diag::error(line, title)
                .note(why.to_string())
                .note(format!("hint: declare `{name}` with `may sin`, or wrap this in `tempt ... forgive`")),
        );
    }

    // ---- program ----

    pub fn program(&mut self, stmts: &[Stmt]) {
        // Kinds first, by name, so they may refer to one another.
        for s in stmts {
            if let StmtKind::Kind { name, body } = &s.kind {
                if self.kind_is_record.contains_key(name) || ["Number", "Decimal", "Text", "Truth", "Pagan", "Nothing", "Maybe"].contains(&name.as_str()) {
                    self.err(s.line, format!("the kind `{name}` already exists"));
                    continue;
                }
                self.kind_is_record.insert(name.clone(), matches!(body, KindBody::Record(_)));
            }
        }
        for s in stmts {
            let StmtKind::Kind { name, body } = &s.kind else { continue };
            if self.kinds.contains_key(name) {
                continue;
            }
            let def = match body {
                KindBody::Record(fields) => {
                    let mut out: Vec<(String, Type)> = Vec::new();
                    for (field, te) in fields {
                        if out.iter().any(|(f, _)| f == field) {
                            self.err(s.line, format!("the kind `{name}` has two fields named `{field}`"));
                        }
                        let t = self.resolve(te);
                        out.push((field.clone(), t));
                    }
                    KindDef::Record(out)
                }
                KindBody::Choice(variants) => {
                    let mut out = Vec::new();
                    for (variant, fields) in variants {
                        let types: Vec<Type> = fields.iter().map(|te| self.resolve(te)).collect();
                        if self.variants.contains_key(variant) || ["Found", "Lost"].contains(&variant.as_str()) || self.kind_is_record.contains_key(variant) {
                            self.err(s.line, format!("the case `{variant}` already exists"));
                        } else {
                            self.variants.insert(variant.clone(), (name.clone(), types.clone()));
                        }
                        out.push((variant.clone(), types));
                    }
                    KindDef::Choice(out)
                }
            };
            self.kinds.insert(name.clone(), def);
        }

        // Parable signatures, so parables may call each other in any order.
        for s in stmts {
            let StmtKind::Parable(p) = &s.kind else { continue };
            if let Some(why) = self.taken(&p.name) {
                self.push(Diag::error(p.line, format!("`{}` already exists", p.name)).note(why));
                continue;
            }
            let params = p.params.iter().map(|param| ParamSig { ty: self.resolve(&param.ty), mode: param.mode }).collect();
            let ret = p.ret.as_ref().map(|te| self.resolve(te)).unwrap_or(Type::Nothing);
            self.parables.insert(p.name.clone(), Sig { params, ret, may_sin: p.may_sin });
        }

        // Laws and pagans are visible everywhere.
        for s in stmts {
            match &s.kind {
                StmtKind::Law { name, init } => {
                    let t = self.expr(init, None, Some(&format!("the law `{name}`")));
                    if t == Type::Nothing {
                        self.err(s.line, "a law must hold a value");
                    }
                    if has_unknown(&t) {
                        self.err(s.line, format!("the type of the law `{name}` cannot be told from its value"));
                    }
                    if let Some(why) = self.taken(name) {
                        self.push(Diag::error(s.line, format!("`{name}` already exists")).note(why));
                    }
                    let mut b = Self::binding(name, t, false, BKind::Law, s.line, false);
                    b.written = literal_text(init);
                    self.globals.push(b);
                }
                StmtKind::Pagan { lang, name, .. } => {
                    if lang != "python" && lang != "lua" {
                        self.push(
                            Diag::error(s.line, format!("unknown pagan tongue `{lang}`"))
                                .note("Gesù speaks with `python` and `lua`"),
                        );
                    }
                    if let Some(why) = self.taken(name) {
                        self.push(Diag::error(s.line, format!("`{name}` already exists")).note(why));
                    }
                    self.globals.push(Self::binding(name, Type::Pagan, false, BKind::Law, s.line, false));
                }
                _ => {}
            }
        }

        // The program itself.
        self.scopes = vec![vec![]];
        for s in stmts {
            match &s.kind {
                StmtKind::Parable(p) => {
                    if self.parables.contains_key(&p.name) {
                        self.parable(p);
                    }
                }
                StmtKind::Law { .. } | StmtKind::Kind { .. } | StmtKind::Pagan { .. } => {}
                _ => self.stmt(s),
            }
        }
        self.pop_scope();
    }

    fn parable(&mut self, p: &Parable) {
        let sig = self.parables[&p.name].clone();
        let saved_scopes = std::mem::take(&mut self.scopes);
        let saved_depths = (self.tempt_depth, self.loop_depth);
        self.tempt_depth = 0;
        self.loop_depth = 0;
        self.scopes.push(vec![]);
        for (param, ps) in p.params.iter().zip(&sig.params) {
            let kind = match ps.mode {
                Mode::Own => BKind::Owned,
                Mode::Lend => BKind::Lent,
                Mode::Entrust => BKind::Entrusted,
            };
            if ps.mode == Mode::Entrust && self.is_copy(&ps.ty) {
                self.push(
                    Diag::error(param.line, format!("`{}` cannot be entrusted: a {} is copied, never shared", param.name, ps.ty))
                        .note("only multitudes, maps and kinds can be entrusted; give the new value back instead"),
                );
            }
            self.declare(&param.name, ps.ty.clone(), false, kind, param.line, false);
        }
        self.cur = Some(Cur { name: p.name.clone(), ret: sig.ret.clone(), may_sin: sig.may_sin, sinned: false });
        self.block(&p.body);
        if sig.ret != Type::Nothing && !ends(&p.body, false) {
            self.push(
                Diag::error(p.line, format!("the parable `{}` promises a {} and may end without giving one", p.name, sig.ret))
                    .note("hint: every path must end in `give` or `sin`"),
            );
        }
        let cur = self.cur.take().expect("a parable");
        if cur.may_sin && !cur.sinned {
            self.push(
                Diag::warning(p.line, format!("the parable `{}` confesses a sin it cannot commit", p.name))
                    .note("nothing in it can sin; remove `may sin`"),
            );
        }
        self.pop_scope();
        self.scopes = saved_scopes;
        self.tempt_depth = saved_depths.0;
        self.loop_depth = saved_depths.1;
    }

    fn block(&mut self, stmts: &[Stmt]) {
        self.scopes.push(vec![]);
        for s in stmts {
            self.stmt(s);
        }
        self.pop_scope();
    }

    /// A loop body may run again, so it is examined twice: the second
    /// reading catches a value given away on an earlier turn.
    fn loop_body(&mut self, stmts: &[Stmt]) {
        let before = self.snap();
        self.loop_depth += 1;
        self.block(stmts);
        self.block(stmts);
        self.loop_depth -= 1;
        self.merge(&before);
    }

    fn mutable_root(&mut self, target: &Expr, line: usize) {
        let Some(name) = root(target) else {
            self.err(line, "only a named value can be changed");
            return;
        };
        let Some(loc) = self.find(name) else { return };
        let b = self.b(loc);
        let (kind, mortal, frozen, written) = (b.kind, b.mortal, b.frozen, b.written.clone());
        match kind {
            BKind::Law => {
                let title = match written {
                    Some(value) => format!("it is written: {name} is {value}"),
                    None => format!("it is written: {name} cannot change"),
                };
                self.push(Diag::error(line, title).note(format!("`{name}` is law")));
            }
            BKind::Lent => self.push(
                Diag::error(line, format!("`{name}` was only lent to you; you cannot change it"))
                    .note("hint: ask for it with `entrust` in the parable's signature"),
            ),
            BKind::Owned if !mortal => self.push(
                Diag::error(line, format!("`{name}` is not mortal and cannot change"))
                    .note(format!("hint: declare it with `let mortal {name}`")),
            ),
            _ => {}
        }
        if frozen > 0 {
            self.push(
                Diag::error(line, format!("`{name}` cannot change while `for each` walks through it"))
                    .note("hint: walk through a `copy(lend ...)`, or use `repeat` with an index"),
            );
        }
    }

    fn truth(&mut self, cond: &Expr, word: &str) {
        let t = self.expr(cond, Some(&Type::Truth), None);
        if !fits(&t, &Type::Truth) {
            self.push(
                Diag::error(cond.line, format!("`{word}` accepts only Jesus or False, and was given a {t}"))
                    .note("hint: compare it to something, as in `x is 0`"),
            );
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        let line = s.line;
        match &s.kind {
            StmtKind::Let { name, mortal, ty, init } => {
                let want = ty.as_ref().map(|te| self.resolve(te));
                let t = self.expr(init, want.as_ref(), Some(&format!("`{name}`")));
                let final_type = match want {
                    Some(w) => {
                        if !fits(&t, &w) {
                            self.err(line, format!("`{name}` is declared a {w} and was given a {t}"));
                        }
                        w
                    }
                    None => {
                        if t == Type::Nothing {
                            self.push(
                                Diag::error(line, format!("`{name}` would hold nothing"))
                                    .note("this parable gives nothing back, and Gesù has no null"),
                            );
                            Type::Error
                        } else if has_unknown(&t) {
                            self.push(
                                Diag::error(line, format!("the type of `{name}` cannot be told from its value"))
                                    .note(format!("hint: declare it, as in `let {name}: [Number] = []`")),
                            );
                            Type::Error
                        } else {
                            t
                        }
                    }
                };
                let watch = final_type != Type::Error;
                self.declare(name, final_type, *mortal, BKind::Owned, line, watch);
            }
            StmtKind::Assign { target, value } => {
                if let ExprKind::Var(name) = &target.kind {
                    let Some(loc) = self.find(name) else {
                        self.push(
                            Diag::error(line, format!("unknown name `{name}`"))
                                .note(format!("hint: declare it first with `let mortal {name} = ...`")),
                        );
                        self.expr(value, None, None);
                        return;
                    };
                    let (ty, kind) = (self.b(loc).ty.clone(), self.b(loc).kind);
                    if kind == BKind::Entrusted {
                        self.push(
                            Diag::error(line, format!("`{name}` was entrusted to you; change what is in it, do not replace it"))
                                .note("hint: assign to its elements or fields"),
                        );
                    } else {
                        self.mutable_root(target, line);
                    }
                    let t = self.expr(value, Some(&ty), Some(&format!("`{name}`")));
                    if !fits(&t, &ty) {
                        self.err(line, format!("`{name}` is a {ty} and cannot hold a {t}"));
                    }
                    self.bm(loc).moved = None;
                } else {
                    let ty = self.expr(target, None, None);
                    if let ExprKind::Index(base, _) = &target.kind {
                        let base_type = self.expr(base, None, None);
                        if base_type == Type::Text {
                            self.err(line, "a Text cannot be changed letter by letter");
                        }
                    }
                    self.mutable_root(target, line);
                    let t = self.expr(value, Some(&ty), Some("its new place"));
                    if !fits(&t, &ty) {
                        self.err(line, format!("this place holds a {ty} and cannot hold a {t}"));
                    }
                }
            }
            StmtKind::Preach(e) => {
                let t = self.expr(e, None, None);
                self.showable(&t, e.line, "preached");
            }
            StmtKind::Expr(e) => {
                if !matches!(e.kind, ExprKind::Call(..)) {
                    self.push(Diag::error(line, "this line does nothing").note("a value stands alone here and is thrown away"));
                }
                self.expr(e, None, None);
            }
            StmtKind::Give(value) => {
                let Some(ret) = self.cur.as_ref().map(|c| c.ret.clone()) else {
                    self.push(Diag::error(line, "`give` belongs inside a parable").note("the program itself gives nothing back"));
                    return;
                };
                match value {
                    None => {
                        if ret != Type::Nothing {
                            self.err(line, format!("this parable promises a {ret}; `give` must give one"));
                        }
                    }
                    Some(e) => {
                        let t = self.expr(e, Some(&ret), Some("the caller"));
                        if ret == Type::Nothing {
                            self.push(
                                Diag::error(line, "this parable promises nothing and gives something")
                                    .note(format!("hint: declare what it gives, as in `-> {t}`")),
                            );
                        } else if !fits(&t, &ret) {
                            self.err(line, format!("this parable promises a {ret} and gives a {t}"));
                        }
                    }
                }
            }
            StmtKind::Sin(e) => {
                let t = self.expr(e, Some(&Type::Text), None);
                if !fits(&t, &Type::Text) {
                    self.err(line, format!("a sin is named with a Text, not a {t}"));
                }
                self.note_sin(line, "it sins outright");
            }
            StmtKind::If { cond, then, otherwise } => {
                self.truth(cond, "if");
                let before = self.snap();
                self.block(then);
                let after_then = (self.snap(), ends(then, true));
                self.restore(&before);
                let after_other = match otherwise {
                    Some(other) => {
                        self.block(other);
                        (self.snap(), ends(other, true))
                    }
                    None => (before.clone(), false),
                };
                self.join(&before, &[after_then, after_other]);
            }
            StmtKind::While { cond, body } => {
                self.truth(cond, "while");
                self.loop_body(body);
            }
            StmtKind::Repeat { count, body } => {
                let t = self.expr(count, Some(&Type::Number), None);
                if !fits(&t, &Type::Number) {
                    self.err(line, format!("`repeat` counts with a Number, not a {t}"));
                }
                self.loop_body(body);
            }
            StmtKind::ForEach { first, second, iter, body } => {
                let t = self.expr(iter, None, None);
                let (first_type, second_type) = match (&t, second) {
                    (Type::List(elem), None) => ((**elem).clone(), None),
                    (Type::List(elem), Some(_)) => (Type::Number, Some((**elem).clone())),
                    (Type::Map(k, v), Some(_)) => ((**k).clone(), Some((**v).clone())),
                    (Type::Map(..), None) => {
                        self.push(
                            Diag::error(line, "a map is walked with two names")
                                .note(format!("hint: for each key, {first} in ...")),
                        );
                        (Type::Error, None)
                    }
                    (Type::Text, _) => {
                        self.push(Diag::error(line, "a Text is not walked directly").note("hint: for each letter in chars(...)"));
                        (Type::Error, Some(Type::Error))
                    }
                    (Type::Error, _) => (Type::Error, Some(Type::Error)),
                    (other, _) => {
                        self.err(line, format!("`for each` walks a multitude or a map, not a {other}"));
                        (Type::Error, Some(Type::Error))
                    }
                };
                let frozen = root(iter).and_then(|name| self.find(name));
                if let Some(loc) = frozen {
                    self.bm(loc).frozen += 1;
                }
                let before = self.snap();
                self.loop_depth += 1;
                for _ in 0..2 {
                    self.scopes.push(vec![]);
                    let kind = |c: &Checker, t: &Type| if c.is_copy(t) { BKind::Owned } else { BKind::Lent };
                    let k = kind(self, &first_type);
                    self.declare(first, first_type.clone(), false, k, line, false);
                    if let (Some(name), Some(ty)) = (second, &second_type) {
                        let k = kind(self, ty);
                        self.declare(name, ty.clone(), false, k, line, false);
                    }
                    self.block(body);
                    self.pop_scope();
                }
                self.loop_depth -= 1;
                self.merge(&before);
                if let Some(loc) = frozen {
                    self.bm(loc).frozen -= 1;
                }
            }
            StmtKind::Break | StmtKind::Continue => {
                if self.loop_depth == 0 {
                    self.err(line, "there is no loop here to leave");
                }
            }
            StmtKind::Tempt { body, name, handler } => {
                let before = self.snap();
                let sins_before = self.sin_events;
                self.tempt_depth += 1;
                self.block(body);
                self.tempt_depth -= 1;
                if self.sin_events == sins_before {
                    self.push(
                        Diag::warning(line, "nothing in this `tempt` can sin")
                            .note("the `forgive` below it will never run"),
                    );
                }
                let after_body = (self.snap(), ends(body, true));
                self.scopes.push(vec![]);
                self.declare(name, Type::Text, false, BKind::Owned, line, false);
                self.block(handler);
                self.pop_scope();
                let after_handler = (self.snap(), ends(handler, true));
                self.join(&before, &[after_body, after_handler]);
            }
            StmtKind::Discern { subject, arms } => self.discern(line, subject, arms),
            StmtKind::Repent => {}
            StmtKind::Law { .. } | StmtKind::Kind { .. } | StmtKind::Pagan { .. } | StmtKind::Parable(_) => {}
        }
    }

    fn discern(&mut self, line: usize, subject: &Expr, arms: &[Arm]) {
        let t = self.expr(subject, None, Some("`discern`"));
        // Every case this type can take, with the types it carries.
        let cases: Vec<(String, Vec<Type>)> = match &t {
            Type::Truth => vec![("Jesus".into(), vec![]), ("False".into(), vec![])],
            Type::Maybe(inner) => vec![("Found".into(), vec![(**inner).clone()]), ("Lost".into(), vec![])],
            Type::Choice(name) => match self.kinds.get(name) {
                Some(KindDef::Choice(variants)) => variants.clone(),
                _ => vec![],
            },
            Type::Error => vec![],
            other => {
                self.push(
                    Diag::error(line, format!("a {other} cannot be discerned"))
                        .note("`discern` works on a Truth, a Maybe, or a kind with cases"),
                );
                vec![]
            }
        };
        let known = !cases.is_empty();
        let before = self.snap();
        let mut seen: Vec<&str> = Vec::new();
        let mut results = Vec::new();
        for arm in arms {
            self.restore(&before);
            let pat = &arm.pattern;
            let fields = cases.iter().find(|(name, _)| *name == pat.name).map(|(_, f)| f.clone());
            if known && fields.is_none() {
                let names: Vec<&str> = cases.iter().map(|(n, _)| n.as_str()).collect();
                self.push(
                    Diag::error(pat.line, format!("`{}` is not a case of {t}", pat.name))
                        .note(format!("its cases: {}", names.join(", "))),
                );
            }
            if seen.contains(&pat.name.as_str()) {
                self.err(pat.line, format!("the case `{}` is faced twice", pat.name));
            }
            seen.push(&pat.name);
            self.scopes.push(vec![]);
            let fields = fields.unwrap_or_default();
            if known && fields.len() != pat.binds.len() {
                self.err(pat.line, format!("`{}` carries {} value(s), and {} name(s) were given", pat.name, fields.len(), pat.binds.len()));
            }
            for (i, bind) in pat.binds.iter().enumerate() {
                let ty = fields.get(i).cloned().unwrap_or(Type::Error);
                self.declare(bind, ty, false, BKind::Owned, pat.line, false);
            }
            self.block(&arm.body);
            self.pop_scope();
            results.push((self.snap(), ends(&arm.body, true)));
        }
        let missing: Vec<String> = cases.iter().filter(|(n, _)| !seen.contains(&n.as_str())).map(|(n, _)| format!("`{n}`")).collect();
        if !missing.is_empty() {
            self.push(
                Diag::error(line, format!("`discern` must face every case; missing {}", missing.join(", ")))
                    .note(format!("the value is a {t}")),
            );
        }
        self.join(&before, &results);
    }

    fn showable(&mut self, t: &Type, line: usize, how: &str) {
        match t {
            Type::Nothing => self.err(line, format!("there is nothing here to be {how}")),
            Type::Pagan => self.push(
                Diag::error(line, format!("a Pagan cannot be {how}"))
                    .note("hint: baptize it first, as in `baptize x as Text`"),
            ),
            _ => {}
        }
    }

    // ---- expressions ----

    /// Checks an expression and returns its type. `want` is the type the
    /// surroundings hope for. `give` names who receives the value when it
    /// is given away; `None` means it is only looked at.
    fn expr(&mut self, e: &Expr, want: Option<&Type>, give: Option<&str>) -> Type {
        let line = e.line;
        match &e.kind {
            ExprKind::Number(_) => Type::Number,
            ExprKind::Decimal(_) => Type::Decimal,
            ExprKind::Jesus | ExprKind::False => Type::Truth,
            ExprKind::Text(parts) => {
                for part in parts {
                    if let TextPart::Code(inner) = part {
                        let t = self.expr(inner, None, None);
                        self.showable(&t, inner.line, "placed in a text");
                    }
                }
                Type::Text
            }
            ExprKind::Var(name) => self.var(name, line, want, give),
            ExprKind::List(items) => {
                let mut elem = match want {
                    Some(Type::List(t)) => (**t).clone(),
                    _ => Type::Unknown,
                };
                for item in items {
                    let hint = if has_unknown(&elem) { None } else { Some(elem.clone()) };
                    let t = self.expr(item, hint.as_ref(), Some("a multitude"));
                    if t == Type::Nothing {
                        self.err(item.line, "a multitude cannot hold nothing");
                    } else if !fits(&t, &elem) {
                        self.err(item.line, format!("a multitude holds one type: expected {elem}, found {t}"));
                    } else if has_unknown(&elem) && !has_unknown(&t) {
                        elem = t;
                    }
                }
                Type::List(Box::new(elem))
            }
            ExprKind::Map(pairs) => {
                let (mut key, mut value) = match want {
                    Some(Type::Map(k, v)) => ((**k).clone(), (**v).clone()),
                    _ => (Type::Unknown, Type::Unknown),
                };
                for (k, v) in pairs {
                    let kt = self.expr(k, None, None);
                    if !matches!(kt, Type::Text | Type::Number | Type::Error) {
                        self.err(k.line, format!("a map is keyed by Text or Number, not {kt}"));
                    } else if !fits(&kt, &key) {
                        self.err(k.line, format!("a map has one type of key: expected {key}, found {kt}"));
                    } else if key == Type::Unknown {
                        key = kt;
                    }
                    let hint = if has_unknown(&value) { None } else { Some(value.clone()) };
                    let vt = self.expr(v, hint.as_ref(), Some("a map"));
                    if vt == Type::Nothing {
                        self.err(v.line, "a map cannot hold nothing");
                    } else if !fits(&vt, &value) {
                        self.err(v.line, format!("a map holds one type of value: expected {value}, found {vt}"));
                    } else if has_unknown(&value) && !has_unknown(&vt) {
                        value = vt;
                    }
                }
                Type::Map(Box::new(key), Box::new(value))
            }
            ExprKind::Neg(inner) => {
                let t = self.expr(inner, want, None);
                if !matches!(t, Type::Number | Type::Decimal | Type::Error) {
                    self.err(line, format!("only a Number or a Decimal can be negated, not a {t}"));
                    return Type::Error;
                }
                t
            }
            ExprKind::Deny(inner) => {
                let t = self.expr(inner, Some(&Type::Truth), None);
                if !fits(&t, &Type::Truth) {
                    self.err(line, format!("only a Truth can be denied, not a {t}"));
                }
                Type::Truth
            }
            ExprKind::Binary(op, l, r) => self.binary(line, *op, l, r),
            ExprKind::Call(callee, args) => self.call(line, callee, args, want),
            ExprKind::Index(base, index) => {
                let bt = self.expr(base, None, None);
                let result = match &bt {
                    Type::List(elem) => {
                        let it = self.expr(index, Some(&Type::Number), None);
                        if !fits(&it, &Type::Number) {
                            self.err(line, format!("a multitude is indexed by Number, not {it}"));
                        }
                        (**elem).clone()
                    }
                    Type::Map(k, v) => {
                        let it = self.expr(index, Some(k), None);
                        if !fits(&it, k) {
                            self.err(line, format!("this map is keyed by {k}, not {it}"));
                        }
                        (**v).clone()
                    }
                    other => {
                        self.expr(index, None, None);
                        match other {
                            Type::Text => self.push(Diag::error(line, "a Text is not indexed").note("hint: chars(...) gives its letters")),
                            Type::Pagan => self.push(Diag::error(line, "a Pagan is not indexed").note("hint: baptize it first, as in `baptize x as [Number]`")),
                            Type::Error => {}
                            t => self.err(line, format!("a {t} cannot be indexed")),
                        }
                        Type::Error
                    }
                };
                self.taken_out(e, base, &result, give);
                result
            }
            ExprKind::Field(base, field) => {
                let bt = self.expr(base, None, None);
                let result = match &bt {
                    Type::Record(kind) => {
                        let found = match self.kinds.get(kind) {
                            Some(KindDef::Record(fields)) => fields.iter().find(|(f, _)| f == field).map(|(_, t)| t.clone()),
                            _ => None,
                        };
                        found.unwrap_or_else(|| {
                            self.err(line, format!("the kind `{kind}` has no field `{field}`"));
                            Type::Error
                        })
                    }
                    Type::Pagan => {
                        self.note_sin(line, "it reaches into foreign code, which can fail");
                        Type::Pagan
                    }
                    Type::Error => Type::Error,
                    other => {
                        self.err(line, format!("a {other} has no fields"));
                        Type::Error
                    }
                };
                self.taken_out(e, base, &result, give);
                result
            }
            ExprKind::Baptize(inner, te) => {
                let t = self.expr(inner, None, None);
                if !matches!(t, Type::Pagan | Type::Error) {
                    self.push(
                        Diag::error(line, format!("only a Pagan can be baptized; this is already a {t}"))
                            .note("`baptize` is for values that come from foreign code or JSON"),
                    );
                }
                let target = self.resolve(te);
                if let Type::Choice(name) = &target {
                    self.err(line, format!("a Pagan cannot be baptized into the kind `{name}`, which has cases"));
                }
                self.note_sin(line, "a baptism fails when the Pagan is not what you claim");
                target
            }
        }
    }

    fn taken_out(&mut self, e: &Expr, base: &Expr, t: &Type, give: Option<&str>) {
        if give.is_some() && !self.is_copy(t) && root(base).is_some() {
            self.push(
                Diag::error(e.line, "you cannot take a part and leave the whole")
                    .note(format!("this {t} would be moved out of the value that holds it"))
                    .note("hint: `copy(lend ...)` it, or lend it"),
            );
        }
    }

    fn var(&mut self, name: &str, line: usize, want: Option<&Type>, give: Option<&str>) -> Type {
        if ["True", "true", "TRUE"].contains(&name) {
            self.push(Diag::error(line, "there is only one Truth, Jesus").note(format!("hint: write `Jesus` in place of `{name}`")));
            return Type::Truth;
        }
        if let Some(loc) = self.find(name) {
            let ty = self.b(loc).ty.clone();
            let copy = self.is_copy(&ty);
            let b = self.bm(loc);
            b.used = true;
            if let Some((moved_line, to)) = b.moved.clone() {
                let when = if moved_line == line {
                    "on an earlier turn of this loop".to_string()
                } else {
                    format!("on line {moved_line}")
                };
                self.push(
                    Diag::error(line, "Thou shalt not steal")
                        .note(format!("`{name}` was given to {to} {when}"))
                        .note(format!("hint: lend it there (`lend {name}`), or give away a `copy(lend {name})`")),
                );
                return ty;
            }
            if let (Some(dest), false) = (give, copy) {
                match b.kind {
                    BKind::Owned if b.frozen > 0 => self.err(line, format!("`{name}` cannot be given away while `for each` walks through it")),
                    BKind::Owned => b.moved = Some((line, dest.to_string())),
                    BKind::Law => self.push(
                        Diag::error(line, format!("it is written: the law `{name}` cannot be given away"))
                            .note(format!("hint: `lend {name}`, or give a `copy(lend {name})`")),
                    ),
                    BKind::Lent | BKind::Entrusted => self.push(
                        Diag::error(line, format!("`{name}` was only lent to you; you cannot give it away"))
                            .note(format!("hint: `copy(lend {name})` makes one of your own")),
                    ),
                }
            }
            return ty;
        }
        if name == "Lost" {
            return match want {
                Some(Type::Maybe(inner)) => Type::Maybe(inner.clone()),
                _ => Type::Maybe(Box::new(Type::Unknown)),
            };
        }
        if let Some((kind, fields)) = self.variants.get(name).cloned() {
            if !fields.is_empty() {
                self.err(line, format!("the case `{name}` carries {} value(s): write `{name}(...)`", fields.len()));
            }
            return Type::Choice(kind);
        }
        if self.parables.contains_key(name) || BUILTINS.contains(&name) || name == "Found" {
            self.push(Diag::error(line, format!("`{name}` must be called: `{name}(...)`")).note("a parable cannot be stored or passed"));
            return Type::Error;
        }
        let mut d = Diag::error(line, format!("unknown name `{name}`"));
        d = match name {
            "None" | "null" | "nil" | "NULL" | "undefined" => d.note("Gesù has no null; an absent value is `Lost`, of type `Maybe T`"),
            "false" | "FALSE" => d.note("hint: write `False`"),
            "jesus" | "JESUS" => d.note("hint: write `Jesus`"),
            _ => d.note(format!("hint: declare it with `let {name} = ...`")),
        };
        self.push(d);
        Type::Error
    }

    fn binary(&mut self, line: usize, op: BinOp, l: &Expr, r: &Expr) -> Type {
        use BinOp::*;
        match op {
            And | Or => {
                for side in [l, r] {
                    let t = self.expr(side, Some(&Type::Truth), None);
                    if !fits(&t, &Type::Truth) {
                        let word = if op == And { "and" } else { "or" };
                        self.err(side.line, format!("`{word}` joins Truths, not a {t}"));
                    }
                }
                Type::Truth
            }
            Is | IsNot => {
                let lt = self.expr(l, None, None);
                let rt = self.expr(r, Some(&lt), None);
                if lt == Type::Pagan || rt == Type::Pagan {
                    self.push(Diag::error(line, "a Pagan cannot be compared").note("hint: baptize it first"));
                } else if !fits(&lt, &rt) {
                    self.push(
                        Diag::error(line, format!("a {lt} is never a {rt}"))
                            .note("Gesù has no implicit conversions; both sides of `is` must have one type"),
                    );
                }
                Type::Truth
            }
            Lt | Gt | Le | Ge => {
                let lt = self.expr(l, None, None);
                let rt = self.expr(r, Some(&lt), None);
                let ordered = |t: &Type| matches!(t, Type::Number | Type::Decimal | Type::Text | Type::Error | Type::Unknown);
                if !ordered(&lt) || !ordered(&rt) {
                    self.err(line, format!("a {lt} and a {rt} cannot be ordered"));
                } else if !fits(&lt, &rt) {
                    self.push(self.mixed(line, &lt, &rt));
                }
                Type::Truth
            }
            Add | Sub | Mul | Div | Rem => {
                let lt = self.expr(l, None, None);
                let rt = self.expr(r, Some(&lt), None);
                match (&lt, &rt) {
                    (Type::Error, other) | (other, Type::Error) => {
                        if matches!(other, Type::Number | Type::Decimal | Type::Text) { other.clone() } else { Type::Error }
                    }
                    (Type::Number, Type::Number) => Type::Number,
                    (Type::Decimal, Type::Decimal) => Type::Decimal,
                    (Type::Text, Type::Text) if op == Add => Type::Text,
                    (Type::Text, _) | (_, Type::Text) if op == Add => {
                        self.push(
                            Diag::error(line, format!("a {lt} and a {rt} cannot be added"))
                                .note("Gesù has no implicit conversions")
                                .note("hint: place the value inside the text, as in \"total: {x}\""),
                        );
                        Type::Text
                    }
                    (Type::Number, Type::Decimal) | (Type::Decimal, Type::Number) => {
                        self.push(self.mixed(line, &lt, &rt));
                        Type::Decimal
                    }
                    _ => {
                        self.err(line, format!("arithmetic needs Numbers or Decimals, not a {lt} and a {rt}"));
                        Type::Error
                    }
                }
            }
        }
    }

    fn mixed(&self, line: usize, lt: &Type, rt: &Type) -> Diag {
        Diag::error(line, format!("a {lt} and a {rt} do not mix"))
            .note("Gesù has no implicit conversions")
            .note("hint: `decimal(n)` turns a Number into a Decimal; `floor(d)` and `round(d)` go back")
    }

    // ---- calls ----

    /// Checks one argument against the way the parable receives it.
    fn arg(&mut self, a: &Arg, want: &Type, mode: Mode, callee: &str) -> Type {
        let line = a.expr.line;
        let hint = if has_unknown(want) { None } else { Some(want) };
        let t = match mode {
            Mode::Own => {
                if a.mode != Mode::Own {
                    let (offered, done) = if a.mode == Mode::Lend { ("lend", "lent") } else { ("entrust", "entrusted") };
                    self.push(
                        Diag::error(line, format!("`{callee}` takes this value for itself; it cannot be only {done}"))
                            .note(format!("hint: remove `{offered}`, or give a `copy(lend ...)`")),
                    );
                }
                self.expr(&a.expr, hint, Some(&format!("`{callee}`")))
            }
            Mode::Lend => {
                let t = self.expr(&a.expr, hint, None);
                if a.mode == Mode::Entrust {
                    self.push(Diag::error(line, format!("`{callee}` only reads this value; `lend` is enough")));
                }
                let borrowed = root(&a.expr)
                    .and_then(|name| self.find(name))
                    .is_some_and(|loc| self.b(loc).kind != BKind::Owned);
                if a.mode == Mode::Own && !self.is_copy(&t) && !borrowed {
                    self.push(
                        Diag::error(line, format!("`{callee}` borrows this value: say so with `lend`"))
                            .note("a loan is written where it is made, so the reader sees what stays yours"),
                    );
                }
                t
            }
            Mode::Entrust => {
                let t = self.expr(&a.expr, hint, None);
                if a.mode != Mode::Entrust {
                    self.push(
                        Diag::error(line, format!("`{callee}` changes this value: say so with `entrust`"))
                            .note("the reader must see where a value may change"),
                    );
                } else if self.is_copy(&t) {
                    if t != Type::Error {
                        self.push(
                            Diag::error(line, format!("a {t} cannot be entrusted"))
                                .note("only multitudes, maps and kinds can be entrusted"),
                        );
                    }
                } else if root(&a.expr).is_none() {
                    self.err(line, "only a named value can be entrusted");
                } else {
                    self.mutable_root(&a.expr, line);
                }
                t
            }
        };
        if !fits(&t, want) {
            self.err(line, format!("`{callee}` expects a {want} here and was given a {t}"));
        }
        t
    }

    /// No one can serve two masters: an entrusted value may not appear
    /// again in the same call.
    fn two_masters(&mut self, line: usize, args: &[Arg]) {
        for (i, a) in args.iter().enumerate() {
            if a.mode != Mode::Entrust {
                continue;
            }
            let Some(name) = root(&a.expr) else { continue };
            if args.iter().enumerate().any(|(j, other)| j != i && mentions(&other.expr, name)) {
                self.push(
                    Diag::error(line, "no one can serve two masters")
                        .note(format!("`{name}` is entrusted and also used elsewhere in this call")),
                );
            }
        }
    }

    fn arity(&mut self, line: usize, name: &str, args: &[Arg], n: usize) -> bool {
        if args.len() != n {
            self.err(line, format!("`{name}` takes {n} argument(s) and was given {}", args.len()));
            for a in args {
                self.expr(&a.expr, None, None);
            }
            return false;
        }
        true
    }

    fn call(&mut self, line: usize, callee: &Expr, args: &[Arg], want: Option<&Type>) -> Type {
        let name = match &callee.kind {
            ExprKind::Var(name) if self.find(name).is_none() => name.as_str(),
            _ => {
                // A value is being called: only a Pagan can be.
                let t = self.expr(callee, None, None);
                for a in args {
                    if a.mode == Mode::Entrust {
                        self.err(a.expr.line, "nothing can be entrusted to foreign code; it receives copies");
                    }
                    let at = self.expr(&a.expr, None, None);
                    if at == Type::Nothing {
                        self.err(a.expr.line, "there is nothing here to pass");
                    }
                }
                return match t {
                    Type::Pagan => {
                        self.note_sin(line, "it calls foreign code, which can fail");
                        Type::Pagan
                    }
                    Type::Error => Type::Error,
                    other => {
                        self.err(line, format!("a {other} cannot be called"));
                        Type::Error
                    }
                };
            }
        };

        if let Some(sig) = self.parables.get(name).cloned() {
            if !self.arity(line, name, args, sig.params.len()) {
                return sig.ret;
            }
            for (a, p) in args.iter().zip(&sig.params) {
                if a.name.is_some() {
                    self.err(a.expr.line, "a parable takes its arguments in order, without names");
                }
                self.arg(a, &p.ty, p.mode, name);
            }
            self.two_masters(line, args);
            if sig.may_sin {
                self.note_sin(line, &format!("it calls `{name}`, which may sin"));
            }
            return sig.ret;
        }

        if let Some(KindDef::Record(fields)) = self.kinds.get(name).cloned() {
            for a in args {
                let Some(field) = &a.name else {
                    self.push(
                        Diag::error(a.expr.line, format!("the fields of `{name}` are given by name"))
                            .note(format!("hint: {name}({}: ...)", fields.first().map(|f| f.0.as_str()).unwrap_or("field"))),
                    );
                    self.expr(&a.expr, None, None);
                    continue;
                };
                match fields.iter().find(|(f, _)| f == field) {
                    Some((_, ty)) => {
                        let t = self.expr(&a.expr, Some(ty), Some(&format!("a `{name}`")));
                        if !fits(&t, ty) {
                            self.err(a.expr.line, format!("the field `{field}` is a {ty} and was given a {t}"));
                        }
                    }
                    None => {
                        self.err(a.expr.line, format!("the kind `{name}` has no field `{field}`"));
                        self.expr(&a.expr, None, None);
                    }
                }
            }
            for (field, _) in &fields {
                let count = args.iter().filter(|a| a.name.as_deref() == Some(field)).count();
                if count == 0 {
                    self.err(line, format!("the field `{field}` of `{name}` is missing"));
                } else if count > 1 {
                    self.err(line, format!("the field `{field}` of `{name}` is given twice"));
                }
            }
            return Type::Record(name.to_string());
        }

        if let Some((kind, fields)) = self.variants.get(name).cloned() {
            if self.arity(line, name, args, fields.len()) {
                for (a, ty) in args.iter().zip(&fields) {
                    let t = self.expr(&a.expr, Some(ty), Some(&format!("a `{name}`")));
                    if !fits(&t, ty) {
                        self.err(a.expr.line, format!("`{name}` expects a {ty} here and was given a {t}"));
                    }
                }
            }
            return Type::Choice(kind);
        }

        if name == "Found" {
            if !self.arity(line, name, args, 1) {
                return Type::Error;
            }
            let inner = match want {
                Some(Type::Maybe(t)) => Some((**t).clone()),
                _ => None,
            };
            let t = self.expr(&args[0].expr, inner.as_ref(), Some("a `Found`"));
            if t == Type::Nothing {
                self.err(line, "`Found` must hold a value");
            }
            return Type::Maybe(Box::new(t));
        }

        if BUILTINS.contains(&name) {
            let t = self.builtin(line, name, args);
            self.two_masters(line, args);
            return t;
        }

        if name == "Lost" {
            self.err(line, "`Lost` holds nothing: write it without parentheses");
            return Type::Error;
        }
        self.push(Diag::error(line, format!("unknown parable `{name}`")).note(format!("hint: declare it with `parable {name}(...)`")));
        for a in args {
            self.expr(&a.expr, None, None);
        }
        Type::Error
    }

    fn builtin(&mut self, line: usize, name: &str, args: &[Arg]) -> Type {
        use Type::*;
        let any = Unknown;
        let list_of = |t: &Type| match t {
            List(elem) => Some((**elem).clone()),
            _ => None,
        };
        let count = match name {
            "read_line" | "args" => 0,
            "length" | "pop" | "keys" | "copy" | "sort" | "reverse" | "trim" | "upper" | "lower" | "chars" | "text"
            | "number" | "decimal" | "floor" | "round" | "abs" | "read_file" | "parse_json" | "to_json" => 1,
            "replace" => 3,
            _ => 2,
        };
        if !self.arity(line, name, args, count) {
            return Error;
        }
        let wrong = |c: &mut Checker, what: &str, got: &Type| {
            if *got != Error {
                c.err(line, format!("`{name}` works on {what}, not a {got}"));
            }
            Error
        };
        match name {
            "length" => {
                let t = self.arg(&args[0], &any, Mode::Lend, name);
                if !matches!(t, Text | List(_) | Map(..) | Error) {
                    return wrong(self, "a Text, a multitude or a map", &t);
                }
                Number
            }
            "push" => {
                let t = self.arg(&args[0], &any, Mode::Entrust, name);
                match list_of(&t) {
                    Some(elem) => {
                        self.arg(&args[1], &elem, Mode::Own, name);
                        Nothing
                    }
                    None => {
                        self.expr(&args[1].expr, None, None);
                        wrong(self, "a multitude", &t)
                    }
                }
            }
            "pop" => {
                let t = self.arg(&args[0], &any, Mode::Entrust, name);
                match list_of(&t) {
                    Some(elem) => Maybe(Box::new(elem)),
                    None => wrong(self, "a multitude", &t),
                }
            }
            "get" => {
                let t = self.arg(&args[0], &any, Mode::Lend, name);
                self.arg(&args[1], &Number, Mode::Own, name);
                match list_of(&t) {
                    Some(elem) => Maybe(Box::new(elem)),
                    None => wrong(self, "a multitude", &t),
                }
            }
            "lookup" => {
                let t = self.arg(&args[0], &any, Mode::Lend, name);
                match &t {
                    Map(k, v) => {
                        self.arg(&args[1], k, Mode::Own, name);
                        Maybe(v.clone())
                    }
                    _ => {
                        self.expr(&args[1].expr, None, None);
                        wrong(self, "a map", &t)
                    }
                }
            }
            "contains" => {
                let t = self.arg(&args[0], &any, Mode::Lend, name);
                match &t {
                    List(elem) => {
                        self.arg(&args[1], elem, Mode::Lend, name);
                    }
                    Map(k, _) => {
                        self.arg(&args[1], k, Mode::Own, name);
                    }
                    Text => {
                        self.arg(&args[1], &Text, Mode::Own, name);
                    }
                    _ => {
                        self.expr(&args[1].expr, None, None);
                        return wrong(self, "a Text, a multitude or a map", &t);
                    }
                }
                Truth
            }
            "remove" => {
                let t = self.arg(&args[0], &any, Mode::Entrust, name);
                match &t {
                    Map(k, v) => {
                        self.arg(&args[1], k, Mode::Own, name);
                        Maybe(v.clone())
                    }
                    List(elem) => {
                        self.arg(&args[1], &Number, Mode::Own, name);
                        (**elem).clone()
                    }
                    _ => {
                        self.expr(&args[1].expr, None, None);
                        wrong(self, "a multitude or a map", &t)
                    }
                }
            }
            "keys" => {
                let t = self.arg(&args[0], &any, Mode::Lend, name);
                match &t {
                    Map(k, _) => List(k.clone()),
                    _ => wrong(self, "a map", &t),
                }
            }
            "copy" => {
                let t = self.arg(&args[0], &any, Mode::Lend, name);
                if t == Pagan {
                    return wrong(self, "values of Gesù", &t);
                }
                t
            }
            "range" => {
                self.arg(&args[0], &Number, Mode::Own, name);
                self.arg(&args[1], &Number, Mode::Own, name);
                List(Box::new(Number))
            }
            "sort" => {
                let t = self.arg(&args[0], &any, Mode::Entrust, name);
                match list_of(&t) {
                    Some(Number) | Some(Decimal) | Some(Text) | Some(Error) => Nothing,
                    _ => wrong(self, "a multitude of Number, Decimal or Text", &t),
                }
            }
            "reverse" => {
                let t = self.arg(&args[0], &any, Mode::Entrust, name);
                if list_of(&t).is_none() {
                    return wrong(self, "a multitude", &t);
                }
                Nothing
            }
            "split" => {
                self.arg(&args[0], &Text, Mode::Own, name);
                self.arg(&args[1], &Text, Mode::Own, name);
                List(Box::new(Text))
            }
            "join" => {
                self.arg(&args[0], &List(Box::new(Text)), Mode::Lend, name);
                self.arg(&args[1], &Text, Mode::Own, name);
                Text
            }
            "trim" | "upper" | "lower" => {
                self.arg(&args[0], &Text, Mode::Own, name);
                Text
            }
            "replace" => {
                for a in args {
                    self.arg(a, &Text, Mode::Own, name);
                }
                Text
            }
            "chars" => {
                self.arg(&args[0], &Text, Mode::Own, name);
                List(Box::new(Text))
            }
            "text" => {
                let t = self.arg(&args[0], &any, Mode::Lend, name);
                self.showable(&t, line, "turned into a Text");
                Text
            }
            "number" => {
                self.arg(&args[0], &Text, Mode::Own, name);
                self.note_sin(line, "`number` sins when the Text is not a number");
                Number
            }
            "decimal" => {
                let t = self.arg(&args[0], &any, Mode::Own, name);
                match t {
                    Number | Error => {}
                    Text => self.note_sin(line, "`decimal` sins when the Text is not a number"),
                    other => return wrong(self, "a Number or a Text", &other),
                }
                Decimal
            }
            "floor" | "round" => {
                self.arg(&args[0], &Decimal, Mode::Own, name);
                Number
            }
            "abs" => {
                let t = self.arg(&args[0], &any, Mode::Own, name);
                if !matches!(t, Number | Decimal | Error) {
                    return wrong(self, "a Number or a Decimal", &t);
                }
                t
            }
            "min" | "max" => {
                let t = self.arg(&args[0], &any, Mode::Own, name);
                if !matches!(t, Number | Decimal | Text | Error) {
                    self.expr(&args[1].expr, None, None);
                    return wrong(self, "Numbers, Decimals or Texts", &t);
                }
                self.arg(&args[1], &t, Mode::Own, name);
                t
            }
            "read_file" => {
                self.arg(&args[0], &Text, Mode::Own, name);
                self.note_sin(line, "`read_file` sins when the file cannot be read");
                Text
            }
            "write_file" => {
                self.arg(&args[0], &Text, Mode::Own, name);
                self.arg(&args[1], &Text, Mode::Own, name);
                self.note_sin(line, "`write_file` sins when the file cannot be written");
                Nothing
            }
            "read_line" => Maybe(Box::new(Text)),
            "args" => List(Box::new(Text)),
            "parse_json" => {
                self.arg(&args[0], &Text, Mode::Own, name);
                self.note_sin(line, "`parse_json` sins when the Text is not JSON");
                Pagan
            }
            "to_json" => {
                let t = self.arg(&args[0], &any, Mode::Lend, name);
                if t == Nothing {
                    return wrong(self, "a value", &t);
                }
                Text
            }
            _ => Error,
        }
    }
}
