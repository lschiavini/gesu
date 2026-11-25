#!/bin/sh
# Compiles the tree-sitter parser into parser/gesu.so, where Neovim finds
# it. Run once, and again when tree-sitter-gesu/grammar.js changes.
# Needs a C compiler and Node (for npx).
set -e
cd "$(dirname "$0")"
mkdir -p parser
npx --yes tree-sitter-cli build -o "$PWD/parser/gesu.so" ../../tree-sitter-gesu
echo "built: $PWD/parser/gesu.so"
