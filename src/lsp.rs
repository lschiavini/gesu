//! The language server: `gesu lsp`. It speaks the Language Server Protocol
//! over stdin and stdout, and gives an editor the checker's errors as you
//! type, completion of the language's words, and a note on hover.

use crate::check::{Checker, BUILTINS};
use crate::diag::{Diag, Level};
use crate::lexer::{lex, KEYWORDS};
use crate::pagan::{parse_json, write_json, Data};
use crate::parser::Parser;
use std::collections::HashMap;
use std::io::{BufRead, Write};

const TYPES: &[&str] = &["Number", "Decimal", "Text", "Truth", "Maybe", "Pagan", "Nothing", "Found", "Lost"];

/// Everything the checker has to say about a source text.
pub fn analyze(src: &str) -> Vec<Diag> {
    let toks = match lex(src, 1) {
        Ok(toks) => toks,
        Err(d) => return vec![d],
    };
    let mut parser = Parser::new(toks);
    let stmts = match parser.program() {
        Ok(stmts) => stmts,
        Err(d) => {
            // A program still being written has no final `amen` yet. Examine
            // it as if it had one, so the other errors are not hidden.
            if d.title.starts_with("a program must end") {
                let mut diags = analyze(&format!("{src}\namen\n"));
                diags.push(d);
                return diags;
            }
            let mut diags = parser.warnings;
            diags.push(d);
            return diags;
        }
    };
    let mut checker = Checker::new();
    checker.program(&stmts);
    let mut diags = parser.warnings;
    diags.append(&mut checker.diags);
    diags.sort_by_key(|d| d.line);
    diags
}

