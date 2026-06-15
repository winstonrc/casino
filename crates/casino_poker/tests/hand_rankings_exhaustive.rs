use casino_poker::casino_cards::card::{Card, Rank, Suit};
use casino_poker::hand_rankings::{best_five, ComparableHand, HandCategory};

const EXPECTED_CATEGORY_COUNTS: [usize; 9] = [
    1_302_540, // High card
    1_098_240, // One pair
    123_552,   // Two pair
    54_912,    // Three of a kind
    10_200,    // Straight
    5_108,     // Flush
    3_744,     // Full house
    624,       // Four of a kind
    40,        // Straight flush
];

fn full_deck() -> [Card; 52] {
    const RANKS: [Rank; 13] = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];
    const SUITS: [Suit; 4] = [Suit::Club, Suit::Diamond, Suit::Heart, Suit::Spade];

    std::array::from_fn(|index| Card::new(RANKS[index / SUITS.len()], SUITS[index % SUITS.len()]))
}

fn reference_score(cards: &[Card; 5]) -> ComparableHand {
    let mut rank_counts = [0u8; 15];
    let mut ranks = [0u8; 5];
    for (index, card) in cards.iter().enumerate() {
        let rank = card.rank.value();
        rank_counts[rank as usize] += 1;
        ranks[index] = rank;
    }
    ranks.sort_unstable_by(|left, right| right.cmp(left));

    let flush = cards.iter().all(|card| card.suit == cards[0].suit);
    let straight_high = if ranks == [14, 5, 4, 3, 2] {
        Some(5)
    } else if ranks.windows(2).all(|pair| pair[0] == pair[1] + 1) {
        Some(ranks[0])
    } else {
        None
    };

    if flush {
        if let Some(high) = straight_high {
            return hand(HandCategory::StraightFlush, &[high]);
        }
    }

    let mut four = 0;
    let mut three = 0;
    let mut pairs = [0u8; 2];
    let mut pair_count = 0;
    let mut singles = [0u8; 5];
    let mut single_count = 0;

    for rank in (2u8..=14).rev() {
        match rank_counts[rank as usize] {
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
        return hand(HandCategory::FourOfAKind, &[four, singles[0]]);
    }
    if three != 0 && pair_count == 1 {
        return hand(HandCategory::FullHouse, &[three, pairs[0]]);
    }
    if flush {
        return hand(HandCategory::Flush, &ranks);
    }
    if let Some(high) = straight_high {
        return hand(HandCategory::Straight, &[high]);
    }
    if three != 0 {
        return hand(HandCategory::ThreeOfAKind, &[three, singles[0], singles[1]]);
    }
    if pair_count == 2 {
        return hand(HandCategory::TwoPair, &[pairs[0], pairs[1], singles[0]]);
    }
    if pair_count == 1 {
        return hand(
            HandCategory::Pair,
            &[pairs[0], singles[0], singles[1], singles[2]],
        );
    }
    hand(HandCategory::HighCard, &ranks)
}

fn hand(category: HandCategory, ranks: &[u8]) -> ComparableHand {
    let mut tiebreak = [0u8; 5];
    tiebreak[..ranks.len()].copy_from_slice(ranks);
    ComparableHand { category, tiebreak }
}

#[test]
#[ignore = "exhaustively checks all 2,598,960 hands in the release CI job"]
fn exhaustive_five_card_evaluation_matches_reference() {
    let deck = full_deck();
    let mut category_counts = [0usize; 9];
    let mut evaluated = 0usize;

    for a in 0..48 {
        for b in (a + 1)..49 {
            for c in (b + 1)..50 {
                for d in (c + 1)..51 {
                    for e in (d + 1)..52 {
                        let cards = [deck[a], deck[b], deck[c], deck[d], deck[e]];
                        let actual = best_five(&cards);
                        let expected = reference_score(&cards);
                        assert_eq!(actual, expected, "mismatch for {cards:?}");
                        category_counts[actual.category as usize] += 1;
                        evaluated += 1;
                    }
                }
            }
        }
    }

    assert_eq!(evaluated, 2_598_960);
    assert_eq!(category_counts, EXPECTED_CATEGORY_COUNTS);
}
