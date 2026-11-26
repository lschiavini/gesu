//! The formatter: `gesu fmt`. It sets every line at the depth its block
//! gives it, puts one space where one belongs, and removes what is idle.
//! It never changes what a program means: the tokens before and after are
//! compared, and if they differ nothing is written.

use crate::diag::Diag;
use crate::lexer::{lex, Tok, KEYWORDS};
use std::io::Read;

const INDENT: &str = "    ";
const OPENERS: &[&str] = &["if", "while", "repeat", "for", "tempt", "parable", "kind", "discern"];

#[derive(Clone, Debug, PartialEq)]
enum Piece {
    /// A name, a word of the language, or a number.
    Word(String),
    /// A text, kept exactly as it was written.
    Text(String),
    Punct(String),
    /// A comment, with the number of blanks that stood before it.
    Comment(String, usize),
}

#[derive(Clone, Copy, PartialEq)]
enum Frame {
    Block,
    Discern,
    /// A case of `discern` whose branch runs over the lines below it.
    Arm,
}

fn scan(line: &str) -> Vec<Piece> {
    let chars: Vec<char> = line.chars().collect();
    let mut pieces = Vec::new();
    let mut i = 0;
    let mut gap = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c.is_whitespace() {
            gap += 1;
            i += 1;
            continue;
        }
        if c == '-' && next == Some('-') {
            let text: String = chars[i..].iter().collect();
            pieces.push(Piece::Comment(text.trim_end().to_string(), gap));
            break;
        }
        gap = 0;
        if c == '"' {
            let end = text_end(&chars, i + 1);
            pieces.push(Piece::Text(chars[i..end].iter().collect()));
            i = end;
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_') {
                i += 1;
            }
            if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            pieces.push(Piece::Word(chars[start..i].iter().collect()));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            pieces.push(Piece::Word(chars[start..i].iter().collect()));
        } else if matches!((c, next), ('-', Some('>')) | ('<', Some('=')) | ('>', Some('='))) {
            pieces.push(Piece::Punct(chars[i..i + 2].iter().collect()));
            i += 2;
        } else {
            pieces.push(Piece::Punct(c.to_string()));
            i += 1;
        }
    }
    pieces
}

/// The index just past the quote that closes a text. Braces inside a text
/// hold code, and that code may hold texts of its own.
fn text_end(chars: &[char], mut i: usize) -> usize {
    let mut depth = 0;
    let mut inner = false;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1,
            '"' if depth == 0 => return i + 1,
            '"' => inner = !inner,
            '{' if !inner => depth += 1,
            '}' if !inner && depth > 0 => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    chars.len()
}

fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}

fn punct(piece: &Piece) -> &str {
    match piece {
        Piece::Punct(p) => p,
        _ => "",
    }
}

/// Whether a `-` that follows `prev` negates what comes after it, as
/// opposed to subtracting it from what came before.
fn negates(prev: Option<&Piece>) -> bool {
    match prev {
        None => true,
        Some(Piece::Word(w)) => is_keyword(w) && !matches!(w.as_str(), "Jesus" | "False"),
        Some(Piece::Text(_)) | Some(Piece::Comment(..)) => false,
        Some(Piece::Punct(p)) => !matches!(p.as_str(), ")" | "]" | "}"),
    }
}

fn space_between(prev: &Piece, cur: &Piece) -> bool {
    let (p, c) = (punct(prev), punct(cur));
    if matches!(c, ")" | "]" | "}" | "," | ":" | ".") || matches!(p, "(" | "[" | "{" | ".") {
        return false;
    }
    // A call, or an index: nothing stands between the name and the bracket.
    let follows_value = match prev {
        Piece::Word(w) => !is_keyword(w),
        Piece::Text(_) => c == "[",
        Piece::Punct(p) => matches!(p.as_str(), ")" | "]"),
        Piece::Comment(..) => false,
    };
    if c == "(" && follows_value {
        return false;
    }
    if c == "[" && follows_value && !matches!(prev, Piece::Word(w) if w == "Maybe") {
        return false;
    }
    true
}

fn render(pieces: &[Piece]) -> String {
    let mut out = String::new();
    let mut glue = false;
    for (i, piece) in pieces.iter().enumerate() {
        if let Piece::Comment(text, gap) = piece {
            if !out.is_empty() {
                out.push_str(&" ".repeat((*gap).max(1)));
            }
            out.push_str(text);
            break;
        }
        let prev = if i == 0 { None } else { pieces.get(i - 1) };
        if let Some(prev) = prev {
            if !glue && space_between(prev, piece) {
                out.push(' ');
            }
        }
        glue = punct(piece) == "-" && negates(prev);
        match piece {
            Piece::Word(t) | Piece::Text(t) | Piece::Punct(t) => out.push_str(t),
            Piece::Comment(..) => {}
        }
    }
    out
}

fn capitalized(word: &str) -> bool {
    word.chars().next().is_some_and(|c| c.is_uppercase())
}

