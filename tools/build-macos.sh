#!/bin/bash
# The macOS client as one installable file: a signed (and, if you set up the
# notarytool profile, notarized) .dmg with the app and a link to Applications.
#
#   tools/build-macos.sh            -> build/StartupSim-<version>.dmg
#   tools/build-macos.sh --app-only -> build/Startup Sim.app, unsigned (sign it
#                                      yourself with tools/macos/entitlements.plist)
#
# Needs: Godot 4.7.2 + its export templates, the "Developer ID Application"
# certificate in the keychain, and permission for the terminal to control
# Finder (it lays out the .dmg window; the background is
# tools/macos/dmg-background.tiff).
#
# Who signs: IDENTITY (and optionally NOTARY_PROFILE) from the environment or
# from tools/macos/signing.env (not in the repository), e.g.
#   IDENTITY="Developer ID Application: Jan Kowalski (TEAMID1234)"
#   NOTARY_PROFILE=notarytool   # xcrun notarytool store-credentials notarytool --apple-id <you> --team-id <TEAMID>
# The servers the build offers: client/net/servers.cfg (see servers.example.cfg).
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
[ -f "$ROOT/tools/macos/signing.env" ] && . "$ROOT/tools/macos/signing.env"
IDENTITY=${IDENTITY:-}
PROFILE=${NOTARY_PROFILE:-}
APP="$ROOT/build/Startup Sim.app"
MIC="Czat głosowy w grze: mówisz do osób w tym samym pomieszczeniu, trzymając V (albo B — szept)."

if [ "${1:-}" != "--app-only" ] && [ -z "$IDENTITY" ]; then
  echo "brak IDENTITY (Developer ID) — ustaw w środowisku albo w tools/macos/signing.env, albo użyj --app-only"
  exit 1
fi
[ -f "$ROOT/client/net/servers.cfg" ] || echo "uwaga: brak client/net/servers.cfg — klient będzie znał tylko serwer lokalny"

rm -rf "$ROOT/build" && mkdir -p "$ROOT/build"
echo "== przeglądarka w grze (godot_wry ze źródeł)"
"$ROOT/tools/build_webview.sh"
echo "== eksport z Godota"
godot --headless --path "$ROOT/client" --import >/dev/null 2>&1 || true
godot --headless --path "$ROOT/client" --export-release "macOS" "$APP"
[ -d "$APP" ] || { echo "brak $APP"; exit 1; }

PLIST="$APP/Contents/Info.plist"
VERSION=$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "$PLIST")
# The microphone text must be there, or macOS kills the app on the first V.
/usr/libexec/PlistBuddy -c "Print :NSMicrophoneUsageDescription" "$PLIST" >/dev/null 2>&1 \
  || /usr/libexec/PlistBuddy -c "Add :NSMicrophoneUsageDescription string $MIC" "$PLIST"

if [ "${1:-}" = "--app-only" ]; then
  echo "gotowe (bez podpisu): $APP — wersja $VERSION"
  echo "podpis: codesign --force --timestamp --options runtime --entitlements tools/macos/entitlements.plist --sign \"<Developer ID>\" \"$APP\""
  exit 0
fi

echo "== podpis ($IDENTITY)"
# Inside out: libraries first, then the app (hardened runtime, timestamp).
find "$APP/Contents" -type f \( -name "*.dylib" -o -name "*.so" \) -print0 | while IFS= read -r -d '' lib; do
  codesign --force --timestamp --options runtime --sign "$IDENTITY" "$lib"
done
# GDExtension frameworks (the in-game browser) as whole bundles.
find "$APP/Contents/Frameworks" -maxdepth 1 -name "*.framework" -print0 2>/dev/null | while IFS= read -r -d '' fw; do
  codesign --force --timestamp --options runtime --sign "$IDENTITY" "$fw"
done
codesign --force --timestamp --options runtime --entitlements "$ROOT/tools/macos/entitlements.plist" --sign "$IDENTITY" "$APP"
codesign --verify --strict --deep --verbose=2 "$APP"

echo "== obraz dysku"
DMG="$ROOT/build/StartupSim-$VERSION.dmg"
STAGE="$ROOT/build/dmg"
RW="$ROOT/build/rw.dmg"
mkdir -p "$STAGE/.background"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Aplikacje"
cp "$ROOT/tools/macos/dmg-background.tiff" "$STAGE/.background/background.tiff"
hdiutil create -volname "Startup Sim" -srcfolder "$STAGE" -fs HFS+ -ov -format UDRW "$RW" >/dev/null
rm -rf "$STAGE"
# The window layout lives in the volume's .DS_Store, which only Finder writes:
# mount read-write, let Finder arrange it, then compress.
MOUNT=$(hdiutil attach "$RW" -noautoopen | grep -o '/Volumes/.*$')
osascript <<EOF
tell application "Finder"
  tell disk "$(basename "$MOUNT")"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set bounds of container window to {200, 120, 860, 548}
    set opts to icon view options of container window
    set arrangement of opts to not arranged
    set icon size of opts to 128
    set text size of opts to 14
    set background picture of opts to file ".background:background.tiff"
    set extension hidden of item "Startup Sim.app" to true
    set position of item "Startup Sim.app" of container window to {170, 210}
    set position of item "Aplikacje" of container window to {490, 210}
    update without registering applications
    delay 1
    close
  end tell
end tell
EOF
# After Finder: it drops a .VolumeIcon.icns that is already there.
cp "$APP/Contents/Resources/icon.icns" "$MOUNT/.VolumeIcon.icns"
SetFile -a C "$MOUNT"
sync
hdiutil detach "$MOUNT" -quiet
hdiutil convert "$RW" -format UDZO -imagekey zlib-level=9 -ov -o "$DMG" >/dev/null
rm -f "$RW"
codesign --force --timestamp --sign "$IDENTITY" "$DMG"

if [ -n "$PROFILE" ] && xcrun notarytool history --keychain-profile "$PROFILE" >/dev/null 2>&1; then
  echo "== notaryzacja (profil $PROFILE)"
  xcrun notarytool submit "$DMG" --keychain-profile "$PROFILE" --wait
  xcrun stapler staple "$DMG"
  spctl --assess --type open --context context:primary-signature -v "$DMG" || true
else
  echo "== bez notaryzacji (brak profilu notarytool '$PROFILE') — podpisany, ale macOS zapyta przy pierwszym otwarciu"
fi
echo "gotowe: $DMG ($(du -h "$DMG" | cut -f1))"
