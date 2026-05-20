#!/usr/bin/env bash
# Bump the workspace version, commit, tag, and push.
# Single-source-of-truth for releases: keeps Cargo.toml in sync with the git tag
# so the embedded CARGO_PKG_VERSION matches the tag name.
#
# Usage:
#   ./scripts/bump-version.sh 0.2.4
#   ./scripts/bump-version.sh v0.2.4

set -euo pipefail

if [ "$#" -lt 1 ]; then
    echo "usage: $0 <version>  (e.g. 0.2.4 or v0.2.4)" >&2
    exit 1
fi

VERSION="${1#v}"
TAG="v${VERSION}"

if ! [[ "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9]+)*$ ]]; then
    echo "error: invalid semver '${VERSION}'" >&2
    exit 1
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "${REPO_ROOT}"

CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "${CURRENT_BRANCH}" != "main" ] && [ "${CURRENT_BRANCH}" != "master" ]; then
    echo "error: must be on main/master (current: ${CURRENT_BRANCH})" >&2
    exit 1
fi

DIRTY=$(git status --porcelain | grep -v '^?? ' | grep -vE '^.M (Cargo\.toml|Cargo\.lock)$' || true)
if [ -n "${DIRTY}" ]; then
    echo "error: working tree has unrelated uncommitted changes:" >&2
    echo "${DIRTY}" >&2
    echo "Commit or stash them first." >&2
    exit 1
fi

if git rev-parse "${TAG}" >/dev/null 2>&1; then
    echo "error: tag ${TAG} already exists" >&2
    exit 1
fi

CURRENT=$(grep -m1 '^version' Cargo.toml | sed -E 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/')
echo "→ Bumping ${CURRENT} → ${VERSION}"

if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' -E "s/^(version[[:space:]]*=[[:space:]]*\")[^\"]+(\".*)$/\1${VERSION}\2/" Cargo.toml
else
    sed -i -E "s/^(version[[:space:]]*=[[:space:]]*\")[^\"]+(\".*)$/\1${VERSION}\2/" Cargo.toml
fi

NEW=$(grep -m1 '^version' Cargo.toml | sed -E 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/')
if [ "${NEW}" != "${VERSION}" ]; then
    echo "error: Cargo.toml version edit failed (got '${NEW}')" >&2
    exit 1
fi

echo "→ cargo build (refreshes Cargo.lock)"
cargo build --quiet

echo "→ git commit"
git add Cargo.toml Cargo.lock
git commit -m "chore: bump ${TAG}"

echo "→ git push"
git push

echo "→ git tag ${TAG}"
git tag "${TAG}"

echo "→ git push --tags"
git push --tags

echo
echo "✓ Released ${TAG}"
echo
echo "Next:"
echo "  - Watch Actions: https://github.com/$(git config --get remote.origin.url | sed -E 's|.*github\.com[:/]([^/]+/[^.]+)(\.git)?$|\1|')/actions"
echo "  - When green, bump tap: cd ../homebrew-tap && ./bump.sh ${VERSION}"
