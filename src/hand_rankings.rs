use cards::card::{Card, Rank, Suit};
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub enum HandRank {
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

impl HandRank {
    fn rank_name(value: u32) -> String {
        let rank = match value {
            2 => Rank::Two,
            3 => Rank::Three,
            4 => Rank::Four,
            5 => Rank::Five,
            6 => Rank::Six,
            7 => Rank::Seven,
            8 => Rank::Eight,
            9 => Rank::Nine,
            10 => Rank::Ten,
            11 => Rank::Jack,
            12 => Rank::Queen,
            13 => Rank::King,
            14 => Rank::Ace,
            _ => return value.to_string(),
        };

        match rank {
            Rank::Jack => "Jack".to_string(),
            Rank::Queen => "Queen".to_string(),
            Rank::King => "King".to_string(),
            Rank::Ace => "Ace".to_string(),
            _ => value.to_string(),
        }
    }
}

impl fmt::Display for HandRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let printable = match self {
            HandRank::HighCard(value) => format!("a High Card of {}", HandRank::rank_name(*value)),
            HandRank::Pair(cards) => format!("a Pair of {}s", HandRank::rank_name(*cards)),
            HandRank::TwoPair(cards) => format!("Two Pairs of {}", HandRank::rank_name(*cards)),
            HandRank::ThreeOfAKind(cards) => {
                format!("Three of a Kind of {}", HandRank::rank_name(*cards))
            }
            HandRank::Straight(cards) => format!("a Straight of {}", HandRank::rank_name(*cards)),
            HandRank::Flush(cards) => format!("a Flush of {}", HandRank::rank_name(*cards)),
            HandRank::FullHouse(cards) => {
                format!("a Full House of {}", HandRank::rank_name(*cards))
            }
            HandRank::FourOfAKind(cards) => {
                format!("Four of a Kind of {}", HandRank::rank_name(*cards))
            }
            HandRank::StraightFlush(cards) => {
                format!("a Straight Flush of {}", HandRank::rank_name(*cards))
            }
            HandRank::RoyalFlush(cards) => format!("a Royal Flush {}", HandRank::rank_name(*cards)),
        };

        write!(f, "{}", printable)
    }
}

/// Determine the value of the given hand.
pub fn rank_hand(cards: Vec<&Card>) -> HandRank {
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

    let (is_pair, pair_value) = check_for_pair(&cards);

    if is_pair {
        return HandRank::Pair(pair_value.unwrap().into());
    }

    let high_card_value = get_high_card_value(&cards);
    HandRank::HighCard(high_card_value.into())
}

/// Returns: A u8 representing the value of the highest card provided for a HandRank::HighCard.
///
/// Note: Unlike the other ranking methods, this does not return a tuple with a bool since it is executed last after exhausting all other hand ranking options.
///
/// Example: An Ace high card will return 14.
fn get_high_card_value(cards: &Vec<&Card>) -> u8 {
    let mut high_card_value: u8 = 0;
    for card in cards {
        let card_rank_value = card.rank.value();

        if card_rank_value > high_card_value {
            high_card_value = card_rank_value;
        }
    }

    high_card_value
}

/// Checks if the provided cards contain a HandRank::Pair.
///
/// Returns: A tuple containing a bool and an Option containing the numerical value of the relevant cards if any.
///
/// Example: a pair of 2s will return (true, 22), and a pair of Kings (13s) will return (true, 1313).
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
        // Convert the high rank into a pair
        // e.g. 2 -> 22
        let pair_value = high_pair_rank_value * 10 + high_pair_rank_value;
        return (true, Some(pair_value));
    }

    (false, None)
}

/// Checks if the provided cards contain a HandRank::TwoPair.
///
/// Returns: A tuple containing a bool and an Option containing the numerical value of the relevant cards if any.
///
/// Example: A pair of 2s and a pair of Kings (13s) will return (true, 221313).
fn is_two_pair(cards: &Vec<&Card>) -> bool {
    if cards.len() < 4 {
        return false;
    }

    // todo: implement

    false
}

fn get_two_pair_value(cards: &Vec<&Card>) -> HandRank {
    // todo: implement
    let value = 0;

    HandRank::TwoPair(value)
}

/// Checks if the provided cards contain a HandRank::ThreeOfAKind.
///
/// Returns: A tuple containing a bool and an Option containing the numerical value of the relevant cards if any.
///
/// Example: Three 2s will return (true, 222).
fn is_three_of_a_kind(cards: &Vec<&Card>) -> bool {
    if cards.len() < 3 {
        return false;
    }

    // todo: implement

    false
}

fn get_three_of_a_kind_value(cards: &Vec<&Card>) -> HandRank {
    // todo: implement
    let value = 0;

    HandRank::ThreeOfAKind(value)
}

