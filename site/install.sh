#!/bin/sh
# Installs the latest gesu for this machine into ~/.local/bin.
#   curl -fsSL https://gesu.lucas-schiavini.com/install.sh | sh
set -e

case "$(uname -s)" in
    Darwin) os="apple-darwin" ;;
    Linux) os="unknown-linux-gnu" ;;
    *) echo "gesu: no binary for $(uname -s). Build from source: https://github.com/lschiavini/gesu" >&2; exit 1 ;;
esac
case "$(uname -m)" in
    arm64 | aarch64) arch="aarch64" ;;
    x86_64 | amd64) arch="x86_64" ;;
    *) echo "gesu: no binary for $(uname -m). Build from source: https://github.com/lschiavini/gesu" >&2; exit 1 ;;
esac

url="https://github.com/lschiavini/gesu/releases/latest/download/gesu-$arch-$os.tar.gz"
dir="${GESU_INSTALL_DIR:-$HOME/.local/bin}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "downloading $url"
curl -fsSL "$url" -o "$tmp/gesu.tar.gz"
tar -xzf "$tmp/gesu.tar.gz" -C "$tmp"
mkdir -p "$dir"
mv "$tmp/gesu" "$dir/gesu"
chmod +x "$dir/gesu"

echo "installed: $dir/gesu"
case ":$PATH:" in
    *":$dir:"*) ;;
    *) echo "add it to your PATH:  export PATH=\"$dir:\$PATH\"" ;;
esac
echo "There is only one Truth, Jesus."
