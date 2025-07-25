#!/usr/bin/env bash
# Usage: set-cargo-version.sh [v1.2.3]
# Updates Cargo.toml workspace version (in place) to match given tag (removes leading v if present)
set -e
if [ -z "$1" ]; then
  echo "Usage: $0 <version>"
  exit 1
fi
RAW_VERSION="$1"
# Remove leading v (if present)
VERSION="${RAW_VERSION#v}"

CARGO_TOML="Cargo.toml"

# Update version in [workspace.package]
if [ -f "$CARGO_TOML" ]; then
  # Substitute version =  PROTECTED_0_ 
  perl -pi -e 'BEGIN { $inws=0 } s/^version =  PROTECTED_0_ ]* PROTECTED_1_ '$VERSION'"/ if $inws; if (/^\[workspace\.package\]/) { $inws=1 } elsif (/^\[/ && !/^\[workspace\.package\]/) { $inws=0 }' "$CARGO_TOML"
  echo "Updated $CARGO_TOML: version = $VERSION"
else
  echo "Error: $CARGO_TOML not found."
  exit 2
fi