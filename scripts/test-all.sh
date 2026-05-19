#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"

if [ "$#" -lt 2 ]; then
    echo "usage: $0 <org-slug>/<project-slug> <task-code>"
    exit 1
fi
PROJECT="$1"
TASK="$2"

echo "=== build ==="
cargo build --quiet

echo
echo "=== deep --help ==="
cargo run --quiet --bin deep -- --help

echo
echo "Run: ./scripts/test-login.sh (separately if not already logged in)"

echo
echo "=== deep init $PROJECT ==="
cargo run --quiet --bin deep -- init "$PROJECT" || \
    echo "(already initialized — skipping)"

echo
echo "=== deep tasks ==="
cargo run --quiet --bin deep -- tasks

echo
echo "=== deep pull $TASK ==="
cargo run --quiet --bin deep -- pull "$TASK"

echo
echo "=== .deep/$TASK/ output ==="
ls -la ".deep/$TASK/"
echo
cat ".deep/$TASK/task.md"
