//! The runtime: a tree-walking interpreter. It trusts the checker, so it
//! carries no types and no ownership rules of its own.

use crate::ast::*;
use crate::pagan::{self, Foreign};
use crate::types::{resolve, KindDef, Kinds};
use crate::value::*;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

pub struct SinInfo {
    pub message: String,
    pub line: usize,
    /// Parables that were running, innermost last, with the line of each call.
    pub trace: Vec<(String, usize)>,
}

/// Everything that interrupts the normal order of statements.
pub enum Flow {
    Give(Value),
    Break,
    Continue,
    /// A sin that `forgive` can catch.
    Sin(SinInfo),
    /// A sin that nothing can catch.
    Mortal(SinInfo),
}

type R<T> = Result<T, Flow>;

const FORGIVENESS: u32 = 490;
const DEPTH: usize = 2000;

pub struct Interp {
    parables: HashMap<String, Rc<Parable>>,
    kinds: Kinds,
    globals: HashMap<String, Value>,
    locals: Vec<(String, Value)>,
    /// Where the running parable's own values begin in `locals`.
    base: usize,
    calls: Vec<(String, usize)>,
    forgiven: u32,
    foreign: Foreign,
    args: Vec<String>,
}

fn as_text(v: &Value) -> Rc<str> {
    match v {
        Value::Text(s) => s.clone(),
        _ => Rc::from(""),
    }
}

fn as_number(v: &Value) -> i64 {
    match v {
        Value::Number(n) => *n,
        _ => 0,
    }
}

