//! Playing-card primitives, hands, and decks for casino games.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Card ranks, suits, parsing, and display.
pub mod card;
/// A standard deck with dealing and shuffling operations.
pub mod deck;
/// An ordered collection of cards held together as a hand.
pub mod hand;
