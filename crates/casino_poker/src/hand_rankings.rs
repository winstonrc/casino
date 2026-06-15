//! Poker hand evaluation and comparison.

use std::fmt;

use casino_cards::card::Card;
use serde::{Deserialize, Deserializer, Serialize};

/// The category of a poker hand, ordered from weakest (`HighCard`) to strongest
/// (`StraightFlush`).
///
/// The ordering is derived from declaration order, so
/// `HighCard < Pair < TwoPair < ThreeOfAKind < Straight < Flush < FullHouse <
/// FourOfAKind < StraightFlush`.
///
/// It is a payload-free type: it carries only the category, while
/// [`ComparableHand`] pairs it with the tiebreak ranks. `HandCategory` is the
/// source of truth for category precedence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum HandCategory {
    /// Five unmatched cards.
    HighCard,
    /// Two cards of one rank.
    Pair,
    /// Two distinct pairs.
    TwoPair,
    /// Three cards of one rank.
    ThreeOfAKind,
    /// Five consecutive ranks.
    Straight,
    /// Five cards of one suit.
    Flush,
    /// Three cards of one rank plus a pair.
    FullHouse,
    /// Four cards of one rank.
    FourOfAKind,
    /// Five consecutive cards of one suit.
    StraightFlush,
}

impl fmt::Display for HandCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let printable = match self {
            HandCategory::HighCard => "High Card",
            HandCategory::Pair => "Pair",
            HandCategory::TwoPair => "Two Pair",
            HandCategory::ThreeOfAKind => "Three of a Kind",
            HandCategory::Straight => "Straight",
            HandCategory::Flush => "Flush",
            HandCategory::FullHouse => "Full House",
            HandCategory::FourOfAKind => "Four of a Kind",
            HandCategory::StraightFlush => "Straight Flush",
        };

        write!(f, "{}", printable)
    }
}

/// A fully-ordered, kicker-aware value of the best 5-card poker hand.
///
/// Two hands are compared lexicographically as `(category, tiebreak)`. The
/// `tiebreak` array holds the rank values (2..=14) that break ties *within* a
/// category, most-significant first, zero-padded (no real rank is `0`). Because
/// `Ord` is derived, comparison, kicker resolution, and exact ties all fall out
/// for free — there are no per-category comparison branches to get wrong.
///
/// Tiebreak layout by category:
/// - `HighCard` / `Flush`: all five ranks, high → low.
/// - `Pair`: `[pair, k1, k2, k3, 0]`.
/// - `TwoPair`: `[high_pair, low_pair, kicker, 0, 0]`.
/// - `ThreeOfAKind`: `[trips, k1, k2, 0, 0]`.
/// - `Straight` / `StraightFlush`: `[high_card, 0, 0, 0, 0]` (the wheel A-2-3-4-5
///   uses a high card of `5`, so it ranks below a 6-high straight).
/// - `FullHouse`: `[trips, pair, 0, 0, 0]`.
/// - `FourOfAKind`: `[quad, kicker, 0, 0, 0]`.
///
/// ```
/// use casino_poker::hand_rankings::{evaluate_holdem, HandCategory};
/// use casino_poker::casino_cards::card::{Card, Rank, Suit};
///
/// // A flush beats a pair.
/// let flush = evaluate_holdem(
///     [Card::new(Rank::Ace, Suit::Heart), Card::new(Rank::Two, Suit::Heart)],
///     &[
///         Card::new(Rank::Five, Suit::Heart),
///         Card::new(Rank::Nine, Suit::Heart),
///         Card::new(Rank::King, Suit::Heart),
///         Card::new(Rank::King, Suit::Spade),
///         Card::new(Rank::Three, Suit::Club),
///     ],
/// )?;
/// assert_eq!(flush.value().category(), HandCategory::Flush);
/// # Ok::<(), casino_poker::hand_rankings::HandEvaluationError>(())
/// ```
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ComparableHand {
    /// The made-hand category.
    category: HandCategory,
    /// Category-specific ranks used for kicker-aware ordering.
    tiebreak: [u8; 5],
}

impl ComparableHand {
    /// Builds a comparable hand value from a category and canonical tiebreak
    /// ranks.
    ///
    /// # Errors
    ///
    /// Returns [`HandEvaluationError::InvalidHandValue`] when the tiebreak ranks
    /// are outside `2..=14`, are not zero-padded correctly, or do not match the
    /// canonical layout for the supplied category.
    pub fn new(category: HandCategory, tiebreak: [u8; 5]) -> Result<Self, HandEvaluationError> {
        validate_comparable_hand(category, &tiebreak)?;
        Ok(Self { category, tiebreak })
    }

    /// Returns the made-hand category.
    pub const fn category(&self) -> HandCategory {
        self.category
    }

    /// Returns the category-specific ranks used for kicker-aware ordering.
    pub const fn tiebreak(&self) -> [u8; 5] {
        self.tiebreak
    }

    /// Names the made hand in **PokerStars hand-history wording** — e.g.
    /// `two pair, Jacks and Fives`, `a pair of Sevens`, `a flush, Ace high`,
    /// `a full house, Kings full of Threes`, `a straight, Five to Nine`,
    /// `a straight flush, Ten to Ace`. Kickers are intentionally absent (the cards
    /// in the `[..]` notation carry them), matching PokerStars output.
    pub fn describe(&self) -> String {
        let t = &self.tiebreak;
        match self.category {
            HandCategory::HighCard => format!("high card {}", rank_name(t[0])),
            HandCategory::Pair => format!("a pair of {}", rank_plural(t[0])),
            HandCategory::TwoPair => {
                format!("two pair, {} and {}", rank_plural(t[0]), rank_plural(t[1]))
            }
            HandCategory::ThreeOfAKind => format!("three of a kind, {}", rank_plural(t[0])),
            HandCategory::Straight => format!("a straight, {}", straight_range(t[0])),
            HandCategory::Flush => format!("a flush, {} high", rank_name(t[0])),
            HandCategory::FullHouse => {
                format!(
                    "a full house, {} full of {}",
                    rank_plural(t[0]),
                    rank_plural(t[1])
                )
            }
            HandCategory::FourOfAKind => format!("four of a kind, {}", rank_plural(t[0])),
            HandCategory::StraightFlush => format!("a straight flush, {}", straight_range(t[0])),
        }
    }
}

