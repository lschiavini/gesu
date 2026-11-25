#!/bin/sh
# Turns og.html into og.png (1200 x 630) with headless Chrome.
set -e
cd "$(dirname "$0")"
chrome="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
"$chrome" --headless=new --hide-scrollbars --force-device-scale-factor=1 \
    --window-size=1200,630 --virtual-time-budget=8000 \
    --screenshot="$PWD/og.png" "file://$PWD/og.html" 2>/dev/null
echo "wrote $PWD/og.png"
