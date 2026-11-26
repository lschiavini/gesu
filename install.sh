#!/bin/sh
# Builds gesu and copies it to ~/.local/bin.
# Pass --python to include the Python pagan (needs Python 3 on this machine).
set -e
cd "$(dirname "$0")"

features=""
if [ "$1" = "--python" ]; then
    features="--features python"
fi
cargo build --release $features
bin=target/release/gesu

if [ -n "$features" ] && [ "$(uname)" = "Darwin" ]; then
    # Some Pythons (mise, pyenv, uv) record a library path that does not
    # exist on this machine. Point the binary at the real library.
    recorded=$(otool -L "$bin" | awk '/libpython/ {print $1}')
    if [ -n "$recorded" ] && [ ! -e "$recorded" ] && [ "${recorded#@}" = "$recorded" ]; then
        python=${PYO3_PYTHON:-python3}
        libdir=$("$python" -c "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))")
        install_name_tool -change "$recorded" "$libdir/$(basename "$recorded")" "$bin"
        codesign --force --sign - "$bin" 2>/dev/null
    fi
fi

mkdir -p "$HOME/.local/bin"
# Remove first: macOS kills a binary overwritten in place while an editor
# still runs the old one.
rm -f "$HOME/.local/bin/gesu"
cp "$bin" "$HOME/.local/bin/gesu"
echo "installed: $HOME/.local/bin/gesu"
