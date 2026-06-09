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

use casino_cards::card::Card;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::betting::{LegalAction, PlayerAction};

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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
}

/// Something that decides a player's action from a [`PlayerView`].
///
/// `&mut self` lets an agent carry mutable state such as a seedable RNG. Returning
/// a `Result` gives human quit/EOF — and future fallible (e.g. network/LLM) agents
/// — a non-panicking path.
pub trait PokerAgent {
    fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError>;
}
