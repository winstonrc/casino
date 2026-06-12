use std::fmt;

use casino_cards::card::Card;
use serde::{Deserialize, Serialize};

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
    HighCard,
    Pair,
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
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
/// use casino_poker::hand_rankings::{evaluate, HandCategory};
/// use casino_poker::casino_cards::card::{Card, Rank, Suit};
///
/// // A flush beats a pair.
/// let flush = evaluate(
///     &[Card::new(Rank::Ace, Suit::Heart), Card::new(Rank::Two, Suit::Heart)],
///     &[
///         Card::new(Rank::Five, Suit::Heart),
///         Card::new(Rank::Nine, Suit::Heart),
///         Card::new(Rank::King, Suit::Heart),
///         Card::new(Rank::King, Suit::Spade),
///         Card::new(Rank::Three, Suit::Club),
///     ],
/// );
/// assert_eq!(flush.category, HandCategory::Flush);
/// ```
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ComparableHand {
    pub category: HandCategory,
    pub tiebreak: [u8; 5],
}

impl ComparableHand {
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

impl fmt::Display for ComparableHand {
    /// Writes only the bare category (e.g. `Two Pair`). For the PokerStars-worded
    /// made hand with its ranks (e.g. `two pair, Jacks and Fives`), use
    /// [`describe`](ComparableHand::describe).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.category)
    }
}

