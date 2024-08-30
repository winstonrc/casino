use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

use casino_cards::card::{Card, Rank, Suit};

#[derive(Clone, Copy, Debug, Eq)]
pub enum HandRank {
    /// Simple value of the card.
    /// Lowest: 2 – Highest: Ace.
    HighCard(Card),
    /// Two cards with the same value.
    Pair([Card; 2]),
    /// Two times two cards with the same value.
    TwoPair([Card; 4]),
    /// Three cards with the same value.
    ThreeOfAKind([Card; 3]),
    /// Sequence of 5 cards in increasing value, not of the same suit.
    /// Ace can precede 2 and follow up King.
    Straight([Card; 5]),
    /// 5 cards of the same suit, not in sequential order.
    Flush([Card; 5]),
    /// Combination of three of a kind and a pair/
    FullHouse([Card; 5]),
    /// Four cards of the same value.
    FourOfAKind([Card; 4]),
    /// Straight of the same suit.
    StraightFlush([Card; 5]),
}

impl HandRank {
    pub fn contains(&self, card: &Card) -> bool {
        match self {
            HandRank::HighCard(cards) => *cards == *card,
            HandRank::Pair(cards) => cards.iter().any(|c| *c == *card),
            HandRank::TwoPair(cards) => cards.iter().any(|c| *c == *card),
            HandRank::ThreeOfAKind(cards) => cards.iter().any(|c| *c == *card),
            HandRank::Straight(cards) => cards.iter().any(|c| *c == *card),
            HandRank::Flush(cards) => cards.iter().any(|c| *c == *card),
            HandRank::FullHouse(cards) => cards.iter().any(|c| *c == *card),
            HandRank::FourOfAKind(cards) => cards.iter().any(|c| *c == *card),
            HandRank::StraightFlush(cards) => cards.iter().any(|c| *c == *card),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            HandRank::HighCard(_) => 1,
            HandRank::Pair(_) => 2,
            HandRank::TwoPair(_) => 4,
            HandRank::ThreeOfAKind(_) => 3,
            HandRank::Straight(_) => 5,
            HandRank::Flush(_) => 5,
            HandRank::FullHouse(_) => 5,
            HandRank::FourOfAKind(_) => 4,
            HandRank::StraightFlush(_) => 5,
        }
    }
}

impl Ord for HandRank {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (HandRank::HighCard(cards1), HandRank::HighCard(cards2)) => {
                cards1.rank.cmp(&cards2.rank)
            }
            (HandRank::HighCard(_), _) => Ordering::Less,
            (_, HandRank::HighCard(_)) => Ordering::Greater,

            (HandRank::Pair(cards1), HandRank::Pair(cards2)) => cards1[1].rank.cmp(&cards2[1].rank),
            (HandRank::Pair(_), _) => Ordering::Less,
            (_, HandRank::Pair(_)) => Ordering::Greater,

            (HandRank::TwoPair(cards1), HandRank::TwoPair(cards2)) => {
                let cmp1 = cards1[1].rank.cmp(&cards2[1].rank);
                if cmp1 != Ordering::Equal {
                    return cmp1;
                }

                cards1[3].rank.cmp(&cards2[3].rank)
            }
            (HandRank::TwoPair(_), _) => Ordering::Less,
            (_, HandRank::TwoPair(_)) => Ordering::Greater,

            (HandRank::ThreeOfAKind(cards1), HandRank::ThreeOfAKind(cards2)) => {
                cards1[2].rank.cmp(&cards2[2].rank)
            }
            (HandRank::ThreeOfAKind(_), _) => Ordering::Less,
            (_, HandRank::ThreeOfAKind(_)) => Ordering::Greater,

            (HandRank::Straight(cards1), HandRank::Straight(cards2)) => {
                // Ace-low straight check
                let is_ace_low_straight1 =
                    cards1[0].rank == Rank::Ace && cards1[1].rank == Rank::Two;
                let is_ace_low_straight2 =
                    cards2[0].rank == Rank::Ace && cards2[1].rank == Rank::Two;

                if is_ace_low_straight1 && !is_ace_low_straight2 {
                    Ordering::Less
                } else if !is_ace_low_straight1 && is_ace_low_straight2 {
                    Ordering::Greater
                } else {
                    // Regular straight comparison
                    let max_card_value1 = &cards1[4].rank;
                    let max_card_value2 = &cards2[4].rank;
                    max_card_value1.cmp(max_card_value2)
                }
            }

            (HandRank::Straight(_), HandRank::Flush(_)) => Ordering::Less,

            (HandRank::Flush(cards1), HandRank::Flush(cards2)) => {
                for i in (0..5).rev() {
                    let cmp = cards1[i].rank.cmp(&cards2[i].rank);
                    if cmp != Ordering::Equal {
                        return cmp;
                    }
                }

                Ordering::Equal
            }
            (HandRank::Flush(_), _) => Ordering::Less,
            (_, HandRank::Flush(_)) => Ordering::Greater,

            (HandRank::FullHouse(cards1), HandRank::FullHouse(cards2)) => {
                let cmp1 = cards1[2].rank.cmp(&cards2[2].rank);
                if cmp1 != Ordering::Equal {
                    return cmp1;
                }

                cards1[4].rank.cmp(&cards2[4].rank)
            }
            (HandRank::FullHouse(_), _) => Ordering::Less,
            (_, HandRank::FullHouse(_)) => Ordering::Greater,

            (HandRank::FourOfAKind(cards1), HandRank::FourOfAKind(cards2)) => {
                cards1[3].rank.cmp(&cards2[3].rank)
            }
            (HandRank::FourOfAKind(_), _) => Ordering::Less,
            (_, HandRank::FourOfAKind(_)) => Ordering::Greater,

            (HandRank::StraightFlush(cards1), HandRank::StraightFlush(cards2)) => {
                // Check for royal flush
                let is_royal_flush1 = cards1[0].rank == Rank::Ten
                    && cards1[1].rank == Rank::Jack
                    && cards1[2].rank == Rank::Queen
                    && cards1[3].rank == Rank::King
                    && cards1[4].rank == Rank::Ace;
                let is_royal_flush2 = cards2[0].rank == Rank::Ten
                    && cards2[1].rank == Rank::Jack
                    && cards2[2].rank == Rank::Queen
                    && cards2[3].rank == Rank::King
                    && cards2[4].rank == Rank::Ace;

                if is_royal_flush1 && is_royal_flush2 {
                    Ordering::Equal
                } else {
                    // Compare the ranks of the highest cards
                    let max_card_rank1 = &cards1[4].rank;
                    let max_card_rank2 = &cards2[4].rank;
                    match max_card_rank1.cmp(max_card_rank2) {
                        Ordering::Less => Ordering::Less,
                        Ordering::Greater => Ordering::Greater,
                        Ordering::Equal => {
                            // If the highest card ranks are equal, compare the suits
                            let max_card_suit1 = &cards1[4].suit;
                            let max_card_suit2 = &cards2[4].suit;
                            max_card_suit1.cmp(max_card_suit2)
                        }
                    }
                }
            }
            (_, HandRank::StraightFlush(_)) => Ordering::Less,
            (HandRank::StraightFlush(_), _) => Ordering::Greater,
        }
    }
}

impl PartialOrd for HandRank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HandRank {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HandRank::HighCard(card1), HandRank::HighCard(card2)) => card1.rank == card2.rank,
            (HandRank::Pair(cards1), HandRank::Pair(cards2)) => cards1[1].rank == cards2[1].rank,
            (HandRank::TwoPair(cards1), HandRank::TwoPair(cards2)) => {
                cards1.first().unwrap().rank == cards2.first().unwrap().rank
                    && cards1.last().unwrap().rank == cards2.last().unwrap().rank
            }
            (HandRank::ThreeOfAKind(cards1), HandRank::ThreeOfAKind(cards2)) => {
                cards1.last().unwrap().rank == cards2.last().unwrap().rank
            }

            (HandRank::Straight(cards1), HandRank::Straight(cards2)) => {
                // Compare the ranks of the highest card of the straight with the assumption that the straight is sorted low to high.
                cards1.last().unwrap().rank == cards2.last().unwrap().rank
            }
            (HandRank::Flush(cards1), HandRank::Flush(cards2)) => {
                // Since a flush requires the use of table cards, it can only be made up of a single suit in a round.
                // Accordingly, the suits should always equal each other.
                // Compare each card in the flush from highest to lowest.
                for i in (0..5).rev() {
                    let cmp = cards1[i].rank.cmp(&cards2[i].rank);
                    if cmp != Ordering::Equal {
                        return false;
                    }
                }

                true
            }
            (HandRank::FullHouse(cards1), HandRank::FullHouse(cards2)) => {
                let (three_of_a_kind1_rank, pair1_rank) =
                    (cards1.first().unwrap().rank, cards1.last().unwrap().rank);
                let (three_of_a_kind2_rank, pair2_rank) =
                    (cards2.first().unwrap().rank, cards2.last().unwrap().rank);

                // Compare the ranks of the Three of a Kind cards first.
                // If they are equal, then compare the ranks of the Pair cards.
                if three_of_a_kind1_rank != three_of_a_kind2_rank {
                    return three_of_a_kind1_rank == three_of_a_kind2_rank;
                }

                pair1_rank == pair2_rank
            }
            (HandRank::FourOfAKind(cards1), HandRank::FourOfAKind(cards2)) => {
                cards1.last().unwrap().rank == cards2.last().unwrap().rank
            }
            (HandRank::StraightFlush(cards1), HandRank::StraightFlush(cards2)) => {
                // Compare the ranks of the highest card of the straight
                cards1.last().unwrap().rank == cards2.last().unwrap().rank
            }
            _ => false,
        }
    }
}

impl fmt::Display for HandRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let printable = match self {
            HandRank::HighCard(card) => format!("a High Card: {}", card),

            HandRank::Pair([card1, card2]) => {
                format!("a Pair: {} {}", card1, card2)
            }

            HandRank::TwoPair([card1, card2, card3, card4]) => {
                format!("Two Pairs: {} {} {} {}", card1, card2, card3, card4)
            }

            HandRank::ThreeOfAKind([card1, card2, card3]) => {
                format!("Three of a Kind: {} {} {}", card1, card2, card3,)
            }

            HandRank::Straight([card1, card2, card3, card4, card5]) => {
                format!(
                    "a Straight: {} {} {} {} {}",
                    card1, card2, card3, card4, card5
                )
            }

            HandRank::Flush([card1, card2, card3, card4, card5]) => {
                format!("a Flush: {} {} {} {} {}", card1, card2, card3, card4, card5)
            }

            HandRank::FullHouse([card1, card2, card3, card4, card5]) => format!(
                "a Full House: {} {} {} {} {}",
                card1, card2, card3, card4, card5
            ),
            HandRank::FourOfAKind([card1, card2, card3, card4]) => {
                format!("Four of a Kind: {} {} {} {}", card1, card2, card3, card4)
            }

            HandRank::StraightFlush(cards) => {
                let is_royal_flush = cards.iter().all(|card| {
                    card.rank == Rank::Ten
                        || card.rank == Rank::Jack
                        || card.rank == Rank::Queen
                        || card.rank == Rank::King
                        || card.rank == Rank::Ace
                });

                let [card1, card2, card3, card4, card5] = cards;

                if is_royal_flush {
                    format!(
                        "a Royal Flush: {} {} {} {} {}",
                        card1, card2, card3, card4, card5
                    )
                } else {
                    format!(
                        "a Straight Flush: {} {} {} {} {}",
                        card1, card2, card3, card4, card5
                    )
                }
            }
        };

        write!(f, "{}", printable)
    }
}

