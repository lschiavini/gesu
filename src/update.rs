//! `gesu update`: replaces this binary with the latest release.
//!
//! It asks GitHub for the newest version and downloads the build for this
//! machine with `curl` and `tar`, which Mac, Linux and Windows all carry.

use std::path::{Path, PathBuf};
use std::process::Command;

const RELEASES: &str = "https://github.com/lschiavini/gesu/releases";

/// The name of the release build for this machine, if there is one.
fn target() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("aarch64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("x86_64-pc-windows-msvc")
    } else {
        None
    }
}

fn curl(args: &[&str]) -> Result<String, String> {
    let out = Command::new("curl")
        .args(args)
        .output()
        .map_err(|e| format!("cannot run curl: {e}"))?;
    if !out.status.success() {
        return Err(format!("the download failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// "0.2.1" as numbers, so that 0.10.0 counts as newer than 0.9.0.
fn numbers(version: &str) -> Vec<u64> {
    version.trim_start_matches('v').split('.').map(|part| part.parse().unwrap_or(0)).collect()
}

/// The newest released version, read from where GitHub redirects "latest".
fn latest() -> Result<String, String> {
    let url = curl(&["-fsSL", "-o", if cfg!(windows) { "NUL" } else { "/dev/null" }, "-w", "%{url_effective}", &format!("{RELEASES}/latest")])?;
    match url.rsplit_once("/tag/") {
        Some((_, tag)) if !tag.is_empty() => Ok(tag.trim_start_matches('v').to_string()),
        _ => Err("could not read the latest version from GitHub".into()),
    }
}

fn replace(new: &Path, exe: &Path) -> std::io::Result<()> {
    // Staged beside the old binary, then swapped in with one rename, so a
    // failure halfway leaves the old one in place.
    let staged = exe.with_extension("new");
    std::fs::copy(new, &staged)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(windows)]
    {
        // A running program cannot be overwritten on Windows, only moved.
        let old = exe.with_extension("old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(exe, &old)?;
    }
    std::fs::rename(&staged, exe)
}

fn run(force: bool) -> Result<String, String> {
    let current = env!("CARGO_PKG_VERSION");
    if cfg!(feature = "python") && !force {
        return Err(format!(
            "this gesu was built from source with the Python pagan, and the released binaries have none.\n\
             Update it from your clone:  git pull && ./install.sh --python\n\
             Or take the released binary anyway:  gesu update --force"
        ));
    }
    let Some(target) = target() else {
        return Err(format!("there is no released binary for this machine. Build from source: {RELEASES}"));
    };
    let newest = latest()?;
    if numbers(&newest) <= numbers(current) && !force {
        return Ok(format!("gesu {current} is the latest. Nothing to do."));
    }

    let exe: PathBuf = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .map_err(|e| format!("cannot find where gesu is installed: {e}"))?;
    let dir = std::env::temp_dir().join(format!("gesu-update-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;

    let archive_name = if cfg!(windows) { format!("gesu-{target}.zip") } else { format!("gesu-{target}.tar.gz") };
    let archive = dir.join(&archive_name);
    let url = format!("{RELEASES}/download/v{newest}/{archive_name}");
    println!("downloading gesu {newest}");
    let result = (|| {
        curl(&["-fsSL", &url, "-o", &archive.to_string_lossy()])?;
        let unpacked = Command::new("tar")
            .arg("-xf")
            .arg(&archive)
            .arg("-C")
            .arg(&dir)
            .status()
            .map_err(|e| format!("cannot run tar: {e}"))?;
        if !unpacked.success() {
            return Err("could not unpack the download".to_string());
        }
        let new = dir.join(if cfg!(windows) { "gesu.exe" } else { "gesu" });
        replace(&new, &exe).map_err(|e| {
            format!("cannot replace {}: {e}\nIf it sits in a protected folder, run the update with the rights to write there.", exe.display())
        })
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result?;
    Ok(format!("gesu {current} is now gesu {newest}, at {}", exe.display()))
}

pub fn update(force: bool) -> i32 {
    match run(force) {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(message) => {
            eprintln!("gesu update: {message}");
            1
        }
    }
}
