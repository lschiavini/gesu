//! Tells Zed how to start the Gesù language server: `gesu lsp`.

use zed_extension_api::{self as zed, Result};

struct GesuExtension;

impl zed::Extension for GesuExtension {
    fn new() -> Self {
        GesuExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let command = worktree.which("gesu").ok_or_else(|| {
            "gesu is not on your PATH. Install it with: \
             curl -fsSL https://gesu-lang.com/install.sh | sh"
                .to_string()
        })?;
        Ok(zed::Command { command, args: vec!["lsp".to_string()], env: Default::default() })
    }
}

zed::register_extension!(GesuExtension);
