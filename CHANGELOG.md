# Changelog

All notable changes to the crates in this workspace are documented here. The
format is based on [Keep a Changelog](https://keepachangelog.com/), and each
crate follows [Semantic Versioning](https://semver.org/).

---

## casino_poker 1.0.0 — 2026-06-09

First stable release. The backend now drives complete Texas Hold'em hands with
correct betting, all-ins, and side pots, and exposes a stable public API.

### Added
- `hand_rankings::evaluate(hole, board) -> ComparableHand` — picks the best
  5-card hand and returns a fully-ordered, kicker-correct value (`HandCategory`
  + tiebreak ranks). Handles the wheel (A-2-3-4-5) and is cross-checked in tests
  against an independent brute-force oracle.
- `agent` module: the `PokerAgent` trait, an owned (serializable) `PlayerView`,
  `LegalAction`, `AgentError`, and `Street`. This is the seam for human, AI, and
  future model-backed players.
- `betting` module: `amount_owed`, `resolve_action`, `legal_actions`,
  `PlayerAction`, and the `BettingRound` state machine.
- `pot` module: `Pot`, `refund_uncalled`, `build_pots`, and `distribute_pots` —
  side pots computed at showdown by layering total contributions.
  `distribute_pots` returns one `PotAward` per pot (main then side), so callers
  can report each pot's winners separately rather than only summed winnings.
- `TexasHoldEm` agent-driven hand lifecycle: `begin_hand`, `deal_flop`/`deal_turn`/
  `deal_river`, `run_betting_round`, `award_pots`, `end_hand`, and `RoundOutcome`.
- `events` module: the engine is **I/O-free** and emits public narration as
  serializable `GameEvent`s (`HandStarted`, `BlindPosted`, `ActionTaken`,
  `StreetDealt`, `UncalledBetReturned`, `ShowdownReveal`, `PotAwarded`) to a
  `GameObserver` set via `TexasHoldEm::set_observer` (default `NullObserver`).
  Render them, log them, or forward them over a network. `PotAwarded` carries an
  optional `PotKind` (`Main`/`Side(n)`) so side-pot payouts can be narrated per
  pot, and `None` for single-pot hands.
- `agents` module: reusable, I/O-free `RandomAgent` and `HeuristicAgent`.

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

## casino_cards 1.1.0 — 2026-06-09

### Added
- `Serialize`/`Deserialize` (serde) for `Card`, `Rank`, `Suit`, and `Hand`.
- `Card::glyph()` returns the single Unicode playing-card glyph (e.g. `🂡`).
- `set_glyph_display`/`glyph_display_enabled` to switch card rendering globally.
- `Display` for `Hand`, and `Default` for `Deck` and `Hand`.

### Changed
- `Card`'s `Display` now renders as rank + suit (e.g. `A♠`, `10♦`) instead of the
  single Unicode playing-card glyph, which renders tiny or missing in many
  terminals. Use `Card::glyph()` for the old glyph.

---

## casino_games 1.0.0 — 2026-06-09

First stable release: a playable terminal Texas Hold'em game.

### Added
- Play Texas Hold'em against computer opponents: a strength-aware `HeuristicAgent`
  (evaluates hole + board and weighs pot odds) plus a looser `RandomAgent`.
- Profile persistence — name, chip balance, and basic stats (hands played/won)
  are saved as JSON in the platform data directory and offered to resume on
  launch. Saves are written atomically.
- Card-display preference (portable text `A♠` or Unicode glyphs `🂡`), chosen at
  launch and saved with the profile.

### Changed
- Pick a game by number (or press Enter — there's only one for now). On your turn
  choose an action by a single-letter shortcut or full word: `(f)old`, `(x) check`,
  `(c)all`, `(r)aise to <amount>`, `(a)ll-in`. Showdowns reveal each remaining
  player's hole cards.
