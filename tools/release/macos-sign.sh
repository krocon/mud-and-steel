#!/usr/bin/env bash
# Signs (and optionally notarizes) the exported macOS app (prompt 044).
#
#   tools/release/macos-sign.sh "build/export/macos/Mud and Steel.app"
#
# Without MACOS_SIGN_IDENTITY: ad-hoc signature. Enough to start the game on
# the machine that built it; Gatekeeper on other Macs refuses it (right-click >
# Open, or xattr -dr com.apple.quarantine). Steam does not need notarization,
# but players get the Gatekeeper warning without it.
#
# With a Developer ID (docs/RELEASE.md, "macOS signing"), from the environment
# only, never from files in the repository:
#   MACOS_SIGN_IDENTITY   e.g. "Developer ID Application: Name (TEAMID)"
#   MACOS_CERT_P12_BASE64, MACOS_CERT_PASSWORD
#                         optional (CI): the certificate as base64 .p12; it is
#                         imported into a temporary keychain, deleted after
#   APPLE_ID, APPLE_TEAM_ID, APPLE_APP_PASSWORD
#                         optional: notarize with notarytool and staple
set -euo pipefail

APP="${1:?usage: $0 <app bundle>}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENTITLEMENTS="$HERE/macos/entitlements.plist"
IDENTITY="${MACOS_SIGN_IDENTITY:-}"

if [[ -z "$IDENTITY" ]]; then
    echo "macos-sign: no MACOS_SIGN_IDENTITY, signing ad-hoc"
    codesign --force --deep --sign - --entitlements "$ENTITLEMENTS" "$APP"
    codesign --verify --deep --strict "$APP"
    exit 0
fi

KEYCHAIN=""
if [[ -n "${MACOS_CERT_P12_BASE64:-}" ]]; then
    KEYCHAIN="${RUNNER_TEMP:-$(mktemp -d)}/mud-and-steel-signing.keychain-db"
    KEYCHAIN_PASSWORD="$(uuidgen)"
    P12="$(dirname "$KEYCHAIN")/cert.p12"
    echo "$MACOS_CERT_P12_BASE64" | base64 --decode > "$P12"
    security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
    security set-keychain-settings -lut 21600 "$KEYCHAIN"
    security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
    security import "$P12" -P "${MACOS_CERT_PASSWORD:-}" -A -t cert -f pkcs12 -k "$KEYCHAIN"
    security set-key-partition-list -S apple-tool:,apple: -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN" >/dev/null
    security list-keychains -d user -s "$KEYCHAIN" $(security list-keychains -d user | tr -d '"')
    rm -f "$P12"
    trap 'security delete-keychain "$KEYCHAIN" || true' EXIT
fi

echo "macos-sign: signing with $IDENTITY (hardened runtime)"
# Inside out: libraries first, then the app.
find "$APP/Contents/Frameworks" -type f \( -name '*.dylib' -o -name '*.so' \) -print0 2>/dev/null |
    while IFS= read -r -d '' lib; do
        codesign --force --timestamp --options runtime --sign "$IDENTITY" "$lib"
    done
codesign --force --timestamp --options runtime --entitlements "$ENTITLEMENTS" --sign "$IDENTITY" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

if [[ -n "${APPLE_ID:-}" && -n "${APPLE_TEAM_ID:-}" && -n "${APPLE_APP_PASSWORD:-}" ]]; then
    ZIP="$(mktemp -d)/notarize.zip"
    ditto -c -k --keepParent "$APP" "$ZIP"
    echo "macos-sign: notarizing (takes minutes)"
    xcrun notarytool submit "$ZIP" --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" \
        --password "$APPLE_APP_PASSWORD" --wait
    xcrun stapler staple "$APP"
    spctl --assess --type execute --verbose "$APP"
else
    echo "macos-sign: no APPLE_ID / APPLE_TEAM_ID / APPLE_APP_PASSWORD, not notarized"
fi
