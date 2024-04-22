use std::collections::HashMap;
use std::fmt;

use cards::card::{Card, Rank, Suit};

#[derive(Clone, Copy, Debug, Eq, PartialOrd)]
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

impl PartialEq for HandRank {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HandRank::HighCard(card1), HandRank::HighCard(card2)) => card1.rank == card2.rank,
            (HandRank::Pair(cards1), HandRank::Pair(cards2)) => cards1
                .iter()
                .map(|card| card.rank)
                .eq(cards2.iter().map(|card| card.rank)),
            (HandRank::TwoPair(cards1), HandRank::TwoPair(cards2)) => cards1
                .iter()
                .map(|card| card.rank)
                .eq(cards2.iter().map(|card| card.rank)),
            (HandRank::ThreeOfAKind(cards1), HandRank::ThreeOfAKind(cards2)) => cards1
                .iter()
                .map(|card| card.rank)
                .eq(cards2.iter().map(|card| card.rank)),
            (HandRank::Straight(cards1), HandRank::Straight(cards2)) => {
                // Compare the ranks of the highest cards
                cards1[4].rank == cards2[4].rank
            }
            (HandRank::Flush(cards1), HandRank::Flush(cards2)) => {
                // Compare the suits of the highest cards
                cards1[4].suit == cards2[4].suit
            }
            (HandRank::FullHouse(cards1), HandRank::FullHouse(cards2)) => {
                let (triple1, pair1) = (cards1[0].rank, cards1[3].rank);
                let (triple2, pair2) = (cards2[0].rank, cards2[3].rank);
                // Compare the ranks of the triple and the pair separately
                (triple1 == triple2 && pair1 == pair2) || (triple1 == pair2 && pair1 == triple2)
            }
            (HandRank::FourOfAKind(cards1), HandRank::FourOfAKind(cards2)) => cards1
                .iter()
                .map(|card| card.rank)
                .eq(cards2.iter().map(|card| card.rank)),
            (HandRank::StraightFlush(cards1), HandRank::StraightFlush(cards2)) => {
                // Compare the ranks of the highest cards
                if cards1[4].rank != cards2[4].rank {
                    return false;
                }
                // Compare the suits of the highest cards
                cards1[4].suit == cards2[4].suit
            }
            _ => false,
        }
    }
}

