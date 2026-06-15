//! Pot construction and distribution for Texas Hold'em, including side pots.
//!
//! The central idea is that side pots are computed **at showdown** from each
//! player's total contribution to the hand, rather than being built up
//! incrementally during betting. This is both simpler and less error-prone.
//!
//! The flow at the end of a hand is:
//! 1. [`refund_uncalled`](crate::pot::refund_uncalled) — return any uncalled
//!    overbet to the lone top bettor.
//! 2. [`build_pots`](crate::pot::build_pots) — split the remaining contributions
//!    into a main pot and any side pots via layering.
//! 3. [`distribute_pots`](crate::pot::distribute_pots) — award each pot to the
//!    best eligible hand(s), splitting ties with the odd chip going to the first
//!    eligible seat left of the dealer.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::hand_rankings::ComparableHand;

/// A pot (main or side) holding chips and the set of players eligible to win it.
///
/// `eligible` is a [`BTreeSet`] so iteration order is deterministic for tests;
/// it must **not** be relied on for seat ordering — odd-chip distribution
/// re-sorts by seat position relative to the dealer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Pot {
    /// Chips in this pot.
    pub amount: u64,
    /// Players eligible to win this pot: those who contributed to its layer and
    /// have not folded. Folded players' chips remain in the pot as dead money but
    /// they cannot win it.
    pub eligible: BTreeSet<Uuid>,
}

/// Returns the uncalled portion of the largest single bet to its bettor.
///
/// When exactly one player has contributed strictly more than everyone else, the
/// excess over the second-highest contribution was never called and must be
/// returned. This function reduces that player's entry in `contributed` and
/// returns `(player, refund)` so the caller can credit it back to their stack.
///
/// If two or more players tie for the highest contribution, nothing is uncalled
/// and `None` is returned. In legal play the strict-max contributor is always
/// live (a folded player faced a bet ≥ their contribution); the `folded` guard is
/// defensive — a folded top contributor is never refunded.
pub fn refund_uncalled(
    contributed: &mut HashMap<Uuid, u32>,
    folded: &HashSet<Uuid>,
) -> Option<(Uuid, u32)> {
    let mut entries: Vec<(u32, Uuid)> = contributed
        .iter()
        .filter(|(_, &amount)| amount > 0)
        .map(|(&id, &amount)| (amount, id))
        .collect();

    if entries.is_empty() {
        return None;
    }

    // Sort by amount descending. Ties keep no particular player order, which is
    // fine: a tie means there is no uncalled bet.
    entries.sort_unstable_by_key(|e| std::cmp::Reverse(e.0));

    let top_amount = entries[0].0;
    let second_amount = entries.get(1).map_or(0, |e| e.0);
    let top_id = entries[0].1;

    if top_amount > second_amount && !folded.contains(&top_id) {
        let refund = top_amount - second_amount;
        contributed.insert(top_id, second_amount);
        return Some((top_id, refund));
    }

    None
}

/// Builds the main pot and any side pots from each player's total contribution.
///
/// Call [`refund_uncalled`] first so no pot is funded solely by a single live
/// over-contributor. Folded players are included as dead money (their chips fund
/// the layers they reached) but are never eligible to win.
///
/// The returned vector is ordered lowest-layer first, so index `0` is the main
/// pot. Consecutive layers with identical eligibility are merged, and any layer
/// funded entirely by folded players (no eligible winner) is merged into the
/// nearest lower pot so chips are never orphaned.
///
/// ```
/// use std::collections::{HashMap, HashSet};
/// use casino_poker::pot::build_pots;
/// use casino_poker::uuid::Uuid;
///
/// let (a, b, c) = (Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3));
/// // After refunding C's uncalled 40 (all-in 100 vs B's 60), contributions are:
/// let contributed = HashMap::from([(a, 20), (b, 60), (c, 60)]);
/// let pots = build_pots(&contributed, &HashSet::new());
///
/// assert_eq!(pots.len(), 2);
/// assert_eq!(pots[0].amount, 60); // main: 20 * 3, eligible {A, B, C}
/// assert_eq!(pots[0].eligible.len(), 3);
/// assert_eq!(pots[1].amount, 80); // side: 40 * 2, eligible {B, C}
/// assert!(pots[1].eligible.contains(&b) && pots[1].eligible.contains(&c));
/// ```
pub fn build_pots(contributed: &HashMap<Uuid, u32>, folded: &HashSet<Uuid>) -> Vec<Pot> {
    let mut remaining: HashMap<Uuid, u32> = contributed
        .iter()
        .filter(|(_, &amount)| amount > 0)
        .map(|(&id, &amount)| (id, amount))
        .collect();

    let mut layers: Vec<Pot> = Vec::new();
    while let Some(&layer) = remaining.values().filter(|&&a| a > 0).min() {
        let mut amount = 0u64;
        let mut eligible = BTreeSet::new();
        for (id, amt) in remaining.iter_mut() {
            if *amt > 0 {
                amount += u64::from(layer);
                *amt -= layer;
                if !folded.contains(id) {
                    eligible.insert(*id);
                }
            }
        }
        layers.push(Pot { amount, eligible });
    }

    // Merge consecutive layers with identical eligibility, and fold any
    // dead-money-only layer (empty eligible) into the nearest lower pot.
    let mut pots: Vec<Pot> = Vec::new();
    for layer in layers {
        if let Some(last) = pots.last_mut() {
            if last.eligible == layer.eligible || layer.eligible.is_empty() {
                last.amount += layer.amount;
                continue;
            }
        }
        pots.push(layer);
    }

    pots
}

