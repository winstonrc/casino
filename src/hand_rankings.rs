use cards::card::{Card, Rank, Suit};
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub enum HandRank {
    /// Simple value of the card.
    /// Lowest: 2 – Highest: Ace.
    HighCard(u8),
    /// Two cards with the same value.
    Pair([u8; 2]),
    /// Two times two cards with the same value.
    TwoPair([u8; 4]),
    /// Three cards with the same value.
    ThreeOfAKind([u8; 3]),
    /// Sequence of 5 cards in increasing value, not of the same suit.
    /// Ace can precede 2 and follow up King.
    Straight([u8; 5]),
    /// 5 cards of the same suit, not in sequential order.
    Flush([u8; 5]),
    /// Combination of three of a kind and a pair/
    FullHouse([u8; 5]),
    /// Four cards of the same value.
    FourOfAKind([u8; 4]),
    /// Straight of the same suit.
    StraightFlush([u8; 5]),
}

impl HandRank {
    fn rank_name(value: u8) -> String {
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
            HandRank::HighCard(value) => format!("a High Card"),
            HandRank::Pair(cards) => format!("a Pair"),
            HandRank::TwoPair(cards) => format!("Two Pairs"),
            HandRank::ThreeOfAKind(cards) => format!("Three of a Kind"),
            HandRank::Straight(cards) => format!("a Straight"),
            HandRank::Flush(cards) => format!("a Flush"),
            HandRank::FullHouse(cards) => format!("a Full House"),
            HandRank::FourOfAKind(cards) => format!("Four of a Kind"),
            HandRank::StraightFlush(cards) => {
                if *cards == [10, 11, 12, 13, 14] {
                    format!("a Royal Flush")
                } else {
                    format!("a Straight Flush")
                }
            }
        };

        write!(f, "{}", printable)
    }
}

/// Determine the value of the given hand.
pub fn rank_hand(cards: Vec<&Card>) -> HandRank {
    if cards.len() != 2 && cards.len() != 5 && cards.len() != 6 && cards.len() != 7 {
        panic!("Expected the cards count to be equal to 2 (pre-flop), 5 (post-flop), 6 (post-turn), or 7 (post-river) to rank the hand.\nThe cards count provided was: {}.", cards.len())
    }

    let (is_straight_flush, straight_flush_value) = check_for_straight_flush(&cards);

    if is_straight_flush {
        return HandRank::StraightFlush(straight_flush_value.unwrap());
    }

    let (is_four_of_a_kind, four_of_a_kind_value) = check_for_four_of_a_kind(&cards);

    if is_four_of_a_kind {
        return HandRank::FourOfAKind(four_of_a_kind_value.unwrap());
    }

    let (is_full_house, full_house_value) = check_for_full_house(&cards);

    if is_full_house {
        return HandRank::FullHouse(full_house_value.unwrap());
    }

    let (is_flush, flush_value) = check_for_flush(&cards);

    if is_flush {
        return HandRank::Flush(flush_value.unwrap());
    }

    let (is_straight, straight_value) = check_for_straight(&cards);

    if is_straight {
        return HandRank::Straight(straight_value.unwrap());
    }

    let (is_three_of_a_kind, three_of_a_kind_value) = check_for_three_of_a_kind(&cards);

    if is_three_of_a_kind {
        return HandRank::ThreeOfAKind(three_of_a_kind_value.unwrap());
    }

    let (is_two_pair, two_pair_value) = check_for_two_pair(&cards);

    if is_two_pair {
        return HandRank::TwoPair(two_pair_value.unwrap());
    }

    let (is_pair, pair_value) = check_for_pair(&cards);

    if is_pair {
        return HandRank::Pair(pair_value.unwrap());
    }

    let high_card_value = get_high_card_value(&cards);
    HandRank::HighCard(high_card_value)
}

/// Returns: A u8 representing the value of the highest card provided for a HandRank::HighCard.
///
/// Note: Unlike the other ranking methods, this does not return a tuple with a bool since it is executed last after exhausting all other hand ranking options.
///
/// Example: An Ace-high card will return 14.
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
/// Returns: A tuple containing a bool and an Option containing [u8; 2] representing the relevant cards if any.
///
/// Example: a pair of 2s will return (true, 22), and a pair of Kings (13s) will return (true, 1313).
fn check_for_pair(cards: &Vec<&Card>) -> (bool, Option<[u8; 2]>) {
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
        // e.g. 2 -> [2, 2]
        return (true, Some([high_pair_rank_value; 2]));
    }

    (false, None)
}

/// Checks if the provided cards contain a HandRank::TwoPair.
///
/// Returns: A tuple containing a bool and an Option containing [u8; 4] representing the relevant cards if any.
///
/// Example: A pair of 2s and a pair of Kings (13s) will return (true, 221313).
fn check_for_two_pair(cards: &Vec<&Card>) -> (bool, Option<[u8; 4]>) {
    if cards.len() < 4 {
        return (false, None);
    }

    // todo: implement

    (false, None)
}

/// Checks if the provided cards contain a HandRank::ThreeOfAKind.
///
/// Returns: A tuple containing a bool and an Option containing [u8; 3] representing the relevant cards if any.
///
/// Example: Three 2s will return (true, 222).
fn check_for_three_of_a_kind(cards: &Vec<&Card>) -> (bool, Option<[u8; 3]>) {
    if cards.len() < 3 {
        return (false, None);
    }

    // todo: implement

    (false, None)
}

