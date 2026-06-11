# Changelog

All notable changes to the crates in this workspace are documented here. The
format is based on [Keep a Changelog](https://keepachangelog.com/), and each
crate follows [Semantic Versioning](https://semver.org/).

---

## casino_poker 1.0.0 — 2026-06-10

First stable release. The backend now drives complete Texas Hold'em hands with
correct betting, all-ins, and side pots, and exposes a stable public API.

### Added

- `hand_rankings::evaluate(hole, board) -> ComparableHand` — picks the best
  5-card hand and returns a fully-ordered, kicker-correct value (`HandCategory`
  - tiebreak ranks). Handles the wheel (A-2-3-4-5) and is cross-checked in tests
    against an independent brute-force oracle. `evaluate_with_cards` additionally
    returns the exact five cards that form the hand, and `ComparableHand::describe`
    names a hand in PokerStars wording ("two pair, Jacks and Fives", "a flush, Ace
    high", "a full house, Kings full of Threes", "a straight, Five to Nine").
    `ComparableHand` derives serde.
- `hand_rankings::best_omaha(hole, board) -> ComparableHand` — Omaha
  evaluation, which must use exactly two of the four hole cards plus three
  board cards (panics unless `hole.len() == 4` and `board.len()` is 3–5).
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
  `is_all_in`, `button_seat`, `pots`, and `current_view`. `table()` returns a
  single owned, serializable `TableView` (with per-seat `SeatView`s); both are
  `#[non_exhaustive]`.
- `events` module: the engine is **I/O-free** and emits public narration as
  serializable `GameEvent`s (`HandStarted`, `HoleCardsDealt`, `BlindPosted`,
  `ActionTaken`, `StreetDealt`, `UncalledBetReturned`, `Showdown`,
  `ShowdownReveal`, `PotAwarded`, `HandComplete`) to a `GameObserver` set via
  `TexasHoldEm::set_observer` (default `NullObserver`) — render them, log them, or
  forward them over a network. The events carry exactly what a PokerStars-format
  hand history needs: `HandStarted` the seat roster (`SeatInfo`), button, and
  blinds; `ActionTaken` the `Street` and `ActionView::Raised { by, to }`;
  `ShowdownReveal`/`PotAwarded` the full `ComparableHand` (call `describe()` to
  name it); and `PotAwarded` an optional `PotKind` (`Main`/`Side(n)`) for per-pot
  side-pot narration. The public event enums (`GameEvent`, `ActionView`, `PotKind`,
  `Blind`) are `#[non_exhaustive]`, so future variants can be added without a
  breaking change. `BroadcastObserver` fans a single event stream out to several
  `GameObserver`s (notified in order), so e.g. a logger and a renderer can both
  observe one hand.

### Changed

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

### Removed

- `HandRank`, `rank_hand`, `get_high_card_value`, and the `check_for_*` helpers.
  **Migration:** call `evaluate(hole, board)` and compare the returned
  `ComparableHand` values directly with `>`, `<`, and `==` — these handle hand
  category, kickers, and ties correctly.

### Fixed

- All-ins and side pots: a short stack used to be force-folded and side pots were
  never paid. Unequal all-ins now resolve into correct main/side pots.
- Calling no longer double-charges — a player owes only the delta over what they
  have already committed this street (blinds and prior calls are credited).
- Straight-flush ties are no longer (incorrectly) broken by suit.

---

## casino_cards 2.0.0 — 2026-06-10

### Added

- `Serialize`/`Deserialize` (serde) for `Card`, `Rank`, `Suit`, and `Hand`.
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

### Changed

- **Breaking:** `Card`'s text `Display` now renders the PokerStars two-character
  code (e.g. `As`, `Td`, `Ten` → `T`, lowercase suit) instead of the Unicode
  playing-card glyph, so output is parseable by standard hand-history tools. Use
  `set_glyph_display(true)` for the glyph form (`🂡`), or `Card::glyph()`.
