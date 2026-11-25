//! The border with the pagans: Lua, Python and JSON. Values cross as
//! copies. Nothing foreign becomes a value of Gesù until it is baptized.

use crate::types::{KindDef, Kinds, Type};
use crate::value::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

/// Plain data, the common shape of everything that crosses the border.
#[derive(Clone, Debug)]
pub enum Data {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Data>),
    Map(Vec<(String, Data)>),
    /// An empty Lua table: it may be a list or a map.
    Empty,
    /// A foreign thing with no data shape, such as a function.
    Opaque(String),
}

#[derive(Clone)]
pub enum Pagan {
    Data(Rc<Data>),
    Lua(mlua::Value),
    #[cfg(feature = "python")]
    Py(Rc<pyo3::Py<pyo3::PyAny>>),
}

pub struct Foreign {
    lua: Option<mlua::Lua>,
    pub dir: PathBuf,
}

fn describe(d: &Data) -> String {
    match d {
        Data::Null => "nothing".into(),
        Data::Bool(_) => "a Truth".into(),
        Data::Int(i) => format!("the Number {i}"),
        Data::Float(f) => format!("the Decimal {}", show_decimal(*f)),
        Data::Str(s) => format!("the Text \"{s}\""),
        Data::List(_) => "a list".into(),
        Data::Map(_) => "a map".into(),
        Data::Empty => "an empty table".into(),
        Data::Opaque(what) => format!("a foreign {what}"),
    }
}

/// Converts foreign data into a value of the claimed type, or says why not.
pub fn baptize(d: &Data, ty: &Type, kinds: &Kinds) -> Result<Value, String> {
    let refuse = || Err(format!("cannot baptize as {ty}: the pagan gave {}", describe(d)));
    match (ty, d) {
        (Type::Pagan, _) => Ok(Value::Pagan(Pagan::Data(Rc::new(d.clone())))),
        (Type::Maybe(_), Data::Null) => Ok(Value::lost()),
        (Type::Maybe(inner), _) => Ok(Value::found(baptize(d, inner, kinds)?)),
        (Type::Number, Data::Int(i)) => Ok(Value::Number(*i)),
        (Type::Number, Data::Float(f)) if f.fract() == 0.0 && f.abs() < 9.0e15 => Ok(Value::Number(*f as i64)),
        (Type::Decimal, Data::Float(f)) => Ok(Value::Decimal(*f)),
        (Type::Decimal, Data::Int(i)) => Ok(Value::Decimal(*i as f64)),
        (Type::Text, Data::Str(s)) => Ok(Value::text(s)),
        (Type::Truth, Data::Bool(b)) => Ok(Value::Truth(*b)),
        (Type::List(_), Data::Empty) => Ok(Value::list(vec![])),
        (Type::List(elem), Data::List(items)) => {
            let mut out = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                out.push(baptize(item, elem, kinds).map_err(|e| format!("element {i}: {e}"))?);
            }
            Ok(Value::list(out))
        }
        (Type::Map(..), Data::Empty) => Ok(Value::Map(Rc::new(RefCell::new(BTreeMap::new())))),
        (Type::Map(key, value), Data::Map(pairs)) => {
            let mut out = BTreeMap::new();
            for (k, v) in pairs {
                let k_val = match **key {
                    Type::Number => Key::Number(k.parse().map_err(|_| format!("the key \"{k}\" is not a Number"))?),
                    _ => Key::Text(Rc::from(k.as_str())),
                };
                out.insert(k_val, baptize(v, value, kinds).map_err(|e| format!("key \"{k}\": {e}"))?);
            }
            Ok(Value::Map(Rc::new(RefCell::new(out))))
        }
        (Type::Record(name), Data::Map(pairs)) => {
            let Some(KindDef::Record(fields)) = kinds.get(name) else { return refuse() };
            let mut out = Vec::new();
            for (field, field_type) in fields {
                let given = pairs.iter().find(|(k, _)| k == field).map(|(_, v)| v).unwrap_or(&Data::Null);
                out.push((field.clone(), baptize(given, field_type, kinds).map_err(|e| format!("field `{field}`: {e}"))?));
            }
            Ok(Value::Record(Rc::new(RecordVal { kind: name.clone(), fields: RefCell::new(out) })))
        }
        _ => refuse(),
    }
}