impl fmt::Display for HandRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let printable = match self {
            HandRank::HighCard(_) => format!("a High Card"),
            HandRank::Pair(_) => format!("a Pair"),
            HandRank::TwoPair(_) => format!("Two Pairs"),
            HandRank::ThreeOfAKind(_) => format!("Three of a Kind"),
            HandRank::Straight(_) => format!("a Straight"),
            HandRank::Flush(_) => format!("a Flush"),
            HandRank::FullHouse(_) => format!("a Full House"),
            HandRank::FourOfAKind(_) => format!("Four of a Kind"),
            HandRank::StraightFlush(cards) => {
                let is_royal_flush = cards.iter().all(|card| {
                    card.rank == Rank::Ten
                        || card.rank == Rank::Jack
                        || card.rank == Rank::Queen
                        || card.rank == Rank::King
                        || card.rank == Rank::Ace
                });

                if is_royal_flush {
                    format!("a Royal Flush")
                } else {
                    format!("a Straight Flush")
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
        return HandRank::HighCard(high_card);
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
fn get_high_card_value(cards: &Vec<Card>) -> Option<Card> {
    let mut high_card: Option<Card> = None;
    let mut high_card_value: u8 = 0;
    for &card in cards {
        let card_rank_value = card.rank.value();

        if card_rank_value > high_card_value {
            high_card_value = card_rank_value;
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
    let mut high_pair_rank_value = 0;

    for (rank, cards) in ranks.iter() {
        let rank_value = rank.value();

        if cards.len() == 2 && rank_value > high_pair_rank_value {
            high_pair_rank_value = rank_value;
            high_pair_cards = Some([cards[0], cards[1]]);
        }
    }

    if high_pair_rank_value > 0 {
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

    let mut high_rank = 0;
    let mut three_of_a_kind_cards: Option<[Card; 3]> = None;
    for (rank, cards) in ranks.iter() {
        if cards.len() == 3 && rank.value() > high_rank {
            high_rank = rank.value();
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

    let contains_ace = cards.iter().any(|&card| card.rank == Rank::Ace);

    // Check for a straight containing an Ace if an Ace is present
    if contains_ace {
        // Check for Ace-low straight
        if cards[0].rank == Rank::Two
            && cards[1].rank == Rank::Three
            && cards[2].rank == Rank::Four
            && cards[3].rank == Rank::Five
            && cards[cards.len() - 1].rank == Rank::Ace
        {
            return Some([
                cards[cards.len() - 1],
                cards[cards.len() - 5],
                cards[cards.len() - 4],
                cards[cards.len() - 3],
                cards[cards.len() - 2],
            ]);
        }

        // Check for Ace-high straight
        if cards[cards.len() - 5].rank == Rank::Ten
            && cards[cards.len() - 4].rank == Rank::Jack
            && cards[cards.len() - 3].rank == Rank::Queen
            && cards[cards.len() - 2].rank == Rank::King
            && cards[cards.len() - 1].rank == Rank::Ace
        {
            return Some([
                cards[cards.len() - 5],
                cards[cards.len() - 4],
                cards[cards.len() - 3],
                cards[cards.len() - 2],
                cards[cards.len() - 1],
            ]);
        }

        return None;
    }

    // Check for non-Ace straight
    let mut straight_cards: Vec<Card> = Vec::new();
    straight_cards.push(cards[0]);

    for i in 1..cards.len() {
        if cards[i].rank.value() == cards[i - 1].rank.value() + 1 {
            straight_cards.push(cards[i]);
        } else {
            // Check if the current card is not part of a sequence and happens to equal previous card.
            if cards[i].rank.value() != cards[i - 1].rank.value() {
                straight_cards.clear();
                straight_cards.push(cards[i]);
            }
        }
    }

    if straight_cards.len() >= 5 {
        return Some([
            cards[cards.len() - 5],
            cards[cards.len() - 4],
            cards[cards.len() - 3],
            cards[cards.len() - 2],
            cards[cards.len() - 1],
        ]);
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
            let contains_ace = cards.iter().any(|&card| card.rank == Rank::Ace);
            if contains_ace {
                // Check for Ace-low flush, which helps when this is also a straight
                if cards[cards.len() - 5].rank == Rank::Two
                    && cards[cards.len() - 4].rank == Rank::Three
                    && cards[cards.len() - 3].rank == Rank::Four
                    && cards[cards.len() - 2].rank == Rank::Five
                    && cards[cards.len() - 1].rank == Rank::Ace
                {
                    return Some([
                        cards[cards.len() - 1],
                        cards[cards.len() - 5],
                        cards[cards.len() - 4],
                        cards[cards.len() - 3],
                        cards[cards.len() - 2],
                    ]);
                }
            }

            return Some([
                cards[cards.len() - 5],
                cards[cards.len() - 4],
                cards[cards.len() - 3],
                cards[cards.len() - 2],
                cards[cards.len() - 1],
            ]);
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
            return Some([cards[0], cards[1], cards[2], cards[3]]);
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

    use cards::card;

    #[test]
    fn high_card_ranks_are_valued_correctly() {
        // Tests that the suit is ignored when comparing High Cards of equal rank.
        assert_eq!(
            HandRank::HighCard(card!(Two, Club)),
            HandRank::HighCard(card!(Two, Diamond))
        );

        assert_ne!(
            HandRank::HighCard(card!(Two, Club)),
            HandRank::HighCard(card!(Three, Club))
        );
    }

    #[test]
    fn high_card_rankings_are_ordered_correctly() {
        assert!(HandRank::HighCard(card!(Two, Club)) < HandRank::HighCard(card!(Three, Club)));

        assert!(HandRank::HighCard(card!(Three, Club)) < HandRank::HighCard(card!(Four, Spade)));

        assert!(HandRank::HighCard(card!(Four, Spade)) < HandRank::HighCard(card!(Five, Spade)));

        assert!(HandRank::HighCard(card!(Five, Spade)) < HandRank::HighCard(card!(Six, Spade)));

        assert!(HandRank::HighCard(card!(Six, Spade)) < HandRank::HighCard(card!(Seven, Spade)));

        assert!(HandRank::HighCard(card!(Seven, Spade)) < HandRank::HighCard(card!(Eight, Club)));

        assert!(HandRank::HighCard(card!(Eight, Club)) < HandRank::HighCard(card!(Nine, Club)));

        assert!(HandRank::HighCard(card!(Nine, Club)) < HandRank::HighCard(card!(Ten, Spade)));

        assert!(HandRank::HighCard(card!(Ten, Spade)) < HandRank::HighCard(card!(Jack, Spade)));

        assert!(HandRank::HighCard(card!(Jack, Spade)) < HandRank::HighCard(card!(Queen, Spade)));

        assert!(HandRank::HighCard(card!(Queen, Spade)) < HandRank::HighCard(card!(King, Spade)));

        assert!(HandRank::HighCard(card!(King, Spade)) < HandRank::HighCard(card!(Ace, Spade)));
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
            card!(Three, Club),
            card!(Four, Heart),
            card!(Five, Diamond),
            card!(Six, Club),
            card!(Seven, Spade),
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

        let high_card = HandRank::HighCard(king_of_clubs);

        let mut cards: Vec<Card> = vec![
            ten_of_clubs,
            four_of_hearts,
            seven_of_diamonds,
            king_of_clubs,
            two_of_spades,
        ];
        cards.sort();

        if let Some(cards) = get_high_card_value(&cards) {
            let identified_high_card = HandRank::HighCard(cards);
            assert_eq!(identified_high_card, high_card);
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

        let pair = HandRank::Pair([king_of_clubs, king_of_hearts]);

        // Base case
        let mut cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            seven_of_diamonds,
            two_of_clubs,
            five_of_spades,
        ];
        cards.sort();

        if let Some(cards) = check_for_pair(&cards) {
            let identified_pair = HandRank::Pair(cards);
            assert_eq!(identified_pair, pair);
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

        if let Some(cards) = check_for_pair(&cards2) {
            let identified_pair = HandRank::Pair(cards);
            assert_eq!(identified_pair, pair);
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

        let two_pair = HandRank::TwoPair([
            king_of_clubs,
            king_of_hearts,
            seven_of_clubs,
            seven_of_diamonds,
        ]);

        let mut cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            seven_of_diamonds,
            seven_of_clubs,
            five_of_spades,
        ];
        cards.sort();

        if let Some(cards) = check_for_two_pair(&cards) {
            let identified_two_pair = HandRank::TwoPair(cards);
            assert_eq!(identified_two_pair, two_pair);
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

        if let Some(cards) = check_for_two_pair(&cards2) {
            let identified_two_pair = HandRank::TwoPair(cards);
            assert_eq!(identified_two_pair, two_pair);
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

        let three_of_a_kind =
            HandRank::ThreeOfAKind([king_of_clubs, king_of_diamonds, king_of_hearts]);

        // Base case
        let mut cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            king_of_diamonds,
            seven_of_clubs,
            five_of_spades,
        ];
        cards.sort();

        if let Some(cards) = check_for_three_of_a_kind(&cards) {
            let identified_three_of_a_kind = HandRank::ThreeOfAKind(cards);
            assert_eq!(identified_three_of_a_kind, three_of_a_kind);
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

        if let Some(cards) = check_for_three_of_a_kind(&cards2) {
            let identified_three_of_a_kind = HandRank::ThreeOfAKind(cards);
            assert_eq!(identified_three_of_a_kind, three_of_a_kind);
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

        let straight = HandRank::Straight([
            three_of_clubs,
            four_of_hearts,
            five_of_diamonds,
            six_of_clubs,
            seven_of_spades,
        ]);

        // Base case
        let mut cards: Vec<Card> = vec![
            three_of_clubs,
            four_of_hearts,
            five_of_diamonds,
            six_of_clubs,
            seven_of_spades,
        ];
        cards.sort();

        if let Some(cards) = check_for_straight(&cards) {
            let identified_straight = HandRank::Straight(cards);
            assert_eq!(identified_straight, straight);
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

        if let Some(cards) = check_for_straight(&cards2) {
            let identified_straight = HandRank::Straight(cards);
            assert_eq!(identified_straight, straight);
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
    }

    /// Tests check_for_straight().
    ///
    /// Tests if an Ace-low Straight is correctly identified.
    #[test]
    fn check_for_straight_ace_low_straight_works() {
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
        let mut cards: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            ace_of_spades,
        ];
        cards.sort();

        if let Some(cards) = check_for_straight(&cards) {
            let identified_straight = HandRank::Straight(cards);
            assert_eq!(identified_straight, ace_low_straight);
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

        if let Some(cards) = check_for_straight(&cards2) {
            let identified_straight = HandRank::Straight(cards);
            assert_eq!(identified_straight, ace_low_straight);
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

        if let Some(cards) = check_for_straight(&cards3) {
            let identified_straight = HandRank::Straight(cards);
            assert_eq!(identified_straight, ace_low_straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }

        // Tests that an Ace-low Straight is ignored, and a higher Straight is identified.
        let mut cards4: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            six_of_diamonds,
            ace_of_spades,
        ];
        cards4.sort();

        if let Some(cards) = check_for_straight(&cards4) {
            let identified_straight = HandRank::Straight(cards);
            assert_eq!(identified_straight, ace_low_straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing an Ace-low Straight is ranked correctly.
    #[test]
    fn rank_hand_ace_low_straight_works() {
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
        let cards4: Vec<Card> = vec![
            two_of_clubs,
            three_of_hearts,
            four_of_spades,
            five_of_hearts,
            six_of_diamonds,
            ace_of_spades,
        ];

        let hand_rank4 = rank_hand(cards4);
        assert_eq!(hand_rank4, ace_low_straight);
    }

    /// Tests check_for_straight().
    ///
    /// Tests if an Ace-high Straight is correctly identified.
    #[test]
    fn check_for_straight_ace_high_straight_works() {
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
        let mut cards: Vec<Card> = vec![
            ten_of_clubs,
            jack_of_hearts,
            queen_of_spades,
            king_of_hearts,
            ace_of_spades,
        ];
        cards.sort();

        if let Some(cards) = check_for_straight(&cards) {
            let identified_straight = HandRank::Straight(cards);
            assert_eq!(identified_straight, ace_high_straight);
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

        if let Some(cards) = check_for_straight(&cards2) {
            let identified_straight = HandRank::Straight(cards);
            assert_eq!(identified_straight, ace_high_straight);
        } else {
            panic!("Expected a Straight, but none was found.");
        }
    }

    /// Tests rank_hand().
    ///
    /// Tests if a hand containing an Ace-high Straight is ranked correctly.
    #[test]
    fn rank_hand_ace_high_straight_works() {
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

        let flush = HandRank::Flush([
            two_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ]);

        // Base case
        let mut cards: Vec<Card> = vec![
            king_of_clubs,
            queen_of_clubs,
            nine_of_clubs,
            eight_of_clubs,
            two_of_clubs,
        ];
        cards.sort();

        if let Some(cards) = check_for_flush(&cards) {
            let identified_flush = HandRank::Flush(cards);
            assert_eq!(identified_flush, flush);
        } else {
            panic!("Expected a Flush, but none was found.");
        }

        let flush2 = HandRank::Flush([
            three_of_clubs,
            eight_of_clubs,
            nine_of_clubs,
            queen_of_clubs,
            king_of_clubs,
        ]);

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

        if let Some(cards) = check_for_flush(&cards2) {
            let identified_flush = HandRank::Flush(cards);
            assert_eq!(identified_flush, flush2);
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

        let full_house = HandRank::FullHouse([
            king_of_clubs,
            king_of_diamonds,
            king_of_hearts,
            seven_of_clubs,
            seven_of_spades,
        ]);

        // Base case
        let mut cards: Vec<Card> = vec![
            king_of_clubs,
            king_of_hearts,
            king_of_diamonds,
            seven_of_clubs,
            seven_of_spades,
        ];
        cards.sort();

        if let Some(cards) = check_for_full_house(&cards) {
            let identified_full_house = HandRank::FullHouse(cards);
            assert_eq!(identified_full_house, full_house);
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

        if let Some(cards) = check_for_full_house(&cards2) {
            let identified_full_house = HandRank::FullHouse(cards);
            assert_eq!(identified_full_house, full_house);
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

        let four_of_a_kind =
            HandRank::FourOfAKind([six_of_clubs, six_of_diamonds, six_of_hearts, six_of_spades]);

        // Base case
        let mut cards: Vec<Card> = vec![
            six_of_spades,
            six_of_diamonds,
            six_of_hearts,
            six_of_clubs,
            king_of_spades,
        ];
        cards.sort();

        if let Some(cards) = check_for_four_of_a_kind(&cards) {
            let identified_four_of_a_kind = HandRank::FourOfAKind(cards);
            assert_eq!(identified_four_of_a_kind, four_of_a_kind);
        } else {
            panic!("Expected a Four of a Kind, but none was found.");
        }

        let hand_rank = rank_hand(cards);
        assert_eq!(hand_rank, four_of_a_kind);

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

        if let Some(cards) = check_for_four_of_a_kind(&cards2) {
            let identified_four_of_a_kind = HandRank::FourOfAKind(cards);
            assert_eq!(identified_four_of_a_kind, four_of_a_kind);
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

        let straight_flush = HandRank::StraightFlush([
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
        ]);

        let mut cards: Vec<Card> = vec![
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
        ];
        cards.sort();

        if let Some(cards) = check_for_straight_flush(&cards) {
            let identified_straight_flush = HandRank::StraightFlush(cards);
            assert_eq!(identified_straight_flush, straight_flush);
        } else {
            panic!("Expected a Straight Flush, but none was found.");
        }

        // Tests that the higher Straight of 3, 4, 5, 6, 7 is identified over the lower Straight of 2, 3, 4, 5, 6.
        let straight_flush2 = HandRank::StraightFlush([
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
            seven_of_spades,
        ]);

        let mut cards2: Vec<Card> = vec![
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
            seven_of_spades,
        ];
        cards2.sort();

        if let Some(cards) = check_for_straight_flush(&cards2) {
            let identified_straight_flush = HandRank::StraightFlush(cards);
            assert_eq!(identified_straight_flush, straight_flush2);
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

        let ace_low_straight_flush = HandRank::StraightFlush([
            ace_of_diamonds,
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
        ]);

        // Base case
        let mut cards: Vec<Card> = vec![
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
            ace_of_diamonds,
        ];
        cards.sort();

        if let Some(cards) = check_for_straight_flush(&cards) {
            let identified_straight_flush = HandRank::StraightFlush(cards);
            assert_eq!(identified_straight_flush, ace_low_straight_flush);
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

        if let Some(cards) = check_for_straight_flush(&cards2) {
            let identified_straight_flush = HandRank::StraightFlush(cards);
            assert_eq!(identified_straight_flush, ace_low_straight_flush);
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

        if let Some(cards) = check_for_straight_flush(&cards3) {
            let identified_straight_flush = HandRank::StraightFlush(cards);
            assert_eq!(identified_straight_flush, ace_low_straight_flush);
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

        let ace_high_straight_flush = HandRank::StraightFlush([
            ten_of_hearts,
            jack_of_hearts,
            queen_of_hearts,
            king_of_hearts,
            ace_of_hearts,
        ]);

        // Base case
        let mut cards: Vec<Card> = vec![
            ten_of_hearts,
            jack_of_hearts,
            queen_of_hearts,
            king_of_hearts,
            ace_of_hearts,
        ];
        cards.sort();

        if let Some(cards) = check_for_straight_flush(&cards) {
            let identified_straight_flush = HandRank::StraightFlush(cards);
            assert_eq!(identified_straight_flush, ace_high_straight_flush);
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

        if let Some(cards) = check_for_straight_flush(&cards2) {
            let identified_straight_flush = HandRank::StraightFlush(cards);
            assert_eq!(identified_straight_flush, ace_high_straight_flush);
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
}
