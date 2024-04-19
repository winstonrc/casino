use crate::card::Card;

#[derive(Clone, Debug)]
pub struct Hand {
    pub cards: Vec<Card>,
}

impl Hand {
    pub fn print_symbols(&self) {
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