// todo: implement
/// Checks if the provided cards contain a HandRank::Straight.
///
/// This also checks for both Ace low and Ace high when determining if a straight is present.
///
/// Returns: A tuple containing a bool and an Option containing [u8; 5] representing the relevant cards if any.
///
/// Example: A straight of 2, 3, 4, 5, and 6 will return (true, 23456).
///
/// Example: A straight of Ace (low = 1), 2, 3, 4, and 5 will return (true, 12345).
///
/// Example: A straight of 10, J (11), Q (12), K (13), Ace (high = 14) will return (true, 1011121314).
fn check_for_straight(cards: &Vec<&Card>) -> (bool, Option<[u8; 5]>) {
    if cards.len() < 5 {
        return (false, None);
    }

    let sorted_cards = sort_cards_by_rank(cards);

    // Check if the cards have an Ace
    let contains_ace = sorted_cards.iter().any(|&card| card.rank == Rank::Ace);

    if contains_ace {
        // todo: implement Ace high / Ace low logic
    } else {
        // todo: implement
    }

    (false, None)
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

/// Checks if the provided cards contain a HandRank::Flush.
///
/// Returns: A tuple containing a bool and an Option containing [u8; 5] representing the relevant cards if any.
///
/// Example: A flush of 2♥, 5♥, 7♥, 9♥, and J♥ (11♥) will return (true, 257911).
fn check_for_flush(cards: &Vec<&Card>) -> (bool, Option<[u8; 5]>) {
    if cards.len() < 5 {
        return (false, None);
    }

    // todo: implement

    (false, None)
}

/// Checks if the provided cards contain a HandRank::FullHouse.
///
/// Returns: A tuple containing a bool and an Option containing [u8; 5] representing the relevant cards if any.
///
/// Example: Two 2s and three 3s will return (true, 22333).
fn check_for_full_house(cards: &Vec<&Card>) -> (bool, Option<[u8; 5]>) {
    if cards.len() < 5 {
        return (false, None);
    }

    // todo: implement

    (false, None)
}

/// Checks if the provided cards contain a HandRank::FourOfAKind.
///
/// Returns: A tuple containing a bool and an Option containing [u8; 4] representing the relevant cards if any.
///
/// Example: Four 2s will return (true, 2222).
fn check_for_four_of_a_kind(cards: &Vec<&Card>) -> (bool, Option<[u8; 4]>) {
    if cards.len() < 4 {
        return (false, None);
    }

    // todo: implement

    (false, None)
}

/// Checks if the provided cards contain a HandRank::StraightFlush.
///
/// Returns: A tuple containing a bool and an Option containing [u8; 5] representing the relevant cards if any.
///
/// Example: A flush of 2♣, 3♣, 4♣, 5♣, and 6♣ will return (true, 23456).
///
/// Example: An Ace-low flush of A♦ (low = 1♦), 2♦, 3♦, 4♦, and 5♦ will return (true, 12345).
///
/// Example: An Ace-high flush (aka Royal Flush) of 10♠, J♠ (11♠), Q♠ (12♠), K♠ (13♠) and A♠ (high = 14♠) will return (true, 1011121314).
fn check_for_straight_flush(cards: &Vec<&Card>) -> (bool, Option<[u8; 5]>) {
    if cards.len() < 5 {
        return (false, None);
    }

    let (is_straight, straight_value) = check_for_straight(cards);
    let (is_flush, flush_value) = check_for_flush(cards);

    if is_straight && is_flush {
        return (true, straight_value);
    }

    (false, None)
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
        assert!(HandRank::HighCard(14) < HandRank::Pair([2, 2]));
        assert!(HandRank::Pair([13, 13]) < HandRank::TwoPair([2, 2, 3, 3]));
        assert!(HandRank::TwoPair([13, 13, 14, 14]) < HandRank::ThreeOfAKind([2, 2, 2]));
        assert!(HandRank::ThreeOfAKind([14, 14, 14]) < HandRank::Straight([1, 2, 3, 4, 5]));
        assert!(HandRank::Straight([10, 11, 12, 13, 14]) < HandRank::Flush([2, 5, 7, 9, 11]));
        assert!(HandRank::Flush([2, 5, 7, 9, 11]) < HandRank::FullHouse([2, 2, 3, 3, 3]));
        assert!(HandRank::FullHouse([13, 13, 14, 14, 14]) < HandRank::FourOfAKind([2, 2, 2, 2]));
        assert!(HandRank::FourOfAKind([14, 14, 14, 14]) < HandRank::StraightFlush([1, 2, 3, 4, 5]));
        assert!(
            HandRank::StraightFlush([9, 10, 11, 12, 13])
                < HandRank::StraightFlush([10, 11, 12, 13, 14])
        );
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

        assert_eq!(hand_rank, HandRank::Pair([2, 2]));
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

        assert_eq!(hand_rank, HandRank::TwoPair([2, 2, 13, 13]));
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

        assert_eq!(hand_rank, HandRank::ThreeOfAKind([2, 2, 2]));
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

        assert_eq!(hand_rank, HandRank::Straight([2, 3, 4, 5, 6]));
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

        assert_eq!(hand_rank, HandRank::Straight([1, 2, 3, 4, 5]));
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

        assert_eq!(hand_rank, HandRank::Straight([10, 11, 12, 13, 14]));
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

        assert_eq!(hand_rank, HandRank::Flush([2, 5, 7, 9, 11]));
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

        assert_eq!(hand_rank, HandRank::FullHouse([2, 2, 3, 3, 3]));
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

        assert_eq!(hand_rank, HandRank::FourOfAKind([2, 2, 2, 2]));
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

        assert_eq!(hand_rank, HandRank::Straight([2, 3, 4, 5, 6]));
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

        assert_eq!(hand_rank, HandRank::StraightFlush([1, 2, 3, 4, 5]));
    }

    #[test]
    fn ace_high_straight_flush_aka_royal_flush_is_ranked_correctly() {
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

        assert_eq!(hand_rank, HandRank::StraightFlush([10, 11, 12, 13, 14]));
    }
}
