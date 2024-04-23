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

    pub fn to_string(&self) {
        let mut string_to_print = String::new();

        for &card in self.cards.iter() {
            string_to_print.push_str(&card.to_string());
        }

        println!("{}", string_to_print);
    }

    pub fn to_symbols(&self) {
        let mut string_to_print = String::new();

        for (i, &card) in self.cards.iter().enumerate() {
            string_to_print.push_str(&card.to_symbol().to_string());

            if i < self.cards.len() - 1 {
                string_to_print.push_str(" ");
            }
        }

        println!("{}", string_to_print);
    }
}
