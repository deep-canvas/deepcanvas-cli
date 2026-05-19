#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/dev-env.sh"

if [ "$#" -ge 1 ]; then
    cargo run --quiet --bin deep -- done "$1"
else
    cargo run --quiet --bin deep -- done
fi