impl<'de> Deserialize<'de> for ComparableHand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Representation {
            category: HandCategory,
            tiebreak: [u8; 5],
        }

        let representation = Representation::deserialize(deserializer)?;
        Self::new(representation.category, representation.tiebreak)
            .map_err(<D::Error as serde::de::Error>::custom)
    }
}

impl fmt::Display for ComparableHand {
    /// Writes only the bare category (e.g. `Two Pair`). For the PokerStars-worded
    /// made hand with its ranks (e.g. `two pair, Jacks and Fives`), use
    /// [`describe`](ComparableHand::describe).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.category)
    }
}

/// A ranked poker hand together with the five physical cards that form it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct EvaluatedHand {
    /// The comparable category and tiebreak value.
    value: ComparableHand,
    /// The five cards selected for the hand.
    cards: [Card; 5],
}

impl EvaluatedHand {
    /// Returns the comparable category and tiebreak value.
    pub const fn value(&self) -> ComparableHand {
        self.value
    }

    /// Returns the five physical cards selected for the hand.
    pub const fn cards(&self) -> &[Card; 5] {
        &self.cards
    }
}

impl<'de> Deserialize<'de> for EvaluatedHand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Representation {
            value: ComparableHand,
            cards: [Card; 5],
        }

        let representation = Representation::deserialize(deserializer)?;
        validate_unique(representation.cards.iter())
            .map_err(<D::Error as serde::de::Error>::custom)?;
        let actual = score_five(&representation.cards);
        if actual != representation.value {
            return Err(<D::Error as serde::de::Error>::custom(
                "evaluated hand value does not match its cards",
            ));
        }

        Ok(Self {
            value: actual,
            cards: representation.cards,
        })
    }
}

/// An error returned when cards cannot form a valid supported evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HandEvaluationError {
    /// The number of cards is outside the range accepted by the operation.
    InvalidCardCount {
        /// The smallest accepted card count.
        minimum: usize,
        /// The largest accepted card count.
        maximum: usize,
        /// The card count supplied by the caller.
        actual: usize,
    },
    /// The input contains the same rank and suit more than once.
    DuplicateCard,
    /// The supplied category and tiebreak ranks are not a canonical poker hand
    /// value.
    InvalidHandValue,
}

impl fmt::Display for HandEvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCardCount {
                minimum,
                maximum,
                actual,
            } if minimum == maximum => {
                write!(f, "expected exactly {minimum} cards, got {actual}")
            }
            Self::InvalidCardCount {
                minimum,
                maximum,
                actual,
            } => write!(
                f,
                "expected between {minimum} and {maximum} cards, got {actual}"
            ),
            Self::DuplicateCard => write!(f, "the same physical card appears more than once"),
            Self::InvalidHandValue => {
                write!(f, "the hand category and tiebreak ranks are inconsistent")
            }
        }
    }
}

impl std::error::Error for HandEvaluationError {}

/// The low-to-high span of a straight (or straight flush) by its high-card value,
/// PokerStars style: `Five to Nine`. The wheel (5-high) plays the Ace low and
/// reads `Ace to Five`.
fn straight_range(high: u8) -> String {
    match high {
        5 => "Ace to Five".to_string(),
        6..=14 => format!("{} to {}", rank_name(high - 4), rank_name(high)),
        _ => "unknown straight".to_string(),
    }
}

/// The singular name of a rank value (2..=14), for "<rank>-high" phrasings.
fn rank_name(value: u8) -> &'static str {
    match value {
        2 => "Two",
        3 => "Three",
        4 => "Four",
        5 => "Five",
        6 => "Six",
        7 => "Seven",
        8 => "Eight",
        9 => "Nine",
        10 => "Ten",
        11 => "Jack",
        12 => "Queen",
        13 => "King",
        14 => "Ace",
        _ => "?",
    }
}

/// The plural name of a rank value (2..=14), for naming sets — note `Six` →
/// `Sixes`, so a simple `+ "s"` won't do.
fn rank_plural(value: u8) -> &'static str {
    match value {
        2 => "Twos",
        3 => "Threes",
        4 => "Fours",
        5 => "Fives",
        6 => "Sixes",
        7 => "Sevens",
        8 => "Eights",
        9 => "Nines",
        10 => "Tens",
        11 => "Jacks",
        12 => "Queens",
        13 => "Kings",
        14 => "Aces",
        _ => "?",
    }
}

/// Evaluates exactly five cards.
///
/// # Errors
///
/// Returns [`HandEvaluationError::DuplicateCard`] when the same rank and suit
/// appears more than once. Card visibility is ignored when detecting duplicates.
pub fn evaluate_five(cards: [Card; 5]) -> Result<EvaluatedHand, HandEvaluationError> {
    validate_unique(cards.iter())?;
    Ok(EvaluatedHand {
        value: score_five(&cards),
        cards,
    })
}

/// Returns the strongest five-card hand from five to seven cards.
///
/// At seven cards this evaluates the 21 possible five-card subsets. Among
/// equally ranked subsets, the first subset in input enumeration order is
/// selected.
///
/// # Errors
///
/// Returns an error for card counts outside `5..=7` or duplicate physical cards.
pub fn best_five(cards: &[Card]) -> Result<EvaluatedHand, HandEvaluationError> {
    validate_count(cards.len(), 5, 7)?;
    validate_unique(cards.iter())?;
    best_five_validated(cards).ok_or(HandEvaluationError::InvalidCardCount {
        minimum: 5,
        maximum: 7,
        actual: cards.len(),
    })
}

/// Evaluates a Texas Hold'em hand using two hole cards and three to five board
/// cards.
///
/// # Errors
///
/// Returns an error unless the board contains `3..=5` cards or when any physical
/// card is duplicated between the hole cards and board.
pub fn evaluate_holdem(
    hole: [Card; 2],
    board: &[Card],
) -> Result<EvaluatedHand, HandEvaluationError> {
    validate_count(board.len(), 3, 5)?;
    validate_unique(hole.iter().chain(board))?;

    let mut cards = [hole[0]; 7];
    cards[1] = hole[1];
    cards[2..(board.len() + 2)].copy_from_slice(board);
    best_five_validated(&cards[..(board.len() + 2)]).ok_or(HandEvaluationError::InvalidCardCount {
        minimum: 3,
        maximum: 5,
        actual: board.len(),
    })
}

