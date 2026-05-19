#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"
echo "→ API: $DEEPCANVAS_API_URL"
echo "→ FE:  $DEEPCANVAS_FRONTEND_URL"
echo
cargo run --quiet --bin deep -- login
