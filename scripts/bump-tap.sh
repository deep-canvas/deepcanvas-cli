#!/usr/bin/env bash
# Update deep-canvas/homebrew-tap Formula/deep.rb to a new release version.
# Run from the homebrew-tap repo root.
#
# Usage:
#   ./bump-tap.sh 0.1.0
#   ./bump-tap.sh v0.1.0

set -euo pipefail

if [ "$#" -lt 1 ]; then
    echo "usage: $0 <version>  (e.g. 0.1.0 or v0.1.0)" >&2
    exit 1
fi

VERSION="${1#v}"
TAG="v${VERSION}"
REPO="deep-canvas/deepcanvas-cli"
FORMULA="Formula/deep.rb"

if [ ! -d Formula ]; then
    echo "error: Formula/ not found. Run from the homebrew-tap repo root." >&2
    exit 1
fi

CHECKSUMS_FILE="deep-${TAG}-checksums.txt"
echo "→ Fetching ${CHECKSUMS_FILE} from ${REPO} ${TAG}"

if ! command -v gh >/dev/null; then
    echo "error: gh CLI not found. Install with: brew install gh" >&2
    exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
    echo "error: gh not authenticated. Run: gh auth login" >&2
    exit 1
fi

TMP=$(mktemp -d)
trap 'rm -rf "${TMP}"' EXIT

gh release download "${TAG}" \
    --repo "${REPO}" \
    --pattern "${CHECKSUMS_FILE}" \
    --dir "${TMP}" \
    --clobber

CHECKSUMS=$(cat "${TMP}/${CHECKSUMS_FILE}")
SHA_ARM_DARWIN=$(echo "${CHECKSUMS}" | grep "aarch64-apple-darwin.tar.gz" | awk '{print $1}')

if [ -z "${SHA_ARM_DARWIN}" ]; then
    echo "error: aarch64-apple-darwin SHA not found in checksums" >&2
    exit 1
fi

echo "→ SHA (aarch64-apple-darwin): ${SHA_ARM_DARWIN}"
echo "→ Writing ${FORMULA}"

cat > "${FORMULA}" <<EOF
class Deep < Formula
  desc "DeepCanvas CLI — task and document context for coding agents"
  homepage "https://deepcanvas.studio"
  version "${VERSION}"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/${REPO}/releases/download/v#{version}/deep-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "${SHA_ARM_DARWIN}"
    end
  end

  def install
    bin.install "deep"
  end

  test do
    assert_match "deep #{version}", shell_output("#{bin}/deep --version")
  end
end
EOF

echo "→ git commit"
git add "${FORMULA}"
git commit -m "deep ${TAG}"

echo "→ git push"
git push

echo
echo "✓ Done. Test:"
echo "    brew uninstall deep 2>/dev/null || true"
echo "    brew untap deep-canvas/tap 2>/dev/null || true"
echo "    brew install deep-canvas/tap/deep"
echo "    deep --version"
