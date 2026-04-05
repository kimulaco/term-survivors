#!/bin/bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <version>" >&2
    exit 1
fi

VERSION="$1"
DATE=$(date +%Y-%m-%d)
REPO="https://github.com/kimulaco/term-survivors"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in semver format (e.g., 1.2.3)" >&2
    exit 1
fi

cd "$ROOT_DIR"

CURRENT=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
CURRENT_ESCAPED="${CURRENT//./\\.}"

echo "Bumping $CURRENT -> $VERSION"

# Cargo.toml
sed -i '' "s/^version = \"$CURRENT_ESCAPED\"/version = \"$VERSION\"/" Cargo.toml
echo "  Updated Cargo.toml"

# npm packages
for pkg in \
    "npm/term-survivors/package.json" \
    "npm/@term-survivors/darwin-arm64/package.json" \
    "npm/@term-survivors/darwin-x64/package.json" \
    "npm/@term-survivors/linux-x64/package.json" \
    "npm/@term-survivors/linux-arm64/package.json" \
    "npm/@term-survivors/win32-x64/package.json"; do
    sed -i '' "s|\"$CURRENT_ESCAPED\"|\"$VERSION\"|g" "$pkg"
    echo "  Updated $pkg"
done

# CHANGELOG.md — insert new section after "# Changelog"
TEMP=$(mktemp)
awk -v ver="$VERSION" -v repo="$REPO" -v date="$DATE" '
NR == 1 {
    print
    print ""
    print "## [" ver "](" repo "/releases/tag/v" ver ") - " date
    print ""
    print "- "
    next
}
{ print }
' CHANGELOG.md > "$TEMP" && mv "$TEMP" CHANGELOG.md
echo "  Updated CHANGELOG.md"

# README.md — update version in help output example
sed -i '' "s/term-survivors $CURRENT_ESCAPED$/term-survivors $VERSION/" README.md
echo "  Updated README.md"

echo "Done! Version bumped to $VERSION"
echo "Note: Fill in the CHANGELOG.md entry before committing."
