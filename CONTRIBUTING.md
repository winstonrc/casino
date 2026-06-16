# Contributing

## Safe Rust only

All workspace-owned Rust code must use safe Rust exclusively. Never introduce
`unsafe` blocks, functions, traits, implementations, FFI declarations, or lint
overrides that permit unsafe code. This rule applies to library code, tests,
examples, benchmarks, build scripts, and generated Rust code committed to the
repository.

Performance work must remain in safe Rust and be supported by profiling,
benchmarks, and correctness tests. Dependencies that use unsafe code internally
are evaluated separately during dependency review.

## Commit attribution

Do not add AI tools, assistants, or their vendors as commit co-authors. Do not
add AI-attribution trailers or notices to commits, pull requests, changelogs, or
source files unless a maintainer explicitly requests them.

## Set up git hooks (one-time)

This repo ships git hooks in [`.githooks/`](.githooks). They are **not** active
until you point git at them once per clone:

```sh
git config core.hooksPath .githooks
```

After that:

- **pre-commit** runs `cargo fmt --all` (fast — formatting only).
- **pre-push** runs clippy + tests (the same gate CI enforces).

Hooks are a convenience, not a substitute for the checks below or for CI — you
can bypass them in a pinch with `git commit --no-verify` / `git push
--no-verify`, but CI remains the source of truth.

## Before you commit

Whether or not you use the hooks above, these are what CI enforces. Running them
locally first avoids red builds and review round-trips.

```sh
# 1. Format (CI runs this with --check and fails on any diff)
cargo fmt --all

# 2. Format any touched Markdown files with Prettier
prettier --write <touched Markdown files>

# 3. Lint — warnings are treated as errors in CI. clippy is a full compile,
#    so it also catches anything a plain `cargo build`/`check` would.
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# 4. Test
cargo test --workspace --all-targets --locked

# 5. Test documentation examples
cargo test --workspace --doc --locked

# 6. Build strict public API documentation
RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --no-deps --locked
```

If `cargo fmt --all` changes any files, include those changes in the same
commit. CI runs `cargo fmt --all --check`, which only _checks_ formatting and
fails on any difference — it does not fix anything for you.

`cargo fmt` does not format Markdown. When you edit Markdown, including
`CHANGELOG.md`, `README.md`, crate READMEs, or docs under `docs/`, run Prettier
on the touched files and include any formatting changes in the same commit.

## Public API snapshots

The `public_api` CI job compares the generated `casino_poker` public API against
[`docs/public-api/casino_poker-1.0.txt`](docs/public-api/casino_poker-1.0.txt).
Any intentional public API change must update that snapshot in the same PR.

Reproduce the CI gate locally with:

```sh
scripts/check-casino-poker-public-api.sh
```

If the diff is expected, regenerate the snapshot with the pinned
`cargo-public-api` output and commit the changed snapshot:

```sh
RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=/tmp/casino-public-api-update \
  cargo rustdoc --manifest-path "$PWD/Cargo.toml" -p casino_poker --locked --lib -- \
  -Z unstable-options --output-format json

cargo public-api \
  --rustdoc-json /tmp/casino-public-api-update/doc/casino_poker.json \
  -ss --color never > docs/public-api/casino_poker-1.0.txt

scripts/check-casino-poker-public-api.sh
```

## Commit messages

Keep commits focused and write messages in the imperative mood
(e.g. "add hand-level state machine").
