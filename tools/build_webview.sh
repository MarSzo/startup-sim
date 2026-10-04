#!/usr/bin/env bash
# The in-game web browser: godot_wry (MIT, https://github.com/doceazedo/godot_wry)
# - a native WebView (WebKit on macOS) as a GDExtension. Built here from
# source at a pinned commit, never from prebuilt binaries:
#
#   tools/build_webview.sh        # -> client/addons/godot_wry/ (not in git)
#
# Patched with tools/webview/*.patch. Without it the game still runs: the
# office browser then only offers to open the page in the player's own
# browser. macOS only for now (a universal framework: Apple Silicon + Intel).
set -euo pipefail

TAG="v1.0.2"
COMMIT="7c6f33292eaf781dcb4d2473c7048d576854dd81"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/.cache/godot_wry"  # (build/ is wiped by build-macos.sh)
OUT="$ROOT/client/addons/godot_wry"

[[ "$(uname)" == "Darwin" ]] || { echo "build_webview.sh: macOS only for now" >&2; exit 1; }
command -v cargo >/dev/null || export PATH="$HOME/.cargo/bin:$PATH"

if [[ ! -d "$SRC/.git" ]]; then
    mkdir -p "$ROOT/.cache"
    git clone --quiet --depth 1 --branch "$TAG" https://github.com/doceazedo/godot_wry.git "$SRC"
fi
got="$(git -C "$SRC" rev-parse HEAD)"
if [[ "$got" != "$COMMIT" ]]; then
    echo "build_webview.sh: $SRC is at $got, expected $COMMIT ($TAG) - refusing to build" >&2
    exit 1
fi

# Our patch: the view's rect in window pixels (the game's UI is stretched
# from 1280x720 and sits in canvas layers) - tools/webview/*.patch.
git -C "$SRC" checkout --quiet -- .
for patch in "$ROOT"/tools/webview/*.patch; do
    git -C "$SRC" apply "$patch"
done

rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
cd "$SRC/rust"
cargo build --quiet --target aarch64-apple-darwin --locked --release
cargo build --quiet --target x86_64-apple-darwin --locked --release

fw="$SRC/rust/target/universal/libgodot_wry.framework"
rm -rf "$fw"
mkdir -p "$fw/Resources"
lipo -create -output "$fw/libgodot_wry.dylib" \
    target/aarch64-apple-darwin/release/libgodot_wry.dylib \
    target/x86_64-apple-darwin/release/libgodot_wry.dylib
cp "$SRC/assets/Info.plist" "$fw/Resources/Info.plist"

rm -rf "$OUT"
mkdir -p "$OUT/bin/universal-apple-darwin"
cp -R "$fw" "$OUT/bin/universal-apple-darwin/"
cp "$SRC/godot/addons/godot_wry/WRY.gdextension" "$OUT/"
cp -R "$SRC/godot/addons/godot_wry/icons" "$OUT/"
cp "$SRC/LICENSE" "$OUT/LICENSE"
echo "webview: $OUT ($TAG, $(lipo -archs "$fw/libgodot_wry.dylib"))"
