#!/usr/bin/env bash
# Run the full workspace test suite with a single summary line at the end.
set -euo pipefail

cd "$(dirname "$0")/.."

if command -v cargo-nextest >/dev/null 2>&1; then
  exec cargo nextest run --workspace "$@"
fi

echo "cargo-nextest not found; falling back to cargo test (noisy output)." >&2
echo "Install for a clean summary: cargo install cargo-nextest --locked --version 0.9.114" >&2
echo >&2

cargo test --workspace "$@"
