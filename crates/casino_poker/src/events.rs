//! Game events emitted by the engine, and the observer that receives them.
//!
//! The [`TexasHoldEm`](crate::games::texas_hold_em::TexasHoldEm) engine is
//! I/O-free: instead of printing, it emits [`GameEvent`]s to a [`GameObserver`].
//! A terminal front-end renders them; a future TUI, GUI, or network layer can
//! render or forward the same stream (events are serializable).
//!
//! Events carry only **public** information — what every player at the table
//! would see. A player's own hole cards are private and are read separately via
//! the engine's getters, never broadcast as an event.

use serde::{Deserialize, Serialize};

use casino_cards::card::Card;

use crate::agent::Street;
use crate::hand_rankings::HandCategory;

/// Which blind a player posted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Blind {
    Small,
    Big,
}

/// A player's resolved action, as the rest of the table would see it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActionView {
    Folded,
    Checked,
    Called {
        amount: u32,
        all_in: bool,
    },
    /// An opening bet (the bet was zero before this action).
    Bet {
        amount: u32,
        all_in: bool,
    },
    /// A raise to the given total committed this street.
    Raised {
        to: u32,
        all_in: bool,
    },
}

/// A piece of public narration emitted during a hand.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum GameEvent {
    /// A new hand began with the given dealer.
    HandStarted { dealer: String },
    /// A blind was posted (possibly all-in for less than the full blind).
    BlindPosted {
        player: String,
        blind: Blind,
        amount: u32,
        all_in: bool,
    },
    /// A player acted.
    ActionTaken { player: String, action: ActionView },
    /// Community cards were dealt for a street, with the running pot total.
    StreetDealt {
        street: Street,
        board: Vec<Card>,
        pot: u32,
    },
    /// An uncalled bet was returned to its bettor.
    UncalledBetReturned { player: String, amount: u32 },
    /// A player's hand was revealed at showdown.
    ShowdownReveal {
        player: String,
        cards: Vec<Card>,
        hand: HandCategory,
    },
    /// A player won chips. `hand` is `None` when the pot was uncontested.
    PotAwarded {
        player: String,
        amount: u32,
        hand: Option<HandCategory>,
    },
}

/// Receives [`GameEvent`]s emitted by the engine. The default [`NullObserver`]
/// drops them, so callers that don't render pay nothing.
pub trait GameObserver {
    fn notify(&mut self, event: &GameEvent);
}

/// A [`GameObserver`] that discards every event.
pub struct NullObserver;

impl GameObserver for NullObserver {
    fn notify(&mut self, _event: &GameEvent) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    use casino_cards::card::{Card, Rank, Suit};

    /// Events must survive a serialize/deserialize round-trip so they can be
    /// logged or sent over a network.
    #[test]
    fn game_event_round_trips_through_json() {
        let events = [
            GameEvent::HandStarted {
                dealer: "Alice".to_string(),
            },
            GameEvent::BlindPosted {
                player: "Bob".to_string(),
                blind: Blind::Big,
                amount: 2,
                all_in: false,
            },
            GameEvent::ActionTaken {
                player: "Bob".to_string(),
                action: ActionView::Raised {
                    to: 10,
                    all_in: true,
                },
            },
            GameEvent::ShowdownReveal {
                player: "Alice".to_string(),
                cards: vec![
                    Card::new(Rank::Ace, Suit::Spade),
                    Card::new(Rank::King, Suit::Heart),
                ],
                hand: HandCategory::Pair,
            },
            GameEvent::PotAwarded {
                player: "Alice".to_string(),
                amount: 20,
                hand: Some(HandCategory::Pair),
            },
        ];

        for event in events {
            let json = serde_json::to_string(&event).expect("serialize");
            let back: GameEvent = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(event, back);
        }
    }
}
