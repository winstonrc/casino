use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::card::{Card, Rank, Suit};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Deck {
    cards: Vec<Card>,
}

impl Default for Deck {
    /// A new, full 52-card deck (same as [`Deck::new`]).
    fn default() -> Self {
        Self::new()
    }
}

impl Deck {
    /// Create a new deck with 52 cards (13 cards for each of the 4 suits).
    pub fn new() -> Self {
        let mut cards = Vec::<Card>::new();

        for suit in Suit::iter() {
            for rank in Rank::iter() {
                let card = Card::new(rank, suit);
                cards.push(card);
            }
        }

        Self { cards }
    }

    /// Creates a new deck from a given set of cards.
    pub fn from_cards(cards: Vec<Card>) -> Self {
        Deck { cards }
    }

    /// Checks if a given card is in the deck.
    pub fn contains(&self, card: &Card) -> bool {
        self.cards.contains(card)
    }

    /// Deals a card with the default face_up value.
    ///
    /// Returns `None` when the deck is exhausted.
    pub fn deal(&mut self) -> Option<Card> {
        self.cards.pop()
    }

    /// Deals a card face up with the Rank and Suit visible.
    ///
    /// Returns `None` when the deck is exhausted.
    pub fn deal_face_up(&mut self) -> Option<Card> {
        let mut card = self.cards.pop()?;
        card.face_up = true;
        Some(card)
    }

    /// Deals a card face down with the Rank and Suit hidden.
    ///
    /// Returns `None` when the deck is exhausted.
    pub fn deal_face_down(&mut self) -> Option<Card> {
        let mut card = self.cards.pop()?;
        card.face_up = false;
        Some(card)
    }

    /// Inserts a given card into the provided position in the deck.
    pub fn insert(&mut self, position: usize, card: Card) -> Result<(), &'static str> {
        if position > self.cards.len() {
            return Err("Position out of bounds.");
        }

        self.cards.insert(position, card);
        Ok(())
    }

    /// Inserts a given card at the bottom of the deck.
    pub fn insert_at_bottom(&mut self, card: Card) -> Result<(), &'static str> {
        self.cards.insert(0, card);
        Ok(())
    }

    /// Inserts a given card into the middle of the deck.
    pub fn insert_at_middle(&mut self, card: Card) -> Result<(), &'static str> {
        let middle_position = self.cards.len() / 2;

        self.cards.insert(middle_position, card);
        Ok(())
    }

    /// Inserts a given card at the top of the deck.
    pub fn insert_at_top(&mut self, card: Card) -> Result<(), &'static str> {
        self.cards.push(card);
        Ok(())
    }

    /// Returns whether or not the deck is empty.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Returns the size of the deck.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Returns a reference to the card on top of the deck (the next one [`deal`]
    /// would return) without removing it.
    ///
    /// [`deal`]: Deck::deal
    pub fn peek(&self) -> Option<&Card> {
        self.cards.last()
    }

    /// Iterates over the cards in the deck. The top card — the next to be dealt —
    /// is yielded **last**, since dealing pops from the tail. Returns a slice
    /// iterator, so it supports `rev`, `len`, and the other slice-iterator methods.
    pub fn iter(&self) -> std::slice::Iter<'_, Card> {
        self.cards.iter()
    }

    /// Removes a given card from the deck.
    ///
    /// The deal() function should normally be used instead of this.
    pub fn remove(&mut self, card: &Card) -> Result<(), &'static str> {
        if !self.cards.contains(card) {
            return Err("Card is not in the deck.");
        }

        self.cards.retain(|value| value != card);
        Ok(())
    }

    /// Shuffles the cards in the deck using a thread-local RNG.
    ///
    /// For reproducible shuffles (seeded simulations, replays, tests), use
    /// [`shuffle_with`](Deck::shuffle_with) with a seeded RNG.
    pub fn shuffle(&mut self) -> &mut Self {
        self.shuffle_with(&mut thread_rng())
    }

    /// Shuffles the cards in the deck using the provided RNG. Pass a seeded RNG
    /// (e.g. `rand::rngs::StdRng::seed_from_u64(..)`) for a deterministic shuffle.
    pub fn shuffle_with<R: rand::Rng + ?Sized>(&mut self, rng: &mut R) -> &mut Self {
        self.cards.shuffle(rng);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::card::{Card, Rank, Suit};

    #[test]
    fn new_deck_has_correct_number_of_cards() {
        let deck = Deck::new();
        assert_eq!(deck.len(), 52);
    }

    #[test]
    fn new_deck_contains_all_cards() {
        let deck = Deck::new();

        for suit in Suit::iter() {
            for rank in Rank::iter() {
                let card = Card::new(rank, suit);
                assert!(deck.contains(&card));
            }
        }
    }

    #[test]
    fn shuffling_cards_works() {
        let unshuffled_deck = Deck::new();
        let mut shuffled_deck = unshuffled_deck.clone();
        shuffled_deck.shuffle();

        assert_ne!(unshuffled_deck.cards, shuffled_deck.cards);
    }

    #[test]
    fn dealing_cards_works() {
        let mut deck = Deck::new();

        if let Some(_card) = deck.deal() {
            assert_eq!(deck.cards.len(), 51);
        }
    }

    #[test]
    fn shuffle_with_is_deterministic_for_a_seed() {
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut a = Deck::new();
        let mut b = Deck::new();
        a.shuffle_with(&mut StdRng::seed_from_u64(42));
        b.shuffle_with(&mut StdRng::seed_from_u64(42));
        assert_eq!(a, b, "the same seed must produce the same shuffle");
    }

    #[test]
    fn deck_round_trips_through_json() {
        let deck = Deck::new();
        let json = serde_json::to_string(&deck).expect("serialize");
        let back: Deck = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deck, back);
    }

    #[test]
    fn dealing_from_empty_deck_returns_none() {
        let mut deck = Deck::from_cards(Vec::new());
        assert!(deck.is_empty());
        assert_eq!(deck.deal(), None);
        assert_eq!(deck.deal_face_up(), None);
        assert_eq!(deck.deal_face_down(), None);
    }

    #[test]
    fn peek_shows_the_card_deal_returns() {
        let mut deck = Deck::new();
        let peeked = deck.peek().copied();
        let dealt = deck.deal();
        assert_eq!(peeked, dealt, "peek must show the next card deal pops");
        assert_eq!(deck.iter().count(), 51);
    }
}