/// Turns a value of Gesù into plain data, to hand to foreign code.
pub fn to_data(v: &Value) -> Result<Data, String> {
    Ok(match v {
        Value::Number(n) => Data::Int(*n),
        Value::Decimal(d) => Data::Float(*d),
        Value::Text(s) => Data::Str(s.to_string()),
        Value::Truth(b) => Data::Bool(*b),
        Value::Nothing => Data::Null,
        Value::List(items) => Data::List(items.borrow().iter().map(to_data).collect::<Result<_, _>>()?),
        Value::Map(map) => {
            let mut out = Vec::new();
            for (k, x) in map.borrow().iter() {
                out.push((show(&k.to_value(), false), to_data(x)?));
            }
            Data::Map(out)
        }
        Value::Record(r) => {
            let mut out = Vec::new();
            for (f, x) in r.fields.borrow().iter() {
                out.push((f.clone(), to_data(x)?));
            }
            Data::Map(out)
        }
        Value::Variant(var) => match (var.tag.as_str(), var.fields.as_slice()) {
            ("Found", [inner]) => to_data(inner)?,
            ("Lost", []) => Data::Null,
            (tag, []) => Data::Str(tag.to_string()),
            (tag, fields) => Data::Map(vec![(tag.to_string(), Data::List(fields.iter().map(to_data).collect::<Result<_, _>>()?))]),
        },
        Value::Pagan(Pagan::Data(d)) => (**d).clone(),
        Value::Pagan(_) => return Err("a foreign handle cannot be placed inside a value sent abroad".into()),
    })
}

impl Foreign {
    pub fn new(dir: PathBuf) -> Self {
        Foreign { lua: None, dir }
    }

    fn lua(&mut self) -> Result<&mlua::Lua, String> {
        if self.lua.is_none() {
            let lua = mlua::Lua::new();
            let dir = self.dir.to_string_lossy().to_string();
            let set_path = || -> mlua::Result<()> {
                let package: mlua::Table = lua.globals().get("package")?;
                let old: String = package.get("path")?;
                package.set("path", format!("{dir}/?.lua;{old}"))
            };
            set_path().map_err(|e| lua_error(&e))?;
            self.lua = Some(lua);
        }
        Ok(self.lua.as_ref().expect("lua"))
    }

