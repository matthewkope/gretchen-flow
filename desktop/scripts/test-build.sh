#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
cd src-tauri
cargo tauri build --features test-build --config tauri.test.conf.json --bundles app
printf '\nTest app: %s/target/release/bundle/macos/Gretchen Flow Test.app\n' "$PWD"