/// If the pieces begin a case of `discern`, the index just past its colon.
fn arm_colon(pieces: &[Piece]) -> Option<usize> {
    match pieces.first() {
        Some(Piece::Word(w)) if capitalized(w) => {}
        _ => return None,
    }
    let mut i = 1;
    if pieces.get(i).map(punct) == Some("(") {
        i += 1;
        loop {
            match pieces.get(i) {
                Some(Piece::Word(_)) => i += 1,
                Some(Piece::Punct(p)) if p == "," => i += 1,
                Some(Piece::Punct(p)) if p == ")" => {
                    i += 1;
                    break;
                }
                _ => return None,
            }
        }
    }
    (pieces.get(i).map(punct) == Some(":")).then_some(i + 1)
}

fn opened_by(pieces: &[Piece]) -> Option<Frame> {
    match pieces.first() {
        Some(Piece::Word(w)) if w == "discern" => Some(Frame::Discern),
        Some(Piece::Word(w)) if OPENERS.contains(&w.as_str()) => Some(Frame::Block),
        _ => None,
    }
}

fn tokens(src: &str) -> Result<Vec<Tok>, Diag> {
    Ok(lex(src, 1)?.into_iter().map(|(tok, _)| tok).collect())
}

/// The source, formatted. A text that cannot be read as Gesù is refused.
pub fn format(src: &str) -> Result<String, Diag> {
    let before = tokens(src)?;

    // Each line that will be written: its depth and its text. `None` is a
    // blank line.
    let mut lines: Vec<Option<(usize, String)>> = Vec::new();
    // Comments wait for the line of code below them, to stand at its depth.
    let mut waiting: Vec<usize> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    // Brackets still open from the lines above, and the depth of the line
    // that opened them.
    let mut brackets = 0usize;
    let mut base = 0usize;

    for raw in src.lines() {
        let pieces = scan(raw);
        if pieces.is_empty() {
            if !matches!(lines.last(), None | Some(None)) {
                lines.push(None);
            }
            continue;
        }
        let code: Vec<Piece> = pieces.iter().filter(|p| !matches!(p, Piece::Comment(..))).cloned().collect();
        let text = render(&pieces);

        let depth = if brackets > 0 {
            let closers = code.iter().take_while(|p| matches!(punct(p), ")" | "]" | "}")).count();
            base + brackets.saturating_sub(closers)
        } else if code.is_empty() {
            waiting.push(lines.len());
            lines.push(Some((0, text)));
            continue;
        } else {
            let first = match &code[0] {
                Piece::Word(w) => w.as_str(),
                _ => "",
            };
            let body = frames.len();
            let depth = match first {
                "amen" => {
                    if frames.last() == Some(&Frame::Arm) {
                        frames.pop();
                    }
                    frames.pop();
                    frames.len()
                }
                "otherwise" | "forgive" => frames.len().saturating_sub(1),
                _ => {
                    let in_discern = matches!(frames.last(), Some(Frame::Discern | Frame::Arm));
                    match arm_colon(&code).filter(|_| in_discern) {
                        Some(after) => {
                            if frames.last() == Some(&Frame::Arm) {
                                frames.pop();
                            }
                            let depth = frames.len();
                            if after == code.len() {
                                frames.push(Frame::Arm);
                            } else if let Some(frame) = opened_by(&code[after..]) {
                                frames.push(frame);
                            }
                            depth
                        }
                        None => {
                            let depth = frames.len();
                            if let Some(frame) = opened_by(&code) {
                                frames.push(frame);
                            }
                            depth
                        }
                    }
                }
            };
            // A comment above a line that closes a block belongs to the block.
            let closes = matches!(first, "amen" | "otherwise" | "forgive");
            for index in waiting.drain(..) {
                if let Some(Some(line)) = lines.get_mut(index) {
                    line.0 = if closes { body } else { depth };
                }
            }
            base = depth;
            depth
        };

        for piece in &code {
            match punct(piece) {
                "(" | "[" | "{" => brackets += 1,
                ")" | "]" | "}" => brackets = brackets.saturating_sub(1),
                _ => {}
            }
        }
        lines.push(Some((depth, text)));
    }
    for index in waiting {
        if let Some(Some(line)) = lines.get_mut(index) {
            line.0 = frames.len();
        }
    }
    while matches!(lines.last(), Some(None)) {
        lines.pop();
    }

    let mut out = String::new();
    for line in &lines {
        if let Some((depth, text)) = line {
            out.push_str(&INDENT.repeat(*depth));
            out.push_str(text);
        }
        out.push('\n');
    }

    if tokens(&out).ok().as_ref() != Some(&before) {
        return Err(Diag::error(1, "the formatter would have changed what this program means")
            .note("nothing was written; please report this file at https://github.com/lschiavini/gesu/issues"));
    }
    Ok(out)
}

const HELP: &str = "usage:
  gesu fmt <file.gesu>...       format the files in place
  gesu fmt --check <file>...    change nothing; fail if a file is not formatted
  gesu fmt -                    read a program on stdin, write it formatted on stdout