/// Determine the highest value of a hand from the given cards.
pub fn rank_hand(cards: Vec<Card>) -> HandRank {
    if cards.len() != 2 && cards.len() != 5 && cards.len() != 6 && cards.len() != 7 {
        panic!("Expected the cards count to be equal to 2 (pre-flop), 5 (post-flop), 6 (post-turn), or 7 (post-river) to rank the hand.\nThe cards count provided was: {}.", cards.len())
    }

    let mut cards = cards.clone();
    cards.sort();

    if let Some(straight_flush_cards) = check_for_straight_flush(&cards) {
        return HandRank::StraightFlush(straight_flush_cards);
    }

    if let Some(four_of_a_kind_cards) = check_for_four_of_a_kind(&cards) {
        return HandRank::FourOfAKind(four_of_a_kind_cards);
    }

    if let Some(full_house_cards) = check_for_full_house(&cards) {
        return HandRank::FullHouse(full_house_cards);
    }

    if let Some(flush_cards) = check_for_flush(&cards) {
        return HandRank::Flush(flush_cards);
    }

    if let Some(straight_cards) = check_for_straight(&cards) {
        return HandRank::Straight(straight_cards);
    }

    if let Some(three_of_a_kind_cards) = check_for_three_of_a_kind(&cards) {
        return HandRank::ThreeOfAKind(three_of_a_kind_cards);
    }

    if let Some(two_pair_cards) = check_for_two_pair(&cards) {
        return HandRank::TwoPair(two_pair_cards);
    }

    if let Some(pair_cards) = check_for_pair(&cards) {
        return HandRank::Pair(pair_cards);
    }

    if let Some(high_card) = get_high_card_value(&cards) {
        HandRank::HighCard(high_card)
    } else {
        panic!(
            "An unexpected error occured while ranking the hand. There should at least be a high card returned at minimum."
        );
    }
}

/// Determines the HandRank::HighCard by finding the card with the highest rank value.
///
/// Returns: An Option containing the relevant card if any.
///
/// Note: Unlike the other ranking methods, this does not return a tuple with a bool
/// since it is executed last after exhausting all other hand ranking options and
/// should always return a card.
///
/// Example: A table with 10 of Clubs, 4 of Hearts, 7 of Diamonds, King of Clubs,
/// and 2 of Spades will return the King of Clubs.
pub fn get_high_card_value(cards: &Vec<Card>) -> Option<Card> {
    let mut high_card: Option<Card> = None;

    for &card in cards {
        if let Some(max_high_card) = high_card {
            if card.rank > max_high_card.rank {
                high_card = Some(card);
            }
        } else {
            high_card = Some(card);
        }
    }

    high_card
}

/// Checks if the provided cards contain a HandRank::Pair.
///
/// Returns: An Option containing the relevant cards if any.
///
/// Example: A pair of Kings.
fn check_for_pair(cards: &Vec<Card>) -> Option<[Card; 2]> {
    if cards.len() < 2 {
        return None;
    }

    let mut ranks: HashMap<Rank, Vec<Card>> = HashMap::new();

    for &card in cards {
        let rank_entry = ranks.entry(card.rank).or_default();
        rank_entry.push(card);
    }

    let mut high_pair_cards: Option<[Card; 2]> = None;

    for (rank, cards) in ranks.iter() {
        if let Some(max_high_pair_cards) = high_pair_cards {
            if cards.len() == 2 && rank > &max_high_pair_cards[0].rank {
                high_pair_cards = Some([cards[0], cards[1]]);
            }
        } else if cards.len() == 2 {
            high_pair_cards = Some([cards[0], cards[1]]);
        }
    }

    if high_pair_cards.is_some() {
        return high_pair_cards;
    }

    None
}

/// Checks if the provided cards contain a HandRank::TwoPair.
///
/// Returns: An Option containing the relevant cards if any.
///
/// Example: A pair of Kings and a pair of 7s.
fn check_for_two_pair(cards: &Vec<Card>) -> Option<[Card; 4]> {
    if cards.len() < 4 {
        return None;
    }

    // Retrieve the highest pair
    let first_pair_cards = check_for_pair(cards);

    // If there is a highest pair then check for a second highest pair.
    // If not, then exit the function.
    if let Some(first_pair_cards) = first_pair_cards {
        let first_pair_card1 = first_pair_cards[0];
        let first_pair_card2 = first_pair_cards[1];

        // Remove the highest pair so that calling check_for_pair again will now return the
        // second highest pair.
        let mut reduced_cards = cards.clone();
        reduced_cards.retain(|&card| card != first_pair_card1 && card != first_pair_card2);

        // Retrieve the second highest pair
        let second_pair_cards = check_for_pair(&reduced_cards);

        // If there is a second highest pair then return the two pairs.
        // If not, then exit the function.
        if let Some(second_pair_cards) = second_pair_cards {
            let second_pair_card1 = second_pair_cards[0];
            let second_pair_card2 = second_pair_cards[1];

            // Return both pairs, highest-to-lowest
            let two_pair = [
                first_pair_card1,
                first_pair_card2,
                second_pair_card1,
                second_pair_card2,
            ];

            return Some(two_pair);
        }
    }

    None
}

/// Checks if the provided cards contain a HandRank::ThreeOfAKind.
///
/// Returns: An Option containing the relevant cards if any.
///
/// Example: Three Kings.
fn check_for_three_of_a_kind(cards: &Vec<Card>) -> Option<[Card; 3]> {
    if cards.len() < 3 {
        return None;
    }

    let mut ranks: HashMap<Rank, Vec<Card>> = HashMap::new();

    for &card in cards {
        let rank_entry = ranks.entry(card.rank).or_default();
        rank_entry.push(card);
    }

    let mut three_of_a_kind_cards: Option<[Card; 3]> = None;

    for (rank, cards) in ranks.iter() {
        if let Some(max_three_of_a_kind_cards) = three_of_a_kind_cards {
            if cards.len() == 3 && rank > &max_three_of_a_kind_cards[0].rank {
                three_of_a_kind_cards = Some([cards[0], cards[1], cards[2]]);
            }
        } else if cards.len() == 3 {
            three_of_a_kind_cards = Some([cards[0], cards[1], cards[2]]);
        }
    }

    if let Some(three_of_a_kind_cards) = three_of_a_kind_cards {
        return Some(three_of_a_kind_cards);
    }

    None
}

/// Checks if the provided cards contain a HandRank::Straight.
///
/// This checks for both Ace-low and Ace-high when an Ace is present.
///
/// Returns: An Option containing the relevant cards if any.
///
/// Example: A straight of 3, 4, 5, 6, 7.
///
/// Example: An Ace-low straight of Ace (1), 2, 3, 4, 5.
///
/// Example: An Ace-high straight of 10, J (11), Q (12), K (13), Ace (14).
fn check_for_straight(cards: &Vec<Card>) -> Option<[Card; 5]> {
    if cards.len() < 5 {
        return None;
    }

    // Check for non-Ace Straight or an Ace-high Straight
    let mut longest_straight: Vec<Card> = Vec::new();
    let mut current_straight: Vec<Card> = vec![cards[0]];

    for i in 1..cards.len() {
        let current_rank = cards[i].rank;
        let previous_rank = current_straight.last().unwrap().rank;
        if current_rank.value() == previous_rank.value() + 1 {
            current_straight.push(cards[i]);
        } else if current_rank == previous_rank {
            // Skip over duplicate values
            continue;
        } else {
            // Start a new sequence if the current card breaks the sequence
            if current_straight.len() > longest_straight.len() {
                longest_straight = current_straight.clone();
            }
            current_straight.clear();
            current_straight.push(cards[i]);
        }
    }

    // Check if the last sequence is the longest
    if current_straight.len() > longest_straight.len() {
        longest_straight = current_straight;
    }

    if longest_straight.len() >= 5 {
        let straight_cards = [
            longest_straight[longest_straight.len() - 5],
            longest_straight[longest_straight.len() - 4],
            longest_straight[longest_straight.len() - 3],
            longest_straight[longest_straight.len() - 2],
            longest_straight[longest_straight.len() - 1],
        ];

        return Some(straight_cards);
    }

    // Check for an Ace-low Straight.
    // This check comes last in the function since it's the lowest possible straight.
    let ace_pos = cards.iter().position(|&card| card.rank == Rank::Ace);
    let two_pos = cards.iter().position(|&card| card.rank == Rank::Two);
    let three_pos = cards.iter().position(|&card| card.rank == Rank::Three);
    let four_pos = cards.iter().position(|&card| card.rank == Rank::Four);
    let five_pos = cards.iter().position(|&card| card.rank == Rank::Five);

    if let (Some(ace_pos), Some(two_pos), Some(three_pos), Some(four_pos), Some(five_pos)) =
        (ace_pos, two_pos, three_pos, four_pos, five_pos)
    {
        let straight_cards = [
            cards[ace_pos],
            cards[two_pos],
            cards[three_pos],
            cards[four_pos],
            cards[five_pos],
        ];

        return Some(straight_cards);
    }

    None
}

/// Checks if the provided cards contain a HandRank::Flush.
///
/// Returns: An Option containing the relevant cards if any.
///
/// Example: A flush of K♣ (13♣), Q♣ (12♣), 9♣, 8♣, 2♣.
fn check_for_flush(cards: &Vec<Card>) -> Option<[Card; 5]> {
    if cards.len() < 5 {
        return None;
    }

    let mut suits: HashMap<Suit, Vec<Card>> = HashMap::new();

    for &card in cards {
        let suit_entry = suits.entry(card.suit).or_default();
        suit_entry.push(card);
    }

    for (_suit, cards) in suits.iter() {
        if cards.len() >= 5 {
            // Check for an Ace-low Flush, which helps when this is also a Straight.
            // This check comes first in the function since the ranks that make up a Flush don't matter,
            // but the ranks for a Straight Flush do matter.
            let ace_pos = cards.iter().position(|&card| card.rank == Rank::Ace);
            let two_pos = cards.iter().position(|&card| card.rank == Rank::Two);
            let three_pos = cards.iter().position(|&card| card.rank == Rank::Three);
            let four_pos = cards.iter().position(|&card| card.rank == Rank::Four);
            let five_pos = cards.iter().position(|&card| card.rank == Rank::Five);

            // Check if all ranks are present
            if let (Some(ace_pos), Some(two_pos), Some(three_pos), Some(four_pos), Some(five_pos)) =
                (ace_pos, two_pos, three_pos, four_pos, five_pos)
            {
                let straight_cards = [
                    cards[ace_pos],
                    cards[two_pos],
                    cards[three_pos],
                    cards[four_pos],
                    cards[five_pos],
                ];

                return Some(straight_cards);
            }

            let flush_cards = [
                cards[cards.len() - 5],
                cards[cards.len() - 4],
                cards[cards.len() - 3],
                cards[cards.len() - 2],
                cards[cards.len() - 1],
            ];

            return Some(flush_cards);
        }
    }

    None
}

