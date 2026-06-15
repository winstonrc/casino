use std::fmt;

use serde::{Deserialize, Serialize};

use crate::card::Card;

/// An ordered collection of cards.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Hand {
    /// The cards in insertion order.
    pub cards: Vec<Card>,
}

impl Hand {
    /// Creates an empty hand.
    pub fn new() -> Self {
        Self { cards: Vec::new() }
    }

    /// Creates a hand containing the provided cards in their existing order.
    pub fn new_from_cards(cards: Vec<Card>) -> Self {
        Self { cards }
    }

    /// Returns the cards in the Hand.
    pub fn get_cards(&self) -> &Vec<Card> {
        &self.cards
    }

    /// Appends a Card to the back of the Hand.
    pub fn push(&mut self, card: Card) {
        self.cards.push(card);
    }

    /// Returns the Card at the back of the Hand if any.
    pub fn pop(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    /// Returns the hand rendered as a space-separated string of cards, honoring
    /// the current card display style (see [`crate::card::set_glyph_display`]).
    pub fn to_symbols(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Hand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cards: Vec<String> = self.cards.iter().map(|card| card.to_string()).collect();
        write!(f, "{}", cards.join(" "))
    }
}
