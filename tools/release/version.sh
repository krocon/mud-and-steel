#!/usr/bin/env bash
# Prints the game's version: `version` of [workspace.package] in Cargo.toml,
# the one source (prompt 044). godot/project.godot repeats it in
# application/config/version; crates/bridge/tests/release.rs keeps both equal.
# With a tag argument (e.g. v0.1.0) it fails unless the tag is v<version>.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
version="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml")"
if [[ -z "$version" ]]; then
    echo "error: no version in [workspace.package] of Cargo.toml" >&2
    exit 1
fi
if [[ $# -gt 0 && "$1" != "v$version" ]]; then
    echo "error: tag $1 does not match the version $version (expected v$version)" >&2
    exit 1
fi
echo "$version"
