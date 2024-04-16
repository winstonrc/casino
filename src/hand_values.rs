use cards::card::{Card, Rank, Suit};

pub enum HandValue {
    /// Simple value of the card.
    /// Lowest: 2 – Highest: Ace.
    HighCard(u32),
    /// Two cards with the same value.
    Pair(u32),
    /// Two times two cards with the same value.
    TwoPair(u32),
    /// Three cards with the same value.
    ThreeOfAKind(u32),
    /// Sequence of 5 cards in increasing value, not of the same suit.
    /// Ace can precede 2 and follow up King.
    Straight(u32),
    /// 5 cards of the same suit, not in sequential order.
    Flush(u32),
    /// Combination of three of a kind and a pair/
    FullHouse(u32),
    /// Four cards of the same value.
    FourOfAKind(u32),
    /// Straight of the same suit.
    StraightFlush(u32),
    /// Straight comprising 10 - Ace of the same suit.
    RoyalFlush(u32),
}

/// Determine the value of the given hand.
fn rank_hand(cards: &Vec<Card>) -> HandValue {
    if cards.len() != 2 || cards.len() != 5 || cards.len() != 6 || cards.len() != 7 {
        panic!("Expected the cards count to be equal to 2 (pre-flop), 5 (post-flop), 6 (post-turn), or 7 (post-river) to rank the hand.\nThe cards count provided was: {}.", cards.len())
    }

    if is_royal_flush(cards) {
        get_royal_flush_value(cards)
    }

    if is_straight_flush(cards) {
        get_straight_flush_value(cards)
    }

    if is_four_of_a_kind(cards) {
        get_four_of_a_kind_value(cards)
    }

    if is_full_house(cards) {
        (get_full_house_value(cards))
    }

    if is_flush(cards) {
        get_flush_value(cards)
    }

    if is_straight(cards) {
        get_straight_value(cards)
    }

    if is_three_of_a_kind(cards) {
        get_three_of_a_kind_value(cards)
    }

    if is_two_pair(cards) {
        get_two_pair_value(cards)
    }

    if is_pair(cards) {
        get_pair_value(cards)
    }

    get_high_card_value(cards)
}

fn get_high_card_value(cards: &Vec<Card>) -> HandValue::HighCard {
    let mut max = 0;
    for card in cards {
        let card_rank_value = card.rank.value();

        if card_rank_value > max {
            max = card_rank_value;
        }
    }

    max
}

fn is_pair(hand: (Card, Card)) -> bool {
    if cards.len() < 2 {
        false
    }

    // todo: implement

    false
}

fn get_pair_value(cards: (Card, Card)) -> HandValue::Pair {
    let (card1, card2): (Card, Card) = cards;

    14 + 2 * (card1.rank.value() + card2.rank.value())
}

fn is_two_pair(cards: &Vec<Card>) -> bool {
    if cards.len() < 4 {
        false
    }

    // todo: implement

    false
}

fn get_two_pair_value(cards: &Vec<Card>) -> HandValue::TwoPair {
    // todo: implement
    0
}

fn is_three_of_a_kind(cards: &Vec<Card>) -> bool {
    if cards.len() < 3 {
        false
    }

    // todo: implement

    false
}

fn get_three_of_a_kind_value(cards: &Vec<Card>) -> HandValue::ThreeOfAKind {
    // todo: implement
    0
}

// todo: implement
fn is_straight(cards: &Vec<Card>) -> bool {
    if cards.len() < 5 {
        false
    }

    let sorted_cards = sort_cards_by_rank(cards);

    // Check if the cards have an Ace
    let contains_ace = sorted_cards.iter().any(|&card| card.rank == Rank::Ace);

    if contains_ace {
        // todo: implement
    } else {
        // todo: implement
    }

    false
}

// todo: implement
fn sort_cards_by_rank(cards: &Vec<Card>) -> Vec<Card> {
    // todo: implement
    let sorted_cards = cards.clone();
}

fn get_straight_value(cards: &Vec<Card>) -> HandValue::Straight {
    // todo: implement
    0
}

fn is_flush(cards: &Vec<Card>) -> bool {
    if cards.len() < 5 {
        false
    }

    // todo: implement

    false
}

fn get_flush_value(cards: &Vec<Card>) -> HandValue::Flush {
    // todo: implement
    0
}

fn is_full_house(cards: &Vec<Card>) -> bool {
    if cards.len() < 5 {
        false
    }

    // todo: implement

    false
}

fn get_full_house_value(cards: &Vec<Card>) -> HandValue::FullHouse {
    // todo: implement
    0
}

fn is_four_of_a_kind(cards: &Vec<Card>) -> bool {
    if cards.len() < 4 {
        false
    }

    // todo: implement

    false
}

fn get_four_of_a_kind_value(cards: &Vec<Card>) -> HandValue::FourOfAKind {
    // todo: implement
    0
}

fn is_straight_flush(cards: &Vec<Card>) -> bool {
    if is_straight(cards) && is_flush(cards) {
        true
    }

    false
}

fn get_straight_flush_value(cards: &Vec<Card>) -> HandValue::StraightFlush {
    // todo: implement
    0
}

fn is_royal_flush(cards: &Vec<Card>) -> bool {
    if cards.len() < 5 {
        false
    }

    if is_straight_flush(cards) {
        // todo: implement
        // Check cards contain 10 - Ace
    }

    false
}

fn get_royal_flush_value(cards: &Vec<Card>) -> HandValue::RoyalFlush {
    // todo: implement
    0
}