/// What hovering over a word of the language says.
fn doc(word: &str) -> Option<&'static str> {
    Some(match word {
        "let" => "`let x = 5` declares a value. It cannot change unless declared `let mortal`.",
        "mortal" => "`let mortal x = 0` declares a value that can change.",
        "law" => "`law X = 12` declares a constant, visible in every parable. Changing it: *it is written*.",
        "parable" => "`parable name(a: Number) -> Number` declares a function. Add `may sin` if it can fail.",
        "give" => "`give x` returns a value from a parable.",
        "amen" => "Closes every block, and the program.",
        "if" | "otherwise" => "`if cond ... otherwise ... amen`. The condition must be a Truth: `Jesus` or `False`.",
        "while" => "`while cond ... amen` repeats while the condition is `Jesus`.",
        "repeat" | "times" => "`repeat 40 times ... amen` runs the block a fixed number of times.",
        "for" | "each" | "in" => "`for each x in list`, `for each i, x in list`, `for each key, value in map`.",
        "break" => "Leaves the nearest loop.",
        "continue" => "Skips to the next turn of the nearest loop.",
        "tempt" | "forgive" => "`tempt ... forgive trespass ... amen` runs a block and catches its sin. `trespass` is the sin's Text. Matthew 18:22: forgive can only be run 490 times.",
        "sin" => "`sin \"message\"` fails. The parable must be declared `may sin`, or the sin caught by `tempt ... forgive`.",
        "may" => "`may sin` in a signature confesses that the parable can fail.",
        "preach" => "`preach x` prints a value.",
        "deny" => "`deny x` negates a Truth. Deny Jesus three times and the rooster crows.",
        "and" | "or" => "Joins two Truths.",
        "is" | "not" => "`a is b` and `a is not b` compare two values of one type. There is no `==`.",
        "lend" => "`f(lend x)` lets a parable read `x`. It is still yours afterwards.",
        "entrust" => "`f(entrust x)` lets a parable change `x`. It is still yours afterwards. `x` must be `mortal`.",
        "discern" => "`discern value` runs one branch per case. Every case must be faced.",
        "kind" => "`kind Name` declares a record with named fields, or a set of cases.",
        "pagan" => "`pagan lua \"file.lua\" as name` or `pagan python \"module\" as name` imports foreign code. What it returns is a `Pagan`.",
        "baptize" | "as" => "`baptize x as T` turns a Pagan into a value of Gesù. It sins if the pagan is not a `T`.",
        "Jesus" => "The Truth. The only one.",
        "False" => "The other value of type `Truth`.",
        "True" | "true" => "There is only one Truth, Jesus.",
        "Number" => "A whole number, 64 bits. Overflow is a mortal sin.",
        "Decimal" => "A number with a fraction. Does not mix with `Number` without `decimal(n)`.",
        "Text" => "Text in quotes. Values are placed inside with `{braces}`.",
        "Truth" => "The type of `Jesus` and `False`.",
        "Maybe" => "`Maybe T` is `Found(x)` or `Lost`. Gesù has no null.",
        "Found" => "`Found(x)`: a `Maybe` that holds a value.",
        "Lost" => "A `Maybe` that holds nothing.",
        "Pagan" => "A value from Lua, Python or JSON. It must be baptized before use.",
        "Nothing" => "What a parable gives when it gives nothing.",
        "length" => "`length(lend x) -> Number`: size of a Text, multitude or map.",
        "push" => "`push(entrust list, value)`: adds a value at the end.",
        "pop" => "`pop(entrust list) -> Maybe T`: removes and gives the last value.",
        "get" => "`get(lend list, i) -> Maybe T`: the value at `i`, or `Lost`. Never sins.",
        "lookup" => "`lookup(lend map, key) -> Maybe V`: the value under `key`, or `Lost`. Never sins.",
        "contains" => "`contains(lend x, value) -> Truth`: is the value in the multitude, the key in the map, or the piece in the Text?",
        "remove" => "`remove(entrust map, key) -> Maybe V`, or `remove(entrust list, i) -> T`.",
        "keys" => "`keys(lend map) -> [K]`: the keys, in order.",
        "copy" => "`copy(lend x) -> T`: a full copy that you own.",
        "range" => "`range(from, to) -> [Number]`: from `from` up to, not including, `to`.",
        "sort" => "`sort(entrust list)`: sorts Numbers, Decimals or Texts in place.",
        "reverse" => "`reverse(entrust list)`: reverses in place.",
        "split" => "`split(text, separator) -> [Text]`.",
        "join" => "`join(lend list, separator) -> Text`.",
        "trim" => "`trim(text) -> Text`: without spaces at either end.",
        "upper" => "`upper(text) -> Text`.",
        "lower" => "`lower(text) -> Text`.",
        "replace" => "`replace(text, old, new) -> Text`.",
        "chars" => "`chars(text) -> [Text]`: its letters.",
        "text" => "`text(x) -> Text`: any value as Text.",
        "number" => "`number(text) -> Number`. May sin.",
        "decimal" => "`decimal(number) -> Decimal`, or `decimal(text) -> Decimal`, which may sin.",
        "floor" => "`floor(decimal) -> Number`: rounds down.",
        "round" => "`round(decimal) -> Number`: rounds to the nearest.",
        "abs" => "`abs(x)`: without its sign.",
        "min" => "`min(a, b)`: the lesser.",
        "max" => "`max(a, b)`: the greater.",
        "read_file" => "`read_file(path) -> Text`. May sin.",
        "write_file" => "`write_file(path, text)`. May sin.",
        "read_line" => "`read_line() -> Maybe Text`: one line of input, or `Lost` at the end.",
        "args" => "`args() -> [Text]`: what was written after the program's name.",
        "parse_json" => "`parse_json(text) -> Pagan`. May sin. Baptize the result.",
        "to_json" => "`to_json(lend x) -> Text`.",
        _ => return None,
    })
}

fn get<'a>(d: &'a Data, key: &str) -> Option<&'a Data> {
    match d {
        Data::Map(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn path<'a>(d: &'a Data, keys: &[&str]) -> Option<&'a Data> {
    keys.iter().try_fold(d, |d, key| get(d, key))
}