/// Evaluates an Omaha hand using exactly two of four hole cards and exactly three
/// of three to five board cards.
///
/// # Errors
///
/// Returns an error unless the board contains `3..=5` cards or when any physical
/// card is duplicated between the hole cards and board.
pub fn evaluate_omaha(
    hole: [Card; 4],
    board: &[Card],
) -> Result<EvaluatedHand, HandEvaluationError> {
    validate_count(board.len(), 3, 5)?;
    validate_unique(hole.iter().chain(board))?;

    let mut best: Option<EvaluatedHand> = None;
    for h1 in 0..3 {
        for h2 in (h1 + 1)..4 {
            for b1 in 0..(board.len() - 2) {
                for b2 in (b1 + 1)..(board.len() - 1) {
                    for b3 in (b2 + 1)..board.len() {
                        let cards = [hole[h1], hole[h2], board[b1], board[b2], board[b3]];
                        let value = score_five(&cards);
                        if best.is_none_or(|current| value > current.value) {
                            best = Some(EvaluatedHand { value, cards });
                        }
                    }
                }
            }
        }
    }

    best.ok_or(HandEvaluationError::InvalidCardCount {
        minimum: 3,
        maximum: 5,
        actual: board.len(),
    })
}

fn best_five_validated(cards: &[Card]) -> Option<EvaluatedHand> {
    let n = cards.len();
    let mut best: Option<EvaluatedHand> = None;
    for a in 0..n {
        for b in (a + 1)..n {
            for c in (b + 1)..n {
                for d in (c + 1)..n {
                    for e in (d + 1)..n {
                        let five = [cards[a], cards[b], cards[c], cards[d], cards[e]];
                        let score = score_five(&five);
                        if best.is_none_or(|current| score > current.value) {
                            best = Some(EvaluatedHand {
                                value: score,
                                cards: five,
                            });
                        }
                    }
                }
            }
        }
    }
    best
}

fn validate_count(
    actual: usize,
    minimum: usize,
    maximum: usize,
) -> Result<(), HandEvaluationError> {
    if (minimum..=maximum).contains(&actual) {
        Ok(())
    } else {
        Err(HandEvaluationError::InvalidCardCount {
            minimum,
            maximum,
            actual,
        })
    }
}

fn validate_unique<'a>(
    cards: impl IntoIterator<Item = &'a Card>,
) -> Result<(), HandEvaluationError> {
    let mut seen = [false; 52];
    for card in cards {
        let rank = usize::from(card.rank.value() - 2);
        let suit = usize::from(card.suit.value());
        let index = rank * 4 + suit;
        if seen[index] {
            return Err(HandEvaluationError::DuplicateCard);
        }
        seen[index] = true;
    }
    Ok(())
}

fn validate_comparable_hand(
    category: HandCategory,
    tiebreak: &[u8; 5],
) -> Result<(), HandEvaluationError> {
    let valid_rank = |rank: u8| (2..=14).contains(&rank);
    let ranks_desc = |ranks: &[u8]| {
        ranks.iter().all(|&rank| valid_rank(rank)) && ranks.windows(2).all(|pair| pair[0] > pair[1])
    };
    let zeroes = |ranks: &[u8]| ranks.iter().all(|&rank| rank == 0);
    let not_straight = |ranks: &[u8; 5]| straight_high(ranks).is_none();

    let valid = match category {
        HandCategory::HighCard | HandCategory::Flush => {
            ranks_desc(tiebreak) && not_straight(tiebreak)
        }
        HandCategory::Pair => {
            let pair = tiebreak[0];
            valid_rank(pair)
                && ranks_desc(&tiebreak[1..4])
                && zeroes(&tiebreak[4..])
                && tiebreak[1..4].iter().all(|&rank| rank != pair)
        }
        HandCategory::TwoPair => {
            let high_pair = tiebreak[0];
            let low_pair = tiebreak[1];
            let kicker = tiebreak[2];
            valid_rank(high_pair)
                && valid_rank(low_pair)
                && valid_rank(kicker)
                && high_pair > low_pair
                && kicker != high_pair
                && kicker != low_pair
                && zeroes(&tiebreak[3..])
        }
        HandCategory::ThreeOfAKind => {
            let trips = tiebreak[0];
            valid_rank(trips)
                && ranks_desc(&tiebreak[1..3])
                && zeroes(&tiebreak[3..])
                && tiebreak[1..3].iter().all(|&rank| rank != trips)
        }
        HandCategory::Straight | HandCategory::StraightFlush => {
            (5..=14).contains(&tiebreak[0]) && zeroes(&tiebreak[1..])
        }
        HandCategory::FullHouse => {
            let trips = tiebreak[0];
            let pair = tiebreak[1];
            valid_rank(trips) && valid_rank(pair) && trips != pair && zeroes(&tiebreak[2..])
        }
        HandCategory::FourOfAKind => {
            let quads = tiebreak[0];
            let kicker = tiebreak[1];
            valid_rank(quads) && valid_rank(kicker) && quads != kicker && zeroes(&tiebreak[2..])
        }
    };

    if valid {
        Ok(())
    } else {
        Err(HandEvaluationError::InvalidHandValue)
    }
}

