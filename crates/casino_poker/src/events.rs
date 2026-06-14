//! Game events emitted by the engine, and the observer that receives them.
//!
//! The [`TexasHoldEm`](crate::games::texas_hold_em::TexasHoldEm) engine is
//! I/O-free: instead of printing, it emits [`GameEvent`]s to a [`GameObserver`].
//! A terminal front-end can render the direct observer stream. Network layers
//! should instead use the filtered public or authenticated client copies below.
//!
//! Most event data is public table narration. When a hero is configured,
//! [`GameEvent::HoleCardsDealt`] carries that perspective player's private cards
//! to the direct observer stream and [`TexasHoldEm::replay_log`](crate::games::texas_hold_em::TexasHoldEm::replay_log).
//! Use [`TexasHoldEm::public_events`](crate::games::texas_hold_em::TexasHoldEm::public_events)
//! or [`TexasHoldEm::client_view`](crate::games::texas_hold_em::TexasHoldEm::client_view)
//! for redacted network/agent-facing copies.

use serde::{Deserialize, Serialize};

use casino_cards::card::Card;

use crate::agent::Street;
use crate::hand_rankings::ComparableHand;
use crate::player::PlayerRef;

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

/// One seat in the hand-start roster: a 1-based seat number, a stable reference to
/// the player, and their chip stack at the start of the hand (before blinds are
/// posted).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SeatInfo {
    pub seat_no: usize,
    pub player: PlayerRef,
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

/// A hand-narration event. Most variants are public; `HoleCardsDealt.hero` may
/// contain the configured perspective player's private cards on the direct
/// observer stream.
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
        player: PlayerRef,
        blind: Blind,
        amount: u32,
        all_in: bool,
    },
    /// Hole cards were dealt — the marker that betting is about to begin. `hero`
    /// carries the perspective player and two cards when one is set (for the
    /// `Dealt to …` line); `None` when no hero is designated.
    HoleCardsDealt {
        hero: Option<(PlayerRef, Vec<Card>)>,
    },
    /// A player acted on the given street.
    ActionTaken {
        player: PlayerRef,
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
    UncalledBetReturned { player: PlayerRef, amount: u32 },
    /// Two or more players reached a showdown. Emitted once, after the final
    /// betting round and before any [`ShowdownReveal`](GameEvent::ShowdownReveal),
    /// carrying the final board and pot so a front-end can re-show the table the
    /// hands are read against.
    Showdown { board: Vec<Card>, pot: u32 },
    /// A player's hand was revealed at showdown.
    ShowdownReveal {
        player: PlayerRef,
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
        player: PlayerRef,
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

/// A [`GameObserver`] that fans every event out to several observers, in order.
///
/// The engine holds a single observer, so this composes multiple sinks behind one
/// `set_observer` call — e.g. a terminal renderer, a session logger, and a network
/// broadcaster all receiving the same stream. The observer stream is
/// perspective-aware: do not broadcast it to multiple players when a hero is set;
/// use `public_events` or per-player `client_view` instead.
pub struct BroadcastObserver {
    observers: Vec<Box<dyn GameObserver>>,
}

impl BroadcastObserver {
    /// Create a fan-out over the given observers (notified in order).
    pub fn new(observers: Vec<Box<dyn GameObserver>>) -> Self {
        Self { observers }
    }

    /// Add another observer to the fan-out.
    pub fn push(&mut self, observer: Box<dyn GameObserver>) {
        self.observers.push(observer);
    }
}

impl GameObserver for BroadcastObserver {
    fn notify(&mut self, event: &GameEvent) {
        for observer in &mut self.observers {
            observer.notify(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use casino_cards::card::{Card, Rank, Suit};
    use uuid::Uuid;

    use crate::hand_rankings::HandCategory;

    /// A `PlayerRef` with a fresh id and the given name, for test events.
    fn pref(name: &str) -> PlayerRef {
        PlayerRef {
            id: Uuid::new_v4(),
            name: name.to_string(),
        }
    }

    #[test]
    fn broadcast_observer_forwards_to_every_observer() {
        use std::cell::RefCell;
        use std::rc::Rc;

        struct Counter(Rc<RefCell<usize>>);
        impl GameObserver for Counter {
            fn notify(&mut self, _event: &GameEvent) {
                *self.0.borrow_mut() += 1;
            }
        }

        let (a, b) = (Rc::new(RefCell::new(0)), Rc::new(RefCell::new(0)));
        let mut broadcast = BroadcastObserver::new(vec![
            Box::new(Counter(a.clone())),
            Box::new(Counter(b.clone())),
        ]);

        broadcast.notify(&GameEvent::HandComplete);
        broadcast.notify(&GameEvent::HandComplete);

        assert_eq!(*a.borrow(), 2);
        assert_eq!(*b.borrow(), 2);
    }

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
                        player: pref("Alice"),
                        stack: 100,
                    },
                    SeatInfo {
                        seat_no: 2,
                        player: pref("Bob"),
                        stack: 200,
                    },
                ],
            },
            GameEvent::BlindPosted {
                player: pref("Bob"),
                blind: Blind::Big,
                amount: 2,
                all_in: false,
            },
            GameEvent::HoleCardsDealt {
                hero: Some((
                    pref("Alice"),
                    vec![
                        Card::new(Rank::Ace, Suit::Spade),
                        Card::new(Rank::King, Suit::Heart),
                    ],
                )),
            },
            GameEvent::ActionTaken {
                player: pref("Bob"),
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
                player: pref("Alice"),
                hole: vec![
                    Card::new(Rank::Ace, Suit::Spade),
                    Card::new(Rank::King, Suit::Heart),
                ],
                hand: pair,
            },
            GameEvent::PotAwarded {
                player: pref("Alice"),
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