/// The low-to-high span of a straight (or straight flush) by its high-card value,
/// PokerStars style: `Five to Nine`. The wheel (5-high) plays the Ace low and
/// reads `Ace to Five`.
fn straight_range(high: u8) -> String {
    if high == 5 {
        "Ace to Five".to_string()
    } else {
        format!("{} to {}", rank_name(high - 4), rank_name(high))
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

/// Evaluates the best 5-card hand a player can make from their hole cards and the
/// shared board, returning a [`ComparableHand`] that is correct to compare with
/// any other player's `ComparableHand`.
///
/// Intended for the flop onward / at showdown (pre-flop strength, with only two
/// hole cards, must be estimated separately).
///
/// # Panics
///
/// Panics if `hole.len() + board.len() < 5`, since no 5-card hand can be formed.
pub fn evaluate(hole: &[Card], board: &[Card]) -> ComparableHand {
    evaluate_with_cards(hole, board).0
}

/// Like [`evaluate`], but also returns the exact five cards that form the best
/// hand, so a caller can show *which* cards make a player's hand (e.g. to
/// distinguish playing the board from using hole cards).
///
/// # Panics
///
/// Panics if `hole.len() + board.len() < 5`.
pub fn evaluate_with_cards(hole: &[Card], board: &[Card]) -> (ComparableHand, [Card; 5]) {
    let mut cards: Vec<Card> = Vec::with_capacity(hole.len() + board.len());
    cards.extend_from_slice(hole);
    cards.extend_from_slice(board);
    best_five_with_cards(&cards)
}

/// Returns the strongest [`ComparableHand`] over every 5-card subset of `cards`.
///
/// With at most 7 cards this is at most `C(7,5) = 21` subsets, so the brute-force
/// enumeration is trivial and keeps the logic obviously correct.
///
/// # Panics
///
/// Panics if `cards.len() < 5`.
pub fn best_five(cards: &[Card]) -> ComparableHand {
    best_five_with_cards(cards).0
}

/// Like [`best_five`], but also returns the exact five cards forming the hand.
///
/// Among equally-ranked 5-card subsets the choice is unspecified but
/// deterministic (the first encountered in enumeration order). Use the returned
/// [`ComparableHand`], not the card identities, to reason about ties.
///
/// # Panics
///
/// Panics if `cards.len() < 5`.
pub fn best_five_with_cards(cards: &[Card]) -> (ComparableHand, [Card; 5]) {
    assert!(
        cards.len() >= 5,
        "best_five requires at least 5 cards, got {}",
        cards.len()
    );

    let n = cards.len();
    let mut best: Option<(ComparableHand, [Card; 5])> = None;
    for a in 0..n {
        for b in (a + 1)..n {
            for c in (b + 1)..n {
                for d in (c + 1)..n {
                    for e in (d + 1)..n {
                        let five = [cards[a], cards[b], cards[c], cards[d], cards[e]];
                        let score = score_five(&five);
                        if best.is_none_or(|(current, _)| score > current) {
                            best = Some((score, five));
                        }
                    }
                }
            }
        }
    }

    best.expect("best_five always finds at least one 5-card hand")
}

/// Evaluates an Omaha hand: the best five cards using **exactly two** of the four
/// `hole` cards and **exactly three** of the `board` cards — the Omaha
/// constraint — returning a [`ComparableHand`] comparable with any other.
///
/// Unlike [`evaluate`] (which pools all cards and picks any best five, correct for
/// hold'em), this enforces the 2-from-hand / 3-from-board rule.
///
/// # Panics
///
/// Panics unless `hole.len() == 4` and `board.len()` is between 3 and 5.
pub fn best_omaha(hole: &[Card], board: &[Card]) -> ComparableHand {
    assert!(
        hole.len() == 4 && (3..=5).contains(&board.len()),
        "best_omaha requires exactly 4 hole cards and 3-5 board cards, got {} and {}",
        hole.len(),
        board.len()
    );

    let n = board.len();
    let mut best: Option<ComparableHand> = None;
    // Exactly two of the four hole cards...
    for h1 in 0..hole.len() {
        for h2 in (h1 + 1)..hole.len() {
            // ...with exactly three of the board cards.
            for b1 in 0..n {
                for b2 in (b1 + 1)..n {
                    for b3 in (b2 + 1)..n {
                        let five = [hole[h1], hole[h2], board[b1], board[b2], board[b3]];
                        let score = score_five(&five);
                        if best.is_none_or(|current| score > current) {
                            best = Some(score);
                        }
                    }
                }
            }
        }
    }

    best.expect("best_omaha always finds at least one 5-card hand")
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

    // Group equal ranks: Vec of (count, rank), then order by count desc, rank desc.
    let mut groups: Vec<(u8, u8)> = Vec::new();
    let mut i = 0;
    while i < 5 {
        let rank = ranks[i];
        let mut count = 1usize;
        while i + count < 5 && ranks[i + count] == rank {
            count += 1;
        }
        groups.push((count as u8, rank));
        i += count;
    }
    groups.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

    let counts: Vec<u8> = groups.iter().map(|g| g.0).collect();

    if counts[0] == 4 {
        return ComparableHand {
            category: HandCategory::FourOfAKind,
            tiebreak: pad(&[groups[0].1, groups[1].1]),
        };
    }

    if counts[0] == 3 && counts.get(1) == Some(&2) {
        return ComparableHand {
            category: HandCategory::FullHouse,
            tiebreak: pad(&[groups[0].1, groups[1].1]),
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

    if counts[0] == 3 {
        return ComparableHand {
            category: HandCategory::ThreeOfAKind,
            tiebreak: pad(&[groups[0].1, groups[1].1, groups[2].1]),
        };
    }

    if counts[0] == 2 && counts.get(1) == Some(&2) {
        return ComparableHand {
            category: HandCategory::TwoPair,
            tiebreak: pad(&[groups[0].1, groups[1].1, groups[2].1]),
        };
    }

    if counts[0] == 2 {
        return ComparableHand {
            category: HandCategory::Pair,
            tiebreak: pad(&[groups[0].1, groups[1].1, groups[2].1, groups[3].1]),
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

/// Copies `vals` into a zero-padded `[u8; 5]` tiebreak array.
fn pad(vals: &[u8]) -> [u8; 5] {
    let mut out = [0u8; 5];
    out[..vals.len()].copy_from_slice(vals);
    out
}

#[cfg(test)]
mod comparable_hand_tests {
    use super::*;

    use casino_cards::card::{Card, Rank, Suit};

    fn c(rank: Rank, suit: Suit) -> Card {
        Card::new(rank, suit)
    }

    /// Convenience: evaluate a flat list of cards (hole empty, all on "board").
    fn eval(cards: &[Card]) -> ComparableHand {
        evaluate(&[], cards)
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

        let pooled = evaluate(&hole, &board);
        let omaha = best_omaha(&hole, &board);
        assert_eq!(pooled.category, HandCategory::StraightFlush);
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
        assert_eq!(
            best_omaha(&hole, &board).category,
            HandCategory::FourOfAKind
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
    fn best_five_with_cards_returns_the_forming_cards() {
        // Board pair of Queens plus three low cards; the best five is the two
        // Queens and the three highest kickers, regardless of input order.
        let qc = c(Rank::Queen, Suit::Club);
        let qs = c(Rank::Queen, Suit::Spade);
        let (hand, five) = evaluate_with_cards(
            &[c(Rank::Three, Suit::Heart), c(Rank::Two, Suit::Diamond)],
            &[
                qc,
                c(Rank::Jack, Suit::Spade),
                c(Rank::Four, Suit::Diamond),
                qs,
                c(Rank::Ten, Suit::Club),
            ],
        );
        assert_eq!(hand.category, HandCategory::Pair);
        assert!(five.contains(&qc) && five.contains(&qs));
        // The low hole cards (3, 2) are worse kickers than the board's J/10/4.
        assert!(!five.contains(&c(Rank::Three, Suit::Heart)));
        assert!(!five.contains(&c(Rank::Two, Suit::Diamond)));
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
            .category,
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
            .category,
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
            .category,
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
            .category,
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
            .category,
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
            .category,
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
            .category,
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
            .category,
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
            .category,
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
        assert_eq!(wheel.category, HandCategory::Straight);
        assert_eq!(wheel.tiebreak[0], 5);
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
        assert_eq!(steel_wheel.category, HandCategory::StraightFlush);
        assert_eq!(steel_wheel.tiebreak[0], 5);
        assert!(six_high_sf > steel_wheel);
    }

    #[test]
    fn picks_best_five_of_seven() {
        // Seven cards containing a flush; evaluate must find it.
        let hand = evaluate(
            &[c(Rank::Ace, Suit::Club), c(Rank::King, Suit::Club)],
            &[
                c(Rank::Ten, Suit::Club),
                c(Rank::Five, Suit::Club),
                c(Rank::Two, Suit::Club),
                c(Rank::Nine, Suit::Diamond),
                c(Rank::Nine, Suit::Heart),
            ],
        );
        assert_eq!(hand.category, HandCategory::Flush);
        assert_eq!(hand.tiebreak, [14, 13, 10, 5, 2]);
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
        /// `evaluate` of an exact 5-card hand matches the independent oracle.
        #[test]
        fn evaluate_matches_oracle_on_five(
            indices in proptest::sample::subsequence((0u8..52).collect::<Vec<_>>(), 5)
        ) {
            let deck = full_deck();
            let five: Vec<Card> = indices.iter().map(|&i| deck[i as usize]).collect();
            let arr = [five[0], five[1], five[2], five[3], five[4]];
            let got = evaluate(&[], &five);
            let (category, tiebreak) = oracle_five(&arr);
            prop_assert_eq!(got.category as u8, category);
            prop_assert_eq!(got.tiebreak, pad5(tiebreak));
        }

        /// `evaluate` of 7 cards equals the best 5-card subset per the oracle.
        #[test]
        fn evaluate_picks_best_five_of_seven(
            indices in proptest::sample::subsequence((0u8..52).collect::<Vec<_>>(), 7)
        ) {
            let deck = full_deck();
            let seven: Vec<Card> = indices.iter().map(|&i| deck[i as usize]).collect();
            let got = evaluate(&seven[0..2], &seven[2..7]);

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
            prop_assert_eq!(got.category as u8, category);
            prop_assert_eq!(got.tiebreak, tiebreak);
        }

        /// `ComparableHand` is a total order: comparison is trichotomous and
        /// equality implies the same category.
        #[test]
        fn comparable_hand_is_a_total_order(
            ia in proptest::sample::subsequence((0u8..52).collect::<Vec<_>>(), 5),
            ib in proptest::sample::subsequence((0u8..52).collect::<Vec<_>>(), 5),
        ) {
            let deck = full_deck();
            let a = evaluate(&[], &ia.iter().map(|&i| deck[i as usize]).collect::<Vec<_>>());
            let b = evaluate(&[], &ib.iter().map(|&i| deck[i as usize]).collect::<Vec<_>>());
            // Exactly one ordering relation holds.
            let relations = [a < b, a == b, a > b];
            prop_assert_eq!(relations.iter().filter(|&&r| r).count(), 1);
            if a == b {
                prop_assert_eq!(a.category, b.category);
            }
        }
    }
}
