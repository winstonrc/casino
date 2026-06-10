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
use crate::hand_rankings::ComparableHand;

/// Which blind a player posted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum Blind {
    Small,
    Big,
}

/// Which pot an award came from, when a hand produced side pots.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum PotKind {
    Main,
    /// A side pot, numbered from `1` for the smallest (lowest) side-pot layer
    /// upward.
    Side(u8),
}

/// One seat in the hand-start roster: a 1-based seat number, the player's name,
/// and their chip stack at the start of the hand (before blinds are posted).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SeatInfo {
    pub seat_no: usize,
    pub name: String,
    pub stack: u32,
}

/// A player's resolved action, as the rest of the table would see it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
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
    /// A raise of `by` chips over the prior bet, to a total of `to` committed this
    /// street (PokerStars writes "raises `by` to `to`").
    Raised {
        by: u32,
        to: u32,
        all_in: bool,
    },
}

/// A piece of public narration emitted during a hand.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[non_exhaustive]
pub enum GameEvent {
    /// A new hand began. Carries everything a hand-history header needs: the hand
    /// number, the button seat (1-based), the blinds, and the seat roster with
    /// each player's starting (pre-blind) stack.
    HandStarted {
        hand_number: u32,
        button_seat: usize,
        small_blind: u32,
        big_blind: u32,
        seats: Vec<SeatInfo>,
    },
    /// A blind was posted (possibly all-in for less than the full blind).
    BlindPosted {
        player: String,
        blind: Blind,
        amount: u32,
        all_in: bool,
    },
    /// Hole cards were dealt — the marker that betting is about to begin. `hero`
    /// carries the perspective player's name and two cards when one is set (for
    /// the `Dealt to …` line); `None` when no hero is designated.
    HoleCardsDealt { hero: Option<(String, Vec<Card>)> },
    /// A player acted on the given street.
    ActionTaken {
        player: String,
        street: Street,
        action: ActionView,
    },
    /// Community cards were dealt for a street, with the running pot total.
    StreetDealt {
        street: Street,
        board: Vec<Card>,
        pot: u32,
    },
    /// An uncalled bet was returned to its bettor.
    UncalledBetReturned { player: String, amount: u32 },
    /// Two or more players reached a showdown. Emitted once, after the final
    /// betting round and before any [`ShowdownReveal`](GameEvent::ShowdownReveal),
    /// carrying the final board and pot so a front-end can re-show the table the
    /// hands are read against.
    Showdown { board: Vec<Card>, pot: u32 },
    /// A player's hand was revealed at showdown.
    ShowdownReveal {
        player: String,
        /// The player's two hole cards.
        hole: Vec<Card>,
        /// The player's best hand value. Call `hand.describe()` to name it or read
        /// `hand.category` for the bare category.
        hand: ComparableHand,
    },
    /// A player won chips. `hand` is `None` when the pot was uncontested. `pot`
    /// identifies which pot the chips came from when the hand had side pots, and
    /// is `None` when there was a single pot (nothing to distinguish).
    PotAwarded {
        player: String,
        amount: u32,
        /// The winning hand value (`None` when the pot was uncontested). Call
        /// `hand.describe()` to name it.
        hand: Option<ComparableHand>,
        pot: Option<PotKind>,
    },
    /// The hand is fully resolved (all pots awarded). Signals a front-end to flush
    /// any accumulated per-hand summary.
    HandComplete,
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

    use crate::hand_rankings::HandCategory;

    /// Events must survive a serialize/deserialize round-trip so they can be
    /// logged or sent over a network.
    #[test]
    fn game_event_round_trips_through_json() {
        let pair = ComparableHand {
            category: HandCategory::Pair,
            tiebreak: [14, 13, 12, 11, 0],
        };
        let events = [
            GameEvent::HandStarted {
                hand_number: 7,
                button_seat: 1,
                small_blind: 1,
                big_blind: 2,
                seats: vec![
                    SeatInfo {
                        seat_no: 1,
                        name: "Alice".to_string(),
                        stack: 100,
                    },
                    SeatInfo {
                        seat_no: 2,
                        name: "Bob".to_string(),
                        stack: 200,
                    },
                ],
            },
            GameEvent::BlindPosted {
                player: "Bob".to_string(),
                blind: Blind::Big,
                amount: 2,
                all_in: false,
            },
            GameEvent::HoleCardsDealt {
                hero: Some((
                    "Alice".to_string(),
                    vec![
                        Card::new(Rank::Ace, Suit::Spade),
                        Card::new(Rank::King, Suit::Heart),
                    ],
                )),
            },
            GameEvent::ActionTaken {
                player: "Bob".to_string(),
                street: Street::Preflop,
                action: ActionView::Raised {
                    by: 8,
                    to: 10,
                    all_in: true,
                },
            },
            GameEvent::Showdown {
                board: vec![
                    Card::new(Rank::Ace, Suit::Diamond),
                    Card::new(Rank::King, Suit::Heart),
                    Card::new(Rank::Queen, Suit::Club),
                    Card::new(Rank::Jack, Suit::Club),
                    Card::new(Rank::Two, Suit::Spade),
                ],
                pot: 40,
            },
            GameEvent::ShowdownReveal {
                player: "Alice".to_string(),
                hole: vec![
                    Card::new(Rank::Ace, Suit::Spade),
                    Card::new(Rank::King, Suit::Heart),
                ],
                hand: pair,
            },
            GameEvent::PotAwarded {
                player: "Alice".to_string(),
                amount: 20,
                hand: Some(pair),
                pot: Some(PotKind::Side(1)),
            },
            GameEvent::HandComplete,
        ];

        for event in events {
            let json = serde_json::to_string(&event).expect("serialize");
            let back: GameEvent = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(event, back);
        }
    }
}