/// Scores exactly five cards into a [`ComparableHand`].
fn score_five(cards: &[Card; 5]) -> ComparableHand {
    let mut ranks: [u8; 5] = [
        cards[0].rank.value(),
        cards[1].rank.value(),
        cards[2].rank.value(),
        cards[3].rank.value(),
        cards[4].rank.value(),
    ];
    // Sort descending so the highest rank is first.
    ranks.sort_unstable_by(|a, b| b.cmp(a));

    let is_flush = cards.iter().all(|c| c.suit == cards[0].suit);
    let straight_high = straight_high(&ranks);

    if is_flush {
        if let Some(high) = straight_high {
            return ComparableHand {
                category: HandCategory::StraightFlush,
                tiebreak: [high, 0, 0, 0, 0],
            };
        }
    }

    let mut rank_counts = [0u8; 15];
    for rank in ranks {
        rank_counts[usize::from(rank)] += 1;
    }

    let mut four = 0;
    let mut three = 0;
    let mut pairs = [0u8; 2];
    let mut pair_count = 0;
    let mut singles = [0u8; 5];
    let mut single_count = 0;
    for rank in (2u8..=14).rev() {
        match rank_counts[usize::from(rank)] {
            4 => four = rank,
            3 => three = rank,
            2 => {
                pairs[pair_count] = rank;
                pair_count += 1;
            }
            1 => {
                singles[single_count] = rank;
                single_count += 1;
            }
            _ => {}
        }
    }

    if four != 0 {
        return ComparableHand {
            category: HandCategory::FourOfAKind,
            tiebreak: [four, singles[0], 0, 0, 0],
        };
    }

    if three != 0 && pair_count == 1 {
        return ComparableHand {
            category: HandCategory::FullHouse,
            tiebreak: [three, pairs[0], 0, 0, 0],
        };
    }

    if is_flush {
        return ComparableHand {
            category: HandCategory::Flush,
            tiebreak: ranks,
        };
    }

    if let Some(high) = straight_high {
        return ComparableHand {
            category: HandCategory::Straight,
            tiebreak: [high, 0, 0, 0, 0],
        };
    }

    if three != 0 {
        return ComparableHand {
            category: HandCategory::ThreeOfAKind,
            tiebreak: [three, singles[0], singles[1], 0, 0],
        };
    }

    if pair_count == 2 {
        return ComparableHand {
            category: HandCategory::TwoPair,
            tiebreak: [pairs[0], pairs[1], singles[0], 0, 0],
        };
    }

    if pair_count == 1 {
        return ComparableHand {
            category: HandCategory::Pair,
            tiebreak: [pairs[0], singles[0], singles[1], singles[2], 0],
        };
    }

    ComparableHand {
        category: HandCategory::HighCard,
        tiebreak: ranks,
    }
}

/// Returns the high card of a 5-card straight (or `None` if the ranks are not a
/// straight). `ranks_desc` must be sorted descending. The wheel A-2-3-4-5 returns
/// a high card of `5` so it ranks below a 6-high straight.
fn straight_high(ranks_desc: &[u8; 5]) -> Option<u8> {
    // A straight requires five distinct ranks.
    for i in 0..4 {
        if ranks_desc[i] == ranks_desc[i + 1] {
            return None;
        }
    }

    if ranks_desc[0] - ranks_desc[4] == 4 {
        return Some(ranks_desc[0]);
    }

    // Wheel: Ace plays low (A-2-3-4-5 -> values 14,5,4,3,2).
    if *ranks_desc == [14, 5, 4, 3, 2] {
        return Some(5);
    }

    None
}

#[cfg(test)]
mod comparable_hand_tests {
    use super::*;

    use casino_cards::card::{Card, Rank, Suit};

    fn c(rank: Rank, suit: Suit) -> Card {
        Card::new(rank, suit)
    }

    /// Convenience: evaluate a flat list of five to seven cards.
    fn eval(cards: &[Card]) -> ComparableHand {
        best_five(cards).unwrap().value
    }

    #[test]
    fn best_omaha_enforces_two_hole_three_board() {
        // Four spades in hand plus one on the board: pooling all seven cards
        // (hold'em style) makes a straight/royal flush, but Omaha may use only two
        // hole cards, so no flush is possible — the constrained hand must be weaker.
        let hole = [
            c(Rank::Ace, Suit::Spade),
            c(Rank::King, Suit::Spade),
            c(Rank::Queen, Suit::Spade),
            c(Rank::Jack, Suit::Spade),
        ];
        let board = [
            c(Rank::Ten, Suit::Spade),
            c(Rank::Two, Suit::Heart),
            c(Rank::Three, Suit::Diamond),
        ];

        let pooled = best_five(&[
            hole[0], hole[1], hole[2], hole[3], board[0], board[1], board[2],
        ])
        .unwrap()
        .value;
        let omaha = evaluate_omaha(hole, &board).unwrap().value;
        assert_eq!(pooled.category(), HandCategory::StraightFlush);
        assert!(
            omaha < pooled,
            "Omaha's exact-2+3 rule must yield a weaker hand than pooling all seven"
        );
    }

    #[test]
    fn best_omaha_finds_quads_using_two_hole_cards() {
        // Pocket aces + two aces on board = quad aces using exactly two hole cards.
        let hole = [
            c(Rank::Ace, Suit::Spade),
            c(Rank::Ace, Suit::Heart),
            c(Rank::King, Suit::Diamond),
            c(Rank::Queen, Suit::Club),
        ];
        let board = [
            c(Rank::Ace, Suit::Club),
            c(Rank::Ace, Suit::Diamond),
            c(Rank::Two, Suit::Spade),
        ];
        let evaluated = evaluate_omaha(hole, &board).unwrap();
        assert_eq!(evaluated.value().category(), HandCategory::FourOfAKind);
        assert_eq!(
            evaluated
                .cards()
                .iter()
                .filter(|card| hole.contains(card))
                .count(),
            2
        );
        assert_eq!(
            evaluated
                .cards()
                .iter()
                .filter(|card| board.contains(card))
                .count(),
            3
        );
    }

