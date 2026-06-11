//! The agent interface: how a human or AI is asked to act.
//!
//! The engine builds an owned [`PlayerView`] snapshot each turn and hands it to a
//! [`PokerAgent`], which returns a [`PlayerAction`]. Both a terminal human prompt
//! and an AI implement the same trait, so swapping in a smarter (or model-backed)
//! opponent later requires no engine changes.
//!
//! [`PlayerView`] is deliberately **owned** (no borrows) so it both sidesteps
//! borrow-checker conflicts in the engine's action loop and can be serialized to
//! hand to an external/local model in the future.

use casino_cards::card::{Card, Rank, Suit};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::betting::{LegalAction, PlayerAction};
use crate::events::GameEvent;
use crate::games::texas_hold_em::SeatView;

/// Which betting street is in progress.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
}

/// An error returned by an agent that prevents it from producing an action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentError {
    /// The player chose to quit the game.
    Quit,
    /// Input ended unexpectedly (e.g. EOF on stdin).
    Eof,
}

/// An owned, read-only snapshot of everything an agent needs to decide.
///
/// All fields are owned (cards are `Copy`, the board is at most five cards), so a
/// view can be freely held, stored, or serialized without borrowing engine state.
///
/// `#[non_exhaustive]`: the engine produces these (agents only read them), so new
/// fields can be added in a minor release without breaking downstream readers.
/// Construct one outside the engine — e.g. in an agent's unit tests — with
/// [`PlayerView::builder`]; derived training stats are available via
/// [`PlayerView::metrics`].
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PlayerView {
    /// The deciding player's id.
    pub you: Uuid,
    /// The deciding player's name (for display).
    pub name: String,
    /// The current street.
    pub street: Street,
    /// The player's two hole cards.
    pub hole: [Card; 2],
    /// The shared board cards (0, 3, 4, or 5 of them).
    pub board: Vec<Card>,
    /// The player's remaining stack.
    pub chips: u32,
    /// Chips the player must put in to call.
    pub amount_owed: u32,
    /// The amount to match this street.
    pub current_bet: u32,
    /// The smallest legal raise-to total (only meaningful if a raise is legal).
    pub min_raise_to: u32,
    /// Total chips currently in all pots.
    pub pot_total: u32,
    /// The number of players still in the hand.
    pub players_remaining: usize,
    /// The legal actions available to the player this turn.
    pub legal_actions: Vec<LegalAction>,
    /// The table's big blind amount.
    pub big_blind: u32,
    /// The public table roster, in seat order — every seat's id, stack, this-street
    /// commitment, and fold/all-in status (including the deciding player's own seat,
    /// identified by [`you`](Self::you)). The objective table state an opponent
    /// model keys off; carries no hole cards.
    pub seats: Vec<SeatView>,
    /// Seat index of the dealer button, or `None` before the first hand — the
    /// reference point for reading position from `seats`.
    pub button_seat: Option<usize>,
}

/// Derived, ready-to-display stats for the current decision — a front-end can
/// surface these as a training overlay (pot odds, stack depth, etc.) without
/// re-deriving the formulas. Produced by [`PlayerView::metrics`].
///
/// `#[non_exhaustive]`: more metrics can be added in a minor release; consumers
/// read the fields they care about.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HandMetrics {
    /// Total chips in the pot right now (all pots), before your call.
    pub pot: u32,
    /// Chips you must put in to call (`0` if you can check).
    pub to_call: u32,
    /// Pot odds as the fraction of the post-call pot your call contributes — i.e.
    /// the equity you need to break even by calling. `None` when nothing is owed.
    pub pot_odds: Option<f64>,
    /// Stack-to-pot ratio: your stack ÷ the current pot. `None` before any chips
    /// are in the pot.
    pub spr: Option<f64>,
    /// Your stack measured in big blinds.
    pub stack_in_bb: f64,
    /// The call price measured in big blinds.
    pub call_in_bb: f64,
    /// Players still live in the hand.
    pub players_remaining: usize,
}

impl PlayerView {
    /// Start building a `PlayerView` outside the engine (e.g. for an agent's unit
    /// tests). All fields start at neutral defaults; override what the test needs.
    /// New fields added in future releases pick up defaults here, so existing
    /// `build()` calls keep compiling.
    pub fn builder() -> PlayerViewBuilder {
        PlayerViewBuilder {
            view: PlayerView {
                you: Uuid::nil(),
                name: String::new(),
                street: Street::Preflop,
                hole: [
                    Card::new(Rank::Ace, Suit::Spade),
                    Card::new(Rank::King, Suit::Heart),
                ],
                board: Vec::new(),
                chips: 0,
                amount_owed: 0,
                current_bet: 0,
                min_raise_to: 0,
                pot_total: 0,
                players_remaining: 0,
                legal_actions: Vec::new(),
                big_blind: 0,
                seats: Vec::new(),
                button_seat: None,
            },
        }
    }

    /// Derive ready-to-display [`HandMetrics`] for this decision (pot odds, SPR,
    /// stack depth, …) — the data a training overlay renders.
    pub fn metrics(&self) -> HandMetrics {
        let pot = self.pot_total as f64;
        let owed = self.amount_owed as f64;
        let pot_odds = if self.amount_owed == 0 {
            None
        } else {
            Some(owed / (pot + owed))
        };
        let spr = if self.pot_total == 0 {
            None
        } else {
            Some(self.chips as f64 / pot)
        };
        let bb = self.big_blind as f64;
        let (stack_in_bb, call_in_bb) = if bb > 0.0 {
            (self.chips as f64 / bb, owed / bb)
        } else {
            (0.0, 0.0)
        };
        HandMetrics {
            pot: self.pot_total,
            to_call: self.amount_owed,
            pot_odds,
            spr,
            stack_in_bb,
            call_in_bb,
            players_remaining: self.players_remaining,
        }
    }
}

