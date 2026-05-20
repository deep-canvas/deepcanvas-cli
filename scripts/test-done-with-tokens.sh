#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"

if [ "$#" -lt 1 ]; then
    echo "usage: $0 <task-code>"
    echo "       Önce pull edilmiş, .deep/<code>/.state.json olmalı."
    exit 1
fi
CODE="$1"

if [ ! -f ".deep/$CODE/.state.json" ]; then
    echo "warning: .deep/$CODE/.state.json yok — agent_session null gönderilecek"
fi

echo "=== state ==="
cat ".deep/$CODE/.state.json" 2>/dev/null || echo "(no state)"

echo
echo "=== deep done (headless) ==="
cargo run --quiet --bin deep -- --headless done "$CODE"