/// Checks if the provided cards contain a HandRank::FullHouse.
///
/// Returns: An Option containing the relevant cards if any.
///
/// Example: Three Kings and two 7s.
fn check_for_full_house(cards: &Vec<Card>) -> Option<[Card; 5]> {
    if cards.len() < 5 {
        return None;
    }

    // Retrieve the highest pair
    let three_of_a_kind = check_for_three_of_a_kind(cards);

    // If there is a highest pair then check for a second highest pair.
    // If not, then exit the function.
    if let Some(three_of_a_kind_cards) = three_of_a_kind {
        let three_of_a_kind_card1 = three_of_a_kind_cards[0];
        let three_of_a_kind_card2 = three_of_a_kind_cards[1];
        let three_of_a_kind_card3 = three_of_a_kind_cards[2];

        // Remove the three of a kind so that calling check_for_pair will now return the pair.
        let mut reduced_cards = cards.clone();
        reduced_cards.retain(|&card| {
            card != three_of_a_kind_card1
                && card != three_of_a_kind_card2
                && card != three_of_a_kind_card3
        });

        // Retrieve the second highest pair
        let pair_cards = check_for_pair(&reduced_cards);

        // If there is a second highest pair then return the two pairs.
        // If not, then exit the function.
        if let Some(pair_cards) = pair_cards {
            let pair_card1 = pair_cards[0];
            let pair_card2 = pair_cards[1];

            // Return both pairs, highest-to-lowest
            let full_house = [
                three_of_a_kind_card1,
                three_of_a_kind_card2,
                three_of_a_kind_card3,
                pair_card1,
                pair_card2,
            ];

            return Some(full_house);
        }
    }

    None
}

/// Checks if the provided cards contain a HandRank::FourOfAKind.
///
/// Returns: An Option containing the relevant cards if any.
///
/// Example: Four 6s.
fn check_for_four_of_a_kind(cards: &Vec<Card>) -> Option<[Card; 4]> {
    if cards.len() < 4 {
        return None;
    }

    let mut ranks: HashMap<Rank, Vec<Card>> = HashMap::new();

    for &card in cards {
        let rank_entry = ranks.entry(card.rank).or_default();
        rank_entry.push(card);
    }

    for (_rank, cards) in ranks.iter() {
        if cards.len() == 4 {
            let four_of_a_kind_cards = [cards[0], cards[1], cards[2], cards[3]];

            return Some(four_of_a_kind_cards);
        }
    }

    None
}

