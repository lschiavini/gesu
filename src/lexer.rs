//! Turns source text into tokens. Newlines end statements, except inside
//! brackets, and `--` starts a comment.

use crate::diag::Diag;

#[derive(Clone, Debug, PartialEq)]
pub enum StrPart {
    Lit(String),
    Code(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Number(i64),
    Decimal(f64),
    Str(Vec<StrPart>),
    Ident(String),
    Kw(&'static str),
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Dot,
    Arrow,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Lt,
    Gt,
    Le,
    Ge,
    Newline,
    Eof,
}

pub const KEYWORDS: &[&str] = &[
    "let", "mortal", "law", "parable", "give", "amen", "if", "otherwise", "while", "repeat", "times", "for",
    "each", "in", "break", "continue", "tempt", "forgive", "sin", "may", "preach", "deny", "and", "or", "is",
    "not", "lend", "entrust", "discern", "kind", "pagan", "baptize", "as", "Jesus", "False",
];

pub fn lex(src: &str, first_line: usize) -> Result<Vec<(Tok, usize)>, Diag> {
    let chars: Vec<char> = src.chars().collect();
    let mut toks: Vec<(Tok, usize)> = Vec::new();
    let mut i = 0;
    let mut line = first_line;
    let mut depth = 0usize;

    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            '\n' => {
                if depth == 0 && !matches!(toks.last(), None | Some((Tok::Newline, _))) {
                    toks.push((Tok::Newline, line));
                }
                line += 1;
                i += 1;
            }
            ' ' | '\t' | '\r' => i += 1,
            '-' if next == Some('-') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '-' if next == Some('>') => {
                toks.push((Tok::Arrow, line));
                i += 2;
            }
            '"' => {
                let (parts, end) = lex_string(&chars, i + 1, line)?;
                toks.push((Tok::Str(parts), line));
                i = end;
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_') {
                    i += 1;
                }
                let mut decimal = false;
                if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                    decimal = true;
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let text: String = chars[start..i].iter().filter(|c| **c != '_').collect();
                if decimal {
                    toks.push((Tok::Decimal(text.parse().unwrap_or(0.0)), line));
                } else {
                    match text.parse::<i64>() {
                        Ok(n) => toks.push((Tok::Number(n), line)),
                        Err(_) => {
                            return Err(Diag::error(line, format!("the number {text} is too great"))
                                .note("a Number holds at most 9223372036854775807"))
                        }
                    }
                }
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                match KEYWORDS.iter().find(|k| **k == word) {
                    Some(k) => toks.push((Tok::Kw(k), line)),
                    None => toks.push((Tok::Ident(word), line)),
                }
            }
            '=' if next == Some('=') => {
                return Err(Diag::error(line, "`==` is not Gesù").note("hint: compare with `is`"));
            }
            '!' => {
                return Err(Diag::error(line, "`!` is not Gesù")
                    .note("hint: write `is not` for inequality and `deny` for negation"));
            }
            '<' if next == Some('=') => {
                toks.push((Tok::Le, line));
                i += 2;
            }
            '>' if next == Some('=') => {
                toks.push((Tok::Ge, line));
                i += 2;
            }
            _ => {
                let tok = match c {
                    '(' => Tok::LParen,
                    ')' => Tok::RParen,
                    '[' => Tok::LBracket,
                    ']' => Tok::RBracket,
                    '{' => Tok::LBrace,
                    '}' => Tok::RBrace,
                    ',' => Tok::Comma,
                    ':' => Tok::Colon,
                    '.' => Tok::Dot,
                    '=' => Tok::Assign,
                    '+' => Tok::Plus,
                    '-' => Tok::Minus,
                    '*' => Tok::Star,
                    '/' => Tok::Slash,
                    '%' => Tok::Percent,
                    '<' => Tok::Lt,
                    '>' => Tok::Gt,
                    other => return Err(Diag::error(line, format!("the character `{other}` has no meaning here"))),
                };
                match tok {
                    Tok::LParen | Tok::LBracket | Tok::LBrace => depth += 1,
                    Tok::RParen | Tok::RBracket | Tok::RBrace => depth = depth.saturating_sub(1),
                    _ => {}
                }
                toks.push((tok, line));
                i += 1;
            }
        }
    }
    if !matches!(toks.last(), None | Some((Tok::Newline, _))) {
        toks.push((Tok::Newline, line));
    }
    toks.push((Tok::Eof, line));
    Ok(toks)
}

/// Reads a string body starting after the opening quote. Returns the parts
/// and the index just past the closing quote.
fn lex_string(chars: &[char], mut i: usize, line: usize) -> Result<(Vec<StrPart>, usize), Diag> {
    let mut parts = Vec::new();
    let mut lit = String::new();
    loop {
        let Some(&c) = chars.get(i) else {
            return Err(Diag::error(line, "this text never ends").note("hint: close it with `\"`"));
        };
        match c {
            '\n' => return Err(Diag::error(line, "this text never ends").note("hint: close it with `\"`")),
            '"' => {
                if !lit.is_empty() || parts.is_empty() {
                    parts.push(StrPart::Lit(lit));
                }
                return Ok((parts, i + 1));
            }
            '\\' => {
                let escaped = match chars.get(i + 1) {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('"') => '"',
                    Some('\\') => '\\',
                    Some('{') => '{',
                    Some('}') => '}',
                    _ => return Err(Diag::error(line, "unknown escape in text").note("known: \\n \\t \\\" \\\\ \\{ \\}")),
                };
                lit.push(escaped);
                i += 2;
            }
            '{' => {
                let mut code = String::new();
                let mut depth = 1;
                i += 1;
                loop {
                    match chars.get(i) {
                        None | Some('\n') => {
                            return Err(Diag::error(line, "`{` in text was never closed")
                                .note("hint: write `\\{` for a plain brace"))
                        }
                        Some('"') => {
                            // A text inside the braces: copy it whole.
                            code.push('"');
                            i += 1;
                            while let Some(&c) = chars.get(i) {
                                if c == '\n' || c == '"' {
                                    break;
                                }
                                if c == '\\' {
                                    code.push(c);
                                    i += 1;
                                }
                                if let Some(&c) = chars.get(i) {
                                    code.push(c);
                                    i += 1;
                                }
                            }
                            if chars.get(i) != Some(&'"') {
                                return Err(Diag::error(line, "this text never ends").note("hint: close it with `\"`"));
                            }
                        }
                        Some('{') => depth += 1,
                        Some('}') => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    code.push(chars[i]);
                    i += 1;
                }
                i += 1;
                if !lit.is_empty() {
                    parts.push(StrPart::Lit(std::mem::take(&mut lit)));
                }
                parts.push(StrPart::Code(code));
            }
            other => {
                lit.push(other);
                i += 1;
            }
        }
    }
}