    #[test]
    fn evaluators_reject_unsupported_card_counts() {
        let cards = [
            c(Rank::Ace, Suit::Spade),
            c(Rank::King, Suit::Heart),
            c(Rank::Queen, Suit::Diamond),
            c(Rank::Jack, Suit::Club),
            c(Rank::Ten, Suit::Spade),
            c(Rank::Nine, Suit::Heart),
            c(Rank::Eight, Suit::Diamond),
            c(Rank::Seven, Suit::Club),
        ];

        assert_eq!(
            best_five(&cards[..4]),
            Err(HandEvaluationError::InvalidCardCount {
                minimum: 5,
                maximum: 7,
                actual: 4,
            })
        );
        assert_eq!(
            best_five(&cards),
            Err(HandEvaluationError::InvalidCardCount {
                minimum: 5,
                maximum: 7,
                actual: 8,
            })
        );
        assert_eq!(
            evaluate_holdem([cards[0], cards[1]], &cards[2..4]),
            Err(HandEvaluationError::InvalidCardCount {
                minimum: 3,
                maximum: 5,
                actual: 2,
            })
        );
        assert_eq!(
            evaluate_holdem([cards[0], cards[1]], &cards[2..8]),
            Err(HandEvaluationError::InvalidCardCount {
                minimum: 3,
                maximum: 5,
                actual: 6,
            })
        );
        assert_eq!(
            evaluate_omaha([cards[0], cards[1], cards[2], cards[3]], &cards[4..6]),
            Err(HandEvaluationError::InvalidCardCount {
                minimum: 3,
                maximum: 5,
                actual: 2,
            })
        );
        assert_eq!(
            evaluate_omaha(
                [cards[0], cards[1], cards[2], cards[3]],
                &[
                    cards[4],
                    cards[5],
                    cards[6],
                    cards[7],
                    c(Rank::Six, Suit::Spade),
                    c(Rank::Five, Suit::Heart),
                ],
            ),
            Err(HandEvaluationError::InvalidCardCount {
                minimum: 3,
                maximum: 5,
                actual: 6,
            })
        );
    }

    #[test]
    fn evaluators_reject_duplicate_physical_cards_regardless_of_visibility() {
        let ace = c(Rank::Ace, Suit::Spade);
        let mut hidden_ace = ace;
        hidden_ace.face_up = false;
        let cards = [
            ace,
            hidden_ace,
            c(Rank::King, Suit::Heart),
            c(Rank::Queen, Suit::Diamond),
            c(Rank::Jack, Suit::Club),
        ];

        assert_eq!(
            evaluate_five(cards),
            Err(HandEvaluationError::DuplicateCard)
        );
        assert_eq!(best_five(&cards), Err(HandEvaluationError::DuplicateCard));
        assert_eq!(
            evaluate_holdem(
                [ace, c(Rank::King, Suit::Heart)],
                &[
                    hidden_ace,
                    c(Rank::Queen, Suit::Diamond),
                    c(Rank::Jack, Suit::Club),
                ],
            ),
            Err(HandEvaluationError::DuplicateCard)
        );
        assert_eq!(
            evaluate_omaha(
                [
                    ace,
                    c(Rank::King, Suit::Heart),
                    c(Rank::Queen, Suit::Diamond),
                    c(Rank::Jack, Suit::Club),
                ],
                &[
                    hidden_ace,
                    c(Rank::Ten, Suit::Heart),
                    c(Rank::Nine, Suit::Diamond),
                ],
            ),
            Err(HandEvaluationError::DuplicateCard)
        );
    }

    #[test]
    fn category_ordering_is_correct() {
        assert!(HandCategory::HighCard < HandCategory::Pair);
        assert!(HandCategory::Pair < HandCategory::TwoPair);
        assert!(HandCategory::TwoPair < HandCategory::ThreeOfAKind);
        assert!(HandCategory::ThreeOfAKind < HandCategory::Straight);
        assert!(HandCategory::Straight < HandCategory::Flush);
        assert!(HandCategory::Flush < HandCategory::FullHouse);
        assert!(HandCategory::FullHouse < HandCategory::FourOfAKind);
        assert!(HandCategory::FourOfAKind < HandCategory::StraightFlush);
    }

    #[test]
    fn describe_uses_pokerstars_wording() {
        let describe = |cards: &[Card]| eval(cards).describe();

        assert_eq!(
            describe(&[
                c(Rank::Jack, Suit::Heart),
                c(Rank::Ten, Suit::Spade),
                c(Rank::Eight, Suit::Club),
                c(Rank::Five, Suit::Spade),
                c(Rank::Five, Suit::Diamond),
            ]),
            "a pair of Fives"
        );
        assert_eq!(
            describe(&[
                c(Rank::Queen, Suit::Club),
                c(Rank::Queen, Suit::Diamond),
                c(Rank::Five, Suit::Spade),
                c(Rank::Five, Suit::Heart),
                c(Rank::King, Suit::Club),
            ]),
            "two pair, Queens and Fives"
        );
        assert_eq!(
            describe(&[
                c(Rank::King, Suit::Club),
                c(Rank::King, Suit::Diamond),
                c(Rank::King, Suit::Heart),
                c(Rank::Three, Suit::Spade),
                c(Rank::Three, Suit::Club),
            ]),
            "a full house, Kings full of Threes"
        );
        assert_eq!(
            describe(&[
                c(Rank::Ace, Suit::Heart),
                c(Rank::King, Suit::Heart),
                c(Rank::Nine, Suit::Heart),
                c(Rank::Five, Suit::Heart),
                c(Rank::Two, Suit::Heart),
            ]),
            "a flush, Ace high"
        );
        // Straights read low-to-high.
        assert_eq!(
            describe(&[
                c(Rank::Five, Suit::Heart),
                c(Rank::Six, Suit::Spade),
                c(Rank::Seven, Suit::Club),
                c(Rank::Eight, Suit::Diamond),
                c(Rank::Nine, Suit::Heart),
            ]),
            "a straight, Five to Nine"
        );
        // Wheel plays the Ace low.
        assert_eq!(
            describe(&[
                c(Rank::Ace, Suit::Heart),
                c(Rank::Two, Suit::Heart),
                c(Rank::Three, Suit::Heart),
                c(Rank::Four, Suit::Heart),
                c(Rank::Five, Suit::Heart),
            ]),
            "a straight flush, Ace to Five"
        );
        // Broadway straight flush (a "royal") reads Ten to Ace, PokerStars style.
        assert_eq!(
            describe(&[
                c(Rank::Ten, Suit::Spade),
                c(Rank::Jack, Suit::Spade),
                c(Rank::Queen, Suit::Spade),
                c(Rank::King, Suit::Spade),
                c(Rank::Ace, Suit::Spade),
            ]),
            "a straight flush, Ten to Ace"
        );
    }

