# casino_poker 1.0 Public API

This document records the public API decisions for the `casino_poker` 1.0.0
release. It describes the final supported surface rather than earlier
pre-release proposals.

## Supported Surface

- `games::texas_hold_em::TexasHoldEm` provides both the resumable
  `drive_hand`/`submit_hand_action` interface and blocking agent-driven
  convenience methods.
- `agent` contains player decision snapshots, legal actions, metrics, and the
  optional `PokerAgent` interface.
- `betting` exposes the standalone betting state machine and action-validation
  helpers.
- `events` exposes serializable game events and observer adapters.
- `hand_rankings` exposes checked five-card, Hold'em, and Omaha evaluation.
- `player` exposes player identity and chip-stack values.
- `pot` exposes pot construction, refund, and distribution helpers.
- `casino_cards` and `uuid` are re-exported so consumers can use the exact types
  carried by the API.

`TexasHoldEm` remains directly serializable for 1.0. Restored state is validated
before transitions or awards proceed. Nonblocking transitions are the preferred
integration for servers and asynchronous applications; the blocking wrappers
remain supported for terminal games and simulations.

## Hand Evaluation

The stable evaluator entry points are:

- `evaluate_five([Card; 5])`
- `best_five(&[Card])` for five to seven cards
- `evaluate_holdem([Card; 2], &[Card])` for three to five board cards
- `evaluate_omaha([Card; 4], &[Card])` using exactly two hole and three board
  cards

Each returns `Result<EvaluatedHand, HandEvaluationError>`. Invalid card counts
and duplicate physical cards are errors rather than panics. `EvaluatedHand`
keeps its fields private and exposes `value()` and `cards()` so callers cannot
construct contradictory card/value pairs.
`ComparableHand` likewise validates manually supplied and deserialized category
/ tiebreak pairs; use `ComparableHand::new(...)`, `category()`, and
`tiebreak()`.

The retired pre-release names were `evaluate`, `evaluate_with_cards`,
`best_five_with_cards`, and `best_omaha`.

## Safety And Correctness

- Workspace-owned Rust forbids `unsafe` code.
- Public evaluator input is validated before subset enumeration.
- Showdown evaluation completes before refunds, events, or payouts mutate the
  engine.
- The release test checks all 2,598,960 distinct five-card hands against an
  independent oracle.
- Property tests cover ordering, kicker resolution, and best-of-seven
  selection.
- An integration contract test compiles representative external evaluator and
  agent usage. The sibling terminal and AI workspaces are also tested against
  the local release candidate before publishing.

## Performance Baseline

Criterion 0.8.2 produced these approximate local release-mode medians after the
checked, allocation-free evaluator redesign. They were measured on Linux
`x86_64`, an AMD Ryzen 5 7600X, and Rust 1.96.0:

| Public workload | Median |
| --- | ---: |
| `evaluate_five` | 16.28 ns |
| `evaluate_holdem` with seven cards | 381.97 ns |
| `evaluate_omaha` with nine cards | 935.75 ns |
| Nine-player Hold'em showdown | 3.64 us |

These are local reference values, not portable CI thresholds. They establish a
new baseline for the fallible APIs and should not be compared directly with the
older infallible return-value benchmarks.

## Compatibility Gate

[`casino_poker-1.0.txt`](casino_poker-1.0.txt) is the simplified
`cargo-public-api 0.52.0` snapshot for the supported 1.0 surface, generated with
Rust 1.96.0. CI regenerates the same representation and rejects any difference,
including changes to derived trait implementations. Intentional API changes
therefore require an explicit snapshot review and update.

`casino_poker` depends on `casino_cards = "2"`, so `casino_cards 2.0.0` must be
published before the final `cargo package` verification and `casino_poker`
publication.