fn order(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.cmp(y),
        (Value::Decimal(x), Value::Decimal(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (Value::Text(x), Value::Text(y)) => x.cmp(y),
        _ => Ordering::Equal,
    }
}

impl Interp {
    pub fn new(kinds: Kinds, dir: PathBuf, args: Vec<String>) -> Self {
        Interp {
            parables: HashMap::new(),
            kinds,
            globals: HashMap::new(),
            locals: Vec::new(),
            base: 0,
            calls: Vec::new(),
            forgiven: 0,
            foreign: Foreign::new(dir),
            args,
        }
    }

    fn info(&self, line: usize, message: impl Into<String>) -> SinInfo {
        SinInfo { message: message.into(), line, trace: self.calls.clone() }
    }

    fn sin(&self, line: usize, message: impl Into<String>) -> Flow {
        Flow::Sin(self.info(line, message))
    }

    fn mortal(&self, line: usize, message: impl Into<String>) -> Flow {
        Flow::Mortal(self.info(line, message))
    }

    fn bug(&self, line: usize, what: &str) -> Flow {
        self.mortal(line, format!("the interpreter itself has erred ({what}); please report this"))
    }

    pub fn run(&mut self, stmts: &[Stmt]) -> R<()> {
        for s in stmts {
            if let StmtKind::Parable(p) = &s.kind {
                self.parables.insert(p.name.clone(), p.clone());
            }
        }
        for s in stmts {
            self.exec(s)?;
        }
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        self.locals[self.base..].iter().rev().find(|(n, _)| n == name).map(|(_, v)| v).or_else(|| self.globals.get(name))
    }

    fn block(&mut self, stmts: &[Stmt]) -> R<()> {
        let start = self.locals.len();
        let mut result = Ok(());
        for s in stmts {
            result = self.exec(s);
            if result.is_err() {
                break;
            }
        }
        self.locals.truncate(start);
        result
    }

    /// Runs a loop body. Returns false when the loop must stop.
    fn turn(&mut self, body: &[Stmt]) -> R<bool> {
        match self.block(body) {
            Ok(()) | Err(Flow::Continue) => Ok(true),
            Err(Flow::Break) => Ok(false),
            Err(other) => Err(other),
        }
    }

    fn exec(&mut self, s: &Stmt) -> R<()> {
        let line = s.line;
        match &s.kind {
            StmtKind::Let { name, init, .. } => {
                let v = self.eval(init)?;
                self.locals.push((name.clone(), v));
            }
            StmtKind::Law { name, init } => {
                let v = self.eval(init)?;
                self.globals.insert(name.clone(), v);
            }
            StmtKind::Pagan { lang, path, name } => {
                let module = self.foreign.import(lang, path).map_err(|m| self.sin(line, m))?;
                self.globals.insert(name.clone(), Value::Pagan(module));
            }
            StmtKind::Parable(_) | StmtKind::Kind { .. } => {}
            StmtKind::Assign { target, value } => {
                let v = self.eval(value)?;
                match &target.kind {
                    ExprKind::Var(name) => {
                        let base = self.base;
                        match self.locals[base..].iter_mut().rev().find(|(n, _)| n == name) {
                            Some(slot) => slot.1 = v,
                            None => return Err(self.bug(line, "assignment to an unknown name")),
                        }
                    }
                    ExprKind::Index(base, index) => {
                        let container = self.eval(base)?;
                        let key = self.eval(index)?;
                        match container {
                            Value::List(items) => {
                                let i = as_number(&key);
                                let len = items.borrow().len();
                                if i < 0 || i as usize >= len {
                                    return Err(self.mortal(line, format!("there is no element {i} in a multitude of {len}")));
                                }
                                items.borrow_mut()[i as usize] = v;
                            }
                            Value::Map(map) => {
                                let Some(k) = key.to_key() else { return Err(self.bug(line, "bad map key")) };
                                map.borrow_mut().insert(k, v);
                            }
                            _ => return Err(self.bug(line, "assignment into a value that holds nothing")),
                        }
                    }
                    ExprKind::Field(base, field) => {
                        let Value::Record(record) = self.eval(base)? else {
                            return Err(self.bug(line, "field assignment on a value without fields"));
                        };
                        let mut fields = record.fields.borrow_mut();
                        if let Some(slot) = fields.iter_mut().find(|(f, _)| f == field) {
                            slot.1 = v;
                        }
                        drop(fields);
                    }
                    _ => return Err(self.bug(line, "bad assignment target")),
                }
            }
            StmtKind::Preach(e) => {
                let v = self.eval(e)?;
                let _ = writeln!(std::io::stdout(), "{}", show(&v, false));
            }
            StmtKind::Expr(e) => {
                self.eval(e)?;
            }
            StmtKind::Give(value) => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Nothing,
                };
                return Err(Flow::Give(v));
            }
            StmtKind::Sin(e) => {
                let message = self.eval(e)?;
                return Err(self.sin(line, show(&message, false)));
            }
            StmtKind::If { cond, then, otherwise } => {
                if matches!(self.eval(cond)?, Value::Truth(true)) {
                    self.block(then)?;
                } else if let Some(other) = otherwise {
                    self.block(other)?;
                }
            }
            StmtKind::While { cond, body } => {
                while matches!(self.eval(cond)?, Value::Truth(true)) {
                    if !self.turn(body)? {
                        break;
                    }
                }
            }
            StmtKind::Repeat { count, body } => {
                let n = as_number(&self.eval(count)?);
                for _ in 0..n.max(0) {
                    if !self.turn(body)? {
                        break;
                    }
                }
            }
            StmtKind::ForEach { first, second, iter, body } => {
                // The walk is over a snapshot; the checker forbids changing
                // the original while the walk goes on.
                let pairs: Vec<(Value, Value)> = match self.eval(iter)? {
                    Value::List(items) => {
                        items.borrow().iter().enumerate().map(|(i, x)| (Value::Number(i as i64), x.clone())).collect()
                    }
                    Value::Map(map) => map.borrow().iter().map(|(k, x)| (k.to_value(), x.clone())).collect(),
                    _ => return Err(self.bug(line, "walking a value that is not a multitude")),
                };
                for (a, b) in pairs {
                    let start = self.locals.len();
                    match second {
                        Some(second) => {
                            self.locals.push((first.clone(), a));
                            self.locals.push((second.clone(), b));
                        }
                        None => self.locals.push((first.clone(), b)),
                    }
                    let go_on = self.turn(body);
                    self.locals.truncate(start);
                    if !go_on? {
                        break;
                    }
                }
            }
            StmtKind::Break => return Err(Flow::Break),
            StmtKind::Continue => return Err(Flow::Continue),
            StmtKind::Tempt { body, name, handler } => match self.block(body) {
                Err(Flow::Sin(info)) => {
                    self.forgiven += 1;
                    if self.forgiven > FORGIVENESS {
                        let message = format!("Matthew 18:22: forgive can only be run {FORGIVENESS} times (unforgiven: {})", info.message);
                        return Err(Flow::Mortal(SinInfo { message, ..info }));
                    }
                    let start = self.locals.len();
                    self.locals.push((name.clone(), Value::text(&info.message)));
                    let result = self.block(handler);
                    self.locals.truncate(start);
                    result?;
                }
                other => other?,
            },
            StmtKind::Discern { subject, arms } => {
                let v = self.eval(subject)?;
                let (tag, fields): (&str, Vec<Value>) = match &v {
                    Value::Truth(true) => ("Jesus", vec![]),
                    Value::Truth(false) => ("False", vec![]),
                    Value::Variant(var) => (var.tag.as_str(), var.fields.clone()),
                    _ => return Err(self.bug(line, "discerning a value without cases")),
                };
                let Some(arm) = arms.iter().find(|a| a.pattern.name == tag) else {
                    return Err(self.bug(line, "no case matched"));
                };
                let start = self.locals.len();
                for (bind, field) in arm.pattern.binds.iter().zip(fields) {
                    self.locals.push((bind.clone(), field));
                }
                let result = self.block(&arm.body);
                self.locals.truncate(start);
                result?;
            }
        }
        Ok(())
    }

    fn eval(&mut self, e: &Expr) -> R<Value> {
        let line = e.line;
        Ok(match &e.kind {
            ExprKind::Number(n) => Value::Number(*n),
            ExprKind::Decimal(d) => Value::Decimal(*d),
            ExprKind::Jesus => Value::Truth(true),
            ExprKind::False => Value::Truth(false),
            ExprKind::Text(parts) => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        TextPart::Lit(s) => out.push_str(s),
                        TextPart::Code(inner) => out.push_str(&show(&self.eval(inner)?, false)),
                    }
                }
                Value::text(out)
            }
            ExprKind::Var(name) => match self.lookup(name) {
                Some(v) => v.clone(),
                None if name == "Lost" => Value::lost(),
                None if name.chars().next().is_some_and(|c| c.is_uppercase()) && !self.parables.contains_key(name) => {
                    // A case that carries nothing, or a law not yet reached.
                    let is_case = self.kinds.values().any(|k| matches!(k, KindDef::Choice(vs) if vs.iter().any(|(n, _)| n == name)));
                    if is_case {
                        Value::Variant(Rc::new(VariantVal { tag: name.clone(), fields: vec![] }))
                    } else {
                        return Err(self.mortal(line, format!("the law `{name}` is used before it is written")));
                    }
                }
                None => return Err(self.mortal(line, format!("`{name}` is used before it is written"))),
            },
            ExprKind::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.eval(item)?);
                }
                Value::list(out)
            }
            ExprKind::Map(pairs) => {
                let mut out = BTreeMap::new();
                for (k, v) in pairs {
                    let Some(key) = self.eval(k)?.to_key() else { return Err(self.bug(line, "bad map key")) };
                    let value = self.eval(v)?;
                    out.insert(key, value);
                }
                Value::Map(Rc::new(RefCell::new(out)))
            }
            ExprKind::Neg(inner) => match self.eval(inner)? {
                Value::Number(n) => Value::Number(n.checked_neg().ok_or_else(|| self.mortal(line, "the number grew too great to hold"))?),
                Value::Decimal(d) => Value::Decimal(-d),
                _ => return Err(self.bug(line, "negating a value that is not a number")),
            },
            ExprKind::Deny(inner) => Value::Truth(!matches!(self.eval(inner)?, Value::Truth(true))),
            ExprKind::Binary(op, l, r) => self.binary(line, *op, l, r)?,
            ExprKind::Call(callee, args) => self.call(line, callee, args)?,
            ExprKind::Index(base, index) => {
                let container = self.eval(base)?;
                let key = self.eval(index)?;
                match container {
                    Value::List(items) => {
                        let i = as_number(&key);
                        let items = items.borrow();
                        if i < 0 || i as usize >= items.len() {
                            return Err(self.mortal(line, format!("there is no element {i} in a multitude of {}", items.len())));
                        }
                        items[i as usize].clone()
                    }
                    Value::Map(map) => {
                        let found = key.to_key().and_then(|k| map.borrow().get(&k).cloned());
                        match found {
                            Some(v) => v,
                            None => {
                                return Err(self.mortal(line, format!("the map has no key {}", show(&key, true))));
                            }
                        }
                    }
                    _ => return Err(self.bug(line, "indexing a value that holds nothing")),
                }
            }
            ExprKind::Field(base, field) => match self.eval(base)? {
                Value::Record(record) => {
                    let found = record.fields.borrow().iter().find(|(f, _)| f == field).map(|(_, v)| v.clone());
                    match found {
                        Some(v) => v,
                        None => return Err(self.bug(line, "missing field")),
                    }
                }
                Value::Pagan(g) => {
                    let member = self.foreign.member(&g, field);
                    Value::Pagan(member.map_err(|m| self.sin(line, m))?)
                }
                _ => return Err(self.bug(line, "field of a value without fields")),
            },
            ExprKind::Baptize(inner, te) => {
                let Value::Pagan(g) = self.eval(inner)? else { return Err(self.bug(line, "baptizing a value of Gesù")) };
                let kinds = &self.kinds;
                let ty = resolve(te, &|name| kinds.get(name).map(|k| matches!(k, KindDef::Record(_))))
                    .map_err(|_| self.bug(line, "unknown type"))?;
                let data = self.foreign.data(&g).map_err(|m| self.sin(line, m))?;
                pagan::baptize(&data, &ty, &self.kinds).map_err(|m| self.sin(line, m))?
            }
        })
    }

    fn binary(&mut self, line: usize, op: BinOp, l: &Expr, r: &Expr) -> R<Value> {
        use BinOp::*;
        if op == And || op == Or {
            let left = matches!(self.eval(l)?, Value::Truth(true));
            if left != (op == And) {
                return Ok(Value::Truth(left));
            }
            return Ok(Value::Truth(matches!(self.eval(r)?, Value::Truth(true))));
        }
        let a = self.eval(l)?;
        let b = self.eval(r)?;
        Ok(match op {
            Is => Value::Truth(equal(&a, &b)),
            IsNot => Value::Truth(!equal(&a, &b)),
            Lt => Value::Truth(order(&a, &b) == Ordering::Less),
            Gt => Value::Truth(order(&a, &b) == Ordering::Greater),
            Le => Value::Truth(order(&a, &b) != Ordering::Greater),
            Ge => Value::Truth(order(&a, &b) != Ordering::Less),
            _ => match (&a, &b) {
                (Value::Number(x), Value::Number(y)) => {
                    if matches!(op, Div | Rem) && *y == 0 {
                        return Err(self.mortal(line, "divided by nothing"));
                    }
                    let result = match op {
                        Add => x.checked_add(*y),
                        Sub => x.checked_sub(*y),
                        Mul => x.checked_mul(*y),
                        Div => x.checked_div(*y),
                        _ => x.checked_rem(*y),
                    };
                    Value::Number(result.ok_or_else(|| self.mortal(line, "the number grew too great to hold"))?)
                }
                (Value::Decimal(x), Value::Decimal(y)) => {
                    if matches!(op, Div | Rem) && *y == 0.0 {
                        return Err(self.mortal(line, "divided by nothing"));
                    }
                    Value::Decimal(match op {
                        Add => x + y,
                        Sub => x - y,
                        Mul => x * y,
                        Div => x / y,
                        _ => x % y,
                    })
                }
                (Value::Text(x), Value::Text(y)) => Value::text(format!("{x}{y}")),
                _ => return Err(self.bug(line, "arithmetic on values that are not numbers")),
            },
        })
    }

    fn args(&mut self, args: &[Arg]) -> R<Vec<Value>> {
        let mut out = Vec::with_capacity(args.len());
        for a in args {
            out.push(self.eval(&a.expr)?);
        }
        Ok(out)
    }

    fn call(&mut self, line: usize, callee: &Expr, args: &[Arg]) -> R<Value> {
        if let ExprKind::Var(name) = &callee.kind {
            if self.lookup(name).is_none() {
                if let Some(p) = self.parables.get(name).cloned() {
                    let values = self.args(args)?;
                    return self.call_parable(&p, values, line);
                }
                if let Some(KindDef::Record(fields)) = self.kinds.get(name) {
                    let order: Vec<String> = fields.iter().map(|(f, _)| f.clone()).collect();
                    let values = self.args(args)?;
                    let mut out = Vec::new();
                    for field in order {
                        let at = args.iter().position(|a| a.name.as_deref() == Some(field.as_str()));
                        let Some(at) = at else { return Err(self.bug(line, "missing field")) };
                        out.push((field, values[at].clone()));
                    }
                    return Ok(Value::Record(Rc::new(RecordVal { kind: name.clone(), fields: RefCell::new(out) })));
                }
                let values = self.args(args)?;
                if name == "Found" {
                    return Ok(Value::found(values.into_iter().next().unwrap_or(Value::Nothing)));
                }
                if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                    return Ok(Value::Variant(Rc::new(VariantVal { tag: name.clone(), fields: values })));
                }
                return self.builtin(line, name, values);
            }
        }
        let Value::Pagan(target) = self.eval(callee)? else { return Err(self.bug(line, "calling a value")) };
        let values = self.args(args)?;
        let result = self.foreign.call(&target, &values);
        Ok(Value::Pagan(result.map_err(|m| self.sin(line, m))?))
    }

    fn call_parable(&mut self, p: &Rc<Parable>, values: Vec<Value>, line: usize) -> R<Value> {
        if self.calls.len() >= DEPTH {
            return Err(self.mortal(line, format!("the tower fell: parables called {DEPTH} deep")));
        }
        self.calls.push((p.name.clone(), line));
        let saved_base = self.base;
        let start = self.locals.len();
        self.base = start;
        for (param, v) in p.params.iter().zip(values) {
            self.locals.push((param.name.clone(), v));
        }
        let result = self.block(&p.body);
        self.locals.truncate(start);
        self.base = saved_base;
        self.calls.pop();
        match result {
            Ok(()) => Ok(Value::Nothing),
            Err(Flow::Give(v)) => Ok(v),
            Err(other) => Err(other),
        }
    }

    fn builtin(&mut self, line: usize, name: &str, a: Vec<Value>) -> R<Value> {
        let arg = |i: usize| a.get(i).cloned().unwrap_or(Value::Nothing);
        Ok(match name {
            "length" => Value::Number(match arg(0) {
                Value::Text(s) => s.chars().count() as i64,
                Value::List(items) => items.borrow().len() as i64,
                Value::Map(map) => map.borrow().len() as i64,
                _ => 0,
            }),
            "push" => {
                if let Value::List(items) = arg(0) {
                    items.borrow_mut().push(arg(1));
                }
                Value::Nothing
            }
            "pop" => match arg(0) {
                Value::List(items) => Value::maybe(items.borrow_mut().pop()),
                _ => Value::lost(),
            },
            "get" => match arg(0) {
                Value::List(items) => {
                    let i = as_number(&arg(1));
                    let items = items.borrow();
                    Value::maybe(if i < 0 { None } else { items.get(i as usize).map(deep_copy) })
                }
                _ => Value::lost(),
            },
            "lookup" => match (arg(0), arg(1).to_key()) {
                (Value::Map(map), Some(key)) => Value::maybe(map.borrow().get(&key).map(deep_copy)),
                _ => Value::lost(),
            },
            "contains" => Value::Truth(match arg(0) {
                Value::List(items) => items.borrow().iter().any(|x| equal(x, &arg(1))),
                Value::Map(map) => arg(1).to_key().is_some_and(|k| map.borrow().contains_key(&k)),
                Value::Text(s) => s.contains(&*as_text(&arg(1))),
                _ => false,
            }),
            "remove" => match arg(0) {
                Value::Map(map) => Value::maybe(arg(1).to_key().and_then(|k| map.borrow_mut().remove(&k))),
                Value::List(items) => {
                    let i = as_number(&arg(1));
                    let len = items.borrow().len();
                    if i < 0 || i as usize >= len {
                        return Err(self.mortal(line, format!("there is no element {i} in a multitude of {len}")));
                    }
                    items.borrow_mut().remove(i as usize)
                }
                _ => Value::lost(),
            },
            "keys" => match arg(0) {
                Value::Map(map) => Value::list(map.borrow().keys().map(|k| k.to_value()).collect()),
                _ => Value::list(vec![]),
            },
            "copy" => deep_copy(&arg(0)),
            "range" => {
                let (from, to) = (as_number(&arg(0)), as_number(&arg(1)));
                if to.saturating_sub(from) > 50_000_000 {
                    return Err(self.mortal(line, "this range is too great to hold"));
                }
                Value::list((from..to).map(Value::Number).collect())
            }
            "sort" => {
                if let Value::List(items) = arg(0) {
                    items.borrow_mut().sort_by(order);
                }
                Value::Nothing
            }
            "reverse" => {
                if let Value::List(items) = arg(0) {
                    items.borrow_mut().reverse();
                }
                Value::Nothing
            }
            "split" => {
                let (text, sep) = (as_text(&arg(0)), as_text(&arg(1)));
                if sep.is_empty() {
                    Value::list(text.chars().map(|c| Value::text(c.to_string())).collect())
                } else {
                    Value::list(text.split(&*sep).map(Value::text).collect())
                }
            }
            "join" => match arg(0) {
                Value::List(items) => {
                    let parts: Vec<String> = items.borrow().iter().map(|x| show(x, false)).collect();
                    Value::text(parts.join(&as_text(&arg(1))))
                }
                _ => Value::text(""),
            },
            "trim" => Value::text(as_text(&arg(0)).trim()),
            "upper" => Value::text(as_text(&arg(0)).to_uppercase()),
            "lower" => Value::text(as_text(&arg(0)).to_lowercase()),
            "replace" => Value::text(as_text(&arg(0)).replace(&*as_text(&arg(1)), &as_text(&arg(2)))),
            "chars" => Value::list(as_text(&arg(0)).chars().map(|c| Value::text(c.to_string())).collect()),
            "text" => Value::text(show(&arg(0), false)),
            "number" => {
                let text = as_text(&arg(0));
                match text.trim().parse::<i64>() {
                    Ok(n) => Value::Number(n),
                    Err(_) => return Err(self.sin(line, format!("\"{text}\" is not a Number"))),
                }
            }
            "decimal" => match arg(0) {
                Value::Number(n) => Value::Decimal(n as f64),
                other => {
                    let text = as_text(&other);
                    match text.trim().parse::<f64>() {
                        Ok(d) => Value::Decimal(d),
                        Err(_) => return Err(self.sin(line, format!("\"{text}\" is not a Decimal"))),
                    }
                }
            },
            "floor" | "round" => {
                let Value::Decimal(d) = arg(0) else { return Err(self.bug(line, "rounding a value that is not a Decimal")) };
                let rounded = if name == "floor" { d.floor() } else { d.round() };
                if !rounded.is_finite() || rounded.abs() >= 9.2e18 {
                    return Err(self.mortal(line, "the number grew too great to hold"));
                }
                Value::Number(rounded as i64)
            }
            "abs" => match arg(0) {
                Value::Number(n) => Value::Number(n.checked_abs().ok_or_else(|| self.mortal(line, "the number grew too great to hold"))?),
                Value::Decimal(d) => Value::Decimal(d.abs()),
                other => other,
            },
            "min" => {
                if order(&arg(0), &arg(1)) == Ordering::Greater { arg(1) } else { arg(0) }
            }
            "max" => {
                if order(&arg(0), &arg(1)) == Ordering::Less { arg(1) } else { arg(0) }
            }
            "read_file" => {
                let path = as_text(&arg(0));
                match std::fs::read_to_string(&*path) {
                    Ok(text) => Value::text(text),
                    Err(e) => return Err(self.sin(line, format!("cannot read \"{path}\": {e}"))),
                }
            }
            "write_file" => {
                let path = as_text(&arg(0));
                if let Err(e) = std::fs::write(&*path, &*as_text(&arg(1))) {
                    return Err(self.sin(line, format!("cannot write \"{path}\": {e}")));
                }
                Value::Nothing
            }
            "read_line" => {
                let mut input = String::new();
                match std::io::stdin().read_line(&mut input) {
                    Ok(n) if n > 0 => Value::found(Value::text(input.trim_end_matches(['\n', '\r']))),
                    _ => Value::lost(),
                }
            }
            "args" => Value::list(self.args.iter().map(Value::text).collect()),
            "parse_json" => match pagan::parse_json(&as_text(&arg(0))) {
                Ok(data) => Value::Pagan(pagan::Pagan::Data(Rc::new(data))),
                Err(e) => return Err(self.sin(line, format!("not JSON: {e}"))),
            },
            "to_json" => {
                let data = pagan::to_data(&arg(0)).map_err(|m| self.mortal(line, m))?;
                let mut out = String::new();
                pagan::write_json(&data, &mut out);
                Value::text(out)
            }
            _ => return Err(self.bug(line, "unknown built-in parable")),
        })
    }
}