fn string(d: Option<&Data>) -> Option<String> {
    match d {
        Some(Data::Str(s)) => Some(s.clone()),
        _ => None,
    }
}

fn obj(pairs: Vec<(&str, Data)>) -> Data {
    Data::Map(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn s(text: &str) -> Data {
    Data::Str(text.to_string())
}

fn send(message: Data) {
    let mut body = String::new();
    write_json(&message, &mut body);
    let mut out = std::io::stdout().lock();
    let _ = write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body);
    let _ = out.flush();
}

fn position(line: usize, character: usize) -> Data {
    obj(vec![("line", Data::Int(line as i64)), ("character", Data::Int(character as i64))])
}

/// Sends the checker's findings for one document. Each one covers its
/// whole line, since the checker reports by line.
fn publish(uri: &str, src: &str) {
    let lines: Vec<&str> = src.lines().collect();
    let mut items = Vec::new();
    for d in analyze(src) {
        let line = d.line.saturating_sub(1).min(lines.len().saturating_sub(1));
        let text = lines.get(line).copied().unwrap_or("");
        let start = text.len() - text.trim_start().len();
        let end = text.trim_end().encode_utf16().count().max(start + 1);
        let mut message = d.title.clone();
        for note in &d.notes {
            message.push('\n');
            message.push_str(note);
        }
        let severity = if d.level == Level::Error { 1 } else { 2 };
        items.push(obj(vec![
            ("range", obj(vec![("start", position(line, start)), ("end", position(line, end))])),
            ("severity", Data::Int(severity)),
            ("source", s("gesu")),
            ("message", Data::Str(message)),
        ]));
    }
    send(obj(vec![
        ("jsonrpc", s("2.0")),
        ("method", s("textDocument/publishDiagnostics")),
        ("params", obj(vec![("uri", s(uri)), ("diagnostics", Data::List(items))])),
    ]));
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The word under a position given in UTF-16 units, as editors count.
fn word_at(src: &str, line: usize, character: usize) -> Option<String> {
    let text = src.lines().nth(line)?;
    let chars: Vec<char> = text.chars().collect();
    let mut units = 0;
    let mut at = chars.len();
    for (i, c) in chars.iter().enumerate() {
        if units >= character {
            at = i;
            break;
        }
        units += c.len_utf16();
    }
    let at = at.min(chars.len().saturating_sub(1));
    if chars.is_empty() || !is_word(chars[at]) {
        return None;
    }
    let mut start = at;
    while start > 0 && is_word(chars[start - 1]) {
        start -= 1;
    }
    let mut end = at;
    while end + 1 < chars.len() && is_word(chars[end + 1]) {
        end += 1;
    }
    Some(chars[start..=end].iter().collect())
}

fn completions(src: &str) -> Data {
    let mut items = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut add = |label: &str, kind: i64, seen: &mut Vec<String>| {
        if !seen.iter().any(|x| x == label) {
            seen.push(label.to_string());
            let mut pairs = vec![("label", s(label)), ("kind", Data::Int(kind))];
            if let Some(text) = doc(label) {
                pairs.push(("documentation", obj(vec![("kind", s("markdown")), ("value", s(text))])));
            }
            items.push(obj(pairs));
        }
    };
    for k in KEYWORDS {
        add(k, 14, &mut seen);
    }
    for b in BUILTINS {
        add(b, 3, &mut seen);
    }
    for t in TYPES {
        add(t, 7, &mut seen);
    }
    // Names already written in this document.
    let mut word = String::new();
    for c in src.chars().chain(std::iter::once(' ')) {
        if is_word(c) {
            word.push(c);
        } else if !word.is_empty() {
            let heresy = word.eq_ignore_ascii_case("true");
            if !heresy && !word.chars().next().is_some_and(|c| c.is_ascii_digit()) && word.chars().count() > 1 {
                let kind = if word.chars().next().is_some_and(|c| c.is_uppercase()) { 7 } else { 6 };
                add(&word, kind, &mut seen);
            }
            word.clear();
        }
    }
    Data::List(items)
}

fn read_message(input: &mut impl BufRead) -> Option<Data> {
    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if input.read_line(&mut header).ok()? == 0 {
            return None;
        }
        let header = header.trim();
        if header.is_empty() {
            break;
        }
        if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().ok()?;
        }
    }
    let mut body = vec![0u8; length];
    input.read_exact(&mut body).ok()?;
    parse_json(&String::from_utf8_lossy(&body)).ok()
}

