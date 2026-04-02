#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building spin-lsp..."
cargo build --manifest-path "$REPO_ROOT/Cargo.toml" -p spin-lsp

echo "Installing spin-lsp to ~/.cargo/bin..."
cp "$REPO_ROOT/target/debug/spin-lsp" ~/.cargo/bin/spin-lsp

echo "Installing npm dependencies..."
cd "$SCRIPT_DIR"
npm install --silent

echo "Symlinking extension..."
LINK="$HOME/.vscode/extensions/spin-lang"
if [ -L "$LINK" ]; then
    rm "$LINK"
fi
ln -s "$SCRIPT_DIR" "$LINK"

echo "Done. Reload VS Code (Cmd+Shift+P → Developer: Reload Window)"
