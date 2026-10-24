#!/usr/bin/env bash
# Prints one version's section of CHANGELOG.md, for the release notes.
#
#   scripts/changelog-section.sh 0.1.0
#
# A section starts at a heading like "## 0.1.0" or "## 0.1.0 (unreleased)" and
# runs until the next "## " heading. The heading itself is left out, because
# the release already has a title.

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ $# -ne 1 ]]; then
    echo "usage: scripts/changelog-section.sh <version, like 0.1.0>" >&2
    exit 2
fi
version="$1"

section="$(awk -v version="$version" '
    /^## / {
        if (in_section) exit
        # Match "## 0.1.0" exactly or followed by a space, so 0.1.0 never matches 0.1.10.
        if ($2 == version) { in_section = 1; next }
    }
    in_section { print }
' "$repo/CHANGELOG.md")"

if [[ -z "${section//[[:space:]]/}" ]]; then
    echo "CHANGELOG.md has no section for $version; add one before tagging v$version." >&2
    exit 1
fi

echo "$section"
