mod ast;
mod check;
mod diag;
mod interp;
mod lexer;
mod lsp;
mod pagan;
mod parser;
mod types;
mod value;

use diag::{render, render_diag, Diag, Level};
use interp::{Flow, Interp, SinInfo};
use std::io::IsTerminal;
use std::path::Path;

const HELP: &str = "Gesù: there is only one Truth, Jesus.

usage:
  gesu <file.gesu> [args...]          run a program
  gesu run <file.gesu> [args...]      run a program
  gesu check <file.gesu>              examine a program without running it
  gesu lsp                            serve an editor (Language Server Protocol)
";

fn main() {
    // Parables may call each other deeply, so the work runs on a thread
    // with a stack large enough for the 2000-call limit.
    let worker = std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(real_main).expect("a thread");
    std::process::exit(worker.join().unwrap_or(101));
}

fn paint(label: &str, code: &str) -> String {
    if std::io::stderr().is_terminal() {
        format!("\x1b[1;{code}m{label}\x1b[0m")
    } else {
        label.to_string()
    }
}

fn real_main() -> i32 {
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    let mut check_only = false;
    match argv.first().map(|s| s.as_str()) {
        None | Some("help") | Some("--help") | Some("-h") => {
            print!("{HELP}");
            return 0;
        }
        Some("--version") | Some("version") => {
            println!("gesu {}", env!("CARGO_PKG_VERSION"));
            return 0;
        }
        Some("lsp") => return lsp::serve(),
        Some("run") => {
            argv.remove(0);
        }
        Some("check") => {
            check_only = true;
            argv.remove(0);
        }
        _ => {}
    }
    if argv.is_empty() {
        eprint!("{HELP}");
        return 2;
    }
    let file = argv.remove(0);
    let src = match std::fs::read_to_string(&file) {
        Ok(src) => src,
        Err(e) => {
            eprintln!("{}: cannot read {file}: {e}", paint("error", "31"));
            return 2;
        }
    };

    let report = |d: &Diag| {
        let text = render_diag(d, &file, &src);
        let (plain, painted) = match d.level {
            Level::Error => ("error", paint("error", "31")),
            Level::Warning => ("warning", paint("warning", "33")),
        };
        eprintln!("{}", text.replacen(plain, &painted, 1));
    };

    let program = lexer::lex(&src, 1).and_then(|toks| {
        let mut parser = parser::Parser::new(toks);
        parser.program().map(|stmts| (stmts, parser.warnings))
    });
    let (stmts, mut diags) = match program {
        Ok(ok) => ok,
        Err(d) => {
            report(&d);
            eprintln!("Repent, and compile again.");
            return 1;
        }
    };

    let mut checker = check::Checker::new();
    checker.program(&stmts);
    diags.append(&mut checker.diags);
    diags.sort_by_key(|d| d.line);
    for d in &diags {
        report(d);
    }
    let errors = diags.iter().filter(|d| d.level == Level::Error).count();
    if errors > 0 {
        let noun = if errors == 1 { "error" } else { "errors" };
        eprintln!("{errors} {noun}. Repent, and compile again.");
        return 1;
    }
    if check_only {
        println!("{file} is without fault.");
        return 0;
    }

    let dir = Path::new(&file).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_path_buf();
    let mut interp = Interp::new(checker.kinds, dir, argv);
    let died = |label: &str, info: &SinInfo| {
        let mut notes: Vec<String> =
            info.trace.iter().rev().take(8).map(|(name, line)| format!("in the parable `{name}`, called on line {line}")).collect();
        if info.trace.len() > 8 {
            notes.push(format!("and {} more calls beneath", info.trace.len() - 8));
        }
        notes.push("the wages of sin is death".into());
        let text = render(label, &info.message, info.line, &notes, &file, &src);
        eprintln!("{}", text.replacen(label, &paint(label, "31"), 1));
        1
    };
    match interp.run(&stmts) {
        Ok(()) => 0,
        Err(Flow::Sin(info)) => died("sin", &info),
        Err(Flow::Mortal(info)) => died("mortal sin", &info),
        Err(_) => 0,
    }
}