/// The outcome of awarding a single pot: which pot it was, its size, and the
/// chips each winner received from it. Returned by [`distribute_pots`], one entry
/// per pot, so callers can narrate each pot separately rather than only a player's
/// summed winnings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PotAward {
    /// Pot position: `0` for the main pot, `1..` for side pots from the smallest
    /// (lowest) layer upward.
    pub index: usize,
    /// Total chips in this pot.
    pub amount: u64,
    /// Each winner and the chips they received from this pot, ordered clockwise
    /// from the dealer (the seat that receives any odd chip first).
    pub payouts: Vec<(Uuid, u64)>,
}

/// Awards each pot to the best eligible hand(s), returning one [`PotAward`] per
/// pot in main-then-side order.
///
/// For each pot, the winner is the player(s) in `eligible` with the maximum
/// [`ComparableHand`] in `evaluated`. Ties split the pot evenly; the leftover
/// `amount % winners` chips are distributed one at a time to winners ordered
/// clockwise from the dealer (first eligible seat to the dealer's left first).
///
/// If no eligible player has an evaluated hand (an uncontested pot, e.g. everyone
/// else folded), the whole pot goes to the eligible players — normally a single
/// player.
pub fn distribute_pots(
    pots: &[Pot],
    evaluated: &HashMap<Uuid, ComparableHand>,
    seats: &[Uuid],
    dealer_seat_index: usize,
) -> Vec<PotAward> {
    let mut awards: Vec<PotAward> = Vec::new();

    for (index, pot) in pots.iter().enumerate() {
        if pot.amount == 0 {
            continue;
        }

        let winners = pot_winners(pot, evaluated);
        if winners.is_empty() {
            continue;
        }

        let ordered = order_by_seat(&winners, seats, dealer_seat_index);
        let count = ordered.len() as u64;
        let base = pot.amount / count;
        let remainder = pot.amount % count;

        let payouts = ordered
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let extra = if (i as u64) < remainder { 1 } else { 0 };
                (id, base + extra)
            })
            .collect();

        awards.push(PotAward {
            index,
            amount: pot.amount,
            payouts,
        });
    }

    awards
}

/// Returns the eligible players that win a pot: those tied for the best evaluated
/// hand, or all eligible players if none have an evaluated hand (uncontested).
fn pot_winners(pot: &Pot, evaluated: &HashMap<Uuid, ComparableHand>) -> Vec<Uuid> {
    let mut best: Option<ComparableHand> = None;
    for id in &pot.eligible {
        if let Some(&hand) = evaluated.get(id) {
            if best.is_none_or(|current| hand > current) {
                best = Some(hand);
            }
        }
    }

    match best {
        Some(best) => pot
            .eligible
            .iter()
            .filter(|id| evaluated.get(id) == Some(&best))
            .copied()
            .collect(),
        // Uncontested: no eligible player went to showdown.
        None => pot.eligible.iter().copied().collect(),
    }
}

