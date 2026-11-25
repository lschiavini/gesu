// Starts `gesu lsp` and connects it to the editor.
const fs = require("fs");
const os = require("os");
const path = require("path");
const { workspace, window } = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function findGesu() {
  const configured = workspace.getConfiguration("gesu").get("path") || "gesu";
  if (configured !== "gesu") {
    return configured;
  }
  // An editor opened from the Dock may not see the shell's PATH.
  const local = path.join(os.homedir(), ".local", "bin", "gesu");
  return fs.existsSync(local) ? local : configured;
}

function activate() {
  client = new LanguageClient(
    "gesu",
    "Gesù",
    { command: findGesu(), args: ["lsp"] },
    { documentSelector: [{ language: "gesu" }] }
  );
  client.start().catch((error) => {
    window.showWarningMessage(
      `Gesù: could not start the language server (${error.message}). Syntax colors still work. Set "gesu.path" to the gesu binary.`
    );
  });
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
