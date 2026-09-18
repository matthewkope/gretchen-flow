#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$(uname -m)" != arm64 ]; then
  echo 'The Parakeet app build currently requires Apple Silicon.' >&2
  exit 1
fi
swift build -c release --package-path parakeet
mkdir -p parakeet/staging
cp parakeet/.build/release/gretchen-parakeet parakeet/staging/
for resource in parakeet/.build/release/*.bundle; do
  [ ! -d "$resource" ] || ditto "$resource" "parakeet/staging/$(basename "$resource")"
done
cp parakeet/THIRD_PARTY_NOTICES.md parakeet/FluidAudio-LICENSE.txt parakeet/staging/

# SwiftPM checkouts can supply read-only resources; bundling strips xattrs.
chmod -R u+w parakeet/staging
