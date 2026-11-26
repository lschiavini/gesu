//! Runs every example and every faulty program, and compares what they
//! print with the snapshot next to them (`.out` for stdout, `.err` for
//! stderr). Delete a snapshot and run with GESU_BLESS=1 to write it again.

use std::path::Path;
use std::process::Command;

fn check_dir(dir: &str, ext: &str) {
    let mut count = 0;
    for entry in std::fs::read_dir(dir).expect("directory") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("gesu") {
            continue;
        }
        let snapshot = path.with_extension(ext);
        let bless = std::env::var_os("GESU_BLESS").is_some();
        if !snapshot.exists() && !bless {
            continue;
        }
        let out = Command::new(env!("CARGO_BIN_EXE_gesu")).arg(&path).output().expect("gesu runs");
        let actual = String::from_utf8_lossy(if ext == "out" { &out.stdout } else { &out.stderr }).to_string();
        if bless && !snapshot.exists() {
            std::fs::write(&snapshot, &actual).expect("snapshot written");
        }
        let expected = std::fs::read_to_string(&snapshot).expect("snapshot");
        assert_eq!(actual, expected, "{} does not match {}", path.display(), snapshot.display());
        count += 1;
    }
    assert!(count > 0, "no snapshots found in {dir}");
}

#[test]
fn examples_print_what_they_should() {
    check_dir(Path::new("examples").to_str().unwrap(), "out");
}

#[test]
fn faulty_programs_are_refused() {
    check_dir(Path::new("tests/errors").to_str().unwrap(), "err");
}

#[test]
fn examples_are_formatted() {
    for dir in ["examples", "tests/errors"] {
        for entry in std::fs::read_dir(dir).expect("directory") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("gesu") {
                continue;
            }
            let out = Command::new(env!("CARGO_BIN_EXE_gesu")).arg("fmt").arg("--check").arg(&path).output().expect("gesu runs");
            assert!(out.status.success(), "{} is not as `gesu fmt` would write it", path.display());
        }
    }
}
