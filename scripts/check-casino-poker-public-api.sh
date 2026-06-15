#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="$(mktemp -d "${TMPDIR:-/tmp}/casino-public-api.XXXXXX")"
actual="$target_dir/casino_poker-1.0.txt"
expected="$root/docs/public-api/casino_poker-1.0.txt"
trap 'rm -rf "$target_dir"' EXIT

RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR="$target_dir" \
  cargo rustdoc --manifest-path "$root/Cargo.toml" -p casino_poker --locked --lib -- \
  -Z unstable-options --output-format json

cargo public-api \
  --rustdoc-json "$target_dir/doc/casino_poker.json" \
  -ss --color never > "$actual"

diff -u "$expected" "$actual"
