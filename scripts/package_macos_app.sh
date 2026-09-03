#!/usr/bin/env bash
# Assembles Keres.app — the macOS app bundle Finder/Dock/Launchpad expect —
# from an already-built `gui` binary plus the generated icon. Used by both
# `make macos-app` and the macOS leg of .github/workflows/release.yaml, so
# the bundle layout only needs to be right in one place.
#
# No macOS-only tools required (no iconutil/PlistBuddy): keres.icns is
# pre-built by scripts/gen_app_icon.py, and Info.plist is a static template
# with the version substituted in, so this also runs (and can be sanity
# checked) on Linux — only actually *running* the resulting .app needs a Mac.
#
# Usage: package_macos_app.sh <gui-binary> <output-dir> [version]
set -euo pipefail

BIN_PATH="${1:?usage: package_macos_app.sh <gui-binary> <output-dir> [version]}"
OUT_DIR="${2:?usage: package_macos_app.sh <gui-binary> <output-dir> [version]}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${3:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$REPO_ROOT/Cargo.toml" | head -1)}"

if [ ! -f "$BIN_PATH" ]; then
	echo "error: $BIN_PATH not found (build the gui binary first)" >&2
	exit 1
fi

APP_DIR="$OUT_DIR/Keres.app"
CONTENTS="$APP_DIR/Contents"

rm -rf "$APP_DIR"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

cp "$BIN_PATH" "$CONTENTS/MacOS/keres"
chmod +x "$CONTENTS/MacOS/keres"
cp "$REPO_ROOT/assets/generated/keres.icns" "$CONTENTS/Resources/keres.icns"
sed "s/__VERSION__/$VERSION/g" "$REPO_ROOT/packaging/macos/Info.plist.in" > "$CONTENTS/Info.plist"

echo "packaged $APP_DIR (version $VERSION)"