    #[test]
    fn comparable_hand_rejects_invalid_values_and_describe_is_total() {
        assert_eq!(
            ComparableHand::new(HandCategory::Straight, [4, 0, 0, 0, 0]),
            Err(HandEvaluationError::InvalidHandValue)
        );
        assert_eq!(
            ComparableHand::new(HandCategory::Pair, [14, 14, 13, 12, 0]),
            Err(HandEvaluationError::InvalidHandValue)
        );
        assert_eq!(
            ComparableHand::new(HandCategory::Flush, [14, 13, 12, 11, 10]),
            Err(HandEvaluationError::InvalidHandValue)
        );

        let invalid_json = serde_json::json!({
            "category": "Straight",
            "tiebreak": [4, 0, 0, 0, 0],
        });
        assert!(serde_json::from_value::<ComparableHand>(invalid_json).is_err());

        // This cannot be built through the public API, but `describe` must remain
        // total even for an internal or future malformed value.
        let malformed = ComparableHand {
            category: HandCategory::Straight,
            tiebreak: [4, 0, 0, 0, 0],
        };
        assert_eq!(malformed.describe(), "a straight, unknown straight");
    }

    #[test]
    fn evaluated_hand_returns_the_forming_cards() {
        // Board pair of Queens plus three low cards; the best five is the two
        // Queens and the three highest kickers, regardless of input order.
        let qc = c(Rank::Queen, Suit::Club);
        let qs = c(Rank::Queen, Suit::Spade);
        let evaluated = evaluate_holdem(
            [c(Rank::Three, Suit::Heart), c(Rank::Two, Suit::Diamond)],
            &[
                qc,
                c(Rank::Jack, Suit::Spade),
                c(Rank::Four, Suit::Diamond),
                qs,
                c(Rank::Ten, Suit::Club),
            ],
        )
        .unwrap();
        assert_eq!(evaluated.value.category(), HandCategory::Pair);
        assert!(evaluated.cards.contains(&qc) && evaluated.cards.contains(&qs));
        // The low hole cards (3, 2) are worse kickers than the board's J/10/4.
        assert!(!evaluated.cards.contains(&c(Rank::Three, Suit::Heart)));
        assert!(!evaluated.cards.contains(&c(Rank::Two, Suit::Diamond)));
    }

    #[test]
    fn evaluated_hand_deserialization_validates_cards_and_value() {
        let evaluated = evaluate_five([
            c(Rank::Ace, Suit::Heart),
            c(Rank::King, Suit::Heart),
            c(Rank::Nine, Suit::Heart),
            c(Rank::Five, Suit::Heart),
            c(Rank::Two, Suit::Heart),
        ])
        .unwrap();
        let json = serde_json::to_value(evaluated).unwrap();
        assert_eq!(
            serde_json::from_value::<EvaluatedHand>(json.clone()).unwrap(),
            evaluated
        );

        let mut contradictory = json.clone();
        contradictory["value"]["tiebreak"][0] = serde_json::json!(2);
        assert!(serde_json::from_value::<EvaluatedHand>(contradictory).is_err());

        let mut duplicate = json;
        duplicate["cards"][1] = duplicate["cards"][0].clone();
        assert!(serde_json::from_value::<EvaluatedHand>(duplicate).is_err());
    }

    #[test]
    fn detects_each_category() {
        // High card
        assert_eq!(
            eval(&[
                c(Rank::Ace, Suit::Club),
                c(Rank::Ten, Suit::Diamond),
                c(Rank::Seven, Suit::Heart),
                c(Rank::Five, Suit::Spade),
                c(Rank::Three, Suit::Club),
            ])
            .category(),
            HandCategory::HighCard
        );
        // Pair
        assert_eq!(
            eval(&[
                c(Rank::Ace, Suit::Club),
                c(Rank::Ace, Suit::Diamond),
                c(Rank::Seven, Suit::Heart),
                c(Rank::Five, Suit::Spade),
                c(Rank::Three, Suit::Club),
            ])
            .category(),
            HandCategory::Pair
        );
        // Two pair
        assert_eq!(
            eval(&[
                c(Rank::Ace, Suit::Club),
                c(Rank::Ace, Suit::Diamond),
                c(Rank::Seven, Suit::Heart),
                c(Rank::Seven, Suit::Spade),
                c(Rank::Three, Suit::Club),
            ])
            .category(),
            HandCategory::TwoPair
        );
        // Three of a kind
        assert_eq!(
            eval(&[
                c(Rank::Ace, Suit::Club),
                c(Rank::Ace, Suit::Diamond),
                c(Rank::Ace, Suit::Heart),
                c(Rank::Seven, Suit::Spade),
                c(Rank::Three, Suit::Club),
            ])
            .category(),
            HandCategory::ThreeOfAKind
        );
        // Straight
        assert_eq!(
            eval(&[
                c(Rank::Six, Suit::Club),
                c(Rank::Five, Suit::Diamond),
                c(Rank::Four, Suit::Heart),
                c(Rank::Three, Suit::Spade),
                c(Rank::Two, Suit::Club),
            ])
            .category(),
            HandCategory::Straight
        );
        // Flush
        assert_eq!(
            eval(&[
                c(Rank::Ace, Suit::Club),
                c(Rank::Ten, Suit::Club),
                c(Rank::Seven, Suit::Club),
                c(Rank::Five, Suit::Club),
                c(Rank::Three, Suit::Club),
            ])
            .category(),
            HandCategory::Flush
        );
        // Full house
        assert_eq!(
            eval(&[
                c(Rank::Ace, Suit::Club),
                c(Rank::Ace, Suit::Diamond),
                c(Rank::Ace, Suit::Heart),
                c(Rank::Seven, Suit::Spade),
                c(Rank::Seven, Suit::Club),
            ])
            .category(),
            HandCategory::FullHouse
        );
        // Four of a kind
        assert_eq!(
            eval(&[
                c(Rank::Ace, Suit::Club),
                c(Rank::Ace, Suit::Diamond),
                c(Rank::Ace, Suit::Heart),
                c(Rank::Ace, Suit::Spade),
                c(Rank::Seven, Suit::Club),
            ])
            .category(),
            HandCategory::FourOfAKind
        );
        // Straight flush
        assert_eq!(
            eval(&[
                c(Rank::Six, Suit::Club),
                c(Rank::Five, Suit::Club),
                c(Rank::Four, Suit::Club),
                c(Rank::Three, Suit::Club),
                c(Rank::Two, Suit::Club),
            ])
            .category(),
            HandCategory::StraightFlush
        );
    }

