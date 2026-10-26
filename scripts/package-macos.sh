#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
arch="$(go env GOARCH)"
go tool mygo build -platform "darwin/$arch" -skip-dmg .
ditto -c -k --keepParent "dist/darwin-$arch/NSB.app" "dist/NSB-macos-$arch.zip"
