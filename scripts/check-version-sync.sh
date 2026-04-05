#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

EXPECTED=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
FAILED=0

check() {
    local file="$1"
    local actual="$2"
    if [[ "$actual" == "$EXPECTED" ]]; then
        echo "  ok  $file ($actual)"
    else
        echo "  FAIL $file (expected $EXPECTED, got $actual)"
        FAILED=1
    fi
}

echo "Expected version: $EXPECTED"
echo ""

check "Cargo.toml" "$EXPECTED"

for pkg in \
    "npm/term-survivors/package.json" \
    "npm/@term-survivors/darwin-arm64/package.json" \
    "npm/@term-survivors/darwin-x64/package.json" \
    "npm/@term-survivors/linux-x64/package.json" \
    "npm/@term-survivors/linux-arm64/package.json" \
    "npm/@term-survivors/win32-x64/package.json"; do
    actual=$(grep '"version"' "$pkg" | head -1 | sed 's/.*"version": "\(.*\)".*/\1/')
    check "$pkg" "$actual"
done

readme_ver=$(grep 'term-survivors [0-9]' README.md | head -1 | sed 's/.*term-survivors \([0-9][^ ]*\).*/\1/')
check "README.md" "$readme_ver"

echo ""
if [[ $FAILED -eq 0 ]]; then
    echo "All versions match."
else
    echo "Version mismatch detected."
    exit 1
fi
