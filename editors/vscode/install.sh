#!/bin/sh
# Packs the extension into a .vsix and installs it.
# Usage: ./install.sh          installs into VS Code
#        ./install.sh cursor   installs into Cursor
set -e
cd "$(dirname "$0")"
editor=${1:-code}
npm install --silent --no-audit --no-fund
npx --yes @vscode/vsce package --allow-missing-repository --skip-license --out gesu.vsix
"$editor" --install-extension gesu.vsix --force
