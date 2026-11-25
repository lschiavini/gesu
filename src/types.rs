//! Resolved types, shared by the checker and the runtime.

use crate::ast::TypeExpr;
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Number,
    Decimal,
    Text,
    Truth,
    Nothing,
    Pagan,
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Maybe(Box<Type>),
    Record(String),
    Choice(String),
    /// Not yet known, as in the element type of `[]`.
    Unknown,
    /// Already reported; accepts anything so one mistake gives one error.
    Error,
}

#[derive(Clone, Debug)]
pub enum KindDef {
    Record(Vec<(String, Type)>),
    Choice(Vec<(String, Vec<Type>)>),
}

pub type Kinds = HashMap<String, KindDef>;

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Number => write!(f, "Number"),
            Type::Decimal => write!(f, "Decimal"),
            Type::Text => write!(f, "Text"),
            Type::Truth => write!(f, "Truth"),
            Type::Nothing => write!(f, "Nothing"),
            Type::Pagan => write!(f, "Pagan"),
            Type::List(t) => write!(f, "[{t}]"),
            Type::Map(k, v) => write!(f, "{{{k}: {v}}}"),
            Type::Maybe(t) => write!(f, "Maybe {t}"),
            Type::Record(n) | Type::Choice(n) => write!(f, "{n}"),
            Type::Unknown => write!(f, "?"),
            Type::Error => write!(f, "?"),
        }
    }
}

/// Does a value of type `actual` fit where `expected` is wanted?
pub fn fits(actual: &Type, expected: &Type) -> bool {
    use Type::*;
    match (actual, expected) {
        (Error, _) | (_, Error) | (Unknown, _) | (_, Unknown) => true,
        (List(a), List(e)) => fits(a, e),
        (Map(ak, av), Map(ek, ev)) => fits(ak, ek) && fits(av, ev),
        (Maybe(a), Maybe(e)) => fits(a, e),
        _ => actual == expected,
    }
}

pub fn has_unknown(t: &Type) -> bool {
    match t {
        Type::Unknown => true,
        Type::List(t) | Type::Maybe(t) => has_unknown(t),
        Type::Map(k, v) => has_unknown(k) || has_unknown(v),
        _ => false,
    }
}

/// Turns written type syntax into a type. `is_record` answers, for a kind
/// name, whether it is a record (`Some(true)`) or a choice (`Some(false)`).
pub fn resolve(te: &TypeExpr, is_record: &dyn Fn(&str) -> Option<bool>) -> Result<Type, (usize, String)> {
    Ok(match te {
        TypeExpr::Named(name, line) => match name.as_str() {
            "Number" => Type::Number,
            "Decimal" => Type::Decimal,
            "Text" => Type::Text,
            "Truth" => Type::Truth,
            "Pagan" => Type::Pagan,
            "Nothing" => Type::Nothing,
            other => match is_record(other) {
                Some(true) => Type::Record(other.to_string()),
                Some(false) => Type::Choice(other.to_string()),
                None => return Err((*line, other.to_string())),
            },
        },
        TypeExpr::List(t) => Type::List(Box::new(resolve(t, is_record)?)),
        TypeExpr::Map(k, v) => Type::Map(Box::new(resolve(k, is_record)?), Box::new(resolve(v, is_record)?)),
        TypeExpr::Maybe(t) => Type::Maybe(Box::new(resolve(t, is_record)?)),
    })
}