// todo: implement
/// Checks if the provided cards contain a HandRank::Straight.
///
/// This also checks for both Ace low and Ace high when determining if a straight is present.
///
/// Returns: A tuple containing a bool and an Option containing the numerical value of the relevant cards if any.
///
/// Example: A straight of 2, 3, 4, 5, and 6 will return (true, 23456).
///
/// Example: A straight of Ace (low = 1), 2, 3, 4, and 5 will return (true, 12345).
///
/// Example: A straight of 10, J (11), Q (12), K (13), Ace (high = 14) will return (true, 1011121314).
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
/// This sorts the cards by their rank to assist in determining straights.
///
/// Returns: A new Vec<Card> that represents the provided Vec<Card> in a sorted order.
///
/// Note: This method sorts Aces as high and requires other methods to implement configuring Aces as low if needed.
fn sort_cards_by_rank(cards: &Vec<&Card>) -> Vec<Card> {
    let mut sorted_cards: Vec<Card> = Vec::new();

    // todo: implement sorting
    for &card in cards.iter() {
        sorted_cards.push(card.clone());
    }

    sorted_cards
}

fn get_straight_value(cards: &Vec<&Card>) -> HandRank {
    // todo: implement
    let value = 0;

    HandRank::Straight(value)
}

/// Checks if the provided cards contain a HandRank::Flush.
///
/// Returns: A tuple containing a bool and an Option containing the numerical value of the relevant cards if any.
///
/// Example: A flush of 2♥, 5♥, 7♥, 9♥, and J♥ (11♥) will return (true, 257911).
fn is_flush(cards: &Vec<&Card>) -> bool {
    if cards.len() < 5 {
        return false;
    }

    // todo: implement

    false
}

fn get_flush_value(cards: &Vec<&Card>) -> HandRank {
    // todo: implement
    let value = 0;

    HandRank::Flush(value)
}

/// Checks if the provided cards contain a HandRank::FullHouse.
///
/// Returns: A tuple containing a bool and an Option containing the numerical value of the relevant cards if any.
///
/// Example: Two 2s and three 3s will return (true, 22333).
fn is_full_house(cards: &Vec<&Card>) -> bool {
    if cards.len() < 5 {
        return false;
    }

    // todo: implement

    false
}

fn get_full_house_value(cards: &Vec<&Card>) -> HandRank {
    // todo: implement
    let value = 0;

    HandRank::FullHouse(value)
}

/// Checks if the provided cards contain a HandRank::FourOfAKind.
///
/// Returns: A tuple containing a bool and an Option containing the numerical value of the relevant cards if any.
///
/// Example: Four 2s will return (true, 2222).
fn is_four_of_a_kind(cards: &Vec<&Card>) -> bool {
    if cards.len() < 4 {
        return false;
    }

    // todo: implement

    false
}

fn get_four_of_a_kind_value(cards: &Vec<&Card>) -> HandRank {
    // todo: implement
    let value = 0;

    HandRank::FourOfAKind(value)
}

/// Checks if the provided cards contain a HandRank::StraightFlush.
///
/// Returns: A tuple containing a bool and an Option containing the numerical value of the relevant cards if any.
///
/// Example: A flush of 2♣, 3♣, 4♣, 5♣, and 6♣ will return (true, 23456).
///
/// Example: A flush of A♦ (low = 1♦), 2♦, 3♦, 4♦, and 5♦ will return (true, 12345).
fn is_straight_flush(cards: &Vec<&Card>) -> bool {
    if is_straight(cards) && is_flush(cards) {
        return true;
    }

    false
}

fn get_straight_flush_value(cards: &Vec<&Card>) -> HandRank {
    // todo: implement
    let value = 0;

    HandRank::StraightFlush(value)
}

/// Checks if the provided cards contain a HandRank::RoyalFlush.
///
/// Returns: A tuple containing a bool and an Option containing the numerical value of the relevant cards if any.
///
/// Note: This is the highest variant of a HandRank::StraightFlush and may be removed in the future if it is determined to be redundant.
///
/// Example: A flush of 10♠, J♠ (11♠), Q♠ (12♠), K♠ (13♠) and A♠ (high = 14♠) will return (true, 1011121314).
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