";

/// The `gesu fmt` command.
pub fn run(args: &[String]) -> i32 {
    let check = args.iter().any(|a| a == "--check");
    let files: Vec<&String> = args.iter().filter(|a| *a != "--check").collect();
    if files.is_empty() || files.iter().any(|f| f.starts_with("--") || *f == "-h") {
        eprint!("{HELP}");
        return 2;
    }
    let mut status = 0;
    for file in files {
        let stdin = file == "-";
        let src = if stdin {
            let mut text = String::new();
            std::io::stdin().read_to_string(&mut text).map(|_| text)
        } else {
            std::fs::read_to_string(file)
        };
        let src = match src {
            Ok(src) => src,
            Err(e) => {
                eprintln!("error: cannot read {file}: {e}");
                status = 2;
                continue;
            }
        };
        let name = if stdin { "<stdin>" } else { file.as_str() };
        match format(&src) {
            Err(d) => {
                eprintln!("{}", crate::diag::render_diag(&d, name, &src));
                status = status.max(1);
            }
            Ok(text) if stdin => print!("{text}"),
            Ok(text) if text == src => {}
            Ok(_) if check => {
                println!("{file} is not formatted");
                status = status.max(1);
            }
            Ok(text) => match std::fs::write(file, text) {
                Ok(()) => println!("formatted {file}"),
                Err(e) => {
                    eprintln!("error: cannot write {file}: {e}");
                    status = 2;
                }
            },
        }
    }
    status
}

#[cfg(test)]
mod tests {
    use super::format;

    fn fmt(src: &str) -> String {
        format(src).expect("formats")
    }

    #[test]
    fn blocks_are_indented() {
        let src = "parable f(a:Number)->Number may sin\nif a is 0\nsin \"none\"\notherwise if a<0\ngive -a\notherwise\ngive a*2\namen\ngive a\namen\namen\n";
        let want = "parable f(a: Number) -> Number may sin\n    if a is 0\n        sin \"none\"\n    otherwise if a < 0\n        give -a\n    otherwise\n        give a * 2\n    amen\n    give a\namen\namen\n";
        assert_eq!(fmt(src), want);
    }

    #[test]
    fn discern_arms() {
        let src = "discern soil\nPath: give 1\nThorns(days,weed):\nlet c = days*2\nif c > 1\npreach c\namen\n-- the last\nGood(n): preach n\namen\namen\n";
        let want = "discern soil\n    Path: give 1\n    Thorns(days, weed):\n        let c = days * 2\n        if c > 1\n            preach c\n        amen\n    -- the last\n    Good(n): preach n\namen\namen\n";
        assert_eq!(fmt(src), want);
    }

    #[test]
    fn tempt_and_forgive() {
        let src = "tempt\n  preach f( 1 ,2 )\n forgive fault\npreach fault\n  -- done\namen\namen";
        let want = "tempt\n    preach f(1, 2)\nforgive fault\n    preach fault\n    -- done\namen\namen\n";
        assert_eq!(fmt(src), want);
    }

    #[test]
    fn spacing() {
        assert_eq!(fmt("let x=[1,-2 , 3] [ 0 ]\namen"), "let x = [1, -2, 3][0]\namen\n");
        assert_eq!(fmt("let m:{Text:[Number]}={\"a\":[1]}\namen"), "let m: {Text: [Number]} = {\"a\": [1]}\namen\n");
        assert_eq!(fmt("preach (a+b) *c . d\namen"), "preach (a + b) * c.d\namen\n");
        assert_eq!(fmt("let x = a -1\namen"), "let x = a - 1\namen\n");
        assert_eq!(fmt("parable f(a: Maybe [Text])\namen\namen"), "parable f(a: Maybe [Text])\namen\namen\n");
    }

    #[test]
    fn texts_and_comments_are_left_alone() {
        let src = "preach \"a  {b+1}  {\"x  y\"}  --not a comment\"   -- kept  as is\namen\n";
        assert_eq!(fmt(src), src);
    }

    #[test]
    fn lists_over_many_lines() {
        let src = "let field = [\nPath,\n  Rock(3),\n]\npreach field\namen\n";
        let want = "let field = [\n    Path,\n    Rock(3),\n]\npreach field\namen\n";
        assert_eq!(fmt(src), want);
    }

    #[test]
    fn blank_lines() {
        assert_eq!(fmt("\n\nlet x = 1\n\n\n\npreach x   \namen\n\n\n"), "let x = 1\n\npreach x\namen\n");
    }

    #[test]
    fn formatting_twice_changes_nothing() {
        let src = "kind Soil\nPath\nRock(Number)\namen\nfor each s in [Path,Rock(1)]\ndiscern s\nPath: preach 1\nRock(n):\nwhile n>0\nbreak\namen\namen\namen\namen";
        let once = fmt(src);
        assert_eq!(fmt(&once), once);
    }

    #[test]
    fn unreadable_text_is_refused() {
        assert!(format("preach \"never ends\namen").is_err());
    }
}
