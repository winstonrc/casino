use cards::card::{Card, Rank, Suit};
use std::collections::HashMap;

#[derive(Debug)]
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
pub fn rank_hand(cards: Vec<&Card>) -> HandValue {
    if cards.len() != 2 && cards.len() != 5 && cards.len() != 6 && cards.len() != 7 {
        panic!("Expected the cards count to be equal to 2 (pre-flop), 5 (post-flop), 6 (post-turn), or 7 (post-river) to rank the hand.\nThe cards count provided was: {}.", cards.len())
    }

    if is_royal_flush(&cards) {
        return get_royal_flush_value(&cards);
    }

    if is_straight_flush(&cards) {
        return get_straight_flush_value(&cards);
    }

    if is_four_of_a_kind(&cards) {
        return get_four_of_a_kind_value(&cards);
    }

    if is_full_house(&cards) {
        return get_full_house_value(&cards);
    }

    if is_flush(&cards) {
        return get_flush_value(&cards);
    }

    if is_straight(&cards) {
        return get_straight_value(&cards);
    }

    if is_three_of_a_kind(&cards) {
        return get_three_of_a_kind_value(&cards);
    }

    if is_two_pair(&cards) {
        return get_two_pair_value(&cards);
    }

    let (is_pair, pair) = check_for_pair(&cards);

    if is_pair {
        return HandValue::Pair(pair.unwrap().into());
    }

    return get_high_card_value(&cards);
}

fn get_high_card_value(cards: &Vec<&Card>) -> HandValue {
    let mut high_card_value: u8 = 0;
    for card in cards {
        let card_rank_value = card.rank.value();

        if card_rank_value > high_card_value {
            high_card_value = card_rank_value;
        }
    }

    HandValue::HighCard(high_card_value.into())
}

fn check_for_pair(cards: &Vec<&Card>) -> (bool, Option<u8>) {
    if cards.len() < 2 {
        return (false, None);
    }

    let mut ranks: HashMap<Rank, u32> = HashMap::new();

    for card in cards {
        *ranks.entry(card.rank).or_default() += 1;
    }

    let mut high_pair_rank_value = 0;
    for (rank, count) in ranks.iter() {
        let rank_value = rank.value();

        if *count == 2 && rank_value > high_pair_rank_value {
            high_pair_rank_value = rank_value;
        }
    }

    if high_pair_rank_value > 0 {
        return (true, Some(high_pair_rank_value));
    }

    (false, None)
}

fn is_two_pair(cards: &Vec<&Card>) -> bool {
    if cards.len() < 4 {
        return false;
    }

    // todo: implement

    false
}

fn get_two_pair_value(cards: &Vec<&Card>) -> HandValue {
    // todo: implement
    let value = 0;

    HandValue::TwoPair(value)
}

fn is_three_of_a_kind(cards: &Vec<&Card>) -> bool {
    if cards.len() < 3 {
        return false;
    }

    // todo: implement

    false
}

fn get_three_of_a_kind_value(cards: &Vec<&Card>) -> HandValue {
    // todo: implement
    let value = 0;

    HandValue::ThreeOfAKind(value)
}

// todo: implement
fn is_straight(cards: &Vec<&Card>) -> bool {
    if cards.len() < 5 {
        return false;
    }

    let sorted_cards = sort_cards_by_rank(cards);

    // Check if the cards have an Ace
    let contains_ace = sorted_cards.iter().any(|&card| card.rank == Rank::Ace);

    if contains_ace {
        // todo: implement Ace high / Ace low logic
    } else {
        // todo: implement
    }

    false
}

// todo: implement
fn sort_cards_by_rank(cards: &Vec<&Card>) -> Vec<Card> {
    let mut sorted_cards: Vec<Card> = Vec::new();

    // todo: implement sorting
    for &card in cards.iter() {
        sorted_cards.push(card.clone());
    }

    sorted_cards
}

fn get_straight_value(cards: &Vec<&Card>) -> HandValue {
    // todo: implement
    let value = 0;

    HandValue::Straight(value)
}

fn is_flush(cards: &Vec<&Card>) -> bool {
    if cards.len() < 5 {
        return false;
    }

    // todo: implement

    false
}

fn get_flush_value(cards: &Vec<&Card>) -> HandValue {
    // todo: implement
    let value = 0;

    HandValue::Flush(value)
}

fn is_full_house(cards: &Vec<&Card>) -> bool {
    if cards.len() < 5 {
        return false;
    }

    // todo: implement

    false
}

fn get_full_house_value(cards: &Vec<&Card>) -> HandValue {
    // todo: implement
    let value = 0;

    HandValue::FullHouse(value)
}

fn is_four_of_a_kind(cards: &Vec<&Card>) -> bool {
    if cards.len() < 4 {
        return false;
    }

    // todo: implement

    false
}

fn get_four_of_a_kind_value(cards: &Vec<&Card>) -> HandValue {
    // todo: implement
    let value = 0;

    HandValue::FourOfAKind(value)
}

fn is_straight_flush(cards: &Vec<&Card>) -> bool {
    if is_straight(cards) && is_flush(cards) {
        return true;
    }

    false
}

fn get_straight_flush_value(cards: &Vec<&Card>) -> HandValue {
    // todo: implement
    let value = 0;

    HandValue::StraightFlush(value)
}

fn is_royal_flush(cards: &Vec<&Card>) -> bool {
    if cards.len() < 5 {
        return false;
    }

    if is_straight_flush(cards) {
        // todo: implement
        // Check cards contain 10 - Ace
    }

    false
}

fn get_royal_flush_value(cards: &Vec<&Card>) -> HandValue {
    // todo: implement
    let value = 0;

    HandValue::RoyalFlush(value)
}
