#!/bin/bash
# The macOS client as one installable file: a signed (and, if you set up the
# notarytool profile, notarized) .dmg with the app and a link to Applications.
#
#   tools/build-macos.sh            -> build/StartupSim-<version>.dmg
#   tools/build-macos.sh --app-only -> build/Startup Sim.app, unsigned (sign it
#                                      yourself with tools/macos/entitlements.plist)
#
# Needs: Godot 4.7.2 + its export templates, the "Developer ID Application"
# certificate in the keychain. Notarization (optional, once):
#   xcrun notarytool store-credentials notarytool --apple-id <you> --team-id 45259QZBRQ
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
IDENTITY=${IDENTITY:-"Developer ID Application: Mateusz Palak (45259QZBRQ)"}
PROFILE=${NOTARY_PROFILE:-notarytool}
APP="$ROOT/build/Startup Sim.app"
MIC="Czat głosowy w grze: mówisz do osób w tym samym pomieszczeniu, trzymając V (albo B — szept)."

rm -rf "$ROOT/build" && mkdir -p "$ROOT/build"
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
codesign --force --timestamp --options runtime --entitlements "$ROOT/tools/macos/entitlements.plist" --sign "$IDENTITY" "$APP"
codesign --verify --strict --deep --verbose=2 "$APP"

echo "== obraz dysku"
DMG="$ROOT/build/StartupSim-$VERSION.dmg"
STAGE="$ROOT/build/dmg"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Aplikacje"
hdiutil create -volname "Startup Sim" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGE"
codesign --force --timestamp --sign "$IDENTITY" "$DMG"

if xcrun notarytool history --keychain-profile "$PROFILE" >/dev/null 2>&1; then
  echo "== notaryzacja (profil $PROFILE)"
  xcrun notarytool submit "$DMG" --keychain-profile "$PROFILE" --wait
  xcrun stapler staple "$DMG"
  spctl --assess --type open --context context:primary-signature -v "$DMG" || true
else
  echo "== bez notaryzacji (brak profilu notarytool '$PROFILE') — podpisany, ale macOS zapyta przy pierwszym otwarciu"
fi
echo "gotowe: $DMG ($(du -h "$DMG" | cut -f1))"
