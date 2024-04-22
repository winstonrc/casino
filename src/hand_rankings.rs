use std::collections::HashMap;
use std::fmt;

use cards::card::{Card, Rank, Suit};

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
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

    let mut sorted_cards = cards.clone();
    sorted_cards.sort();

    if let Some(straight_flush_cards) = check_for_straight_flush(&sorted_cards) {
        return HandRank::StraightFlush(straight_flush_cards);
    }

    if let Some(four_of_a_kind_cards) = check_for_four_of_a_kind(&sorted_cards) {
        return HandRank::FourOfAKind(four_of_a_kind_cards);
    }

    if let Some(full_house_cards) = check_for_full_house(&sorted_cards) {
        return HandRank::FullHouse(full_house_cards);
    }

    if let Some(flush_cards) = check_for_flush(&sorted_cards) {
        return HandRank::Flush(flush_cards);
    }

    if let Some(straight_cards) = check_for_straight(&sorted_cards) {
        return HandRank::Straight(straight_cards);
    }

    if let Some(three_of_a_kind_cards) = check_for_three_of_a_kind(&sorted_cards) {
        return HandRank::ThreeOfAKind(three_of_a_kind_cards);
    }

    if let Some(two_pair_cards) = check_for_two_pair(&sorted_cards) {
        return HandRank::TwoPair(two_pair_cards);
    }

    if let Some(pair_cards) = check_for_pair(&sorted_cards) {
        return HandRank::Pair(pair_cards);
    }

    if let Some(high_card) = get_high_card_value(&sorted_cards) {
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

    for (_rank, cards) in ranks.iter() {
        if cards.len() == 3 {
            return Some([cards[0], cards[1], cards[2]]);
        }
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

    let mut sorted_cards = cards.clone();
    sorted_cards.sort();

    let mut best_straight_cards: Vec<Card> = Vec::new();
    let mut current_straight_cards: Vec<Card> = Vec::new();

    for i in 1..sorted_cards.len() {
        if sorted_cards[i].rank.value() == sorted_cards[i - 1].rank.value() + 1 {
            current_straight_cards.push(sorted_cards[i]);
        } else {
            // Check if the current card is not part of a sequence and happens to equal previous card.
            if sorted_cards[i].rank.value() != sorted_cards[i - 1].rank.value() {
                if current_straight_cards.len() >= 5 {
                    best_straight_cards = current_straight_cards.clone();
                }
                current_straight_cards.clear();
                current_straight_cards.push(sorted_cards[i]);
            }
        }
    }

    if best_straight_cards.len() >= 5 {
        // Since the Ace is Ace-high by default, we only need special consideration for Ace-low.
        if best_straight_cards[4].rank == Rank::Ace && best_straight_cards[0].rank == Rank::Two {
            // Ace-low straight
            return Some([
                sorted_cards[sorted_cards.len() - 1],
                sorted_cards[sorted_cards.len() - 5],
                sorted_cards[sorted_cards.len() - 4],
                sorted_cards[sorted_cards.len() - 3],
                sorted_cards[sorted_cards.len() - 2],
            ]);
        }

        // Normal or Ace-high straight
        return Some([
            sorted_cards[sorted_cards.len() - 5],
            sorted_cards[sorted_cards.len() - 4],
            sorted_cards[sorted_cards.len() - 3],
            sorted_cards[sorted_cards.len() - 2],
            sorted_cards[sorted_cards.len() - 1],
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

    #[test]
    fn high_cards_are_unique() {
        assert_eq!(
            HandRank::HighCard(Card::two_of_clubs()),
            HandRank::HighCard(Card::two_of_clubs())
        );

        assert_ne!(
            HandRank::HighCard(Card::two_of_clubs()),
            HandRank::HighCard(Card::two_of_spades())
        );
    }

    #[test]
    fn high_card_rankings_are_ordered_correctly() {
        assert!(
            HandRank::HighCard(Card::two_of_clubs()) < HandRank::HighCard(Card::three_of_clubs())
        );

        assert!(
            HandRank::HighCard(Card::three_of_clubs()) < HandRank::HighCard(Card::four_of_spades())
        );

        assert!(
            HandRank::HighCard(Card::four_of_spades()) < HandRank::HighCard(Card::five_of_spades())
        );

        assert!(
            HandRank::HighCard(Card::five_of_spades()) < HandRank::HighCard(Card::six_of_spades())
        );

        assert!(
            HandRank::HighCard(Card::six_of_spades()) < HandRank::HighCard(Card::seven_of_spades())
        );

        assert!(
            HandRank::HighCard(Card::seven_of_spades())
                < HandRank::HighCard(Card::eight_of_clubs())
        );

        assert!(
            HandRank::HighCard(Card::eight_of_clubs()) < HandRank::HighCard(Card::nine_of_clubs())
        );

        assert!(
            HandRank::HighCard(Card::nine_of_clubs()) < HandRank::HighCard(Card::ten_of_spades())
        );

        assert!(
            HandRank::HighCard(Card::ten_of_spades()) < HandRank::HighCard(Card::jack_of_spades())
        );

        assert!(
            HandRank::HighCard(Card::jack_of_spades())
                < HandRank::HighCard(Card::queen_of_spades())
        );

        assert!(
            HandRank::HighCard(Card::queen_of_spades())
                < HandRank::HighCard(Card::king_of_spades())
        );

        assert!(
            HandRank::HighCard(Card::king_of_spades()) < HandRank::HighCard(Card::ace_of_spades())
        );
    }

    #[test]
    fn hand_rankings_are_ordered_correctly() {
        let high_card = HandRank::HighCard(Card::king_of_clubs());

        let pair = HandRank::Pair([Card::king_of_clubs(), Card::king_of_hearts()]);

        let two_pair = HandRank::TwoPair([
            Card::king_of_clubs(),
            Card::king_of_hearts(),
            Card::seven_of_diamonds(),
            Card::seven_of_clubs(),
        ]);

        let three_of_a_kind = HandRank::ThreeOfAKind([
            Card::king_of_clubs(),
            Card::king_of_hearts(),
            Card::king_of_diamonds(),
        ]);

        let straight = HandRank::Straight([
            Card::three_of_clubs(),
            Card::four_of_hearts(),
            Card::five_of_diamonds(),
            Card::six_of_clubs(),
            Card::seven_of_spades(),
        ]);

        let flush = HandRank::Flush([
            Card::king_of_clubs(),
            Card::queen_of_clubs(),
            Card::nine_of_clubs(),
            Card::eight_of_clubs(),
            Card::two_of_clubs(),
        ]);

        let full_house = HandRank::FullHouse([
            Card::king_of_clubs(),
            Card::king_of_hearts(),
            Card::king_of_diamonds(),
            Card::seven_of_clubs(),
            Card::seven_of_spades(),
        ]);
        let four_of_a_kind = HandRank::FourOfAKind([
            Card::six_of_spades(),
            Card::six_of_diamonds(),
            Card::six_of_hearts(),
            Card::six_of_clubs(),
        ]);

        let straight_flush = HandRank::StraightFlush([
            Card::two_of_spades(),
            Card::three_of_spades(),
            Card::four_of_spades(),
            Card::five_of_spades(),
            Card::six_of_spades(),
        ]);

        let royal_flush = HandRank::StraightFlush([
            Card::ten_of_hearts(),
            Card::jack_of_hearts(),
            Card::queen_of_hearts(),
            Card::king_of_hearts(),
            Card::ace_of_hearts(),
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
    fn high_card_is_ranked_correctly() {
        let two_of_spades = Card::two_of_spades();
        let four_of_hearts = Card::four_of_hearts();
        let seven_of_diamonds = Card::seven_of_diamonds();
        let ten_of_clubs = Card::ten_of_clubs();
        let king_of_clubs = Card::king_of_clubs();

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

    #[test]
    fn pair_is_ranked_correctly() {
        let two_of_clubs = Card::two_of_clubs();
        let five_of_spades = Card::five_of_spades();
        let seven_of_diamonds = Card::seven_of_diamonds();
        let king_of_clubs = Card::king_of_clubs();
        let king_of_hearts = Card::king_of_hearts();
        let ace_of_spades = Card::ace_of_spades();

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

        // Pick out the Pair over the High Card
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

    #[test]
    fn two_pair_is_ranked_correctly() {
        let five_of_clubs = Card::five_of_clubs();
        let five_of_spades = Card::five_of_spades();
        let seven_of_clubs = Card::seven_of_clubs();
        let seven_of_diamonds = Card::seven_of_diamonds();
        let king_of_clubs = Card::king_of_clubs();
        let king_of_hearts = Card::king_of_hearts();

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

        // Pick out the higher Two Pair of Ks & 7s over the lower Two Pair of 5s
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

    #[test]
    fn three_of_a_kind_is_ranked_correctly() {
        let five_of_spades = Card::five_of_spades();
        let seven_of_clubs = Card::seven_of_clubs();
        let seven_of_diamonds = Card::seven_of_diamonds();
        let seven_of_spades = Card::seven_of_spades();
        let king_of_clubs = Card::king_of_clubs();
        let king_of_diamonds = Card::king_of_diamonds();
        let king_of_hearts = Card::king_of_hearts();

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

        // Pick out the higher Three of a Kind of Ks over the lower Three of a Kind of 7s
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

    #[test]
    fn straight_is_ranked_correctly() {
        let two_of_diamonds = Card::two_of_diamonds();
        let three_of_clubs = Card::three_of_clubs();
        let four_of_hearts = Card::four_of_hearts();
        let five_of_diamonds = Card::five_of_diamonds();
        let six_of_clubs = Card::six_of_clubs();
        let seven_of_spades = Card::seven_of_spades();

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

        // Picks out the higher Straight of 3, 4, 5, 6, 7 over the lower Straight of 2, 3, 4, 5, 6
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

    #[test]
    fn ace_low_straight_is_ranked_correctly() {
        let two_of_clubs = Card::two_of_clubs();
        let three_of_hearts = Card::three_of_hearts();
        let four_of_spades = Card::four_of_spades();
        let five_of_hearts = Card::five_of_hearts();
        let seven_of_diamonds = Card::seven_of_diamonds();
        let ace_of_spades = Card::ace_of_spades();

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

        // Ignore the 7♦ & pick out the Ace-low Straight
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
    }

    #[test]
    fn ace_high_straight_is_ranked_correctly() {
        let nine_of_diamonds = Card::nine_of_diamonds();
        let ten_of_clubs = Card::ten_of_clubs();
        let jack_of_hearts = Card::jack_of_hearts();
        let queen_of_spades = Card::queen_of_spades();
        let king_of_hearts = Card::king_of_hearts();
        let ace_of_spades = Card::ace_of_spades();

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

        // Pick out the higher Straight of 10, J, Q, K, Ace over the lower Straight of 9, 10, J, Q, K
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

    #[test]
    fn flush_is_ranked_correctly() {
        let two_of_clubs = Card::two_of_clubs();
        let two_of_diamonds = Card::two_of_diamonds();
        let three_of_clubs = Card::three_of_clubs();
        let eight_of_clubs = Card::eight_of_clubs();
        let nine_of_clubs = Card::nine_of_clubs();
        let queen_of_clubs = Card::queen_of_clubs();
        let king_of_clubs = Card::king_of_clubs();

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

        // Pick out the higher Flush of 3♣, 8♣, 9♣, Q♣, K♣ over the lower Flush of 2♣, 3♣, 8♣, 9♣, Q♣
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

    #[test]
    fn full_house_is_ranked_correctly() {
        let three_of_clubs = Card::three_of_clubs();
        let three_of_spades = Card::three_of_spades();
        let seven_of_clubs = Card::seven_of_clubs();
        let seven_of_spades = Card::seven_of_spades();
        let king_of_clubs = Card::king_of_clubs();
        let king_of_diamonds = Card::king_of_diamonds();
        let king_of_hearts = Card::king_of_hearts();

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

        // Pick out the higher Full House consisting of Ks & 7s over the 3s
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

    #[test]
    fn four_of_a_kind_is_ranked_correctly() {
        let six_of_clubs = Card::six_of_clubs();
        let six_of_diamonds = Card::six_of_diamonds();
        let six_of_hearts = Card::six_of_hearts();
        let six_of_spades = Card::six_of_spades();
        let king_of_clubs = Card::king_of_clubs();
        let king_of_hearts = Card::king_of_hearts();
        let king_of_spades = Card::king_of_spades();

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

        // Pick out the Four of a Kind over the Three of a Kind
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

    #[test]
    fn straight_flush_is_ranked_correctly() {
        let two_of_spades = Card::two_of_spades();
        let three_of_spades = Card::three_of_spades();
        let four_of_spades = Card::four_of_spades();
        let five_of_spades = Card::five_of_spades();
        let six_of_spades = Card::six_of_spades();

        let cards: Vec<Card> = vec![
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
        ];

        let hand_rank = rank_hand(cards);
        let straight_flush = HandRank::StraightFlush([
            two_of_spades,
            three_of_spades,
            four_of_spades,
            five_of_spades,
            six_of_spades,
        ]);

        assert_eq!(hand_rank, straight_flush);
    }

    #[test]
    fn ace_low_straight_flush_is_ranked_correctly() {
        let ace_of_diamonds = Card::ace_of_diamonds();
        let two_of_diamonds = Card::two_of_diamonds();
        let three_of_diamonds = Card::three_of_diamonds();
        let four_of_diamonds = Card::four_of_diamonds();
        let five_of_diamonds = Card::five_of_diamonds();
        let seven_of_diamonds = Card::seven_of_diamonds();

        let cards: Vec<Card> = vec![
            ace_of_diamonds,
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
            seven_of_diamonds,
        ];

        let hand_rank = rank_hand(cards);
        let ace_low_straight_flush = HandRank::StraightFlush([
            ace_of_diamonds,
            two_of_diamonds,
            three_of_diamonds,
            four_of_diamonds,
            five_of_diamonds,
        ]);

        assert_eq!(hand_rank, ace_low_straight_flush);
    }

    #[test]
    /// AKA Royal Flush
    ///
    /// Also tests to ignore the lower straight of 9 - K
    fn ace_high_straight_flush_aka_royal_flush_is_ranked_correctly() {
        let nine_of_hearts = Card::nine_of_hearts();
        let ten_of_hearts = Card::ten_of_hearts();
        let jack_of_hearts = Card::jack_of_hearts();
        let queen_of_hearts = Card::queen_of_hearts();
        let king_of_hearts = Card::king_of_hearts();
        let ace_of_hearts = Card::ace_of_hearts();

        let cards: Vec<Card> = vec![
            nine_of_hearts,
            ten_of_hearts,
            jack_of_hearts,
            queen_of_hearts,
            king_of_hearts,
            ace_of_hearts,
        ];

        let hand_rank = rank_hand(cards);
        let ace_high_straight_flush = HandRank::StraightFlush([
            ten_of_hearts,
            jack_of_hearts,
            queen_of_hearts,
            king_of_hearts,
            ace_of_hearts,
        ]);

        assert_eq!(hand_rank, ace_high_straight_flush);
    }
}
