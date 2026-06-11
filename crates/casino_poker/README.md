[![crates.io](https://img.shields.io/crates/v/casino_poker.svg)](https://crates.io/crates/casino_poker) [![CI](https://github.com/winstonrc/casino/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/winstonrc/casino/actions/workflows/ci.yml)

# casino_poker

A library that provides hand ranking & the backend for poker games. 

**Note:** As of `1.0.0` the public API is considered stable and follows [SemVer](https://semver.org/) — breaking changes will bump the major version. Larger follow-ups (TUI, pluggable/model-backed agents, online multiplayer) are targeted for `2.0.0`.

## Usage

### Evaluating the best hand

`evaluate` returns a `ComparableHand` — a kicker-correct, fully-ordered value of
the best 5-card hand from hole cards plus the board. Compare two players'
`ComparableHand`s directly with `<`, `>`, and `==` (equal hands chop the pot).
Call `hand.describe()` for a PokerStars-style name ("two pair, Jacks and Fives",
"a flush, Ace high", "a full house, Kings full of Threes"), or
`evaluate_with_cards` to also get the exact five cards forming the hand.

```rust
use casino_cards::card::{Card, Rank, Suit};
use casino_poker::hand_rankings::evaluate;

let hole = [Card::new(Rank::Ace, Suit::Heart), Card::new(Rank::Two, Suit::Heart)];
let board = [
    Card::new(Rank::Five, Suit::Heart),
    Card::new(Rank::Nine, Suit::Heart),
    Card::new(Rank::King, Suit::Heart),
    Card::new(Rank::King, Suit::Spade),
    Card::new(Rank::Three, Suit::Club),
];
let hand = evaluate(&hole, &board); // a Flush
```

### Texas hold 'em

The `TexasHoldEm` engine owns the full hand lifecycle and all money (per-player
contributions, side pots, folds, all-ins). A caller drives it with a thin loop
and supplies a `PokerAgent` per player to choose actions:

```rust
use std::collections::HashMap;

use casino_poker::agent::{AgentError, LegalAction, PlayerAction, PlayerView, PokerAgent, Street};
use casino_poker::games::texas_hold_em::{RoundOutcome, TexasHoldEm};
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
game.play_hand(&mut agents);
```

`play_hand` is the blocking convenience over the **resumable hand state machine**:
`drive_hand()` (begin a fresh hand or resume the one in progress) and
`submit_hand_action(action)` yield a `HandStep` (`AwaitingAction { player, view }` or
`HandComplete`), so an async front-end (a network server, a UI) drives a whole hand
action-by-action without re-implementing the deal/bet/award sequence. The same shape
exists one level down for a single street (`begin_betting_round`/`submit_action` →
`BettingStep`, with the blocking `run_betting_round` wrapper).

For **save/resume and reconnection**, `TexasHoldEm` is `serde`-serializable: persist
it mid-hand and restore it to continue from the exact spot (re-attach an observer
with `set_observer`, then `replay_log()` to re-narrate the hand so far). For a
networked client, `client_view(player_id)` returns a `ClientView` — the public
`TableView` plus that player's own cards and pending decision and the hand's events —
leak-safe to send on (re)connect. The button accessors `dealer()` / `set_dealer`
read and place the dealer button. A `PokerAgent`
need only implement `decide`; the `observe` (watch the `GameEvent` stream) and
`session_ended` (persist learned state) hooks default to no-ops, so a stateful AI can
learn and persist across hands and sessions without any engine change.

For a front-end training overlay, `PlayerView::metrics()` returns derived
`HandMetrics` — pot odds (and the equity needed to call), stack-to-pot ratio, and
stack/call sizes in big blinds — so a UI can render correct numbers without
re-deriving them. `PlayerView` is `#[non_exhaustive]` (the engine builds it; use
`PlayerView::builder()` to construct one in your own agent tests), and both it and
`HandMetrics` can gain fields in a minor release.

### Observing a hand

The engine does no I/O. Instead it emits **public** narration (only what every
player at the table can see — opponents' hole cards are never broadcast mid-hand)
as serializable `GameEvent`s to a `GameObserver`. Set one with `set_observer`;
without one the engine runs silently. The stream carries everything a PokerStars
hand history needs, so a front-end can render exactly that; render it in a terminal,
log it, or forward it over a network. Designate the perspective
player with `set_hero` so `HoleCardsDealt` carries their cards (for `Dealt to …`).

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

- `HandStarted` carries the seat roster (`SeatInfo` with name + starting stack),
  the button seat, and the blinds — the data a hand-history header needs.
- `HoleCardsDealt` marks the start of betting; its `hero` field carries the
  perspective player's name and cards (when a hero is set), else `None`.
- `ActionTaken` carries the `Street` and an `ActionView`; `Raised { by, to }`
  gives both the raise increment and the new total (PokerStars "raises by to to").
- `Showdown` is emitted once before the reveals when two or more players reach a
  showdown, carrying the final `board` and `pot`.
- `ShowdownReveal` carries the player's `hole` cards and their `hand` (a
  `ComparableHand` — `hand.describe()` for the named hand, `hand.category` for the
  bare category).
- `PotAwarded` carries the winning `hand` (`Option<ComparableHand>`) and an
  optional `PotKind` (`Main` / `Side(n)`) for per-pot narration (`None` for a
  single pot). `HandComplete` signals the hand is fully resolved.

To award winners yourself rather than from events, `pot::distribute_pots` returns
one `PotAward` per pot (main first, then side pots), each listing that pot's
winners and the chips they receive.