    pub fn import(&mut self, lang: &str, path: &str) -> Result<Pagan, String> {
        match lang {
            "lua" => {
                let file = self.dir.join(path);
                let src = std::fs::read_to_string(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
                let lua = self.lua()?;
                let value: mlua::Value = lua.load(&src).set_name(format!("@{path}")).eval().map_err(|e| lua_error(&e))?;
                Ok(Pagan::Lua(match value {
                    // A file that returns nothing shares its globals.
                    mlua::Value::Nil => mlua::Value::Table(lua.globals()),
                    other => other,
                }))
            }
            "python" => self.import_python(path),
            other => Err(format!("unknown pagan tongue `{other}`")),
        }
    }

    #[cfg(feature = "python")]
    fn import_python(&mut self, name: &str) -> Result<Pagan, String> {
        python::import(&self.dir.to_string_lossy(), name).map(|m| Pagan::Py(Rc::new(m)))
    }

    #[cfg(not(feature = "python"))]
    fn import_python(&mut self, _name: &str) -> Result<Pagan, String> {
        Err("this gesu was built without Python; rebuild it with `cargo build --release --features python`".into())
    }

    pub fn member(&mut self, g: &Pagan, name: &str) -> Result<Pagan, String> {
        match g {
            Pagan::Data(d) => match &**d {
                Data::Map(pairs) => match pairs.iter().find(|(k, _)| k == name) {
                    Some((_, v)) => Ok(Pagan::Data(Rc::new(v.clone()))),
                    None => Err(format!("the pagan has no `{name}`")),
                },
                other => Err(format!("cannot look for `{name}` inside {}", describe(other))),
            },
            Pagan::Lua(mlua::Value::Table(table)) => {
                let v: mlua::Value = table.get(name).map_err(|e| lua_error(&e))?;
                match v {
                    mlua::Value::Nil => Err(format!("the lua pagan has no `{name}`")),
                    other => Ok(Pagan::Lua(other)),
                }
            }
            Pagan::Lua(other) => Err(format!("cannot look for `{name}` inside a lua {}", other.type_name())),
            #[cfg(feature = "python")]
            Pagan::Py(obj) => python::member(obj, name).map(|m| Pagan::Py(Rc::new(m))),
        }
    }

    pub fn call(&mut self, g: &Pagan, args: &[Value]) -> Result<Pagan, String> {
        match g {
            Pagan::Data(d) => Err(format!("{} cannot be called", describe(d))),
            Pagan::Lua(mlua::Value::Function(f)) => {
                let lua = self.lua()?;
                let mut lua_args = Vec::new();
                for a in args {
                    lua_args.push(match a {
                        Value::Pagan(Pagan::Lua(v)) => v.clone(),
                        other => data_to_lua(lua, &to_data(other)?).map_err(|e| lua_error(&e))?,
                    });
                }
                let multi: mlua::MultiValue = lua_args.into_iter().collect();
                let result: mlua::Value = f.call(multi).map_err(|e| lua_error(&e))?;
                Ok(Pagan::Lua(result))
            }
            Pagan::Lua(other) => Err(format!("a lua {} cannot be called", other.type_name())),
            #[cfg(feature = "python")]
            Pagan::Py(obj) => python::call(obj, args).map(|r| Pagan::Py(Rc::new(r))),
        }
    }

    /// Reads a pagan as plain data, ready to be baptized.
    pub fn data(&self, g: &Pagan) -> Result<Data, String> {
        match g {
            Pagan::Data(d) => Ok((**d).clone()),
            Pagan::Lua(v) => lua_to_data(v, 0),
            #[cfg(feature = "python")]
            Pagan::Py(obj) => python::data(obj),
        }
    }
}

// ---- Lua ----

fn lua_error(e: &mlua::Error) -> String {
    let text = e.to_string();
    let first = text.lines().next().unwrap_or("lua failed");
    first.trim_start_matches("runtime error: ").trim_start_matches("syntax error: ").to_string()
}

fn data_to_lua(lua: &mlua::Lua, d: &Data) -> mlua::Result<mlua::Value> {
    Ok(match d {
        Data::Null => mlua::Value::Nil,
        Data::Bool(b) => mlua::Value::Boolean(*b),
        Data::Int(i) => mlua::Value::Integer(*i),
        Data::Float(f) => mlua::Value::Number(*f),
        Data::Str(s) => mlua::Value::String(lua.create_string(s)?),
        Data::List(items) => {
            let table = lua.create_table()?;
            for (i, item) in items.iter().enumerate() {
                table.set(i + 1, data_to_lua(lua, item)?)?;
            }
            mlua::Value::Table(table)
        }
        Data::Map(pairs) => {
            let table = lua.create_table()?;
            for (k, v) in pairs {
                table.set(k.as_str(), data_to_lua(lua, v)?)?;
            }
            mlua::Value::Table(table)
        }
        Data::Empty => mlua::Value::Table(lua.create_table()?),
        Data::Opaque(what) => return Err(mlua::Error::runtime(format!("cannot send a foreign {what}"))),
    })
}

fn lua_to_data(v: &mlua::Value, depth: u32) -> Result<Data, String> {
    if depth > 64 {
        return Err("the lua table is too deep, or contains itself".into());
    }
    Ok(match v {
        mlua::Value::Nil => Data::Null,
        mlua::Value::Boolean(b) => Data::Bool(*b),
        mlua::Value::Integer(i) => Data::Int(*i),
        mlua::Value::Number(f) => Data::Float(*f),
        mlua::Value::String(s) => Data::Str(s.to_string_lossy().to_string()),
        mlua::Value::Table(table) => {
            let len = table.raw_len();
            let mut pairs: Vec<(mlua::Value, mlua::Value)> = Vec::new();
            for pair in table.pairs::<mlua::Value, mlua::Value>() {
                pairs.push(pair.map_err(|e| lua_error(&e))?);
            }
            if pairs.is_empty() {
                Data::Empty
            } else if pairs.len() == len && pairs.iter().all(|(k, _)| matches!(k, mlua::Value::Integer(i) if *i >= 1 && *i as usize <= len)) {
                let mut items = vec![Data::Null; len];
                for (k, value) in &pairs {
                    if let mlua::Value::Integer(i) = k {
                        items[*i as usize - 1] = lua_to_data(value, depth + 1)?;
                    }
                }
                Data::List(items)
            } else {
                let mut out = Vec::new();
                for (k, value) in &pairs {
                    let key = match k {
                        mlua::Value::String(s) => s.to_string_lossy().to_string(),
                        mlua::Value::Integer(i) => i.to_string(),
                        other => return Err(format!("a lua table keyed by {} cannot be baptized", other.type_name())),
                    };
                    out.push((key, lua_to_data(value, depth + 1)?));
                }
                out.sort_by(|a, b| a.0.cmp(&b.0));
                Data::Map(out)
            }
        }
        other => Data::Opaque(other.type_name().to_string()),
    })
}

// ---- Python ----

#[cfg(feature = "python")]
mod python {
    use super::{to_data, Data, Pagan};
    use crate::value::Value;
    use pyo3::prelude::*;
    use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};