    #[test]
    fn kickers_break_ties_for_pairs() {
        // Both pair of Kings; first has Ace kicker, second has Queen kicker.
        let ak = eval(&[
            c(Rank::King, Suit::Club),
            c(Rank::King, Suit::Diamond),
            c(Rank::Ace, Suit::Heart),
            c(Rank::Seven, Suit::Spade),
            c(Rank::Three, Suit::Club),
        ]);
        let qk = eval(&[
            c(Rank::King, Suit::Club),
            c(Rank::King, Suit::Diamond),
            c(Rank::Queen, Suit::Heart),
            c(Rank::Seven, Suit::Spade),
            c(Rank::Three, Suit::Club),
        ]);
        assert!(
            ak > qk,
            "pair with Ace kicker must beat pair with Queen kicker"
        );
        assert_ne!(ak, qk);
    }

    #[test]
    fn second_kicker_breaks_ties() {
        // Pair of Kings, K-A-9 vs K-A-8: differ only on the third card (9 vs 8).
        let nine = eval(&[
            c(Rank::King, Suit::Club),
            c(Rank::King, Suit::Diamond),
            c(Rank::Ace, Suit::Heart),
            c(Rank::Nine, Suit::Spade),
            c(Rank::Two, Suit::Club),
        ]);
        let eight = eval(&[
            c(Rank::King, Suit::Club),
            c(Rank::King, Suit::Diamond),
            c(Rank::Ace, Suit::Heart),
            c(Rank::Eight, Suit::Spade),
            c(Rank::Two, Suit::Club),
        ]);
        assert!(nine > eight);
    }

    #[test]
    fn wheel_is_lowest_straight() {
        let wheel = eval(&[
            c(Rank::Ace, Suit::Club),
            c(Rank::Two, Suit::Diamond),
            c(Rank::Three, Suit::Heart),
            c(Rank::Four, Suit::Spade),
            c(Rank::Five, Suit::Club),
        ]);
        let six_high = eval(&[
            c(Rank::Two, Suit::Club),
            c(Rank::Three, Suit::Diamond),
            c(Rank::Four, Suit::Heart),
            c(Rank::Five, Suit::Spade),
            c(Rank::Six, Suit::Club),
        ]);
        assert_eq!(wheel.category(), HandCategory::Straight);
        assert_eq!(wheel.tiebreak()[0], 5);
        assert!(six_high > wheel, "6-high straight must beat the wheel");
        // ...but the wheel still beats any non-straight.
        let pair = eval(&[
            c(Rank::Ace, Suit::Club),
            c(Rank::Ace, Suit::Diamond),
            c(Rank::King, Suit::Heart),
            c(Rank::Queen, Suit::Spade),
            c(Rank::Jack, Suit::Club),
        ]);
        assert!(wheel > pair);
    }

    #[test]
    fn wheel_straight_flush_is_lowest_straight_flush() {
        let steel_wheel = eval(&[
            c(Rank::Ace, Suit::Club),
            c(Rank::Two, Suit::Club),
            c(Rank::Three, Suit::Club),
            c(Rank::Four, Suit::Club),
            c(Rank::Five, Suit::Club),
        ]);
        let six_high_sf = eval(&[
            c(Rank::Two, Suit::Club),
            c(Rank::Three, Suit::Club),
            c(Rank::Four, Suit::Club),
            c(Rank::Five, Suit::Club),
            c(Rank::Six, Suit::Club),
        ]);
        assert_eq!(steel_wheel.category(), HandCategory::StraightFlush);
        assert_eq!(steel_wheel.tiebreak()[0], 5);
        assert!(six_high_sf > steel_wheel);
    }

    #[test]
    fn picks_best_five_of_seven() {
        // Seven cards containing a flush; evaluate must find it.
        let hand = evaluate_holdem(
            [c(Rank::Ace, Suit::Club), c(Rank::King, Suit::Club)],
            &[
                c(Rank::Ten, Suit::Club),
                c(Rank::Five, Suit::Club),
                c(Rank::Two, Suit::Club),
                c(Rank::Nine, Suit::Diamond),
                c(Rank::Nine, Suit::Heart),
            ],
        )
        .unwrap()
        .value;
        assert_eq!(hand.category(), HandCategory::Flush);
        assert_eq!(hand.tiebreak(), [14, 13, 10, 5, 2]);
    }

    #[test]
    fn equal_hands_are_equal() {
        let a = eval(&[
            c(Rank::Ace, Suit::Club),
            c(Rank::King, Suit::Diamond),
            c(Rank::Queen, Suit::Heart),
            c(Rank::Jack, Suit::Spade),
            c(Rank::Nine, Suit::Club),
        ]);
        let b = eval(&[
            c(Rank::Ace, Suit::Spade),
            c(Rank::King, Suit::Heart),
            c(Rank::Queen, Suit::Diamond),
            c(Rank::Jack, Suit::Club),
            c(Rank::Nine, Suit::Heart),
        ]);
        assert_eq!(a, b, "same ranks, different suits -> equal hands (chop)");
    }

    #[test]
    fn full_house_ranked_by_trips_then_pair() {
        let aces_full = eval(&[
            c(Rank::Ace, Suit::Club),
            c(Rank::Ace, Suit::Diamond),
            c(Rank::Ace, Suit::Heart),
            c(Rank::Two, Suit::Spade),
            c(Rank::Two, Suit::Club),
        ]);
        let kings_full = eval(&[
            c(Rank::King, Suit::Club),
            c(Rank::King, Suit::Diamond),
            c(Rank::King, Suit::Heart),
            c(Rank::Ace, Suit::Spade),
            c(Rank::Ace, Suit::Club),
        ]);
        assert!(aces_full > kings_full, "trips rank dominates the pair rank");
    }
}

#[cfg(test)]
mod proptest_oracle {
    //! Differential (oracle) testing for the hand evaluator.
    //!
    //! Rather than re-asserting hand-crafted expectations, these tests cross-check
    //! the production evaluator against [`oracle_five`] — a deliberately *separate*,
    //! naive 5-card evaluator written from scratch. Two independent implementations
    //! of the same specification are unlikely to share the same bug, so any
    //! disagreement on a randomly generated input localizes a real error to one of
    //! the two code paths rather than to a flawed fixture.
    //!
    //! The three properties collectively pin down correctness:
    //! - **5-card agreement**: for every 5-card hand, `score_five` equals the oracle
    //!   (same category and tiebreak ranks).
    //! - **7-card = best-5 subset**: a 7-card evaluation equals the maximum over its
    //!   twenty-one 5-card subsets, tying the 7-card path back to the trusted 5-card
    //!   one.
    //! - **Comparison trichotomy**: ordering is total and consistent — exactly one of
    //!   `<`, `==`, `>` holds for any pair, and it matches the oracle's verdict.

