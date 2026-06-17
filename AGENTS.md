# Repository Notes For Coding Agents

## Formatting

- Always run `cargo fmt --all` before committing Rust changes
- If you touch Markdown, use pinned Prettier via `npm run format:md` or `node_modules/.bin/prettier --write <files>` before the final diff
- Do not assume `cargo fmt` formats docs, changelogs, README files, or workflow prose

## Public API Snapshots

- `casino_poker` public API changes must update `docs/public-api/casino_poker-1.0.txt`
- Reproduce the CI gate with `scripts/check-casino-poker-public-api.sh`
- If the snapshot diff is intentional, regenerate it with the same `cargo-public-api` output and include the snapshot update in the PR

## PR Hygiene

- Keep formatting-only churn separate from behavioral changes when practical
- Run the exact failing CI script locally after fixing a CI-only failure

## Publishing

- Never run `cargo publish` without explicit final user approval after package and dry-run checks pass
- It is fine to run `cargo package` and `cargo publish --dry-run` during release prep
- Treat the real publish command as the user's final call, even when all checks are green

## Agent Review Gates

- Run focused agent review after each implementation piece when the user requests it
- Surface blocked reviewer permissions immediately instead of waiting silently
- Close completed agents once their findings have been handled
- For GitHub PR reviews, use `gh api graphql` to fetch review threads and inline comments when `gh pr view` does not expose enough review context
- Read the PR's body for additional context that might be relevant for reviewing