    fn text(e: PyErr) -> String {
        e.to_string()
    }

    /// Embedded Python looks for its standard library next to the
    /// executable. Point it at the Python this gesu was built with.
    fn find_home() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            if let Some(home) = option_env!("GESU_PYTHON_HOME") {
                if std::env::var_os("PYTHONHOME").is_none() && std::path::Path::new(home).is_dir() {
                    std::env::set_var("PYTHONHOME", home);
                }
            }
        });
    }

    pub fn import(dir: &str, name: &str) -> Result<Py<PyAny>, String> {
        find_home();
        Python::with_gil(|py| {
            let run = || -> PyResult<Py<PyAny>> {
                let sys = py.import("sys")?;
                let path = sys.getattr("path")?;
                if !path.contains(dir)? {
                    path.call_method1("insert", (0, dir))?;
                }
                Ok(py.import(name)?.into_any().unbind())
            };
            run().map_err(text)
        })
    }

    pub fn member(obj: &Py<PyAny>, name: &str) -> Result<Py<PyAny>, String> {
        Python::with_gil(|py| obj.bind(py).getattr(name).map(|m| m.unbind()).map_err(text))
    }

    pub fn call(obj: &Py<PyAny>, args: &[Value]) -> Result<Py<PyAny>, String> {
        Python::with_gil(|py| {
            let mut py_args = Vec::new();
            for a in args {
                py_args.push(match a {
                    Value::Pagan(Pagan::Py(handle)) => handle.bind(py).clone(),
                    other => to_py(py, &to_data(other)?).map_err(text)?,
                });
            }
            let tuple = PyTuple::new(py, py_args).map_err(text)?;
            obj.bind(py).call1(tuple).map(|r| r.unbind()).map_err(text)
        })
    }

    pub fn data(obj: &Py<PyAny>) -> Result<Data, String> {
        Python::with_gil(|py| from_py(obj.bind(py), 0))
    }

    fn to_py<'py>(py: Python<'py>, d: &Data) -> PyResult<Bound<'py, PyAny>> {
        Ok(match d {
            Data::Null => py.None().into_bound(py),
            Data::Bool(b) => PyBool::new(py, *b).to_owned().into_any(),
            Data::Int(i) => (*i).into_pyobject(py)?.into_any(),
            Data::Float(f) => PyFloat::new(py, *f).into_any(),
            Data::Str(s) => PyString::new(py, s).into_any(),
            Data::List(items) => {
                let list = PyList::empty(py);
                for item in items {
                    list.append(to_py(py, item)?)?;
                }
                list.into_any()
            }
            Data::Map(pairs) => {
                let dict = PyDict::new(py);
                for (k, v) in pairs {
                    dict.set_item(k, to_py(py, v)?)?;
                }
                dict.into_any()
            }
            Data::Empty => PyList::empty(py).into_any(),
            Data::Opaque(what) => {
                return Err(pyo3::exceptions::PyTypeError::new_err(format!("cannot send a foreign {what}")))
            }
        })
    }

    fn from_py(obj: &Bound<'_, PyAny>, depth: u32) -> Result<Data, String> {
        if depth > 64 {
            return Err("the python value is too deep, or contains itself".into());
        }
        if obj.is_none() {
            return Ok(Data::Null);
        }
        if let Ok(b) = obj.downcast::<PyBool>() {
            return Ok(Data::Bool(b.is_true()));
        }
        if obj.is_instance_of::<PyInt>() {
            return obj.extract::<i64>().map(Data::Int).map_err(|_| "the python int is too great for a Number".to_string());
        }
        if obj.is_instance_of::<PyFloat>() {
            return obj.extract::<f64>().map(Data::Float).map_err(text);
        }
        if obj.is_instance_of::<PyString>() {
            return obj.extract::<String>().map(Data::Str).map_err(text);
        }
        if let Ok(list) = obj.downcast::<PyList>() {
            let mut out = Vec::new();
            for item in list.iter() {
                out.push(from_py(&item, depth + 1)?);
            }
            return Ok(Data::List(out));
        }
        if let Ok(tuple) = obj.downcast::<PyTuple>() {
            let mut out = Vec::new();
            for item in tuple.iter() {
                out.push(from_py(&item, depth + 1)?);
            }
            return Ok(Data::List(out));
        }
        if let Ok(dict) = obj.downcast::<PyDict>() {
            let mut out = Vec::new();
            for (k, v) in dict.iter() {
                let key = k.str().map_err(text)?.to_string();
                out.push((key, from_py(&v, depth + 1)?));
            }
            return Ok(Data::Map(out));
        }
        let name = obj.get_type().name().map(|n| n.to_string()).unwrap_or_else(|_| "object".into());
        Ok(Data::Opaque(format!("python {name}")))
    }
}

