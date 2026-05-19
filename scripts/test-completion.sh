#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"
SHELL_TYPE="${1:-bash}"
cargo run --quiet --bin deep -- completion "$SHELL_TYPE"