fn get_royal_flush_value(cards: &Vec<&Card>) -> HandRank {
    // todo: implement
    let value = 0;

    HandRank::RoyalFlush(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_cards_values_are_correct() {
        assert_ne!(HandRank::HighCard(1), HandRank::HighCard(13));
        assert_eq!(HandRank::HighCard(1), HandRank::HighCard(1));

        assert!(HandRank::HighCard(1) < HandRank::HighCard(2));
        assert!(HandRank::HighCard(2) < HandRank::HighCard(3));
        assert!(HandRank::HighCard(3) < HandRank::HighCard(4));
        assert!(HandRank::HighCard(4) < HandRank::HighCard(5));
        assert!(HandRank::HighCard(5) < HandRank::HighCard(6));
        assert!(HandRank::HighCard(6) < HandRank::HighCard(7));
        assert!(HandRank::HighCard(7) < HandRank::HighCard(8));
        assert!(HandRank::HighCard(8) < HandRank::HighCard(9));
        assert!(HandRank::HighCard(9) < HandRank::HighCard(10));
        assert!(HandRank::HighCard(10) < HandRank::HighCard(11));
        assert!(HandRank::HighCard(11) < HandRank::HighCard(12));
        assert!(HandRank::HighCard(12) < HandRank::HighCard(13));
        assert!(HandRank::HighCard(13) < HandRank::HighCard(14));
    }

    #[test]
    fn hand_rankings_are_ordered_correctly() {
        assert!(HandRank::HighCard(14) < HandRank::Pair(22));
        assert!(HandRank::Pair(1313) < HandRank::TwoPair(2233));
        assert!(HandRank::TwoPair(13131414) < HandRank::ThreeOfAKind(222));
        assert!(HandRank::ThreeOfAKind(141414) < HandRank::Straight(12345));
        assert!(HandRank::Straight(1011121314) < HandRank::Flush(257911));
        assert!(HandRank::Flush(257911) < HandRank::FullHouse(22333));
        assert!(HandRank::FullHouse(1313141414) < HandRank::FourOfAKind(2222));
        assert!(HandRank::FourOfAKind(14141414) < HandRank::StraightFlush(12345));
        assert!(HandRank::StraightFlush(910111213) < HandRank::RoyalFlush(1011121314));
    }

    #[test]
    fn high_card_is_ranked_correctly() {
        let card1 = Card::new(Rank::Two, Suit::Club);
        let card2 = Card::new(Rank::Ace, Suit::Heart);
        let card3 = Card::new(Rank::Four, Suit::Spade);
        let card4 = Card::new(Rank::Ten, Suit::Heart);
        let card5 = Card::new(Rank::Six, Suit::Spade);
        let card6 = Card::new(Rank::Queen, Suit::Club);
        let card7 = Card::new(Rank::Jack, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::HighCard(14));
    }

    #[test]
    fn pair_is_ranked_correctly() {
        let card1 = Card::new(Rank::Two, Suit::Club);
        let card2 = Card::new(Rank::Two, Suit::Heart);
        let card3 = Card::new(Rank::Four, Suit::Spade);
        let card4 = Card::new(Rank::Ten, Suit::Heart);
        let card5 = Card::new(Rank::Six, Suit::Spade);
        let card6 = Card::new(Rank::Queen, Suit::Club);
        let card7 = Card::new(Rank::Jack, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::Pair(22));
    }

    #[test]
    fn two_pair_is_ranked_correctly() {
        let card1 = Card::new(Rank::Two, Suit::Club);
        let card2 = Card::new(Rank::Two, Suit::Heart);
        let card3 = Card::new(Rank::King, Suit::Spade);
        let card4 = Card::new(Rank::King, Suit::Heart);
        let card5 = Card::new(Rank::Six, Suit::Spade);
        let card6 = Card::new(Rank::Queen, Suit::Club);
        let card7 = Card::new(Rank::Jack, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::TwoPair(221313));
    }

    #[test]
    fn three_of_a_kind_is_ranked_correctly() {
        let card1 = Card::new(Rank::Two, Suit::Club);
        let card2 = Card::new(Rank::Two, Suit::Heart);
        let card3 = Card::new(Rank::Two, Suit::Spade);
        let card4 = Card::new(Rank::Four, Suit::Heart);
        let card5 = Card::new(Rank::Six, Suit::Spade);
        let card6 = Card::new(Rank::Queen, Suit::Club);
        let card7 = Card::new(Rank::Jack, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::ThreeOfAKind(222));
    }

    #[test]
    fn straight_is_ranked_correctly() {
        let card1 = Card::new(Rank::Two, Suit::Club);
        let card2 = Card::new(Rank::Three, Suit::Heart);
        let card3 = Card::new(Rank::Four, Suit::Spade);
        let card4 = Card::new(Rank::Five, Suit::Heart);
        let card5 = Card::new(Rank::Six, Suit::Spade);
        let card6 = Card::new(Rank::Queen, Suit::Club);
        let card7 = Card::new(Rank::Jack, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::Straight(23456));
    }

    #[test]
    fn ace_low_straight_is_ranked_correctly() {
        let card1 = Card::new(Rank::Ace, Suit::Spade);
        let card2 = Card::new(Rank::Two, Suit::Club);
        let card3 = Card::new(Rank::Three, Suit::Heart);
        let card4 = Card::new(Rank::Four, Suit::Spade);
        let card5 = Card::new(Rank::Five, Suit::Heart);
        let card6 = Card::new(Rank::Queen, Suit::Club);
        let card7 = Card::new(Rank::Jack, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::Straight(12345));
    }

    #[test]
    fn ace_high_straight_is_ranked_correctly() {
        let card1 = Card::new(Rank::Ten, Suit::Club);
        let card2 = Card::new(Rank::Jack, Suit::Heart);
        let card3 = Card::new(Rank::Queen, Suit::Spade);
        let card4 = Card::new(Rank::King, Suit::Heart);
        let card5 = Card::new(Rank::Ace, Suit::Spade);
        let card6 = Card::new(Rank::Two, Suit::Club);
        let card7 = Card::new(Rank::Five, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::Straight(12345));
    }

    #[test]
    fn flush_is_ranked_correctly() {
        let card1 = Card::new(Rank::Two, Suit::Heart);
        let card2 = Card::new(Rank::Five, Suit::Heart);
        let card3 = Card::new(Rank::Seven, Suit::Heart);
        let card4 = Card::new(Rank::King, Suit::Diamond);
        let card5 = Card::new(Rank::Six, Suit::Spade);
        let card6 = Card::new(Rank::Nine, Suit::Heart);
        let card7 = Card::new(Rank::Jack, Suit::Heart);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::Flush(257911));
    }

    #[test]
    fn full_house_is_ranked_correctly() {
        let card1 = Card::new(Rank::Two, Suit::Club);
        let card2 = Card::new(Rank::Two, Suit::Heart);
        let card3 = Card::new(Rank::Three, Suit::Spade);
        let card4 = Card::new(Rank::Three, Suit::Heart);
        let card5 = Card::new(Rank::Three, Suit::Diamond);
        let card6 = Card::new(Rank::Queen, Suit::Club);
        let card7 = Card::new(Rank::Jack, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::FullHouse(22333));
    }

    #[test]
    fn four_of_a_kind_is_ranked_correctly() {
        let card1 = Card::new(Rank::Two, Suit::Club);
        let card2 = Card::new(Rank::Two, Suit::Diamond);
        let card3 = Card::new(Rank::Two, Suit::Heart);
        let card4 = Card::new(Rank::Two, Suit::Spade);
        let card5 = Card::new(Rank::Six, Suit::Spade);
        let card6 = Card::new(Rank::Queen, Suit::Club);
        let card7 = Card::new(Rank::Jack, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::FourOfAKind(2222));
    }

    #[test]
    fn straight_flush_is_ranked_correctly() {
        let card1 = Card::new(Rank::Two, Suit::Club);
        let card2 = Card::new(Rank::Three, Suit::Club);
        let card3 = Card::new(Rank::Four, Suit::Club);
        let card4 = Card::new(Rank::Five, Suit::Club);
        let card5 = Card::new(Rank::Six, Suit::Club);
        let card6 = Card::new(Rank::Queen, Suit::Diamond);
        let card7 = Card::new(Rank::Jack, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::Straight(23456));
    }

    #[test]
    fn ace_low_straight_flush_is_ranked_correctly() {
        let card1 = Card::new(Rank::Ace, Suit::Diamond);
        let card2 = Card::new(Rank::Two, Suit::Diamond);
        let card3 = Card::new(Rank::Three, Suit::Diamond);
        let card4 = Card::new(Rank::Four, Suit::Diamond);
        let card5 = Card::new(Rank::Five, Suit::Diamond);
        let card6 = Card::new(Rank::Queen, Suit::Club);
        let card7 = Card::new(Rank::Jack, Suit::Spade);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::StraightFlush(12345));
    }

    #[test]
    fn royal_flush_aka_ace_high_straight_flush_is_ranked_correctly() {
        // Also tests to ignore the lower flush for 9 - K
        let card1 = Card::new(Rank::Nine, Suit::Spade);
        let card2 = Card::new(Rank::Ten, Suit::Spade);
        let card3 = Card::new(Rank::Jack, Suit::Spade);
        let card4 = Card::new(Rank::Queen, Suit::Spade);
        let card5 = Card::new(Rank::King, Suit::Spade);
        let card6 = Card::new(Rank::Ace, Suit::Spade);
        let card7 = Card::new(Rank::Two, Suit::Club);

        let mut cards: Vec<&Card> = vec![&card1, &card2, &card3, &card4, &card5, &card6, &card7];
        let hand_rank = rank_hand(cards);

        assert_eq!(hand_rank, HandRank::RoyalFlush(1011121314));
    }
}
