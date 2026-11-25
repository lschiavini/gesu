//! Diagnostics: errors and warnings shown to the programmer.

#[derive(Clone, Debug, PartialEq)]
pub enum Level {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diag {
    pub level: Level,
    pub line: usize,
    pub title: String,
    pub notes: Vec<String>,
}

impl Diag {
    pub fn error(line: usize, title: impl Into<String>) -> Self {
        Diag { level: Level::Error, line, title: title.into(), notes: vec![] }
    }

    pub fn warning(line: usize, title: impl Into<String>) -> Self {
        Diag { level: Level::Warning, line, title: title.into(), notes: vec![] }
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

/// Renders a headline, the source line it points at, and the notes under it.
pub fn render(label: &str, title: &str, line: usize, notes: &[String], file: &str, src: &str) -> String {
    let mut out = format!("{label}: {title}\n");
    if line > 0 {
        let width = line.to_string().len().max(2);
        out.push_str(&format!("{:>width$}--> {file}:{line}\n", "", width = width));
        if let Some(text) = src.lines().nth(line - 1) {
            out.push_str(&format!("{:>width$} |\n", "", width = width));
            out.push_str(&format!("{line:>width$} | {}\n", text.trim_end(), width = width));
            out.push_str(&format!("{:>width$} |\n", "", width = width));
        }
    }
    for note in notes {
        out.push_str(&format!("   = {note}\n"));
    }
    out
}

pub fn render_diag(d: &Diag, file: &str, src: &str) -> String {
    let label = match d.level {
        Level::Error => "error",
        Level::Warning => "warning",
    };
    render(label, &d.title, d.line, &d.notes, file, src)
}
