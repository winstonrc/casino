#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
required_toolchain="1.96.0"
required_public_api="0.52.0"

read -r _ rustc_version _ < <(rustc --version)
read -r _ cargo_version _ < <(cargo --version)
read -r _ public_api_version < <(cargo public-api --version)

if [[ "$rustc_version" != "$required_toolchain" || "$cargo_version" != "$required_toolchain" ]]; then
  printf 'casino_poker public API check requires Rust %s (found rustc %s, cargo %s)\n' \
    "$required_toolchain" "$rustc_version" "$cargo_version" >&2
  exit 1
fi

if [[ "$public_api_version" != "$required_public_api" ]]; then
  printf 'casino_poker public API check requires cargo-public-api %s (found %s)\n' \
    "$required_public_api" "$public_api_version" >&2
  exit 1
fi

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
