use crate::card::{Card, Rank, Suit};
use rand::seq::SliceRandom;
use rand::thread_rng;
use strum::IntoEnumIterator;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deck {
    cards: Vec<Card>,
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

    /// Inserts a given card into the deck.
    pub fn insert(&mut self, card: Card) {
        self.cards.push(card);
    }

    /// Removes a given card from the deck.
    ///
    /// The deal() function should normally be used instead of this.
    pub fn remove(&mut self, card: Card) {
        self.cards.retain(|value| *value != card);
    }

    /// Returns the size of the deck.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Returns whether or not the deck is empty.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Shuffles the cards in the deck.
    pub fn shuffle(&mut self) -> &mut Self {
        let mut rng = thread_rng();
        self.cards.shuffle(&mut rng);
        self
    }

    pub fn deal(&mut self) -> Option<Card> {
        if let Some(card) = self.cards.pop() {
            Some(card)
        } else {
            eprintln!("Deck is empty.");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_number_of_cards() {
        let deck = Deck::new();
        assert_eq!(deck.len(), 52);
    }

    #[test]
    fn all_cards_are_included() {
        let deck = Deck::new();

        for suit in Suit::iter() {
            for rank in Rank::iter() {
                let card = Card::new(rank, suit);
                assert!(deck.contains(&card));
            }
        }
    }

    #[test]
    fn cards_shuffle_correctly() {
        let unshuffled_deck = Deck::new();
        let mut shuffled_deck = unshuffled_deck.clone();
        shuffled_deck.shuffle();

        assert_ne!(unshuffled_deck.cards, shuffled_deck.cards);
    }

    #[test]
    fn dealing_card_removes_it_from_the_deck() {
        let mut deck = Deck::new();
        deck.deal().unwrap();
        assert_eq!(deck.cards.len(), 51);
    }
}
