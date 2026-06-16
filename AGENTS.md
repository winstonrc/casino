# Repository Notes For Coding Agents

## Formatting

- Always run `cargo fmt --all` before committing Rust changes
- If you touch Markdown, also run Prettier on the touched Markdown files before the final diff
- Do not assume `cargo fmt` formats docs, changelogs, README files, or workflow prose

## Public API Snapshots

- `casino_poker` public API changes must update `docs/public-api/casino_poker-1.0.txt`
- Reproduce the CI gate with `scripts/check-casino-poker-public-api.sh`
- If the snapshot diff is intentional, regenerate it with the same `cargo-public-api` output and include the snapshot update in the PR

## PR Hygiene

- Keep formatting-only churn separate from behavioral changes when practical
- Run the exact failing CI script locally after fixing a CI-only failure
