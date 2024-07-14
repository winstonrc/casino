use crate::card::Card;

#[derive(Clone, Debug)]
pub struct Hand {
    pub cards: Vec<Card>,
}

impl Hand {
    pub fn new() -> Self {
        let cards = Vec::new();

        Self { cards }
    }

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
        if let Some(card) = self.cards.pop() {
            return Some(card);
        }

        None
    }

    pub fn to_string(&self) -> String {
        let mut cards_string = String::new();

        for (i, &card) in self.cards.iter().enumerate() {
            cards_string.push_str(&card.to_string());

            if i < self.cards.len() - 1 {
                cards_string.push_str(" ");
            }
        }

        cards_string
    }

    pub fn to_symbols(&self) -> String {
        let mut card_symbols = String::new();

        for (i, &card) in self.cards.iter().enumerate() {
            card_symbols.push_str(&card.to_string());

            if i < self.cards.len() - 1 {
                card_symbols.push_str(" ");
            }
        }

        card_symbols
    }
}