pub fn serve() -> i32 {
    let mut input = std::io::stdin().lock();
    let mut docs: HashMap<String, String> = HashMap::new();
    let mut shut_down = false;

    while let Some(message) = read_message(&mut input) {
        let method = string(get(&message, "method")).unwrap_or_default();
        let id = get(&message, "id").cloned();
        let params = get(&message, "params").cloned().unwrap_or(Data::Null);
        let uri = string(path(&params, &["textDocument", "uri"])).unwrap_or_default();
        let reply = |result: Data| {
            if let Some(id) = &id {
                send(obj(vec![("jsonrpc", s("2.0")), ("id", id.clone()), ("result", result)]));
            }
        };

        match method.as_str() {
            "initialize" => reply(obj(vec![
                (
                    "capabilities",
                    obj(vec![
                        ("textDocumentSync", Data::Int(1)),
                        ("completionProvider", Data::Map(vec![])),
                        ("hoverProvider", Data::Bool(true)),
                    ]),
                ),
                ("serverInfo", obj(vec![("name", s("gesu")), ("version", s(env!("CARGO_PKG_VERSION")))])),
            ])),
            "shutdown" => {
                shut_down = true;
                reply(Data::Null);
            }
            "exit" => return if shut_down { 0 } else { 1 },
            "textDocument/didOpen" => {
                let text = string(path(&params, &["textDocument", "text"])).unwrap_or_default();
                publish(&uri, &text);
                docs.insert(uri, text);
            }
            "textDocument/didChange" => {
                // Full sync: the last change holds the whole text.
                let text = match get(&params, "contentChanges") {
                    Some(Data::List(changes)) => changes.last().and_then(|c| string(get(c, "text"))),
                    _ => None,
                };
                if let Some(text) = text {
                    publish(&uri, &text);
                    docs.insert(uri, text);
                }
            }
            "textDocument/didClose" => {
                docs.remove(&uri);
                send(obj(vec![
                    ("jsonrpc", s("2.0")),
                    ("method", s("textDocument/publishDiagnostics")),
                    ("params", obj(vec![("uri", s(&uri)), ("diagnostics", Data::List(vec![]))])),
                ]));
            }
            "textDocument/completion" => reply(completions(docs.get(&uri).map(|t| t.as_str()).unwrap_or(""))),
            "textDocument/hover" => {
                let number = |key: &str| match path(&params, &["position", key]) {
                    Some(Data::Int(n)) => *n as usize,
                    _ => 0,
                };
                let found = docs
                    .get(&uri)
                    .and_then(|src| word_at(src, number("line"), number("character")))
                    .and_then(|word| doc(&word));
                reply(match found {
                    Some(text) => obj(vec![("contents", obj(vec![("kind", s("markdown")), ("value", s(text))]))]),
                    None => Data::Null,
                });
            }
            _ => {
                // An unknown request still deserves an answer; a notification does not.
                if let Some(id) = &id {
                    send(obj(vec![
                        ("jsonrpc", s("2.0")),
                        ("id", id.clone()),
                        ("error", obj(vec![("code", Data::Int(-32601)), ("message", s("method not found"))])),
                    ]));
                }
            }
        }
    }
    0
}
