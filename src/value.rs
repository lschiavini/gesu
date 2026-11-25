//! Values at run time.

use crate::pagan::Pagan;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    Number(i64),
    Text(Rc<str>),
}

pub struct RecordVal {
    pub kind: String,
    pub fields: RefCell<Vec<(String, Value)>>,
}

pub struct VariantVal {
    pub tag: String,
    pub fields: Vec<Value>,
}

/// Multitudes, maps and records are shared by reference. The checker has
/// already proved that only one owner, or its borrowers, can reach them.
#[derive(Clone)]
pub enum Value {
    Number(i64),
    Decimal(f64),
    Text(Rc<str>),
    Truth(bool),
    Nothing,
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<BTreeMap<Key, Value>>>),
    Record(Rc<RecordVal>),
    Variant(Rc<VariantVal>),
    Pagan(Pagan),
}

impl Value {
    pub fn text(s: impl AsRef<str>) -> Value {
        Value::Text(Rc::from(s.as_ref()))
    }

    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(RefCell::new(items)))
    }

    pub fn found(v: Value) -> Value {
        Value::Variant(Rc::new(VariantVal { tag: "Found".into(), fields: vec![v] }))
    }

    pub fn lost() -> Value {
        Value::Variant(Rc::new(VariantVal { tag: "Lost".into(), fields: vec![] }))
    }

    pub fn maybe(v: Option<Value>) -> Value {
        v.map(Value::found).unwrap_or_else(Value::lost)
    }

    pub fn to_key(&self) -> Option<Key> {
        match self {
            Value::Number(n) => Some(Key::Number(*n)),
            Value::Text(s) => Some(Key::Text(s.clone())),
            _ => None,
        }
    }
}

impl Key {
    pub fn to_value(&self) -> Value {
        match self {
            Key::Number(n) => Value::Number(*n),
            Key::Text(s) => Value::Text(s.clone()),
        }
    }
}

pub fn show_decimal(d: f64) -> String {
    if d.is_finite() && d.fract() == 0.0 && d.abs() < 1e15 {
        format!("{d:.1}")
    } else {
        format!("{d}")
    }
}

/// How a value reads when preached. Text inside a container is quoted.
pub fn show(v: &Value, nested: bool) -> String {
    match v {
        Value::Number(n) => n.to_string(),
        Value::Decimal(d) => show_decimal(*d),
        Value::Text(s) if nested => format!("\"{s}\""),
        Value::Text(s) => s.to_string(),
        Value::Truth(true) => "Jesus".into(),
        Value::Truth(false) => "False".into(),
        Value::Nothing => "nothing".into(),
        Value::List(items) => {
            let parts: Vec<String> = items.borrow().iter().map(|x| show(x, true)).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Map(map) => {
            let parts: Vec<String> =
                map.borrow().iter().map(|(k, x)| format!("{}: {}", show(&k.to_value(), true), show(x, true))).collect();
            format!("{{{}}}", parts.join(", "))
        }
        Value::Record(r) => {
            let parts: Vec<String> = r.fields.borrow().iter().map(|(f, x)| format!("{f}: {}", show(x, true))).collect();
            format!("{}({})", r.kind, parts.join(", "))
        }
        Value::Variant(var) => {
            if var.fields.is_empty() {
                var.tag.clone()
            } else {
                let parts: Vec<String> = var.fields.iter().map(|x| show(x, true)).collect();
                format!("{}({})", var.tag, parts.join(", "))
            }
        }
        Value::Pagan(_) => "<pagan>".into(),
    }
}

pub fn equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Decimal(x), Value::Decimal(y)) => x == y,
        (Value::Text(x), Value::Text(y)) => x == y,
        (Value::Truth(x), Value::Truth(y)) => x == y,
        (Value::Nothing, Value::Nothing) => true,
        (Value::List(x), Value::List(y)) => {
            let (x, y) = (x.borrow(), y.borrow());
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| equal(p, q))
        }
        (Value::Map(x), Value::Map(y)) => {
            let (x, y) = (x.borrow(), y.borrow());
            x.len() == y.len() && x.iter().zip(y.iter()).all(|((k1, v1), (k2, v2))| k1 == k2 && equal(v1, v2))
        }
        (Value::Record(x), Value::Record(y)) => {
            let (xf, yf) = (x.fields.borrow(), y.fields.borrow());
            x.kind == y.kind && xf.iter().zip(yf.iter()).all(|((_, p), (_, q))| equal(p, q))
        }
        (Value::Variant(x), Value::Variant(y)) => {
            x.tag == y.tag && x.fields.iter().zip(y.fields.iter()).all(|(p, q)| equal(p, q))
        }
        _ => false,
    }
}

/// A full copy that shares nothing with the original.
pub fn deep_copy(v: &Value) -> Value {
    match v {
        Value::List(items) => Value::list(items.borrow().iter().map(deep_copy).collect()),
        Value::Map(map) => {
            Value::Map(Rc::new(RefCell::new(map.borrow().iter().map(|(k, x)| (k.clone(), deep_copy(x))).collect())))
        }
        Value::Record(r) => Value::Record(Rc::new(RecordVal {
            kind: r.kind.clone(),
            fields: RefCell::new(r.fields.borrow().iter().map(|(f, x)| (f.clone(), deep_copy(x))).collect()),
        })),
        Value::Variant(var) => Value::Variant(Rc::new(VariantVal {
            tag: var.tag.clone(),
            fields: var.fields.iter().map(deep_copy).collect(),
        })),
        other => other.clone(),
    }
}
