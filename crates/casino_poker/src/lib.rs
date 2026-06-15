//! Poker hand evaluation, betting and pot logic, and a resumable Texas Hold'em
//! engine.
//!
//! Most applications start with [`hand_rankings::evaluate`] for hand comparison
//! or [`games::texas_hold_em::TexasHoldEm`] for complete game management.
#![warn(missing_docs)]

/// Poker game engines.
pub mod games {
    /// A no-limit Texas Hold'em engine.
    pub mod texas_hold_em;
}
/// Agent-facing decisions and player snapshots.
pub mod agent;
/// Betting actions, validation, and per-street state.
pub mod betting;
/// Serializable game events and observers.
pub mod events;
/// Poker hand evaluation and comparison.
pub mod hand_rankings;
/// Player identity and chip-stack types.
pub mod player;
/// Main-pot and side-pot construction and distribution.
pub mod pot;

/// Re-export of the card types used by this crate's API.
pub use casino_cards;
/// Re-export of the UUID type used for player identity.
pub use uuid;
