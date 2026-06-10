[![crates.io](https://img.shields.io/crates/v/casino_poker.svg)](https://crates.io/crates/casino_poker) [![Poker Test](https://github.com/winstonrc/casino/actions/workflows/casino_poker.yml/badge.svg?branch=main)](https://github.com/winstonrc/casino/actions/workflows/casino_poker.yml)

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

let mut agents: HashMap<Uuid, Box<dyn PokerAgent>> = HashMap::new();
for &id in game.seats() {
    agents.insert(id, Box::new(CallingAgent));
}

// Play one hand: deal, run each street's betting, then award the pots.
game.begin_hand();
for street in [Street::Preflop, Street::Flop, Street::Turn, Street::River] {
    match street {
        Street::Flop => game.deal_flop(),
        Street::Turn => game.deal_turn(),
        Street::River => game.deal_river(),
        Street::Preflop => {}
    }
    if game.run_betting_round(street, &mut agents) == RoundOutcome::HandOver {
        break;
    }
}
game.award_pots();
game.end_hand();
```

### Observing a hand

The engine does no I/O. Instead it emits **public** narration (only what every
player at the table can see — opponents' hole cards are never broadcast mid-hand)
as serializable `GameEvent`s to a `GameObserver`. Set one with `set_observer`;
without one the engine runs silently. The stream carries everything a PokerStars
hand history needs (the `casino_games` front-end renders exactly that); render it
in a terminal, log it, or forward it over a network. Designate the perspective
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

