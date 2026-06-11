# Contributing

## Before you commit

Always run the following locally before committing. These mirror what CI
enforces, so running them first avoids red builds and round-trips with the
autofix bot.

```sh
# 1. Format (CI runs this with --check and fails on any diff)
cargo fmt --all

# 2. Lint — warnings are treated as errors in CI. clippy is a full compile,
#    so it also catches anything a plain `cargo build`/`check` would.
cargo clippy --workspace --all-targets --locked -- -D warnings

# 3. Test
cargo test --workspace --locked
```

If `cargo fmt --all` changes any files, include those changes in the same
commit. CI runs `cargo fmt --all --check`, which only _checks_ formatting and
fails on any difference — it does not fix anything for you.

A pull request opens an autofix job that applies `cargo fmt` + `clippy --fix`
and pushes the result back to your branch. Treat this as a safety net, not a
substitute: it only runs on PRs (not direct pushes), and its fix commit does
**not** re-trigger the failed CI run, so the original check stays red until your
next manual push. If autofix pushes a commit, `git pull` before continuing to
avoid diverging from your branch.

## Commit messages

Keep commits focused and write messages in the imperative mood
(e.g. "add hand-level state machine").
