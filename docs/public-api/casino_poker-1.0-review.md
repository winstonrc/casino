# casino_poker 1.0 Public API Review

This inventory was prepared from the 393-line simplified output of
`cargo-public-api 0.52.0`:

```sh
cargo +nightly public-api -p casino_poker -sss --color never
```

It classifies the current exports before the 1.0 refactor. The generated API
snapshot will be committed after these decisions are implemented, rather than
preserving the current accidental surface as a compatibility baseline.

## Crate Surface

| Current API | Decision |
| --- | --- |
| `casino_cards` re-export | Retain. Poker API consumers need the exact card types used by the crate. |
| `uuid` re-export | Retain. Consumers need the exact player-identifier type. |
| `agent` module | Retain the decision/view API, but remove engine-owned orchestration. |
| `betting` module | Retain actions and caller-facing errors; make betting implementation details private. |
| `events` module | Retain serializable event values; remove observer infrastructure. |
| `games::texas_hold_em` | Retain as the stable engine facade. Keep its implementation modules private. |
| `hand_rankings` module | Retain with the fallible evaluator redesign. |
| `player` module | Retain player values with private engine-owned fields and validated mutation. |
| `pot` module | Make pot construction and distribution private; expose read-only pot views through the engine facade. |

## Agent API

| Current API | Decision |
| --- | --- |
| `Street` | Retain. |
| `PlayerAction`, `LegalAction` re-exports | Retain from one canonical public path. |
| `PlayerView`, `HandMetrics`, `PlayerViewBuilder` | Retain as owned, serializable, non-exhaustive values. |
| `PokerAgent` | Retain as an optional application-side decision interface. The engine will not own or invoke agents. |
| `AgentError` | Retain with `PokerAgent`; it remains application orchestration state, not an engine transition result. |

## Betting API

| Current API | Decision |
| --- | --- |
| `PlayerAction`, `LegalAction`, `ActionError` | Retain as caller-facing protocol types. |
| `Resolved` | Make private. It is an intermediate validation result. |
| `BettingRound` | Make private. It is engine state, not a supported standalone abstraction. |
| `amount_owed`, `resolve_action`, `legal_actions` | Make private and expose their results through pending-action views and engine transitions. |

## Event API

| Current API | Decision |
| --- | --- |
| `GameEvent`, `ActionView`, `Blind`, `PotKind`, `SeatInfo` | Retain as non-exhaustive serializable event values. |
| `GameObserver`, `NullObserver`, `BroadcastObserver` | Remove. Transitions return events for applications to persist, broadcast, render, or observe. |

## Texas Hold'em Types

| Current API | Decision |
| --- | --- |
| `TexasHoldEm` | Retain as the deterministic, resumable state machine. Remove direct serde representation. |
| `DecisionId`, `PendingAction` | Retain. They provide exactly-once action submission and an owned player view. |
| `HandStep` | Redesign as the step carried by an event-bearing transition result. |
| `ActionSubmissionError` | Retain and extend as needed for transactional caller errors. |
| `HandStartError` | Replace with focused configuration, transition, and restoration errors. |
| `BettingStep`, `RoundOutcome` | Remove. Betting-round internals will not be a parallel public state machine. |
| `PlayError`, `HandOutcome` | Remove with the blocking agent loops. |
| `TableView`, `SeatView`, `ClientView` | Retain as owned, non-exhaustive read models; ensure private cards never enter public views. |

## Texas Hold'em Operations

| Current API | Decision |
| --- | --- |
| `new`, `new_seeded` | Retain as fallible constructors; deterministic construction uses a stable library-owned seed contract. |
| `reseed` | Remove. Replacing randomness mid-game conflicts with reproducible snapshots. |
| `set_hero`, `set_observer`, `replay_log` | Remove. Perspective, rendering, and event delivery belong to applications. |
| `public_events` | Retain as an immutable current-hand event accessor until returned transition events fully replace polling needs. |
| `drive_hand`, `submit_hand_action`, `pending_action` | Retain as the canonical nonblocking engine interface. |
| `play_hand`, `run_betting_round` | Remove. Terminal, simulation, and network applications pump the nonblocking interface. |
| `begin_betting_round`, `submit_action`, `abort_betting_round` | Make private. They are lower-level transition machinery. |
| `begin_hand` | Make private; `drive_hand` owns hand lifecycle sequencing. |
| `begin_hand_with_deck` | Remove from the public API; keep deterministic deck injection as crate-internal test support. |
| Player creation helpers | Remove from the engine; construct players through the player API. |
| Seating, removal, dealer, blind, buy-in, and chip operations | Retain with focused result types and transactional validation. |
| `check_for_game_over`, `end_game`, `remove_losers` | Remove. Tournament lifecycle policy belongs to applications. |
| Seat-index getters | Retain only if needed by final views; rename idiomatically and return `Option` when no seat exists. |
| Board, player, seat, street, pot, and action accessors | Retain as immutable queries, consolidating overlapping accessors where possible. |
| `table`, `client_view` | Retain as the preferred state-projection APIs. |
| `snapshot`, `restore` | Add as the only supported engine persistence boundary, using opaque versioned bytes. |

## Hand Rankings

| Current API | Decision |
| --- | --- |
| `HandCategory`, `ComparableHand` | Retain. Keep ordering and tiebreak representation documented. |
| `EvaluatedHand` | Add with public `value: ComparableHand` and `cards: [Card; 5]`. |
| `evaluate`, `evaluate_with_cards` | Replace with `evaluate_holdem`. |
| `best_five`, `best_five_with_cards` | Replace with one fallible `best_five` returning `EvaluatedHand`. |
| `best_omaha` | Replace with fallible `evaluate_omaha`. |
| Exact five-card evaluation | Add `evaluate_five`. |

All evaluators reject duplicate physical cards by rank and suit, ignore
`face_up` for identity, and accept only poker-relevant bounded card counts.

### Performance Baseline

Criterion 0.8.2 produced these local release-mode medians before evaluator
optimization:

| Workload | Baseline |
| --- | ---: |
| Exact five-card hand | 46.97 ns |
| Seven-card Hold'em hand | 669.80 ns |
| Omaha hand | 1.67 us |
| Nine-player Hold'em showdown | 6.19 us |

These values are review context, not portable CI thresholds. Future performance
PRs should run the same benchmark suite before and after changes on the same
machine.

## Player And Pot APIs

| Current API | Decision |
| --- | --- |
| `Player` | Retain, but make fields private and expose validated construction and immutable accessors. |
| `PlayerRef` | Retain as the owned identity used in events and views. |
| Unchecked chip mutators | Remove from the public API; engine operations own checked bankroll mutation. |
| `Pot`, `PotAward` | Replace with non-exhaustive read-only view types where required publicly. |
| `build_pots`, `distribute_pots`, `refund_uncalled` | Make private engine implementation details. |

## Compatibility Gate

After the refactor:

1. Generate the simplified `casino_poker` API with the command above.
2. Commit that output as the 1.0 compatibility baseline.
3. Add CI that rejects unreviewed differences from the baseline.
4. Keep downstream compile tests for the terminal application alongside the
   generated API check.