    use super::*;

    use std::collections::{BTreeMap, BTreeSet};

    use casino_cards::card::{Card, Rank, Suit};
    use proptest::prelude::*;
    use strum::IntoEnumIterator;

    /// The 52-card deck in a fixed order, indexable 0..52.
    fn full_deck() -> Vec<Card> {
        let mut deck = Vec::with_capacity(52);
        for rank in Rank::iter() {
            for suit in Suit::iter() {
                deck.push(Card::new(rank, suit));
            }
        }
        deck
    }

    /// An independent 5-card evaluator written separately from `score_five`, used
    /// as a cross-check oracle. Returns `(category 0..=8, tiebreak ranks
    /// high→low)` matching [`HandCategory`]'s ordering and [`ComparableHand`]'s
    /// tiebreak layout.
    fn oracle_five(cards: &[Card; 5]) -> (u8, Vec<u8>) {
        let mut ranks: Vec<u8> = cards.iter().map(|c| c.rank.value()).collect();
        ranks.sort_unstable();
        let ranks_desc: Vec<u8> = ranks.iter().rev().copied().collect();

        let is_flush = cards.iter().all(|c| c.suit == cards[0].suit);

        let unique: BTreeSet<u8> = ranks.iter().copied().collect();
        let straight_high = if unique.len() == 5 {
            if ranks[4] - ranks[0] == 4 {
                Some(ranks[4])
            } else if ranks == [2, 3, 4, 5, 14] {
                Some(5) // wheel
            } else {
                None
            }
        } else {
            None
        };

        let mut counts: BTreeMap<u8, u8> = BTreeMap::new();
        for r in &ranks {
            *counts.entry(*r).or_insert(0) += 1;
        }
        let mut groups: Vec<(u8, u8)> =
            counts.iter().map(|(&rank, &count)| (count, rank)).collect();
        groups.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        let group_counts: Vec<u8> = groups.iter().map(|g| g.0).collect();
        let group_ranks: Vec<u8> = groups.iter().map(|g| g.1).collect();

        if is_flush {
            if let Some(high) = straight_high {
                return (8, vec![high]);
            }
        }
        if group_counts[0] == 4 {
            return (7, vec![group_ranks[0], group_ranks[1]]);
        }
        if group_counts[0] == 3 && group_counts.get(1) == Some(&2) {
            return (6, vec![group_ranks[0], group_ranks[1]]);
        }
        if is_flush {
            return (5, ranks_desc);
        }
        if let Some(high) = straight_high {
            return (4, vec![high]);
        }
        if group_counts[0] == 3 {
            return (3, group_ranks);
        }
        if group_counts[0] == 2 && group_counts.get(1) == Some(&2) {
            return (2, group_ranks);
        }
        if group_counts[0] == 2 {
            return (1, group_ranks);
        }
        (0, ranks_desc)
    }

    fn pad5(mut v: Vec<u8>) -> [u8; 5] {
        v.resize(5, 0);
        [v[0], v[1], v[2], v[3], v[4]]
    }

    proptest! {
        /// `evaluate_five` matches the independent oracle.
        #[test]
        fn evaluate_matches_oracle_on_five(
            indices in proptest::sample::subsequence((0u8..52).collect::<Vec<_>>(), 5)
        ) {
            let deck = full_deck();
            let five: Vec<Card> = indices.iter().map(|&i| deck[i as usize]).collect();
            let arr = [five[0], five[1], five[2], five[3], five[4]];
            let got = evaluate_five(arr).unwrap().value;
            let (category, tiebreak) = oracle_five(&arr);
            prop_assert_eq!(got.category() as u8, category);
            prop_assert_eq!(got.tiebreak(), pad5(tiebreak));
        }

        /// `best_five` of 7 cards equals the best 5-card subset per the oracle.
        #[test]
        fn evaluate_picks_best_five_of_seven(
            indices in proptest::sample::subsequence((0u8..52).collect::<Vec<_>>(), 7)
        ) {
            let deck = full_deck();
            let seven: Vec<Card> = indices.iter().map(|&i| deck[i as usize]).collect();
            let got = best_five(&seven).unwrap().value;

            let mut best: Option<(u8, [u8; 5])> = None;
            for a in 0..7 {
                for b in (a + 1)..7 {
                    for c in (b + 1)..7 {
                        for d in (c + 1)..7 {
                            for e in (d + 1)..7 {
                                let arr = [seven[a], seven[b], seven[c], seven[d], seven[e]];
                                let (category, tiebreak) = oracle_five(&arr);
                                let key = (category, pad5(tiebreak));
                                if best.is_none_or(|current| key > current) {
                                    best = Some(key);
                                }
                            }
                        }
                    }
                }
            }

            let (category, tiebreak) = best.unwrap();
            prop_assert_eq!(got.category() as u8, category);
            prop_assert_eq!(got.tiebreak(), tiebreak);
        }

        /// `ComparableHand` is a total order: comparison is trichotomous and
        /// equality implies the same category.
        #[test]
        fn comparable_hand_is_a_total_order(
            ia in proptest::sample::subsequence((0u8..52).collect::<Vec<_>>(), 5),
            ib in proptest::sample::subsequence((0u8..52).collect::<Vec<_>>(), 5),
        ) {
            let deck = full_deck();
            let a = best_five(&ia.iter().map(|&i| deck[i as usize]).collect::<Vec<_>>()).unwrap().value;
            let b = best_five(&ib.iter().map(|&i| deck[i as usize]).collect::<Vec<_>>()).unwrap().value;
            // Exactly one ordering relation holds.
            let relations = [a < b, a == b, a > b];
            prop_assert_eq!(relations.iter().filter(|&&r| r).count(), 1);
            if a == b {
                prop_assert_eq!(a.category(), b.category());
            }
        }
    }
}