// ---- JSON ----

pub fn parse_json(src: &str) -> Result<Data, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut pos = 0;
    let value = json_value(&chars, &mut pos, 0)?;
    json_space(&chars, &mut pos);
    if pos < chars.len() {
        return Err(format!("unexpected `{}` after the JSON value", chars[pos]));
    }
    Ok(value)
}

fn json_space(c: &[char], pos: &mut usize) {
    while *pos < c.len() && c[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn json_word(c: &[char], pos: &mut usize, word: &str, value: Data) -> Result<Data, String> {
    let end = *pos + word.len();
    if end <= c.len() && c[*pos..end].iter().collect::<String>() == word {
        *pos = end;
        Ok(value)
    } else {
        Err(format!("not JSON near position {}", *pos))
    }
}

fn json_string(c: &[char], pos: &mut usize) -> Result<String, String> {
    *pos += 1;
    let mut out = String::new();
    loop {
        let Some(&ch) = c.get(*pos) else { return Err("a JSON text never ends".into()) };
        *pos += 1;
        match ch {
            '"' => return Ok(out),
            '\\' => {
                let Some(&esc) = c.get(*pos) else { return Err("a JSON text never ends".into()) };
                *pos += 1;
                match esc {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'u' => {
                        let hex: String = c.iter().skip(*pos).take(4).collect();
                        *pos += 4;
                        let mut code = u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u escape in JSON".to_string())?;
                        // Characters beyond the basic plane arrive as two halves.
                        if (0xD800..0xDC00).contains(&code) && c.get(*pos) == Some(&'\\') && c.get(*pos + 1) == Some(&'u') {
                            let low: String = c.iter().skip(*pos + 2).take(4).collect();
                            if let Ok(low) = u32::from_str_radix(&low, 16) {
                                if (0xDC00..0xE000).contains(&low) {
                                    code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                    *pos += 6;
                                }
                            }
                        }
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    }
                    other => out.push(other),
                }
            }
            other => out.push(other),
        }
    }
}

fn json_value(c: &[char], pos: &mut usize, depth: u32) -> Result<Data, String> {
    if depth > 200 {
        return Err("the JSON is too deep".into());
    }
    json_space(c, pos);
    let Some(&ch) = c.get(*pos) else { return Err("the JSON is empty".into()) };
    match ch {
        'n' => json_word(c, pos, "null", Data::Null),
        't' => json_word(c, pos, "true", Data::Bool(true)),
        'f' => json_word(c, pos, "false", Data::Bool(false)),
        '"' => json_string(c, pos).map(Data::Str),
        '[' => {
            *pos += 1;
            let mut items = Vec::new();
            json_space(c, pos);
            if c.get(*pos) == Some(&']') {
                *pos += 1;
                return Ok(Data::List(items));
            }
            loop {
                items.push(json_value(c, pos, depth + 1)?);
                json_space(c, pos);
                match c.get(*pos) {
                    Some(',') => *pos += 1,
                    Some(']') => {
                        *pos += 1;
                        return Ok(Data::List(items));
                    }
                    _ => return Err("a JSON list is missing `,` or `]`".into()),
                }
            }
        }
        '{' => {
            *pos += 1;
            let mut pairs = Vec::new();
            json_space(c, pos);
            if c.get(*pos) == Some(&'}') {
                *pos += 1;
                return Ok(Data::Map(pairs));
            }
            loop {
                json_space(c, pos);
                if c.get(*pos) != Some(&'"') {
                    return Err("a JSON key must be a text".into());
                }
                let key = json_string(c, pos)?;
                json_space(c, pos);
                if c.get(*pos) != Some(&':') {
                    return Err("a JSON key is missing its `:`".into());
                }
                *pos += 1;
                pairs.push((key, json_value(c, pos, depth + 1)?));
                json_space(c, pos);
                match c.get(*pos) {
                    Some(',') => *pos += 1,
                    Some('}') => {
                        *pos += 1;
                        return Ok(Data::Map(pairs));
                    }
                    _ => return Err("a JSON object is missing `,` or `}`".into()),
                }
            }
        }
        '-' | '0'..='9' => {
            let start = *pos;
            while *pos < c.len() && matches!(c[*pos], '-' | '+' | '.' | 'e' | 'E' | '0'..='9') {
                *pos += 1;
            }
            let text: String = c[start..*pos].iter().collect();
            if let Ok(i) = text.parse::<i64>() {
                Ok(Data::Int(i))
            } else {
                text.parse::<f64>().map(Data::Float).map_err(|_| format!("`{text}` is not a JSON number"))
            }
        }
        other => Err(format!("unexpected `{other}` in JSON")),
    }
}

pub fn write_json(d: &Data, out: &mut String) {
    match d {
        Data::Null | Data::Opaque(_) => out.push_str("null"),
        Data::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Data::Int(i) => out.push_str(&i.to_string()),
        Data::Float(f) if f.is_finite() => out.push_str(&show_decimal(*f)),
        Data::Float(_) => out.push_str("null"),
        Data::Str(s) => write_json_string(s, out),
        Data::Empty => out.push_str("[]"),
        Data::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(item, out);
            }
            out.push(']');
        }
        Data::Map(pairs) => {
            out.push('{');
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json_string(k, out);
                out.push(':');
                write_json(v, out);
            }
            out.push('}');
        }
    }
}

fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
