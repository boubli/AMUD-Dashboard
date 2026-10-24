#!/usr/bin/env bash
# Mirror .github/workflows/ci.yml locally before push.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> cargo fmt (auto-fix)"
cargo fmt --all

echo "==> cargo fmt --check"
cargo fmt --all -- --check

echo "==> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo test (lib)"
cargo test --workspace --lib

if [[ -d docs ]] && [[ -f docs/package-lock.json ]]; then
  echo "==> docs build"
  (cd docs && npm ci --ignore-scripts && npm run build)
fi

echo "All CI checks passed."
