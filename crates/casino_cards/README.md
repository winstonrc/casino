[![crates.io](https://img.shields.io/crates/v/casino_cards.svg)](https://crates.io/crates/casino_cards) [![Cards Test](https://github.com/winstonrc/casino/actions/workflows/casino_cards.yml/badge.svg?branch=main)](https://github.com/winstonrc/casino/actions/workflows/casino_cards.yml)

# casino_cards

A library that provides a deck of playing cards that can be used for various card games.

## Usage

### Deck creation

```rust
use casino_cards::deck::Deck;

fn main() {
    let mut deck = Deck::new();
    deck.shuffle();

    let card1 = deck.deal();
    // A card can be inserted at a specified position in the deck.
    deck.insert(12, card);

    let card2 = deck.deal();
    // A card can be inserted at the bottom of the deck.
    deck.insert_at_bottom(card2);

    let card3 = deck.deal();
    // A card can be inserted at the middle of the deck.
    deck.insert_at_middle(card3);

    let card4 = deck.deal();
    // A card can be inserted at the top of the deck.
    deck.insert_at_top(card4);

    deck.shuffle();
}
```

### Card creation

```rust
use casino_cards::card;
use casino_cards::card::{Card, Rank, Suit};

fn main() {
    // A card can be created with the new() method.
    let two_of_diamonds = Card::new(Rank::Two, Suit::Diamond);

    // Or a card can be created by using a macro.
    let two_of_clubs = card!(Two, Club);
}
```

### Hand creation

```rust
use casino_cards::card::Card;
use casino_cards::deck::Deck;
use casino_cards::hand::Hand;

fn main() {
    let mut deck = Deck::new();
    deck.shuffle()

    // A hand can be created by pushing cards into it.
    let card1 = deck.deal();
    let card2 = deck.deal();
    let mut hand = Hand::new();
    hand.push(card1);
    hand.push(card2);

    // Or a hand can be created from an existing Vec<Card>.
    let mut cards: Vec<Card> = Vec::new();
    let card3 = deck.deal();
    let card4 = deck.deal();
    cards.push(card3);
    cards.push(card4);
    let mut hand2 = Hand::new_from_cards(cards);
}
```
