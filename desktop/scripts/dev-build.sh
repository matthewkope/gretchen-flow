#!/usr/bin/env bash
#
# Local dev build that keeps macOS permissions (Accessibility, Microphone,
# Input Monitoring) across rebuilds.
#
# The released app is ad-hoc signed (`signingIdentity: "-"` in tauri.conf.json),
# which gives every build a NEW code identity — so macOS wipes the TCC grants on
# each rebuild and you have to re-approve everything. This script instead signs
# every build with the stable self-signed "Gretchen Flow Dev" certificate. Its
# designated requirement is
#
#   identifier "com.matthewkope.gretchenflow" and certificate leaf = H"<cert>"
#
# which depends only on the bundle id + the cert (both stable), NOT the cdhash.
# So TCC keeps the grants across rebuilds: approve once, never again.
#
# One-time setup if the cert is missing: create a self-signed code-signing
# certificate named exactly "Gretchen Flow Dev" in Keychain Access
# (Certificate Assistant ▸ Create a Certificate ▸ Code Signing).
#
# Usage:  desktop/scripts/dev-build.sh        # build, sign, install to /Applications, launch
#         desktop/scripts/dev-build.sh --no-install   # build + sign only
set -euo pipefail

# Must match the identity CI signs releases with (release.yml's SIGNING_IDENTITY
# secret) so local builds and Homebrew installs share one code identity — then a
# single permission grant covers both. Backup of this cert: ~/gretchen-flow-signing.
CERT="Gretchen Flow Signing"
APP_NAME="Gretchen Flow.app"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"   # desktop/
TAURI_DIR="$HERE/src-tauri"
BUNDLE="$TAURI_DIR/target/debug/bundle/macos/$APP_NAME"
ENTITLEMENTS="$TAURI_DIR/entitlements.plist"
INSTALLED="/Applications/$APP_NAME"

# NOTE: no -v — a self-signed cert is "not trusted", so `-v` (valid only) hides
# it even though codesign signs with it fine.
if ! security find-identity -p codesigning | grep -q "$CERT"; then
  echo "error: code-signing certificate \"$CERT\" not found in the keychain." >&2
  echo "Create it via Keychain Access ▸ Certificate Assistant ▸ Create a Certificate" >&2
  echo "(name it exactly \"$CERT\", type: Code Signing), then re-run." >&2
  exit 1
fi

echo "==> building (cargo tauri build --debug)"
( cd "$TAURI_DIR" && cargo fmt && cargo tauri build --debug )

echo "==> signing $APP_NAME with \"$CERT\" (stable identity)"
codesign --force --deep --options runtime \
  --entitlements "$ENTITLEMENTS" \
  --sign "$CERT" "$BUNDLE"

echo "==> designated requirement:"
codesign -d -r- "$BUNDLE" 2>&1 | sed -n 's/^designated => /    /p'

if [[ "${1:-}" == "--no-install" ]]; then
  echo "==> done (not installed). Bundle: $BUNDLE"
  exit 0
fi

echo "==> installing to $INSTALLED"
osascript -e "quit app \"Gretchen Flow\"" 2>/dev/null || true
sleep 1; pkill -f "$APP_NAME" 2>/dev/null || true; sleep 1
rm -rf "$INSTALLED"
cp -R "$BUNDLE" /Applications/
xattr -dr com.apple.quarantine "$INSTALLED" 2>/dev/null || true

echo "==> launching"
open "$INSTALLED"
echo "==> done. Permissions granted once to this identity now persist across rebuilds."