/// Builder for [`PlayerView`] (which is `#[non_exhaustive]`, so it can't be made
/// with a struct literal outside this crate). Obtain one from [`PlayerView::builder`].
pub struct PlayerViewBuilder {
    view: PlayerView,
}

impl PlayerViewBuilder {
    /// Finish building the [`PlayerView`].
    pub fn build(self) -> PlayerView {
        self.view
    }

    /// Set the deciding player's id (defaults to [`Uuid::nil`]).
    pub fn you(mut self, you: Uuid) -> Self {
        self.view.you = you;
        self
    }
    /// Set the deciding player's display name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.view.name = name.into();
        self
    }
    /// Set the current betting street (defaults to [`Street::Preflop`]).
    pub fn street(mut self, street: Street) -> Self {
        self.view.street = street;
        self
    }
    /// Set the player's two hole cards.
    pub fn hole(mut self, hole: [Card; 2]) -> Self {
        self.view.hole = hole;
        self
    }
    /// Set the shared board cards (0, 3, 4, or 5).
    pub fn board(mut self, board: Vec<Card>) -> Self {
        self.view.board = board;
        self
    }
    /// Set the player's remaining stack.
    pub fn chips(mut self, chips: u32) -> Self {
        self.view.chips = chips;
        self
    }
    /// Set the chips the player must put in to call.
    pub fn amount_owed(mut self, amount_owed: u32) -> Self {
        self.view.amount_owed = amount_owed;
        self
    }
    /// Set the amount to match this street.
    pub fn current_bet(mut self, current_bet: u32) -> Self {
        self.view.current_bet = current_bet;
        self
    }
    /// Set the smallest legal raise-to total.
    pub fn min_raise_to(mut self, min_raise_to: u32) -> Self {
        self.view.min_raise_to = min_raise_to;
        self
    }
    /// Set the total chips currently in all pots.
    pub fn pot_total(mut self, pot_total: u32) -> Self {
        self.view.pot_total = pot_total;
        self
    }
    /// Set the number of players still in the hand.
    pub fn players_remaining(mut self, players_remaining: usize) -> Self {
        self.view.players_remaining = players_remaining;
        self
    }
    /// Set the legal actions available this turn.
    pub fn legal_actions(mut self, legal_actions: Vec<LegalAction>) -> Self {
        self.view.legal_actions = legal_actions;
        self
    }
    /// Set the table's big blind amount.
    pub fn big_blind(mut self, big_blind: u32) -> Self {
        self.view.big_blind = big_blind;
        self
    }
    /// Set the public table roster (defaults to empty).
    pub fn seats(mut self, seats: Vec<SeatView>) -> Self {
        self.view.seats = seats;
        self
    }
    /// Set the dealer button's seat index (defaults to `None`).
    pub fn button_seat(mut self, button_seat: Option<usize>) -> Self {
        self.view.button_seat = button_seat;
        self
    }
}

/// Something that decides a player's action from a [`PlayerView`].
///
/// `&mut self` lets an agent carry mutable state such as a seedable RNG. Returning
/// a `Result` gives human quit/EOF — and future fallible (e.g. network/LLM) agents
/// — a non-panicking path.
///
/// Three methods form the agent lifecycle: [`decide`](PokerAgent::decide) (act),
/// [`observe`](PokerAgent::observe) (learn during play), and
/// [`session_ended`](PokerAgent::session_ended) (persist what was learned). Only
/// `decide` is required; the two learning hooks default to no-ops so a stateless
/// agent ignores them and adding them stays backwards-compatible.
pub trait PokerAgent {
    fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError>;

    /// Observe a public [`GameEvent`] as a hand unfolds. The default is a no-op, so
    /// simple agents ignore it; a model-backed opponent uses it to update its model
    /// of the table (action tendencies, [`ShowdownReveal`](GameEvent::ShowdownReveal)
    /// holdings, pot outcomes). The engine emits this stream to its
    /// [`GameObserver`](crate::events::GameObserver); a front-end that wants its
    /// agents to learn fans the same events into them through this hook.
    fn observe(&mut self, _event: &GameEvent) {}

    /// Called once when a play session ends, so a stateful agent can persist what it
    /// learned (e.g. flush a player model to storage). The default is a no-op. This
    /// drives no engine behavior — the front-end that owns the agents calls it at
    /// shutdown.
    fn session_ended(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_compute_pot_odds_and_stack_depth() {
        // Facing a 10 call into a 30 pot: pot odds = 10/40 = 0.25; spr = 100/30.
        let view = PlayerView::builder()
            .chips(100)
            .amount_owed(10)
            .pot_total(30)
            .big_blind(2)
            .players_remaining(3)
            .build();
        let m = view.metrics();
        assert_eq!(m.pot, 30);
        assert_eq!(m.to_call, 10);
        assert_eq!(m.pot_odds, Some(0.25));
        assert_eq!(m.spr, Some(100.0 / 30.0));
        assert_eq!(m.stack_in_bb, 50.0);
        assert_eq!(m.call_in_bb, 5.0);
        assert_eq!(m.players_remaining, 3);
    }

    #[test]
    fn metrics_have_no_pot_odds_when_check_is_free() {
        let view = PlayerView::builder()
            .chips(100)
            .amount_owed(0)
            .pot_total(20)
            .big_blind(2)
            .build();
        let m = view.metrics();
        assert_eq!(m.pot_odds, None);
        assert_eq!(m.spr, Some(5.0));
    }
}