/// Orders players clockwise from the dealer (first seat to the dealer's left
/// first), used to assign odd chips. Players not found in `seats` are placed last.
fn order_by_seat(players: &[Uuid], seats: &[Uuid], dealer_seat_index: usize) -> Vec<Uuid> {
    let len = seats.len();
    let mut ordered: Vec<Uuid> = players.to_vec();
    ordered.sort_by_key(|id| match seats.iter().position(|seat| seat == id) {
        Some(pos) if len > 0 => {
            let first_left_of_dealer = (dealer_seat_index % len + 1) % len;
            (pos + len - first_left_of_dealer) % len
        }
        _ => usize::MAX,
    });
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;

    use casino_cards::card::{Card, Rank, Suit};

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    /// Builds a `ComparableHand` of the given high-card-only strength for tests
    /// where we only care which player has the better hand.
    fn hand(strength: u8) -> ComparableHand {
        // Five distinct cards whose top card encodes `strength`, so a higher
        // `strength` yields a strictly stronger high-card hand.
        use crate::hand_rankings::evaluate;
        let board = [
            Card::new(rank_from(strength), Suit::Club),
            Card::new(Rank::Seven, Suit::Diamond),
            Card::new(Rank::Five, Suit::Heart),
            Card::new(Rank::Three, Suit::Spade),
            Card::new(Rank::Two, Suit::Club),
        ];
        evaluate(&[], &board)
    }

    /// Sums each player's chips across all pot awards, for assertions that only
    /// care about totals rather than which pot paid them.
    fn totals(awards: &[PotAward]) -> HashMap<Uuid, u64> {
        let mut totals: HashMap<Uuid, u64> = HashMap::new();
        for award in awards {
            for &(id, amount) in &award.payouts {
                *totals.entry(id).or_insert(0) += amount;
            }
        }
        totals
    }

    fn rank_from(strength: u8) -> Rank {
        match strength {
            14 => Rank::Ace,
            13 => Rank::King,
            12 => Rank::Queen,
            11 => Rank::Jack,
            _ => Rank::Ten,
        }
    }

    #[test]
    fn refund_returns_uncalled_overbet() {
        let (a, b, c) = (id(1), id(2), id(3));
        let mut contributed = HashMap::from([(a, 20), (b, 60), (c, 100)]);
        let refund = refund_uncalled(&mut contributed, &HashSet::new());
        assert_eq!(refund, Some((c, 40)));
        assert_eq!(contributed[&c], 60);
    }

    #[test]
    fn refund_none_when_top_tied() {
        let (a, b, c) = (id(1), id(2), id(3));
        let mut contributed = HashMap::from([(a, 100), (b, 100), (c, 40)]);
        assert_eq!(refund_uncalled(&mut contributed, &HashSet::new()), None);
        assert_eq!(contributed[&a], 100);
    }

    #[test]
    fn refund_sole_contributor_gets_everything_back() {
        let a = id(1);
        let mut contributed = HashMap::from([(a, 30)]);
        assert_eq!(
            refund_uncalled(&mut contributed, &HashSet::new()),
            Some((a, 30))
        );
        assert_eq!(contributed[&a], 0);
    }

    #[test]
    fn refund_skips_a_folded_top_contributor() {
        // Defensive: a folded strict-max contributor is not refunded.
        let (a, b) = (id(1), id(2));
        let mut contributed = HashMap::from([(a, 30), (b, 50)]);
        let folded = HashSet::from([b]);
        assert_eq!(refund_uncalled(&mut contributed, &folded), None);
        assert_eq!(contributed[&b], 50);
    }

    #[test]
    fn equal_contributions_form_one_pot() {
        let (a, b, c) = (id(1), id(2), id(3));
        let contributed = HashMap::from([(a, 50), (b, 50), (c, 50)]);
        let pots = build_pots(&contributed, &HashSet::new());
        assert_eq!(pots.len(), 1);
        assert_eq!(pots[0].amount, 150);
        assert_eq!(pots[0].eligible.len(), 3);
    }

    #[test]
    fn three_way_all_in_builds_main_and_side() {
        let (a, b, c) = (id(1), id(2), id(3));
        // Post-refund contributions for A=20, B=60, C=100 all-in.
        let contributed = HashMap::from([(a, 20), (b, 60), (c, 60)]);
        let pots = build_pots(&contributed, &HashSet::new());
        assert_eq!(pots.len(), 2);
        assert_eq!(pots[0].amount, 60);
        assert_eq!(pots[0].eligible, BTreeSet::from([a, b, c]));
        assert_eq!(pots[1].amount, 80);
        assert_eq!(pots[1].eligible, BTreeSet::from([b, c]));
    }

    #[test]
    fn folded_dead_money_stays_in_pot_but_not_eligible() {
        let (a, b, c) = (id(1), id(2), id(3));
        // C folded after contributing 20.
        let contributed = HashMap::from([(a, 60), (b, 60), (c, 20)]);
        let folded = HashSet::from([c]);
        let pots = build_pots(&contributed, &folded);
        // Merges to one pot (eligibility {A,B} for all layers), C's 20 is dead money.
        assert_eq!(pots.len(), 1);
        assert_eq!(pots[0].amount, 140);
        assert_eq!(pots[0].eligible, BTreeSet::from([a, b]));
    }

    #[test]
    fn all_fold_to_one_yields_single_eligible_pot() {
        let (a, b, c) = (id(1), id(2), id(3));
        let contributed = HashMap::from([(a, 5), (b, 2), (c, 1)]);
        let folded = HashSet::from([b, c]);
        let pots = build_pots(&contributed, &folded);
        // A is the only live player; dead money from B and C accrues to A's pot.
        let total: u64 = pots.iter().map(|p| p.amount).sum();
        assert_eq!(total, 8);
        for pot in &pots {
            assert!(!pot.eligible.is_empty(), "no orphaned (empty-eligible) pot");
            assert!(pot.eligible.iter().all(|id| *id == a));
        }
    }

    #[test]
    fn distribute_pays_best_eligible_hand() {
        let (a, b, c) = (id(1), id(2), id(3));
        let seats = vec![a, b, c];
        let contributed = HashMap::from([(a, 20), (b, 60), (c, 60)]);
        let pots = build_pots(&contributed, &HashSet::new());
        // C has the best hand, B second, A worst.
        let evaluated = HashMap::from([(a, hand(10)), (b, hand(12)), (c, hand(14))]);
        let winnings = totals(&distribute_pots(&pots, &evaluated, &seats, 0));
        // C wins both the main (60) and side (80) pots.
        assert_eq!(winnings.get(&c), Some(&140));
        assert_eq!(winnings.get(&b), None);
        assert_eq!(winnings.get(&a), None);
    }

    #[test]
    fn distribute_side_pot_to_eligible_when_short_stack_wins_main() {
        let (a, b, c) = (id(1), id(2), id(3));
        let seats = vec![a, b, c];
        let contributed = HashMap::from([(a, 20), (b, 60), (c, 60)]);
        let pots = build_pots(&contributed, &HashSet::new());
        // Short stack A has the best hand but is only eligible for the main pot.
        let evaluated = HashMap::from([(a, hand(14)), (b, hand(13)), (c, hand(11))]);
        let winnings = totals(&distribute_pots(&pots, &evaluated, &seats, 0));
        assert_eq!(winnings.get(&a), Some(&60)); // main pot only
        assert_eq!(winnings.get(&b), Some(&80)); // side pot: best among B, C
        assert_eq!(winnings.get(&c), None);
    }

    #[test]
    fn distribute_reports_each_pot_separately() {
        let (a, b, c) = (id(1), id(2), id(3));
        let seats = vec![a, b, c];
        let contributed = HashMap::from([(a, 20), (b, 60), (c, 60)]);
        let pots = build_pots(&contributed, &HashSet::new());
        // A wins the main pot; B wins the side pot it isn't eligible for.
        let evaluated = HashMap::from([(a, hand(14)), (b, hand(13)), (c, hand(11))]);
        let awards = distribute_pots(&pots, &evaluated, &seats, 0);

        assert_eq!(awards.len(), 2);
        assert_eq!(awards[0].index, 0);
        assert_eq!(awards[0].amount, 60);
        assert_eq!(awards[0].payouts, vec![(a, 60)]);
        assert_eq!(awards[1].index, 1);
        assert_eq!(awards[1].amount, 80);
        assert_eq!(awards[1].payouts, vec![(b, 80)]);
    }

    #[test]
    fn split_pot_odd_chip_goes_left_of_dealer() {
        let (a, b) = (id(1), id(2));
        let seats = vec![a, b];
        // Odd pot of 5 between two equal hands; dealer at index 0 => first seat to
        // the left is b (index 1), so b should get the odd chip.
        let pots = vec![Pot {
            amount: 5,
            eligible: BTreeSet::from([a, b]),
        }];
        let evaluated = HashMap::from([(a, hand(14)), (b, hand(14))]);
        let winnings = totals(&distribute_pots(&pots, &evaluated, &seats, 0));
        assert_eq!(winnings.get(&b), Some(&3));
        assert_eq!(winnings.get(&a), Some(&2));
    }

    #[test]
    fn aggregate_pots_can_exceed_u32_without_overflowing() {
        let (a, b) = (id(1), id(2));
        let contributed = HashMap::from([(a, u32::MAX), (b, u32::MAX)]);
        let pots = build_pots(&contributed, &HashSet::new());

        assert_eq!(pots.len(), 1);
        assert_eq!(pots[0].amount, u64::from(u32::MAX) * 2);
    }
}
