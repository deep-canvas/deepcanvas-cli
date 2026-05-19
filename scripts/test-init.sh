#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"
if [ "$#" -lt 1 ]; then
    echo "usage: $0 <org-slug>/<project-slug>"
    exit 1
fi
cargo run --quiet --bin deep -- init "$1"