/// Checks if the provided cards contain a HandRank::StraightFlush.
///
/// Returns: An Option containing the relevant cards if any.
///
/// Example: A flush of 2♠, 3♠, 4♠, 5♠, 6♠.
///
/// Example: An Ace-low flush of A♦ (1♦), 2♦, 3♦, 4♦, 5♦.
///
/// Example: An Ace-high flush (aka Royal Flush) of 10♥, J♥ (11♥), Q♥ (12♥), K♥ (13♥) A♥ (14♥).
fn check_for_straight_flush(cards: &Vec<Card>) -> Option<[Card; 5]> {
    if cards.len() < 5 {
        return None;
    }

    let straight_cards = check_for_straight(cards);
    let flush_cards = check_for_flush(cards);

    // Check if both a straight and a flush are present
    if let (Some(straight_cards), Some(flush_cards)) = (straight_cards, flush_cards) {
        // Check if the same set of cards make up both the straight and the flush
        if straight_cards == flush_cards {
            return Some(straight_cards);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use strum::IntoEnumIterator;

    use casino_cards::card;
    use casino_cards::card::{Card, Rank, Suit};

    /// Tests that High Cards of the same Rank are equal, regardless of Suit.
    #[test]
    fn high_card_ranks_are_compared_correctly() {
        for rank in Rank::iter() {
            let mut previous_suit: Option<Suit> = None;
            for suit in Suit::iter() {
                if let Some(previous_suit) = previous_suit {
                    // Compare the current suit with the previous suit with the same rank
                    assert_eq!(
                        HandRank::HighCard(Card::new(rank, previous_suit)),
                        HandRank::HighCard(Card::new(rank, suit))
                    );
                }

                previous_suit = Some(suit);
            }
        }
    }

    /// Tests that High Cards of higher Ranks are greater than High Cards of lower Ranks, regardless of Suit.
    #[test]
    fn high_card_ranks_are_ordered_correctly() {
        for suit in Suit::iter() {
            let mut previous_rank: Option<Rank> = None;
            for rank in Rank::iter() {
                if let Some(previous_rank) = previous_rank {
                    // Compare the current rank with the previous rank in the same suit
                    assert!(
                        HandRank::HighCard(Card::new(previous_rank, suit))
                            < HandRank::HighCard(Card::new(rank, suit))
                    );
                }

                previous_rank = Some(rank);
            }
        }
    }

    /// Tests that Pairs of the same Rank are equal, regardless of Suit.
    #[test]
    fn pair_ranks_are_compared_correctly() {
        for rank in Rank::iter() {
            let mut previous_suit: Option<Suit> = None;
            for suit in Suit::iter() {
                if let Some(previous_suit) = previous_suit {
                    // Compare the current suit with the previous suit with the same rank
                    assert_eq!(
                        HandRank::Pair([
                            Card::new(rank, previous_suit),
                            Card::new(rank, previous_suit)
                        ]),
                        HandRank::Pair([Card::new(rank, suit), Card::new(rank, suit)])
                    );
                }

                previous_suit = Some(suit);
            }
        }
    }

    /// Tests that Pairs of higher Ranks are greater than Pairs of lower Ranks, regardless of Suit.
    #[test]
    fn pair_ranks_are_ordered_correctly() {
        for suit in Suit::iter() {
            let mut previous_rank: Option<Rank> = None;
            for rank in Rank::iter() {
                if let Some(previous_rank) = previous_rank {
                    // Compare the current rank with the previous rank in the same suit
                    assert!(
                        HandRank::Pair([
                            Card::new(previous_rank, suit),
                            Card::new(previous_rank, suit)
                        ]) < HandRank::Pair([Card::new(rank, suit), Card::new(rank, suit)])
                    );
                }

                previous_rank = Some(rank);
            }
        }
    }

    #[test]
    fn hand_rankings_are_ordered_correctly() {
        let high_card = HandRank::HighCard(card!(King, Club));

        let pair = HandRank::Pair([card!(King, Club), card!(King, Heart)]);

        let two_pair = HandRank::TwoPair([
            card!(King, Club),
            card!(King, Heart),
            card!(Seven, Diamond),
            card!(Seven, Club),
        ]);

        let three_of_a_kind =
            HandRank::ThreeOfAKind([card!(King, Club), card!(King, Heart), card!(King, Diamond)]);

        let straight = HandRank::Straight([
            card!(Two, Spade),
            card!(Three, Club),
            card!(Four, Heart),
            card!(Five, Diamond),
            card!(Six, Club),
        ]);

        let flush = HandRank::Flush([
            card!(King, Club),
            card!(Queen, Club),
            card!(Nine, Club),
            card!(Eight, Club),
            card!(Two, Club),
        ]);

        let full_house = HandRank::FullHouse([
            card!(King, Club),
            card!(King, Heart),
            card!(King, Diamond),
            card!(Seven, Club),
            card!(Seven, Spade),
        ]);
        let four_of_a_kind = HandRank::FourOfAKind([
            card!(Six, Spade),
            card!(Six, Diamond),
            card!(Six, Heart),
            card!(Six, Club),
        ]);

        let straight_flush = HandRank::StraightFlush([
            card!(Two, Spade),
            card!(Three, Spade),
            card!(Four, Spade),
            card!(Five, Spade),
            card!(Six, Spade),
        ]);

        let royal_flush = HandRank::StraightFlush([
            card!(Ten, Heart),
            card!(Jack, Heart),
            card!(Queen, Heart),
            card!(King, Heart),
            card!(Ace, Heart),
        ]);

        assert!(high_card < pair);
        assert!(pair < two_pair);
        assert!(two_pair < three_of_a_kind);
        assert!(three_of_a_kind < straight);
        assert!(straight < flush);
        assert!(flush < full_house);
        assert!(full_house < four_of_a_kind);
        assert!(four_of_a_kind < straight_flush);
        assert!(straight_flush < royal_flush);
    }

    #[test]
    fn straight_ace_low_straight_is_valued_lower_than_higher_straights() {
        let ace_low_straight = HandRank::Straight([
            card!(Ace, Club),
            card!(Two, Spade),
            card!(Three, Club),
            card!(Four, Heart),
            card!(Five, Diamond),
        ]);

        let two_six_straight = HandRank::Straight([
            card!(Two, Spade),
            card!(Three, Club),
            card!(Four, Heart),
            card!(Five, Diamond),
            card!(Six, Club),
        ]);

        let ace_high_straight = HandRank::Straight([
            card!(Ten, Club),
            card!(Jack, Heart),
            card!(Queen, Diamond),
            card!(King, Spade),
            card!(Ace, Club),
        ]);

        assert!(ace_low_straight < two_six_straight);
        assert!(two_six_straight < ace_high_straight);
    }

    /// Tests get_high_card_value().
    ///
    /// Tests if a High Card is correctly identified.
    #[test]
    fn get_high_card_value_works() {
        let two_of_spades = card!(Two, Spade);
        let four_of_hearts = card!(Four, Heart);
        let seven_of_diamonds = card!(Seven, Diamond);
        let ten_of_clubs = card!(Ten, Club);
        let king_of_clubs = card!(King, Club);

        let high_card = king_of_clubs;

        let mut cards: Vec<Card> = vec![
            ten_of_clubs,
            four_of_hearts,
            seven_of_diamonds,
            king_of_clubs,
            two_of_spades,
        ];
        cards.sort();

        if let Some(result) = get_high_card_value(&cards) {
            assert_eq!(result, high_card);
        } else {
            panic!("Expected a High Card, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing a High Card is ranked correctly.
    #[test]
    fn rank_hand_high_card_works() {
        let two_of_spades = card!(Two, Spade);
        let four_of_hearts = card!(Four, Heart);
        let seven_of_diamonds = card!(Seven, Diamond);
        let ten_of_clubs = card!(Ten, Club);
        let king_of_clubs = card!(King, Club);

        let high_card = HandRank::HighCard(king_of_clubs);

        let cards: Vec<Card> = vec![
            ten_of_clubs,
            four_of_hearts,
            seven_of_diamonds,
            king_of_clubs,
            two_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, high_card);
    }

    /// Tests check_for_pair().
    ///
    /// Tests if a Pair is correctly identified.
    #[test]
    fn check_for_pair_works() {
        let two_of_clubs = card!(Two, Club);
        let five_of_spades = card!(Five, Spade);
        let seven_of_diamonds = card!(Seven, Diamond);
        let king_of_clubs = card!(King, Club);
        let king_of_hearts = card!(King, Heart);
        let ace_of_spades = card!(Ace, Spade);

        let pair = [king_of_clubs, king_of_hearts];

        // Base case
        let mut cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            seven_of_diamonds,
            two_of_clubs,
            five_of_spades,
        ];
        cards.sort();

        if let Some(result) = check_for_pair(&cards) {
            assert_eq!(result, pair);
        } else {
            panic!("Expected a Pair, but none was found.");
        };

        // Tests that the Pair is identified over the High Card.
        let mut cards2: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            seven_of_diamonds,
            two_of_clubs,
            five_of_spades,
            ace_of_spades,
        ];
        cards2.sort();

        if let Some(result) = check_for_pair(&cards2) {
            assert_eq!(result, pair);
        } else {
            panic!("Expected a Pair, but none was found.");
        };
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing a Pair is ranked correctly.
    #[test]
    fn rank_hand_pair_works() {
        let two_of_clubs = card!(Two, Club);
        let five_of_spades = card!(Five, Spade);
        let seven_of_diamonds = card!(Seven, Diamond);
        let king_of_clubs = card!(King, Club);
        let king_of_hearts = card!(King, Heart);
        let ace_of_spades = card!(Ace, Spade);

        let pair = HandRank::Pair([king_of_clubs, king_of_hearts]);

        // Base case
        let cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            seven_of_diamonds,
            two_of_clubs,
            five_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, pair);

        // Tests that the Pair is identified over the High Card.
        let cards2: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            seven_of_diamonds,
            two_of_clubs,
            five_of_spades,
            ace_of_spades,
        ];
        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, pair);
    }

    /// Tests check_for_two_pair().
    ///
    /// Tests if a Pair is correctly identified.
    #[test]
    fn check_for_two_pair_works() {
        let five_of_clubs = card!(Five, Club);
        let five_of_spades = card!(Five, Spade);
        let seven_of_clubs = card!(Seven, Club);
        let seven_of_diamonds = card!(Seven, Diamond);
        let king_of_clubs = card!(King, Club);
        let king_of_hearts = card!(King, Heart);

        let two_pair = [
            king_of_clubs,
            king_of_hearts,
            seven_of_clubs,
            seven_of_diamonds,
        ];

        let mut cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            seven_of_diamonds,
            seven_of_clubs,
            five_of_spades,
        ];
        cards.sort();

        if let Some(result) = check_for_two_pair(&cards) {
            assert_eq!(result, two_pair);
        } else {
            panic!("Expected a Two Pair, but none was found.");
        };

        // Tests that the higher Two Pair of Ks & 7s is identified over the lower Two Pair of 5s.
        let mut cards2: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            seven_of_diamonds,
            seven_of_clubs,
            five_of_spades,
            five_of_clubs,
        ];
        cards2.sort();

        if let Some(result) = check_for_two_pair(&cards2) {
            assert_eq!(result, two_pair);
        } else {
            panic!("Expected a Two Pair, but none was found.");
        };
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing a Pair is ranked correctly.
    #[test]
    fn rank_hand_two_pair_works() {
        let five_of_clubs = card!(Five, Club);
        let five_of_spades = card!(Five, Spade);
        let seven_of_clubs = card!(Seven, Club);
        let seven_of_diamonds = card!(Seven, Diamond);
        let king_of_clubs = card!(King, Club);
        let king_of_hearts = card!(King, Heart);

        let two_pair = HandRank::TwoPair([
            king_of_clubs,
            king_of_hearts,
            seven_of_clubs,
            seven_of_diamonds,
        ]);

        // Base case
        let cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            seven_of_diamonds,
            seven_of_clubs,
            five_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, two_pair);

        // Tests that the higher Two Pair of Ks & 7s is identified over the lower Two Pair of 5s.
        let cards2: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            seven_of_diamonds,
            seven_of_clubs,
            five_of_spades,
            five_of_clubs,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, two_pair);
    }

    /// Tests check_for_three_of_a_kind().
    ///
    /// Tests if a Pair is correctly identified.
    #[test]
    fn check_for_three_of_a_kind_works() {
        let five_of_spades = card!(Five, Spade);
        let seven_of_clubs = card!(Seven, Club);
        let seven_of_diamonds = card!(Seven, Diamond);
        let seven_of_spades = card!(Seven, Spade);
        let king_of_clubs = card!(King, Club);
        let king_of_diamonds = card!(King, Diamond);
        let king_of_hearts = card!(King, Heart);

        let three_of_a_kind = [king_of_clubs, king_of_diamonds, king_of_hearts];

        // Base case
        let mut cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            king_of_diamonds,
            seven_of_clubs,
            five_of_spades,
        ];
        cards.sort();

        if let Some(result) = check_for_three_of_a_kind(&cards) {
            assert_eq!(result, three_of_a_kind);
        } else {
            panic!("Expected a Three of a Kind, but none was found.");
        };

        // Tests that the higher Three of a Kind of Ks is identified over the lower Three of a Kind of 7s.
        let mut cards2: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            king_of_diamonds,
            seven_of_clubs,
            five_of_spades,
            seven_of_diamonds,
            seven_of_spades,
        ];
        cards2.sort();

        if let Some(result) = check_for_three_of_a_kind(&cards2) {
            assert_eq!(result, three_of_a_kind);
        } else {
            panic!("Expected a Three of a Kind, but none was found.");
        };
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing a Pair is ranked correctly.
    #[test]
    fn rank_hand_three_of_a_kind_works() {
        let five_of_spades = card!(Five, Spade);
        let seven_of_clubs = card!(Seven, Club);
        let seven_of_diamonds = card!(Seven, Diamond);
        let seven_of_spades = card!(Seven, Spade);
        let king_of_clubs = card!(King, Club);
        let king_of_diamonds = card!(King, Diamond);
        let king_of_hearts = card!(King, Heart);

        let three_of_a_kind =
            HandRank::ThreeOfAKind([king_of_clubs, king_of_diamonds, king_of_hearts]);

        // Base case
        let cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            king_of_diamonds,
            seven_of_clubs,
            five_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, three_of_a_kind);

        // Tests that the higher Three of a Kind of Ks is identified over the lower Three of a Kind of 7s.
        let cards2: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            king_of_diamonds,
            seven_of_clubs,
            five_of_spades,
            seven_of_diamonds,
            seven_of_spades,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, three_of_a_kind);
    }

    /// Tests check_for_straight().
    ///
    /// Tests if a Straight is correctly identified.
    #[test]
    fn check_for_straight_works() {
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_clubs = card!(Three, Club);
        let four_of_hearts = card!(Four, Heart);
        let five_of_diamonds = card!(Five, Diamond);
        let six_of_clubs = card!(Six, Club);
        let seven_of_spades = card!(Seven, Spade);
        let five_of_clubs = card!(Five, Club);
        let nine_of_spades = card!(Nine, Spade);
        let ten_of_diamonds = card!(Ten, Diamond);
        let jack_of_clubs = card!(Jack, Club);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_diamonds = card!(King, Diamond);

        let straight = [
            three_of_clubs,
            four_of_hearts,
            five_of_diamonds,
            six_of_clubs,
            seven_of_spades,
        ];

        // Base case
        let mut cards: Vec<Card> = vec![
            three_of_clubs,
            four_of_hearts,
            five_of_diamonds,
            six_of_clubs,
            seven_of_spades,
        ];
        cards.sort();

        if let Some(result) = check_for_straight(&cards) {
            assert_eq!(result, straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        // Tests that the higher Straight of 3, 4, 5, 6, 7 is identified over the lower Straight of 2, 3, 4, 5, 6.
        let mut cards2: Vec<Card> = vec![
            two_of_diamonds,
            three_of_clubs,
            four_of_hearts,
            five_of_diamonds,
            six_of_clubs,
            seven_of_spades,
        ];
        cards2.sort();

        if let Some(result) = check_for_straight(&cards2) {
            assert_eq!(result, straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        // Tests that a Straight is still identified with 2 repeating ranks in the middle.
        let straight2 = [
            nine_of_spades,
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
        ];

        let mut cards3: Vec<Card> = vec![
            king_of_diamonds,
            jack_of_spades,
            ten_of_diamonds,
            five_of_clubs,
            jack_of_clubs,
            nine_of_spades,
            queen_of_spades,
        ];
        cards3.sort();

        if let Some(result) = check_for_straight(&cards3) {
            assert_eq!(result, straight2);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        // Tests that a Straight is still identified with 3 repeating ranks in the middle.
        let mut cards4: Vec<Card> = vec![
            king_of_diamonds,
            jack_of_spades,
            ten_of_diamonds,
            jack_of_hearts,
            jack_of_clubs,
            nine_of_spades,
            queen_of_spades,
        ];
        cards4.sort();

        if let Some(result) = check_for_straight(&cards4) {
            assert_eq!(result, straight2);
        } else {
            panic!("Expected a Straight, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing a Straight is ranked correctly.
    #[test]
    fn rank_hand_straight_works() {
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_clubs = card!(Three, Club);
        let four_of_hearts = card!(Four, Heart);
        let five_of_diamonds = card!(Five, Diamond);
        let six_of_clubs = card!(Six, Club);
        let seven_of_spades = card!(Seven, Spade);
        let five_of_clubs = card!(Five, Club);
        let nine_of_spades = card!(Nine, Spade);
        let ten_of_diamonds = card!(Ten, Diamond);
        let jack_of_clubs = card!(Jack, Club);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_diamonds = card!(King, Diamond);

        let straight = HandRank::Straight([
            three_of_clubs,
            four_of_hearts,
            five_of_diamonds,
            six_of_clubs,
            seven_of_spades,
        ]);

        // Base case
        let cards: Vec<Card> = vec![
            three_of_clubs,
            four_of_hearts,
            five_of_diamonds,
            six_of_clubs,
            seven_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, straight);

        // Tests that the higher Straight of 3, 4, 5, 6, 7 is identified over the lower Straight of 2, 3, 4, 5, 6.
        let cards2: Vec<Card> = vec![
            two_of_diamonds,
            three_of_clubs,
            four_of_hearts,
            five_of_diamonds,
            six_of_clubs,
            seven_of_spades,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, straight);

        // Tests that a Straight is still identified with 2 repeating ranks in the middle.
        let straight2 = HandRank::Straight([
            nine_of_spades,
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
        ]);

        let cards3: Vec<Card> = vec![
            king_of_diamonds,
            jack_of_spades,
            ten_of_diamonds,
            five_of_clubs,
            jack_of_clubs,
            nine_of_spades,
            queen_of_spades,
        ];

        let hand_rank3 = rank_hand(cards3);
        assert_eq!(hand_rank3, straight2);

        // Tests that a Straight is still identified with 3 repeating ranks in the middle.
        let cards4: Vec<Card> = vec![
            king_of_diamonds,
            jack_of_spades,
            ten_of_diamonds,
            jack_of_hearts,
            jack_of_clubs,
            nine_of_spades,
            queen_of_spades,
        ];

        let hand_rank4 = rank_hand(cards4);
        assert_eq!(hand_rank4, straight2);
    }

    /// Tests check_for_straight().
    ///
    /// Tests if a winning Straight on the table is correctly identified for all parties.
    #[test]
    fn check_for_straight_on_the_table_works() {
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_clubs = card!(Three, Club);
        let four_of_hearts = card!(Four, Heart);
        let five_of_diamonds = card!(Five, Diamond);
        let six_of_clubs = card!(Six, Club);
        let seven_of_spades = card!(Seven, Spade);
        let five_of_clubs = card!(Five, Club);
        let nine_of_spades = card!(Nine, Spade);
        let ten_of_diamonds = card!(Ten, Diamond);
        let jack_of_clubs = card!(Jack, Club);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_diamonds = card!(King, Diamond);
        let ace_of_hearts = card!(Ace, Heart);

        let straight = [
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ];

        let mut table_cards: Vec<Card> = vec![
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ];
        table_cards.sort();

        if let Some(result) = check_for_straight(&table_cards) {
            assert_eq!(result, straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        let mut player1_cards: Vec<Card> = vec![three_of_clubs, four_of_hearts];
        player1_cards.extend(table_cards.clone());
        player1_cards.sort();

        if let Some(result) = check_for_straight(&player1_cards) {
            assert_eq!(result, straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        let mut player2_cards: Vec<Card> = vec![five_of_diamonds, six_of_clubs];
        player2_cards.extend(table_cards.clone());
        player2_cards.sort();

        if let Some(result) = check_for_straight(&player2_cards) {
            assert_eq!(result, straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        let mut player3_cards: Vec<Card> = vec![seven_of_spades, nine_of_spades];
        player3_cards.extend(table_cards.clone());
        player3_cards.sort();

        if let Some(result) = check_for_straight(&player3_cards) {
            assert_eq!(result, straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        let mut player4_cards: Vec<Card> = vec![two_of_diamonds, five_of_clubs];
        player4_cards.extend(table_cards.clone());
        player4_cards.sort();

        if let Some(result) = check_for_straight(&player4_cards) {
            assert_eq!(result, straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        let mut player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        player5_cards.extend(table_cards.clone());
        player5_cards.sort();

        if let Some(result) = check_for_straight(&player5_cards) {
            assert_eq!(result, straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a winning Straight on the table is ranked correctly for all parties.
    #[test]
    fn rank_hand_straight_on_the_table_works() {
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_clubs = card!(Three, Club);
        let four_of_hearts = card!(Four, Heart);
        let five_of_diamonds = card!(Five, Diamond);
        let six_of_clubs = card!(Six, Club);
        let seven_of_spades = card!(Seven, Spade);
        let five_of_clubs = card!(Five, Club);
        let nine_of_spades = card!(Nine, Spade);
        let ten_of_diamonds = card!(Ten, Diamond);
        let jack_of_clubs = card!(Jack, Club);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_diamonds = card!(King, Diamond);
        let ace_of_hearts = card!(Ace, Heart);

        let straight = HandRank::Straight([
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ]);

        let table_cards: Vec<Card> = vec![
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ];

        let table_cards_hand_rank = rank_hand(table_cards.clone());
        assert_eq!(table_cards_hand_rank, straight);

        let mut player1_cards: Vec<Card> = vec![three_of_clubs, four_of_hearts];
        player1_cards.extend(table_cards.clone());
        let player1_hand_rank = rank_hand(player1_cards);
        assert_eq!(player1_hand_rank, straight);

        let mut player2_cards: Vec<Card> = vec![five_of_diamonds, six_of_clubs];
        player2_cards.extend(table_cards.clone());
        let player2_hand_rank = rank_hand(player2_cards);
        assert_eq!(player2_hand_rank, straight);

        let mut player3_cards: Vec<Card> = vec![seven_of_spades, nine_of_spades];
        player3_cards.extend(table_cards.clone());
        let player3_hand_rank = rank_hand(player3_cards);
        assert_eq!(player3_hand_rank, straight);

        let mut player4_cards: Vec<Card> = vec![two_of_diamonds, five_of_clubs];
        player4_cards.extend(table_cards.clone());
        let player4_hand_rank = rank_hand(player4_cards);
        assert_eq!(player4_hand_rank, straight);

        let mut player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        player5_cards.extend(table_cards.clone());
        let player5_hand_rank = rank_hand(player5_cards);
        assert_eq!(player5_hand_rank, straight);

        assert_eq!(player1_hand_rank, player2_hand_rank);
        assert_eq!(player2_hand_rank, player3_hand_rank);
        assert_eq!(player3_hand_rank, player4_hand_rank);
        assert_eq!(player4_hand_rank, player5_hand_rank);
    }

    /// Tests check_for_straight().
    ///
    /// Tests if a winning Straight on the table is correctly identified for all parties.
    #[test]
    fn check_for_straight_equal_straights_in_hand_works() {
        let three_of_clubs = card!(Three, Club);
        let four_of_hearts = card!(Four, Heart);
        let five_of_diamonds = card!(Five, Diamond);
        let ten_of_diamonds = card!(Ten, Diamond);
        let ten_of_hearts = card!(Ten, Heart);
        let jack_of_clubs = card!(Jack, Club);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_diamonds = card!(King, Diamond);
        let ace_of_hearts = card!(Ace, Heart);

        let straight1 = [
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ];

        let straight2 = [
            ten_of_hearts,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ];

        let mut table_cards: Vec<Card> = vec![
            four_of_hearts,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ];
        table_cards.sort();

        let mut player1_cards: Vec<Card> = vec![three_of_clubs, ten_of_diamonds];
        player1_cards.extend(table_cards.clone());
        player1_cards.sort();

        if let Some(result) = check_for_straight(&player1_cards) {
            assert_eq!(result, straight1);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        let mut player2_cards: Vec<Card> = vec![five_of_diamonds, ten_of_hearts];
        player2_cards.extend(table_cards.clone());
        player2_cards.sort();

        if let Some(result) = check_for_straight(&player2_cards) {
            assert_eq!(result, straight2);
        } else {
            panic!("Expected a Straight, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if players with equal Straights in their hands push.
    #[test]
    fn rank_hand_straight_equal_straights_in_hand_works() {
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_clubs = card!(Three, Club);
        let four_of_hearts = card!(Four, Heart);
        let five_of_diamonds = card!(Five, Diamond);
        let seven_of_spades = card!(Seven, Spade);
        let five_of_clubs = card!(Five, Club);
        let nine_of_spades = card!(Nine, Spade);
        let ten_of_diamonds = card!(Ten, Diamond);
        let ten_of_hearts = card!(Ten, Heart);
        let jack_of_clubs = card!(Jack, Club);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_diamonds = card!(King, Diamond);
        let ace_of_hearts = card!(Ace, Heart);

        let straight1 = HandRank::Straight([
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ]);

        let straight2 = HandRank::Straight([
            ten_of_hearts,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ]);

        let table_cards: Vec<Card> = vec![
            four_of_hearts,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ];

        let mut player1_cards: Vec<Card> = vec![three_of_clubs, ten_of_diamonds];
        player1_cards.extend(table_cards.clone());
        let player1_hand_rank = rank_hand(player1_cards);
        assert_eq!(player1_hand_rank, straight1);

        let mut player2_cards: Vec<Card> = vec![five_of_diamonds, ten_of_hearts];
        player2_cards.extend(table_cards.clone());
        let player2_hand_rank = rank_hand(player2_cards);
        assert_eq!(player2_hand_rank, straight2);

        let mut player3_cards: Vec<Card> = vec![seven_of_spades, nine_of_spades];
        player3_cards.extend(table_cards.clone());
        let player3_hand_rank = rank_hand(player3_cards);

        let mut player4_cards: Vec<Card> = vec![two_of_diamonds, five_of_clubs];
        player4_cards.extend(table_cards.clone());
        let player4_hand_rank = rank_hand(player4_cards);

        let mut player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        player5_cards.extend(table_cards.clone());
        let player5_hand_rank = rank_hand(player5_cards);

        assert_eq!(player1_hand_rank, player2_hand_rank);
        assert_ne!(player1_hand_rank, player3_hand_rank);
        assert_ne!(player1_hand_rank, player4_hand_rank);
        assert_ne!(player1_hand_rank, player5_hand_rank);
    }

    /// Tests check_for_straight().
    ///
    /// Tests if an Ace-low Straight is correctly identified.
    #[test]
    fn check_for_straight_ace_low_works() {
        let two_of_clubs = card!(Two, Club);
        let three_of_hearts = card!(Three, Heart);
        let four_of_spades = card!(Four, Spade);
        let five_of_hearts = card!(Five, Heart);
        let six_of_diamonds = card!(Six, Diamond);
        let seven_of_diamonds = card!(Seven, Diamond);
        let eight_of_clubs = card!(Eight, Club);
        let ace_of_spades = card!(Ace, Spade);

        let ace_low_straight = [
            ace_of_spades,
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
        ];

        // Base case
        let mut cards: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            ace_of_spades,
        ];
        cards.sort();

        if let Some(result) = check_for_straight(&cards) {
            assert_eq!(result, ace_low_straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        // Tests that the 7♦ is ignored, and the Ace-low Straight is identified.
        let mut cards2: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            seven_of_diamonds,
            ace_of_spades,
        ];
        cards2.sort();

        if let Some(result) = check_for_straight(&cards2) {
            assert_eq!(result, ace_low_straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        // Tests that the 7♦ & 8♣ are ignored, and the Ace-low Straight is identified.
        let mut cards3: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            seven_of_diamonds,
            eight_of_clubs,
            ace_of_spades,
        ];
        cards3.sort();

        if let Some(result) = check_for_straight(&cards3) {
            assert_eq!(result, ace_low_straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        // Tests that an Ace-low Straight is ignored, and a higher Straight is identified.
        let non_ace_low_straight = [
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            six_of_diamonds,
        ];

        let mut cards4: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            six_of_diamonds,
            ace_of_spades,
        ];
        cards4.sort();

        if let Some(result) = check_for_straight(&cards4) {
            assert_eq!(result, non_ace_low_straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing an Ace-low Straight is ranked correctly.
    #[test]
    fn rank_hand_straight_ace_low_works() {
        let two_of_clubs = card!(Two, Club);
        let three_of_hearts = card!(Three, Heart);
        let four_of_spades = card!(Four, Spade);
        let five_of_hearts = card!(Five, Heart);
        let six_of_diamonds = card!(Six, Diamond);
        let seven_of_diamonds = card!(Seven, Diamond);
        let eight_of_clubs = card!(Eight, Club);
        let ace_of_spades = card!(Ace, Spade);

        let ace_low_straight = HandRank::Straight([
            ace_of_spades,
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
        ]);

        // Base case
        let cards: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            ace_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, ace_low_straight);

        // Tests that the 7♦ is ignored, and the Ace-low Straight is identified.
        let cards2: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            seven_of_diamonds,
            ace_of_spades,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, ace_low_straight);

        // Tests that the 7♦ & 8♣ are ignored, and the Ace-low Straight is identified.
        let cards3: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            seven_of_diamonds,
            eight_of_clubs,
            ace_of_spades,
        ];

        let hand_rank3 = rank_hand(cards3);
        assert_eq!(hand_rank3, ace_low_straight);

        // Tests that an Ace-low Straight is ignored, and a higher Straight is identified.
        let non_ace_low_straight = HandRank::Straight([
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            six_of_diamonds,
        ]);

        let cards4: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            six_of_diamonds,
            ace_of_spades,
        ];

        let hand_rank4 = rank_hand(cards4);
        assert_eq!(hand_rank4, non_ace_low_straight);
    }

    /// Tests check_for_straight().
    ///
    /// Tests if an Ace-high Straight is correctly identified.
    #[test]
    fn check_for_straight_ace_high_works() {
        let nine_of_diamonds = card!(Nine, Diamond);
        let ten_of_clubs = card!(Ten, Club);
        let jack_of_hearts = card!(Jack, Heart);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_hearts = card!(King, Heart);
        let ace_of_spades = card!(Ace, Spade);

        let ace_high_straight = [
            ten_of_clubs,
            jack_of_hearts,
            queen_of_spades,
            king_of_hearts,
            ace_of_spades,
        ];

        // Base case
        let mut cards: Vec<Card> = vec![
            ten_of_clubs,
            jack_of_hearts,
            queen_of_spades,
            king_of_hearts,
            ace_of_spades,
        ];
        cards.sort();

        if let Some(result) = check_for_straight(&cards) {
            assert_eq!(result, ace_high_straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        // Tests that the higher Straight of 10, J, Q, K, Ace is identified over the lower Straight of 9, 10, J, Q, K.
        let mut cards2: Vec<Card> = vec![
            nine_of_diamonds,
            ten_of_clubs,
            jack_of_hearts,
            queen_of_spades,
            king_of_hearts,
            ace_of_spades,
        ];
        cards2.sort();

        if let Some(result) = check_for_straight(&cards2) {
            assert_eq!(result, ace_high_straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing an Ace-high Straight is ranked correctly.
    #[test]
    fn rank_hand_straight_ace_high_works() {
        let nine_of_diamonds = card!(Nine, Diamond);
        let ten_of_clubs = card!(Ten, Club);
        let jack_of_hearts = card!(Jack, Heart);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_hearts = card!(King, Heart);
        let ace_of_spades = card!(Ace, Spade);

        let ace_high_straight = HandRank::Straight([
            ten_of_clubs,
            jack_of_hearts,
            queen_of_spades,
            king_of_hearts,
            ace_of_spades,
        ]);

        // Base case
        let cards: Vec<Card> = vec![
            ten_of_clubs,
            jack_of_hearts,
            queen_of_spades,
            king_of_hearts,
            ace_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, ace_high_straight);

        // Tests that the higher Straight of 10, J, Q, K, Ace is identified over the lower Straight of 9, 10, J, Q, K.
        let cards2: Vec<Card> = vec![
            nine_of_diamonds,
            ten_of_clubs,
            jack_of_hearts,
            queen_of_spades,
            king_of_hearts,
            ace_of_spades,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, ace_high_straight);
    }

    /// Tests check_for_flush().
    ///
    /// Tests if a Flush is correctly identified.
    #[test]
    fn check_for_flush_works() {
        let two_of_clubs = card!(Two, Club);
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_clubs = card!(Three, Club);
        let eight_of_clubs = card!(Eight, Club);
        let nine_of_clubs = card!(Nine, Club);
        let queen_of_clubs = card!(Queen, Club);
        let king_of_clubs = card!(King, Club);

        let flush = [
            two_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ];

        // Base case
        let mut cards: Vec<Card> = vec![
            king_of_clubs,
            queen_of_clubs,
            nine_of_clubs,
            eight_of_clubs,
            two_of_clubs,
        ];
        cards.sort();

        if let Some(result) = check_for_flush(&cards) {
            assert_eq!(result, flush);
        } else {
            panic!("Expected a Flush, but none was found.");
        }

        let flush2 = [
            three_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ];

        // Tests that the higher Flush of 3♣, 8♣, 9♣, Q♣, K♣ is identified over the lower Flush of 2♣, 3♣, 8♣, 9♣, Q♣.
        let mut cards2: Vec<Card> = vec![
            king_of_clubs,
            queen_of_clubs,
            nine_of_clubs,
            eight_of_clubs,
            two_of_clubs,
            two_of_diamonds,
            three_of_clubs,
        ];
        cards2.sort();

        if let Some(result) = check_for_flush(&cards2) {
            assert_eq!(result, flush2);
        } else {
            panic!("Expected a Flush, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing a Flush is ranked correctly.
    #[test]
    fn rank_hand_flush_works() {
        let two_of_clubs = card!(Two, Club);
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_clubs = card!(Three, Club);
        let eight_of_clubs = card!(Eight, Club);
        let nine_of_clubs = card!(Nine, Club);
        let queen_of_clubs = card!(Queen, Club);
        let king_of_clubs = card!(King, Club);

        let flush = HandRank::Flush([
            two_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ]);

        // Base case
        let cards: Vec<Card> = vec![
            king_of_clubs,
            queen_of_clubs,
            nine_of_clubs,
            eight_of_clubs,
            two_of_clubs,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, flush);

        let flush2 = HandRank::Flush([
            three_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ]);

        // Tests that the higher Flush of 3♣, 8♣, 9♣, Q♣, K♣ is identified over the lower Flush of 2♣, 3♣, 8♣, 9♣, Q♣.
        let cards2: Vec<Card> = vec![
            king_of_clubs,
            queen_of_clubs,
            nine_of_clubs,
            eight_of_clubs,
            two_of_clubs,
            two_of_diamonds,
            three_of_clubs,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, flush2);
    }

    /// Tests check_for_flush().
    ///
    /// Tests if a winning Flush on the table is correctly identified for all parties.
    #[test]
    fn check_for_flush_on_the_table_works() {
        let two_of_clubs = card!(Two, Club);
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_diamonds = card!(Three, Diamond);
        let four_of_hearts = card!(Four, Heart);
        let five_of_diamonds = card!(Five, Diamond);
        let five_of_hearts = card!(Five, Heart);
        let six_of_spades = card!(Six, Spade);
        let seven_of_spades = card!(Seven, Spade);
        let eight_of_clubs = card!(Eight, Club);
        let nine_of_clubs = card!(Nine, Club);
        let nine_of_spades = card!(Nine, Spade);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_clubs = card!(Queen, Club);
        let king_of_clubs = card!(King, Club);

        let flush = [
            two_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ];

        let mut table_cards: Vec<Card> = vec![
            two_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ];
        table_cards.sort();

        if let Some(result) = check_for_flush(&table_cards) {
            assert_eq!(result, flush);
        } else {
            panic!("Expected a Flush, but none was found.");
        }

        let mut player1_cards: Vec<Card> = vec![three_of_diamonds, four_of_hearts];
        player1_cards.extend(table_cards.clone());
        player1_cards.sort();

        if let Some(result) = check_for_flush(&player1_cards) {
            assert_eq!(result, flush);
        } else {
            panic!("Expected a Flush, but none was found.");
        }

        let mut player2_cards: Vec<Card> = vec![five_of_diamonds, six_of_spades];
        player2_cards.extend(table_cards.clone());
        player2_cards.sort();

        if let Some(result) = check_for_flush(&player2_cards) {
            assert_eq!(result, flush);
        } else {
            panic!("Expected a Flush, but none was found.");
        }

        let mut player3_cards: Vec<Card> = vec![seven_of_spades, nine_of_spades];
        player3_cards.extend(table_cards.clone());
        player3_cards.sort();

        if let Some(result) = check_for_flush(&player3_cards) {
            assert_eq!(result, flush);
        } else {
            panic!("Expected a Flush, but none was found.");
        }

        let mut player4_cards: Vec<Card> = vec![two_of_diamonds, five_of_hearts];
        player4_cards.extend(table_cards.clone());
        player4_cards.sort();

        if let Some(result) = check_for_flush(&player4_cards) {
            assert_eq!(result, flush);
        } else {
            panic!("Expected a Flush, but none was found.");
        }

        let mut player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        player5_cards.extend(table_cards.clone());
        player5_cards.sort();

        if let Some(result) = check_for_flush(&player5_cards) {
            assert_eq!(result, flush);
        } else {
            panic!("Expected a Flush, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a winning Flush on the table is ranked correctly for all parties.
    #[test]
    fn rank_hand_flush_table_flush_works() {
        let two_of_clubs = card!(Two, Club);
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_diamonds = card!(Three, Diamond);
        let four_of_hearts = card!(Four, Heart);
        let five_of_diamonds = card!(Five, Diamond);
        let five_of_hearts = card!(Five, Heart);
        let six_of_spades = card!(Six, Spade);
        let seven_of_spades = card!(Seven, Spade);
        let eight_of_clubs = card!(Eight, Club);
        let nine_of_clubs = card!(Nine, Club);
        let nine_of_spades = card!(Nine, Spade);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_clubs = card!(Queen, Club);
        let king_of_clubs = card!(King, Club);

        let flush = HandRank::Flush([
            two_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ]);

        let table_cards: Vec<Card> = vec![
            two_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ];

        let table_cards_hand_rank = rank_hand(table_cards.clone());
        assert_eq!(table_cards_hand_rank, flush);

        let mut player1_cards: Vec<Card> = vec![three_of_diamonds, four_of_hearts];
        player1_cards.extend(table_cards.clone());
        let player1_hand_rank = rank_hand(player1_cards);
        assert_eq!(player1_hand_rank, flush);

        let mut player2_cards: Vec<Card> = vec![five_of_diamonds, six_of_spades];
        player2_cards.extend(table_cards.clone());
        let player2_hand_rank = rank_hand(player2_cards);
        assert_eq!(player2_hand_rank, flush);

        let mut player3_cards: Vec<Card> = vec![seven_of_spades, nine_of_spades];
        player3_cards.extend(table_cards.clone());
        let player3_hand_rank = rank_hand(player3_cards);
        assert_eq!(player3_hand_rank, flush);

        let mut player4_cards: Vec<Card> = vec![two_of_diamonds, five_of_hearts];
        player4_cards.extend(table_cards.clone());
        let player4_hand_rank = rank_hand(player4_cards);
        assert_eq!(player4_hand_rank, flush);

        let mut player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        player5_cards.extend(table_cards.clone());
        let player5_hand_rank = rank_hand(player5_cards);
        assert_eq!(player5_hand_rank, flush);

        assert_eq!(player1_hand_rank, player2_hand_rank);
        assert_eq!(player2_hand_rank, player3_hand_rank);
        assert_eq!(player3_hand_rank, player4_hand_rank);
        assert_eq!(player4_hand_rank, player5_hand_rank);
    }

    /// Tests check_for_flush().
    ///
    /// Tests if a winning Flush on the table is correctly identified for all parties.
    #[test]
    fn check_for_flush_equal_flushes_in_hand_works() {
        let two_of_clubs = card!(Two, Club);
        let two_of_diamonds = card!(Two, Diamond);
        let four_of_hearts = card!(Four, Heart);
        let five_of_clubs = card!(Five, Club);
        let six_of_spades = card!(Six, Spade);
        let eight_of_clubs = card!(Eight, Club);
        let nine_of_clubs = card!(Nine, Club);
        let queen_of_clubs = card!(Queen, Club);
        let king_of_clubs = card!(King, Club);

        let flush1 = [
            two_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ];

        let flush2 = [
            five_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ];

        let mut table_cards: Vec<Card> = vec![
            two_of_diamonds,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ];
        table_cards.sort();

        let mut player1_cards: Vec<Card> = vec![two_of_clubs, four_of_hearts];
        player1_cards.extend(table_cards.clone());
        player1_cards.sort();

        if let Some(result) = check_for_flush(&player1_cards) {
            assert_eq!(result, flush1);
        } else {
            panic!("Expected a Flush, but none was found.");
        }

        let mut player2_cards: Vec<Card> = vec![five_of_clubs, six_of_spades];
        player2_cards.extend(table_cards.clone());
        player2_cards.sort();

        if let Some(result) = check_for_flush(&player2_cards) {
            assert_eq!(result, flush2);
        } else {
            panic!("Expected a Flush, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a player with a higher high card in their Flush beats a player with a lower high card.
    #[test]
    fn rank_hand_higher_flush_beats_lower_flush() {
        let two_of_clubs = card!(Two, Club);
        let two_of_diamonds = card!(Two, Diamond);
        let four_of_hearts = card!(Four, Heart);
        let five_of_clubs = card!(Five, Club);
        let five_of_hearts = card!(Five, Heart);
        let six_of_spades = card!(Six, Spade);
        let seven_of_spades = card!(Seven, Spade);
        let eight_of_clubs = card!(Eight, Club);
        let nine_of_clubs = card!(Nine, Club);
        let nine_of_spades = card!(Nine, Spade);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_clubs = card!(Queen, Club);
        let king_of_clubs = card!(King, Club);

        let flush1 = HandRank::Flush([
            two_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ]);

        let flush2 = HandRank::Flush([
            five_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ]);

        let table_cards: Vec<Card> = vec![
            two_of_diamonds,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ];

        let winning_flush = HandRank::Flush([
            five_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ]);

        let mut player1_cards: Vec<Card> = vec![two_of_clubs, four_of_hearts];
        player1_cards.extend(table_cards.clone());
        let player1_hand_rank = rank_hand(player1_cards);
        assert_eq!(player1_hand_rank, flush1);

        let mut player2_cards: Vec<Card> = vec![five_of_clubs, six_of_spades];
        player2_cards.extend(table_cards.clone());
        let player2_hand_rank = rank_hand(player2_cards);
        assert_eq!(player2_hand_rank, flush2);

        let mut player3_cards: Vec<Card> = vec![seven_of_spades, nine_of_spades];
        player3_cards.extend(table_cards.clone());
        let player3_hand_rank = rank_hand(player3_cards);

        let mut player4_cards: Vec<Card> = vec![two_of_diamonds, five_of_hearts];
        player4_cards.extend(table_cards.clone());
        let player4_hand_rank = rank_hand(player4_cards);

        let mut player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        player5_cards.extend(table_cards.clone());
        let player5_hand_rank = rank_hand(player5_cards);

        assert_ne!(flush1, flush2);
        assert_eq!(winning_flush, flush2);
        assert_eq!(winning_flush, player2_hand_rank);
        assert_ne!(player1_hand_rank, player2_hand_rank);
        assert_ne!(player1_hand_rank, player3_hand_rank);
        assert_ne!(player1_hand_rank, player4_hand_rank);
        assert_ne!(player1_hand_rank, player5_hand_rank);
    }

    /// Tests check_for_full_house().
    ///
    /// Tests if a Full House is correctly identified.
    #[test]
    fn check_for_full_house_works() {
        let three_of_clubs = card!(Three, Club);
        let three_of_spades = card!(Three, Spade);
        let seven_of_clubs = card!(Seven, Club);
        let seven_of_spades = card!(Seven, Spade);
        let king_of_clubs = card!(King, Club);
        let king_of_diamonds = card!(King, Diamond);
        let king_of_hearts = card!(King, Heart);

        let full_house = [
            king_of_clubs,
            king_of_diamonds,
            king_of_hearts,
            seven_of_clubs,
            seven_of_spades,
        ];

        // Base case
        let mut cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            king_of_diamonds,
            seven_of_clubs,
            seven_of_spades,
        ];
        cards.sort();

        if let Some(result) = check_for_full_house(&cards) {
            assert_eq!(result, full_house);
        } else {
            panic!("Expected a Full House, but none was found.");
        }

        // Tests that the higher Full House consisting of Ks & 7s is identified over the lower Full House containing 3s.
        let mut cards2: Vec<Card> = vec![
            three_of_clubs,
            three_of_spades,
            king_of_clubs,
            king_of_hearts,
            king_of_diamonds,
            seven_of_clubs,
            seven_of_spades,
        ];
        cards2.sort();

        if let Some(result) = check_for_full_house(&cards2) {
            assert_eq!(result, full_house);
        } else {
            panic!("Expected a Full House, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing a Full House is ranked correctly.
    #[test]
    fn rank_hand_full_house_works() {
        let three_of_clubs = card!(Three, Club);
        let three_of_spades = card!(Three, Spade);
        let seven_of_clubs = card!(Seven, Club);
        let seven_of_spades = card!(Seven, Spade);
        let king_of_clubs = card!(King, Club);
        let king_of_diamonds = card!(King, Diamond);
        let king_of_hearts = card!(King, Heart);

        let full_house = HandRank::FullHouse([
            king_of_clubs,
            king_of_diamonds,
            king_of_hearts,
            seven_of_clubs,
            seven_of_spades,
        ]);

        // Base case
        let cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            king_of_diamonds,
            seven_of_clubs,
            seven_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, full_house);

        // Tests that the higher Full House consisting of Ks & 7s is identified over the lower Full House containing 3s.
        let cards2: Vec<Card> = vec![
            three_of_clubs,
            three_of_spades,
            king_of_clubs,
            king_of_hearts,
            king_of_diamonds,
            seven_of_clubs,
            seven_of_spades,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, full_house);
    }

    /// Tests check_for_four_of_a_kind().
    ///
    /// Tests if a Four of a Kind is correctly identified.
    #[test]
    fn check_for_four_of_a_kind_works() {
        let six_of_clubs = card!(Six, Club);
        let six_of_diamonds = card!(Six, Diamond);
        let six_of_hearts = card!(Six, Heart);
        let six_of_spades = card!(Six, Spade);
        let king_of_clubs = card!(King, Club);
        let king_of_hearts = card!(King, Heart);
        let king_of_spades = card!(King, Spade);

        let four_of_a_kind = [six_of_clubs, six_of_diamonds, six_of_hearts, six_of_spades];

        // Base case
        let mut cards: Vec<Card> = vec![
            six_of_spades,
            six_of_diamonds,
            six_of_hearts,
            six_of_clubs,
            king_of_spades,
        ];
        cards.sort();

        if let Some(result) = check_for_four_of_a_kind(&cards) {
            assert_eq!(result, four_of_a_kind);
        } else {
            panic!("Expected a Four of a Kind, but none was found.");
        }

        // Tests that the Four of a Kind is identified over the Three of a Kind.
        let mut cards2: Vec<Card> = vec![
            six_of_spades,
            six_of_diamonds,
            six_of_hearts,
            six_of_clubs,
            king_of_spades,
            king_of_clubs,
            king_of_hearts,
        ];
        cards2.sort();

        if let Some(result) = check_for_four_of_a_kind(&cards2) {
            assert_eq!(result, four_of_a_kind);
        } else {
            panic!("Expected a Four of a Kind, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing a Four of a Kind is ranked correctly.
    #[test]
    fn rank_hand_four_of_a_kind_works() {
        let six_of_clubs = card!(Six, Club);
        let six_of_diamonds = card!(Six, Diamond);
        let six_of_hearts = card!(Six, Heart);
        let six_of_spades = card!(Six, Spade);
        let king_of_clubs = card!(King, Club);
        let king_of_hearts = card!(King, Heart);
        let king_of_spades = card!(King, Spade);

        let four_of_a_kind =
            HandRank::FourOfAKind([six_of_clubs, six_of_diamonds, six_of_hearts, six_of_spades]);

        // Base case
        let cards: Vec<Card> = vec![
            six_of_spades,
            six_of_diamonds,
            six_of_hearts,
            six_of_clubs,
            king_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, four_of_a_kind);

        // Tests that the Four of a Kind is identified over the Three of a Kind.
        let cards2: Vec<Card> = vec![
            six_of_spades,
            six_of_diamonds,
            six_of_hearts,
            six_of_clubs,
            king_of_spades,
            king_of_clubs,
            king_of_hearts,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, four_of_a_kind);
    }

    /// Tests check_for_straight_flush().
    ///
    /// Tests if a Straight Flush is correctly identified.
    #[test]
    fn check_for_straight_flush_works() {
        let two_of_spades = card!(Two, Spade);
        let three_of_spades = card!(Three, Spade);
        let four_of_spades = card!(Four, Spade);
        let five_of_spades = card!(Five, Spade);
        let six_of_spades = card!(Six, Spade);
        let seven_of_spades = card!(Seven, Spade);

        let straight_flush = [
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
        ];

        let mut cards: Vec<Card> = vec![
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
        ];
        cards.sort();

        if let Some(result) = check_for_straight_flush(&cards) {
            assert_eq!(result, straight_flush);
        } else {
            panic!("Expected a Straight Flush, but none was found.");
        }

        // Tests that the higher Straight of 3, 4, 5, 6, 7 is identified over the lower Straight of 2, 3, 4, 5, 6.
        let straight_flush2 = [
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
            seven_of_spades,
        ];

        let mut cards2: Vec<Card> = vec![
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
            seven_of_spades,
        ];
        cards2.sort();

        if let Some(result) = check_for_straight_flush(&cards2) {
            assert_eq!(result, straight_flush2);
        } else {
            panic!("Expected a Straight Flush, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing a Straight Flush is ranked correctly.
    #[test]
    fn rank_hand_straight_flush_works() {
        let two_of_spades = card!(Two, Spade);
        let three_of_spades = card!(Three, Spade);
        let four_of_spades = card!(Four, Spade);
        let five_of_spades = card!(Five, Spade);
        let six_of_spades = card!(Six, Spade);
        let seven_of_spades = card!(Seven, Spade);

        let straight_flush = HandRank::StraightFlush([
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
        ]);

        let cards: Vec<Card> = vec![
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, straight_flush);

        // Tests that the higher Straight of 3, 4, 5, 6, 7 is identified over the lower Straight of 2, 3, 4, 5, 6.
        let straight_flush2 = HandRank::StraightFlush([
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
            seven_of_spades,
        ]);

        let cards2: Vec<Card> = vec![
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
            seven_of_spades,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, straight_flush2);
    }

    /// Tests check_for_straight_flush().
    ///
    /// Tests if an Ace-low Straight Flush is correctly identified.
    #[test]
    fn check_for_straight_flush_ace_low_works() {
        let ace_of_diamonds = card!(Ace, Diamond);
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_diamonds = card!(Three, Diamond);
        let four_of_diamonds = card!(Four, Diamond);
        let five_of_diamonds = card!(Five, Diamond);
        let seven_of_diamonds = card!(Seven, Diamond);
        let eight_of_diamonds = card!(Eight, Diamond);

        let ace_low_straight_flush = [
            ace_of_diamonds,
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
        ];

        // Base case
        let mut cards: Vec<Card> = vec![
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
            ace_of_diamonds,
        ];
        cards.sort();

        if let Some(result) = check_for_straight_flush(&cards) {
            assert_eq!(result, ace_low_straight_flush);
        } else {
            panic!("Expected a Straight Flush, but none was found.");
        }

        // Tests that the 7♦ is ignored, and the Ace-low Straight is identified.
        let mut cards2: Vec<Card> = vec![
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
            seven_of_diamonds,
            ace_of_diamonds,
        ];
        cards2.sort();

        if let Some(result) = check_for_straight_flush(&cards2) {
            assert_eq!(result, ace_low_straight_flush);
        } else {
            panic!("Expected a Straight Flush, but none was found.");
        }

        // Tests that the 7♦ & 8♦ are ignored, and the Ace-low Straight is identified.
        let mut cards3: Vec<Card> = vec![
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
            seven_of_diamonds,
            eight_of_diamonds,
            ace_of_diamonds,
        ];
        cards3.sort();

        if let Some(result) = check_for_straight_flush(&cards3) {
            assert_eq!(result, ace_low_straight_flush);
        } else {
            panic!("Expected a Straight Flush, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing an Ace-low Straight Flush is ranked correctly.
    #[test]
    fn rank_hand_straight_flush_ace_low_works() {
        let ace_of_diamonds = card!(Ace, Diamond);
        let two_of_diamonds = card!(Two, Diamond);
        let three_of_diamonds = card!(Three, Diamond);
        let four_of_diamonds = card!(Four, Diamond);
        let five_of_diamonds = card!(Five, Diamond);
        let seven_of_diamonds = card!(Seven, Diamond);
        let eight_of_diamonds = card!(Eight, Diamond);

        let ace_low_straight_flush = HandRank::StraightFlush([
            ace_of_diamonds,
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
        ]);

        // Base case
        let cards: Vec<Card> = vec![
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
            ace_of_diamonds,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, ace_low_straight_flush);

        // Tests that the 7♦ is ignored, and the Ace-low Straight is identified.
        let cards2: Vec<Card> = vec![
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
            seven_of_diamonds,
            ace_of_diamonds,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, ace_low_straight_flush);

        // Tests that the 7♦ & 8♦ are ignored, and the Ace-low Straight is identified.
        let cards3: Vec<Card> = vec![
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
            seven_of_diamonds,
            eight_of_diamonds,
            ace_of_diamonds,
        ];

        let hand_rank3 = rank_hand(cards3);
        assert_eq!(hand_rank3, ace_low_straight_flush);
    }

    /// Tests check_for_straight_flush().
    ///
    /// Tests if an Ace-high Straight Flush is correctly identified.
    #[test]
    fn check_for_straight_flush_ace_high_aka_royal_flush_works() {
        let nine_of_hearts = card!(Nine, Heart);
        let ten_of_hearts = card!(Ten, Heart);
        let jack_of_hearts = card!(Jack, Heart);
        let queen_of_hearts = card!(Queen, Heart);
        let king_of_hearts = card!(King, Heart);
        let ace_of_hearts = card!(Ace, Heart);

        let ace_high_straight_flush = [
            ten_of_hearts,
            jack_of_hearts,
            queen_of_hearts,
            king_of_hearts,
            ace_of_hearts,
        ];

        // Base case
        let mut cards: Vec<Card> = vec![
            ten_of_hearts,
            jack_of_hearts,
            queen_of_hearts,
            king_of_hearts,
            ace_of_hearts,
        ];
        cards.sort();

        if let Some(result) = check_for_straight_flush(&cards) {
            assert_eq!(result, ace_high_straight_flush);
        } else {
            panic!("Expected a Straight Flush, but none was found.");
        }

        // Tests that the higher Straight Flush of 10-A is identified over the lower Straight Flush of 9 - K.
        let mut cards2: Vec<Card> = vec![
            nine_of_hearts,
            ten_of_hearts,
            jack_of_hearts,
            queen_of_hearts,
            king_of_hearts,
            ace_of_hearts,
        ];
        cards2.sort();

        if let Some(result) = check_for_straight_flush(&cards2) {
            assert_eq!(result, ace_high_straight_flush);
        } else {
            panic!("Expected a Straight Flush, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing an Ace-high Straight Flush (aka Royal Flush) is ranked correctly.
    #[test]
    fn rank_hand_straight_flush_ace_high_aka_royal_flush_works() {
        let nine_of_hearts = card!(Nine, Heart);
        let ten_of_hearts = card!(Ten, Heart);
        let jack_of_hearts = card!(Jack, Heart);
        let queen_of_hearts = card!(Queen, Heart);
        let king_of_hearts = card!(King, Heart);
        let ace_of_hearts = card!(Ace, Heart);

        let ace_high_straight_flush = HandRank::StraightFlush([
            ten_of_hearts,
            jack_of_hearts,
            queen_of_hearts,
            king_of_hearts,
            ace_of_hearts,
        ]);

        // Base case
        let cards: Vec<Card> = vec![
            ten_of_hearts,
            jack_of_hearts,
            queen_of_hearts,
            king_of_hearts,
            ace_of_hearts,
        ];

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, ace_high_straight_flush);

        // Tests that the higher Straight Flush of 10-A is identified over the lower Straight Flush of 9 - K.
        let cards2: Vec<Card> = vec![
            nine_of_hearts,
            ten_of_hearts,
            jack_of_hearts,
            queen_of_hearts,
            king_of_hearts,
            ace_of_hearts,
        ];

        let hand_rank2 = rank_hand(cards2);
        assert_eq!(hand_rank2, ace_high_straight_flush);
    }

    #[test]
    fn eq_matches_ord_high_card() {
        let hand_rank1: HandRank = HandRank::HighCard(card!(Two, Club));
        let hand_rank2: HandRank = HandRank::HighCard(card!(Two, Diamond));

        if hand_rank1.cmp(&hand_rank2) == Ordering::Equal {
            assert!(hand_rank1.eq(&hand_rank1));
        }

        if hand_rank1.eq(&hand_rank2) {
            assert!(hand_rank1.cmp(&hand_rank2) == Ordering::Equal);
        }
    }

    #[test]
    fn eq_matches_ord_pair() {
        let hand_rank1: HandRank = HandRank::Pair([card!(Two, Club), card!(Two, Heart)]);
        let hand_rank2: HandRank = HandRank::Pair([card!(Two, Diamond), card!(Two, Spade)]);

        if hand_rank1.cmp(&hand_rank2) == Ordering::Equal {
            assert!(hand_rank1.eq(&hand_rank1));
        }

        if hand_rank1.eq(&hand_rank2) {
            assert!(hand_rank1.cmp(&hand_rank2) == Ordering::Equal);
        }
    }

    #[test]
    fn eq_matches_ord_two_pair() {
        let hand_rank1: HandRank = HandRank::TwoPair([
            card!(Two, Club),
            card!(Two, Heart),
            card!(Three, Club),
            card!(Three, Heart),
        ]);
        let hand_rank2: HandRank = HandRank::TwoPair([
            card!(Two, Diamond),
            card!(Two, Spade),
            card!(Three, Diamond),
            card!(Three, Spade),
        ]);

        if hand_rank1.cmp(&hand_rank2) == Ordering::Equal {
            assert!(hand_rank1.eq(&hand_rank1));
        }

        if hand_rank1.eq(&hand_rank2) {
            assert!(hand_rank1.cmp(&hand_rank2) == Ordering::Equal);
        }
    }

    #[test]
    fn eq_matches_ord_three_of_a_kind() {
        let hand_rank1: HandRank =
            HandRank::ThreeOfAKind([card!(Two, Club), card!(Two, Heart), card!(Two, Spade)]);
        let hand_rank2: HandRank =
            HandRank::ThreeOfAKind([card!(Two, Club), card!(Two, Heart), card!(Two, Diamond)]);

        if hand_rank1.cmp(&hand_rank2) == Ordering::Equal {
            assert!(hand_rank1.eq(&hand_rank1));
        }

        if hand_rank1.eq(&hand_rank2) {
            assert!(hand_rank1.cmp(&hand_rank2) == Ordering::Equal);
        }
    }

    #[test]
    fn eq_matches_ord_straight() {
        let hand_rank1: HandRank = HandRank::Straight([
            card!(Two, Club),
            card!(Three, Heart),
            card!(Four, Diamond),
            card!(Five, Spade),
            card!(Six, Club),
        ]);
        let hand_rank2: HandRank = HandRank::Straight([
            card!(Two, Club),
            card!(Three, Heart),
            card!(Four, Diamond),
            card!(Five, Spade),
            card!(Six, Diamond),
        ]);

        if hand_rank1.cmp(&hand_rank2) == Ordering::Equal {
            assert!(hand_rank1.eq(&hand_rank1));
        }

        if hand_rank1.eq(&hand_rank2) {
            assert!(hand_rank1.cmp(&hand_rank2) == Ordering::Equal);
        }
    }

    #[test]
    fn eq_matches_ord_flush() {
        let hand_rank1: HandRank = HandRank::Flush([
            card!(Two, Club),
            card!(Three, Club),
            card!(Five, Club),
            card!(Seven, Club),
            card!(Jack, Club),
        ]);
        let hand_rank2: HandRank = HandRank::Flush([
            card!(Two, Diamond),
            card!(Three, Diamond),
            card!(Five, Diamond),
            card!(Seven, Diamond),
            card!(Jack, Diamond),
        ]);

        if hand_rank1.cmp(&hand_rank2) == Ordering::Equal {
            assert!(hand_rank1.eq(&hand_rank1));
        }

        if hand_rank1.eq(&hand_rank2) {
            assert!(hand_rank1.cmp(&hand_rank2) == Ordering::Equal);
        }
    }

    #[test]
    fn eq_matches_ord_full_house() {
        let hand_rank1: HandRank = HandRank::FullHouse([
            card!(Two, Club),
            card!(Two, Heart),
            card!(Three, Club),
            card!(Three, Heart),
            card!(Three, Spade),
        ]);
        let hand_rank2: HandRank = HandRank::FullHouse([
            card!(Two, Diamond),
            card!(Two, Spade),
            card!(Three, Club),
            card!(Three, Heart),
            card!(Three, Spade),
        ]);

        if hand_rank1.cmp(&hand_rank2) == Ordering::Equal {
            assert!(hand_rank1.eq(&hand_rank1));
        }

        if hand_rank1.eq(&hand_rank2) {
            assert!(hand_rank1.cmp(&hand_rank2) == Ordering::Equal);
        }
    }

    #[test]
    fn eq_matches_ord_four_of_a_kind() {
        let hand_rank1: HandRank = HandRank::FourOfAKind([
            card!(Two, Club),
            card!(Two, Diamond),
            card!(Two, Heart),
            card!(Two, Spade),
        ]);
        let hand_rank2: HandRank = HandRank::FourOfAKind([
            card!(Two, Club),
            card!(Two, Diamond),
            card!(Two, Heart),
            card!(Two, Spade),
        ]);

        if hand_rank1.cmp(&hand_rank2) == Ordering::Equal {
            assert!(hand_rank1.eq(&hand_rank1));
        }

        if hand_rank1.eq(&hand_rank2) {
            assert!(hand_rank1.cmp(&hand_rank2) == Ordering::Equal);
        }
    }

    #[test]
    fn eq_matches_ord_straight_flush() {
        let hand_rank1: HandRank = HandRank::Straight([
            card!(Two, Club),
            card!(Three, Club),
            card!(Four, Club),
            card!(Five, Club),
            card!(Six, Club),
        ]);
        let hand_rank2: HandRank = HandRank::Straight([
            card!(Two, Diamond),
            card!(Three, Diamond),
            card!(Four, Diamond),
            card!(Five, Spade),
            card!(Six, Diamond),
        ]);

        if hand_rank1.cmp(&hand_rank2) == Ordering::Equal {
            assert!(hand_rank1.eq(&hand_rank1));
        }

        if hand_rank1.eq(&hand_rank2) {
            assert!(hand_rank1.cmp(&hand_rank2) == Ordering::Equal);
        }
    }
}
