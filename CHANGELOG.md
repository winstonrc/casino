# Changelog

All notable changes to the crates in this workspace are documented here. The
format is based on [Keep a Changelog](https://keepachangelog.com/), and each
crate follows [Semantic Versioning](https://semver.org/).

---

## 2026-06-16

### casino_poker 1.1.0

#### Added

- `SeatView::builder()` so downstream consumers and agent tests can construct public seat
  snapshots for `PlayerView::builder().seats(...)` without relying on struct
  literals for a `#[non_exhaustive]` type.

---

## 2026-06-15

### casino_poker 1.0.0

First stable release. The backend drives complete Texas Hold'em hands with
correct betting, all-ins, and side pots, and exposes a stable public API.

#### Added

- Fallible hand evaluation through `evaluate_five`, `best_five`,
  `evaluate_holdem`, and `evaluate_omaha`. Each returns an `EvaluatedHand`
  containing the fully ordered, kicker-correct `ComparableHand` value and the
  exact five physical cards forming it. Invalid card counts and duplicate cards
  return `HandEvaluationError` rather than panicking. Omaha evaluation uses
  exactly two of four hole cards and three board cards. The evaluator handles
  the wheel (A-2-3-4-5) and is cross-checked against an independent brute-force
  oracle. `ComparableHand::describe` names hands in PokerStars wording, and
  `ComparableHand` serializes/deserializes with validated tiebreak ranks.
- `agent` module: the `PokerAgent` trait — `decide`, plus default-no-op `observe`
  (receive the `GameEvent` stream to update a player model) and `session_ended`
  (persist what was learned) lifecycle hooks — an owned (serializable) `PlayerView`,
  `LegalAction`, `AgentError`, and `Street`. This is the seam for human, AI, and
  future model-backed players. `PlayerView` is `#[non_exhaustive]` (built by the
  engine; construct one elsewhere — e.g. in agent tests — via `PlayerView::builder()`)
  so future snapshot fields are non-breaking. `PlayerView::metrics()` returns derived,
  ready-to-display `HandMetrics` (pot odds, SPR, stack/call in big blinds, players
  left) for a front-end training overlay; `HandMetrics` is `#[non_exhaustive]` so more
  metrics can be added without breakage.
- `betting` module: `amount_owed`, `resolve_action`, `legal_actions`,
  `PlayerAction`, and the `BettingRound` state machine.
- `pot` module: `Pot`, `refund_uncalled`, `build_pots`, and `distribute_pots` —
  side pots computed at showdown by layering total contributions.
  `distribute_pots` returns one `PotAward` per pot (main then side), so callers
  can report each pot's winners separately rather than only summed winnings.
- `TexasHoldEm` agent-driven hand lifecycle: `begin_hand`, `deal_flop`/`deal_turn`/
  `deal_river`, `run_betting_round`, `award_pots`, `end_hand`, `RoundOutcome`, and
  `set_hero` (the perspective player for hand histories).
- `TexasHoldEm::randomize_seats` shuffles the seating order so the opening dealer
  button (and the blinds) need not always start with the first player added.
- `TexasHoldEm::dealer` / `set_dealer` read and place the dealer button, so a
  front-end can snapshot an in-progress table and restore it on resume.
- `TexasHoldEm::new_seeded` / `reseed` use a deterministic RNG seeded from a
  `u64`, so shuffles and seat randomization are reproducible for replays,
  provably-fair deals, and tests. (The seed is not persisted by serialization;
  call `reseed` after a restore.)
- `TexasHoldEm::begin_hand_with_deck` begins a hand from a caller-supplied
  `Deck` rather than shuffling, for fixed-board scenarios and tests.
- Read-only table accessors for rendering and snapshotting without driving the
  hand: `to_act`, `current_bet`, `committed_this_street`, `has_folded`,
  `is_all_in`, `button_seat`, `pots`, and `pending_action`. `table()` returns a
  single owned, serializable `TableView` (with per-seat `SeatView`s); both are
  `#[non_exhaustive]`.
- `events` module: the engine is **I/O-free** and emits perspective-aware narration as
  serializable `GameEvent`s (`HandStarted`, `HoleCardsDealt`, `BlindPosted`,
  `ActionTaken`, `StreetDealt`, `UncalledBetReturned`, `Showdown`,
  `ShowdownReveal`, `PotAwarded`, `HandComplete`) to a `GameObserver` set via
  `TexasHoldEm::set_observer` (default `NullObserver`) — render or privately log
  them. Network consumers must use `public_events` or an authenticated
  `client_view`. The events carry exactly what a PokerStars-format
  hand history needs: `HandStarted` the seat roster (`SeatInfo`), button, and
  blinds; `ActionTaken` the `Street` and `ActionView::Raised { by, to }`;
  `ShowdownReveal`/`PotAwarded` the full `ComparableHand` (call `describe()` to
  name it); and `PotAwarded` an optional `PotKind` (`Main`/`Side(n)`) for per-pot
  side-pot narration. The public event enums (`GameEvent`, `ActionView`, `PotKind`,
  `Blind`) are `#[non_exhaustive]`, so future variants can be added without a
  breaking change. `BroadcastObserver` fans a single event stream out to several
  `GameObserver`s (notified in order), so e.g. a logger and a renderer can both
  observe one hand.
- **Serializable, resumable game state.** `TexasHoldEm` derives
  `Serialize`/`Deserialize` (the observer and RNG are `#[serde(skip)]` and
  re-attached/re-seeded on restore), so a front-end can serialize an _exact
  in-progress hand_ — deck, hole cards, board, bets, button — and restore it to
  continue from the precise spot. `hand_in_progress` and `current_street` report
  where a restored hand stands.
- **Betting-street state machine** (the non-blocking driving seam):
  `begin_betting_round`, identified `submit_action`, `abort_betting_round`, and
  `BettingStep` (`AwaitingAction(PendingAction)` / `RoundComplete(RoundOutcome)`).
  Each serializable `PendingAction` carries a `DecisionId`; stale, wrong-player, and
  illegal submissions return `ActionSubmissionError` without mutating the engine. The blocking
  `run_betting_round` is a thin wrapper over it and, on a quit, leaves the round
  paused (resumable) rather than aborting.
- **Hand-level state machine.** `drive_hand` begins a fresh hand or resumes the one
  in progress, and `submit_hand_action` feeds each action — together yielding
  `HandStep` (`AwaitingAction` / `HandComplete`). The engine owns the whole
  deal → bet → deal → award sequence, pausing only for player decisions, so a
  front-end need not re-implement hand orchestration.
  `play_hand(agents) -> Result<HandOutcome, PlayError>` is the blocking wrapper.
- **Reconnect support.** `client_view(player_id) -> ClientView` bundles everything
  one player needs to (re)join a game: the public `TableView`, that player's own
  hole cards and identified pending decision, and the hand's events so far —
  leak-safe by construction. Another configured hero's private deal is redacted.
  `replay_log` re-emits the current hand's recorded `GameEvent`s to a
  freshly-attached observer, replaying the hand-so-far narration for the current
  perspective (header, blinds, every action, board).
- `set_blinds` (rising tournament blind levels) and `add_chips_to` (rebuy/top-up),
  each a no-op while a hand is in progress so it can't corrupt live pot accounting.
- **Stable player identity in the event stream.** `player::PlayerRef { id, name }`
  (a `Uuid` plus display name; `Display`s as its name) identifies the player on
  every player-bearing `GameEvent` (`BlindPosted`, `ActionTaken`,
  `UncalledBetReturned`, `ShowdownReveal`, `PotAwarded`, `HoleCardsDealt.hero`) and in
  `SeatInfo`, so a consumer can key a per-opponent model off the stable `id` instead of
  the non-unique name. `Player::to_ref()` builds one.
- **Opponent roster on the decide-side view.** `PlayerView` carries `seats: Vec<SeatView>`
  (the public per-seat roster — id, stack, this-street commitment, fold/all-in status)
  and `button_seat`, so an agent's `decide` sees the same objective table state a
  spectator does and can map a stored model onto the current table (set via the
  `#[non_exhaustive]` builder `PlayerViewBuilder::seats`/`button_seat`).
- **`public_events() -> Vec<GameEvent>`** returns an owned public copy of the current
  hand's event stream with the private hero payload redacted, suitable for agents
  and network broadcast.

#### Changed

- `begin_hand` and `begin_hand_with_deck` return
  `Result<(), HandStartError>`. Seat/button mutators report rejection, and the
  engine owns street dealing, pot awards, and hand cleanup internally.
- `ActionError`, `ActionSubmissionError`, and `PlayError` are non-exhaustive and
  include validation/overflow failures. `BettingStep` and `HandStep` can return
  `CannotStart`.
- `EvaluatedHand` keeps its validated value and physical cards private. Read
  them with `EvaluatedHand::value()` and `EvaluatedHand::cards()`.
- `ComparableHand` keeps category/tiebreak layout validated. Build manual values
  with `ComparableHand::new(...)` and read them with `category()`/`tiebreak()`.
- `AgentError` is non-exhaustive and includes `InvalidView` plus
  `Failure(String)` for downstream context. It implements `Display` and
  `Error`, and blocking `PlayError::Agent` values expose it as their source.
- Standalone `Pot` and `PotAward` aggregate amounts and payouts use `u64`.
- Tables support 2–10 players and enforce a checked table-wide `u32::MAX`
  bankroll. Hand startup, restored state, and action submission validate before
  mutation.
- `abort_betting_round` abandons the whole hand: contributions are refunded,
  cards are returned, and the table is immediately reusable without emitting
  `HandComplete`.
- Restored games reject duplicate cards, impossible board shapes, and decks too
  short to finish before mutating. `client_view` safely omits malformed hole cards
  instead of indexing them.
- `PlayerAction` is now `Fold` / `Check` / `Call` / `RaiseTo(u32)` / `AllIn`
  (raise-to semantics) instead of the previous unit-style variants.
- The engine no longer prints. `remove_losers` returns the removed players'
  names (`Vec<String>`); `add_player` and `check_for_game_over` no longer print;
  `print_leaderboard`/`print_dealer` were removed (render from events/getters).
- `HandCategory` derives serde.
- Showdowns compare `ComparableHand` values, which resolve all kickers and do
  **not** break ties by suit on straights/straight flushes (equal hands chop).
- `HandCategory`/`ComparableHand` print without articles (e.g. `Pair`, `Flush`).
- `Player::add_chips`/`subtract_chips` now saturate instead of overflowing.

#### Removed

- Pre-release evaluator names `evaluate`, `evaluate_with_cards`,
  `best_five_with_cards`, and `best_omaha`. Use the checked evaluator functions
  documented above.
- `HandRank`, `rank_hand`, `get_high_card_value`, and the `check_for_*` helpers.
  Hand comparison is provided by the fallible `evaluate_holdem`, `best_five`,
  `evaluate_five`, and `evaluate_omaha` APIs described above.

#### Fixed

- All-ins and side pots: a short stack used to be force-folded and side pots were
  never paid. Unequal all-ins now resolve into correct main/side pots.
- Calling no longer double-charges — a player owes only the delta over what they
  have already committed this street (blinds and prior calls are credited).
- Exact-stack blind posters are marked all-in and skipped for action.
- Incomplete all-in raises are enforced at submission time: players whose action
  was not reopened cannot reraise, while a short all-in call remains legal.
- Public and per-player event copies cannot leak a configured hero's private cards.
- Straight-flush ties are no longer (incorrectly) broken by suit.

---

### casino_cards 2.0.0

#### Added

- `Serialize`/`Deserialize` (serde) for `Card`, `Rank`, `Suit`, `Hand`, and `Deck`.
- `Card::glyph()` returns the single Unicode playing-card glyph (e.g. `🂡`).
- `Rank::code()`/`Suit::code()` return the single-character PokerStars codes
  (`A`, `K`, `T`, …, and `c`/`d`/`h`/`s`).
- `set_glyph_display`/`glyph_display_enabled` to switch card rendering globally.
- `Display` for `Hand`, and `Default` for `Deck` and `Hand`.
- `FromStr` for `Card` (with `ParseCardError`): parses the two-character
  PokerStars code (`"As"`, `"Td"`, `"9h"`), round-tripping `Card`'s text
  `Display`, so hand histories can be read back as well as written.
- `Deck::from_cards` builds a deck from a known card list (replays, fixed
  setups, tests), complementing the existing `Deck::new`.
- `Deck::peek` returns the next card `deal` would yield without removing it,
  and `Deck::iter` borrows the cards in order (top card — next to be dealt —
  yielded last).
- `Deck::shuffle_with` shuffles with a caller-supplied RNG, so a seeded RNG
  gives a deterministic shuffle for reproducible simulations, replays, and
  tests (`shuffle` keeps using a thread-local RNG).

#### Changed

- **Breaking:** `Card`'s text `Display` now renders the PokerStars two-character
  code (e.g. `As`, `Td`, `Ten` → `T`, lowercase suit) instead of the Unicode
  playing-card glyph, so output is parseable by standard hand-history tools. Use
  `set_glyph_display(true)` for the glyph form (`🂡`), or `Card::glyph()`.
