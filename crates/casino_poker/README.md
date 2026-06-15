[![crates.io](https://img.shields.io/crates/v/casino_poker.svg)](https://crates.io/crates/casino_poker) [![CI](https://github.com/winstonrc/casino/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/winstonrc/casino/actions/workflows/ci.yml)

# casino_poker

A library that provides hand ranking & the backend for poker games. 

**Note:** The public API follows [SemVer](https://semver.org/). Version `1.0.0`
establishes the engine-owned hand lifecycle and checked, transactional state
transitions as the stable API.

## Usage

### Evaluating the best hand

`evaluate_holdem` returns an `EvaluatedHand` containing the best five physical
cards and a kicker-correct, fully ordered `ComparableHand` value. Compare
players' `hand.value()`s directly with `<`, `>`, and `==` (equal values chop the
pot). Call `hand.value().describe()` for a PokerStars-style name ("two pair, Jacks
and Fives", "a flush, Ace high", "a full house, Kings full of Threes").

Evaluation is fallible: unsupported card counts and duplicate physical cards
return `HandEvaluationError` rather than panicking. Use `evaluate_five` for
exactly five cards, `best_five` for a flat five-to-seven-card pool, and
`evaluate_omaha` for Omaha's exact two-hole-card and three-board-card rule.

```rust
use casino_poker::casino_cards::card::{Card, Rank, Suit};
use casino_poker::hand_rankings::{evaluate_holdem, HandCategory};

fn main() -> Result<(), casino_poker::hand_rankings::HandEvaluationError> {
    let hole = [Card::new(Rank::Ace, Suit::Heart), Card::new(Rank::Two, Suit::Heart)];
    let board = [
        Card::new(Rank::Five, Suit::Heart),
        Card::new(Rank::Nine, Suit::Heart),
        Card::new(Rank::King, Suit::Heart),
        Card::new(Rank::King, Suit::Spade),
        Card::new(Rank::Three, Suit::Club),
    ];
    let hand = evaluate_holdem(hole, &board)?;
    assert_eq!(hand.value().category(), HandCategory::Flush);
    Ok(())
}
```

### Texas hold 'em

The `TexasHoldEm` engine owns the full hand lifecycle and all money (per-player
contributions, side pots, folds, all-ins). A caller drives it with a thin loop
and supplies a `PokerAgent` per player to choose actions:

```rust
use std::collections::HashMap;

use casino_poker::agent::{AgentError, LegalAction, PlayerAction, PlayerView, PokerAgent, Street};
use casino_poker::games::texas_hold_em::TexasHoldEm;
use casino_poker::uuid::Uuid;

// A trivial agent that checks when possible, otherwise calls, otherwise folds.
struct CallingAgent;
impl PokerAgent for CallingAgent {
    fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError> {
        if view.legal_actions.iter().any(|a| matches!(a, LegalAction::Check)) {
            Ok(PlayerAction::Check)
        } else if view.legal_actions.iter().any(|a| matches!(a, LegalAction::Call(_))) {
            Ok(PlayerAction::Call)
        } else {
            Ok(PlayerAction::Fold)
        }
    }
}

let mut game = TexasHoldEm::new(100, 10, 1, 2); // min buy-in, max players, small/big blind

let player = game.new_player_with_chips("Player 1", 100);
game.add_player(player).unwrap();
// ...seat more players...

// Optional: shuffle seating once so the opening dealer button isn't always the
// first player added (players are otherwise seated in `add_player` order).
game.randomize_seats();

let mut agents: HashMap<Uuid, Box<dyn PokerAgent>> = HashMap::new();
for &id in game.seats() {
    agents.insert(id, Box::new(CallingAgent));
}

// Play one hand: the engine deals each street, runs its betting, and awards the
// pots, sourcing each action from that player's agent.
game.play_hand(&mut agents).unwrap();
```

`TexasHoldEm` supports 2–10 players. `begin_hand()` and
`begin_hand_with_deck()` validate the table and return `HandStartError` without
mutating the engine when the player count, deck, roster, or bankroll is invalid.
Seat and button changes are accepted only between hands. Standalone pot helpers
use `u64` aggregate amounts; engine snapshots/events and player stacks remain
`u32` under a checked table-wide bankroll cap.

`play_hand` is the blocking convenience over the **resumable hand state machine**:
`drive_hand()` (begin a fresh hand or resume the one in progress) and
`submit_hand_action(player, decision_id, action)` yield a `HandStep`
(`AwaitingAction(PendingAction)`, `HandComplete`, or transactional
`CannotStart(HandStartError)`), so an async front-end drives a whole hand without
re-implementing the deal/bet/award sequence:

```rust
use casino_poker::betting::PlayerAction;
use casino_poker::games::texas_hold_em::{HandStep, TexasHoldEm};

fn drive_one_action(game: &mut TexasHoldEm, action: PlayerAction) {
    let HandStep::AwaitingAction(pending) = game.drive_hand() else {
        return;
    };
    let next = game
        .submit_hand_action(pending.player, pending.decision_id, action)
        .expect("the action still matches the pending decision");
    // Persist `game` and continue from `next`.
}
```

Every prompt has a serialized `DecisionId`. Repeated reads and save/restore retain
that ID; the engine accepts it once and rejects stale retries, wrong-player input,
and illegal poker actions with `ActionSubmissionError` without changing state.
The same identified submission contract exists one street down through
`begin_betting_round` / `submit_action`. To abandon a hand entirely,
`abort_betting_round()` refunds its contributions, returns its cards to the deck,
and leaves the table ready for a new hand without emitting `HandComplete`.

For **save/resume and reconnection**, `TexasHoldEm` is `serde`-serializable: persist
it mid-hand and restore it to continue from the exact spot (re-attach an observer
with `set_observer`, then `replay_log()` to re-narrate the hand so far). For a
networked client, `client_view(player_id)` returns a `ClientView` — the public
`TableView` plus that player's own cards and pending decision and the hand's events —
leak-safe to send on (re)connect. The button accessors `dealer()` / `set_dealer`
read and place the dealer button. A `PokerAgent`
need only implement `decide`; the `observe` (watch the `GameEvent` stream) and
`session_ended` (persist learned state) hooks default to no-ops, so a stateful AI can
learn and persist across hands and sessions without any engine change. Every
player-bearing event carries a `PlayerRef` (a stable `Uuid` plus display name), so an
agent can key a per-opponent model off the `id` rather than the (non-unique) name. The
engine stays agent-agnostic: `public_events()` returns an owned, hero-redacted copy
that a front-end can forward into its agents' `observe` without exposing private
cards.

For a front-end training overlay, `PlayerView::metrics()` returns derived
`HandMetrics` — pot odds (and the equity needed to call), stack-to-pot ratio, and
stack/call sizes in big blinds — so a UI can render correct numbers without
re-deriving them. `PlayerView` also carries `seats` (the public per-seat roster as
`SeatView`s — id, stack, this-street commitment, fold/all-in status) and `button_seat`,
so an agent sees the same objective table state a spectator does and can map a stored
opponent model onto who is actually at the table. `PlayerView` is `#[non_exhaustive]`
(the engine builds it; use `PlayerView::builder()` to construct one in your own agent
tests), and both it and `HandMetrics` can gain fields in a minor release.

### Observing a hand

The engine does no I/O. It emits serializable `GameEvent`s to a `GameObserver`.
This direct observer stream is perspective-aware: `set_hero` makes
`HoleCardsDealt` carry that player's private cards for a hand-history
`Dealt to ...` line. Do not broadcast that raw stream. Use `public_events()` for a
hero-redacted shared stream, or `client_view(player_id)` for a per-player reconnect
snapshot that retains only that player's private cards.

```rust
use casino_poker::events::{GameEvent, GameObserver};

struct Printer;
impl GameObserver for Printer {
    fn notify(&mut self, event: &GameEvent) {
        println!("{event:?}");
    }
}

// game.set_observer(Box::new(Printer));
```

Notable events:

- Player-bearing events (`BlindPosted`, `ActionTaken`, `UncalledBetReturned`,
  `ShowdownReveal`, `PotAwarded`) identify the player by a `PlayerRef` (stable `id` +
  `name`); `PlayerRef` renders as its name, so it interpolates directly in display.
- `HandStarted` carries the seat roster (`SeatInfo` with a `PlayerRef` + starting
  stack), the button seat, and the blinds — the data a hand-history header needs.
- `HoleCardsDealt` marks the start of betting; its `hero` field carries the
  perspective player (`PlayerRef`) and cards (when a hero is set), else `None`.
- `ActionTaken` carries the `Street` and an `ActionView`; `Raised { by, to }`
  gives both the raise increment and the new total (PokerStars "raises by to to").
- `Showdown` is emitted once before the reveals when two or more players reach a
  showdown, carrying the final `board` and `pot`.
- `ShowdownReveal` carries the player's `hole` cards and their `hand` (a
  `ComparableHand` — `hand.describe()` for the named hand, `hand.category()` for the
  bare category).
- `PotAwarded` carries the winning `hand` (`Option<ComparableHand>`) and an
  optional `PotKind` (`Main` / `Side(n)`) for per-pot narration (`None` for a
  single pot). `HandComplete` signals the hand is fully resolved.

To award winners yourself rather than from events, `pot::distribute_pots` returns
one `PotAward` per pot (main first, then side pots), each listing that pot's
winners and the chips they receive.

## Building on this

Most front-ends only need `play_hand` and `evaluate_holdem`, but the engine
exposes a few capabilities that aren't obvious from the happy path. If you're
building a server, a UI, or a learning agent, these are the ones to reach for:

- **Save / resume.** `TexasHoldEm` is `serde`-serializable, so you can persist a
  game mid-hand and restore it to continue from the exact spot. A restored engine is
  silent until you re-attach narration with `set_observer` (then `replay_log()` to
  re-narrate the hand so far for catch-up rendering).
- **Non-blocking play.** `drive_hand()` yields an identified `PendingAction`;
  submit its player, decision ID, and chosen action together. Rejected submissions
  are mutation-free, so a client can fetch `pending_action()` and resynchronize.
- **Spectator / broadcast.** `public_events()` always redacts the optional hero
  payload. `table()` contains no hole cards, while `client_view(player_id)` adds
  only that player's cards and pending decision.
- **Stable identity.** Every player-bearing event carries a `PlayerRef` (a stable
  `Uuid` plus display name), so an agent can key a per-opponent model off the `id`
  rather than the (non-unique) name, and re-find that opponent across hands and
  reseatings.
- **Derived metrics.** `PlayerView::metrics()` returns `HandMetrics` — pot odds (and
  the equity needed to call), stack-to-pot ratio, and stack/call sizes in big blinds
  — so a training overlay can render correct numbers without re-deriving them.
