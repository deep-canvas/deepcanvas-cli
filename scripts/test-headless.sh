#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"

echo "=== tasks ==="
cargo run --quiet --bin deep -- --headless tasks

echo
echo "=== pull (first task in list) ==="
CODE=$(cargo run --quiet --bin deep -- --headless tasks | jq -r '.tasks[0].code')
cargo run --quiet --bin deep -- --headless pull "$CODE"

echo
echo "=== done ==="
cargo run --quiet --bin deep -- --headless done
