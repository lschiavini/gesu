use std::process::Command;

/// With the `python` feature, records where the build's Python lives, so
/// the installed binary can find the standard library from any directory.
fn main() {
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
    if std::env::var_os("CARGO_FEATURE_PYTHON").is_none() {
        return;
    }
    let python = std::env::var("PYO3_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let home = Command::new(&python).args(["-c", "import sys; print(sys.base_prefix)"]).output();
    if let Ok(out) = home {
        let home = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !home.is_empty() {
            println!("cargo:rustc-env=GESU_PYTHON_HOME={home}");
        }
    }
}
