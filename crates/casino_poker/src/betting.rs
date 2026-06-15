//! Betting math and the per-street betting-round state machine.
//!
//! Everything here is engine-side and UI-agnostic. The pure functions
//! ([`amount_owed`], [`resolve_action`], [`legal_actions`]) validate and price a
//! player's action; [`BettingRound`] tracks whose turn it is and when a street's
//! betting is complete.
//!
//! Key invariants encoded here (and the bugs they prevent):
//! - A player only owes `current_bet - committed_this_round` — blinds and prior
//!   calls are credited, so a call never double-charges.
//! - A player who cannot cover the bet goes **all-in** for their remaining chips
//!   rather than being forced to fold.
//! - The minimum legal raise is `current_bet + last_raise_increment`, where
//!   `last_raise_increment` starts at the big blind and grows to the size of each
//!   full raise. An all-in below that raises the bet but does **not** grant
//!   already-acted players the right to re-raise (the incomplete-raise rule).

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An action a player takes on their turn.
///
/// `RaiseTo` is expressed as the **total** the player wants committed this street
/// (raise-to, not raise-by), which composes cleanly with the contribution model.
/// User interfaces that collect "raise by X" convert with `RaiseTo(current_bet + X)`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PlayerAction {
    Fold,
    Check,
    Call,
    /// Raise so the player's total committed this street becomes this amount.
    RaiseTo(u32),
    /// Commit all remaining chips (a shove, a sub-min all-in, or a short call).
    AllIn,
}

/// A legal action offered to a player, carrying the chip amounts involved so a UI
/// or AI can present/choose without recomputing them.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LegalAction {
    Fold,
    Check,
    /// Call by paying this many chips.
    Call(u32),
    /// Raise to a total between `min` and `max` (inclusive) chips committed.
    RaiseTo {
        min: u32,
        max: u32,
    },
    /// Go all-in, committing to this total this street.
    AllIn(u32),
}

/// Why an action could not be applied.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum ActionError {
    /// Tried to check while owing chips.
    CannotCheck,
    /// Raise was below the minimum legal raise and not an all-in.
    RaiseTooSmall,
    /// Raise required more chips than the player has.
    InsufficientChips,
    /// Raise target did not exceed the current bet.
    NotARaise,
    /// Betting was not reopened after an incomplete all-in raise.
    RaiseNotAllowed,
    /// Committed chips plus the remaining stack exceeded the supported amount.
    ChipAmountOverflow,
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CannotCheck => "cannot check while facing a bet",
            Self::RaiseTooSmall => "raise is below the minimum",
            Self::InsufficientChips => "insufficient chips for that raise",
            Self::NotARaise => "raise target must exceed the current bet",
            Self::RaiseNotAllowed => "betting was not reopened for this player",
            Self::ChipAmountOverflow => "chip amount exceeds the supported u32 range",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ActionError {}

/// The chip movement and state effects of applying a [`PlayerAction`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resolved {
    /// Chips moved from the player's stack into the pot.
    pub paid: u32,
    /// Whether the player is now all-in.
    pub all_in: bool,
    /// Whether the player folded.
    pub folded: bool,
    /// If the action raised the table bet, the new total (`current_bet`).
    pub raised_to: Option<u32>,
    /// Whether the raise was a *full* legal raise (reopens betting) vs an
    /// incomplete all-in (raises the bet but does not reopen re-raising rights).
    pub is_full_raise: bool,
}

/// The number of chips a player still owes to call.
///
/// ```
/// use casino_poker::betting::amount_owed;
///
/// // Posted the small blind of 1; the current bet is the big blind of 2.
/// assert_eq!(amount_owed(2, 1), 1);
/// // Already matched the bet — nothing owed.
/// assert_eq!(amount_owed(2, 2), 0);
/// ```
pub fn amount_owed(current_bet: u32, committed_this_round: u32) -> u32 {
    current_bet.saturating_sub(committed_this_round)
}

/// Validates and prices a player's action, returning the resulting chip movement.
///
/// - `chips` is the player's remaining stack.
/// - `committed_this_round` is what they have already put in this street.
/// - `current_bet` is the amount to match this street.
/// - `last_raise_increment` is the minimum raise size (starts at the big blind).
pub fn resolve_action(
    action: PlayerAction,
    chips: u32,
    committed_this_round: u32,
    current_bet: u32,
    last_raise_increment: u32,
) -> Result<Resolved, ActionError> {
    let owed = amount_owed(current_bet, committed_this_round);

    match action {
        PlayerAction::Fold => Ok(Resolved {
            paid: 0,
            all_in: false,
            folded: true,
            raised_to: None,
            is_full_raise: false,
        }),

        PlayerAction::Check => {
            if owed != 0 {
                return Err(ActionError::CannotCheck);
            }
            Ok(Resolved {
                paid: 0,
                all_in: false,
                folded: false,
                raised_to: None,
                is_full_raise: false,
            })
        }

        PlayerAction::Call => {
            // Pay only the delta owed; if the stack can't cover it, it's a short
            // all-in call that does not change the current bet.
            let paid = owed.min(chips);
            let all_in = paid == chips && owed >= chips;
            Ok(Resolved {
                paid,
                all_in,
                folded: false,
                raised_to: None,
                is_full_raise: false,
            })
        }

        PlayerAction::RaiseTo(target) => {
            if target <= current_bet {
                return Err(ActionError::NotARaise);
            }
            let need = target.saturating_sub(committed_this_round);
            if need > chips {
                return Err(ActionError::InsufficientChips);
            }
            let all_in = need == chips;
            let is_full_raise = current_bet
                .checked_add(last_raise_increment)
                .is_some_and(|min_to| target >= min_to);
            if !is_full_raise {
                // Below a full raise: only legal as an all-in (handled here so a
                // UI passing RaiseTo for a shove still works).
                if all_in {
                    return Ok(Resolved {
                        paid: need,
                        all_in: true,
                        folded: false,
                        raised_to: Some(target),
                        is_full_raise: false,
                    });
                }
                return Err(ActionError::RaiseTooSmall);
            }
            Ok(Resolved {
                paid: need,
                all_in,
                folded: false,
                raised_to: Some(target),
                is_full_raise: true,
            })
        }

        PlayerAction::AllIn => {
            let paid = chips;
            let target = committed_this_round
                .checked_add(chips)
                .ok_or(ActionError::ChipAmountOverflow)?;
            if target <= current_bet {
                // Short all-in that doesn't even match the bet: a call, no raise.
                return Ok(Resolved {
                    paid,
                    all_in: true,
                    folded: false,
                    raised_to: None,
                    is_full_raise: false,
                });
            }
            let is_full_raise = current_bet
                .checked_add(last_raise_increment)
                .is_some_and(|min_to| target >= min_to);
            Ok(Resolved {
                paid,
                all_in: true,
                folded: false,
                raised_to: Some(target),
                is_full_raise,
            })
        }
    }
}

/// Computes the legal actions for a player given their stack and the betting state.
///
/// `can_raise` is `false` when an incomplete all-in has removed this player's
/// right to re-raise; they may then only call or fold.
pub fn legal_actions(
    chips: u32,
    committed_this_round: u32,
    current_bet: u32,
    last_raise_increment: u32,
    can_raise: bool,
) -> Vec<LegalAction> {
    let mut actions = vec![LegalAction::Fold];
    let owed = amount_owed(current_bet, committed_this_round);
    let max_to = committed_this_round.checked_add(chips);

    if owed == 0 {
        actions.push(LegalAction::Check);
    } else if chips > owed {
        actions.push(LegalAction::Call(owed));
    }

    if max_to.is_some_and(|max_to| max_to > current_bet) {
        if can_raise {
            let max_to = max_to.expect("checked above");
            if let Some(min_to) = current_bet.checked_add(last_raise_increment) {
                if max_to >= min_to {
                    actions.push(LegalAction::RaiseTo {
                        min: min_to,
                        max: max_to,
                    });
                }
            }
            actions.push(LegalAction::AllIn(max_to));
        }
    } else if max_to.is_some() && owed > 0 && chips <= owed {
        // Can't fully call — the only way to commit chips is a short all-in call.
        actions.push(LegalAction::AllIn(max_to.expect("checked above")));
    }

    actions
}

/// Per-street betting state: whose action remains, the current bet, and the
/// minimum raise. Created fresh for each street and driven by the engine.
///
/// `BettingRound` is intended to be a short-lived local in the engine's
/// round driver, not a long-lived field — it borrows nothing and is reset each
/// street.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BettingRound {
    /// The amount to match this street.
    pub current_bet: u32,
    /// The minimum raise size; min legal raise-to is `current_bet + this`.
    pub last_raise_increment: u32,
    committed_this_round: HashMap<Uuid, u32>,
    /// Live players who still owe an action this street.
    needs_to_act: HashSet<Uuid>,
    /// Bet level each player faced when they last acted. A player may raise again
    /// once subsequent incomplete all-ins cumulatively increase the bet by at
    /// least `last_raise_increment`.
    acted_at_bet: HashMap<Uuid, u32>,
    /// Pre-flop only: the big blind still has the option to check or raise.
    bb_option_pending: bool,
    /// The big blind's id (pre-flop), used to clear the option when they act.
    big_blind: Option<Uuid>,
}

impl BettingRound {
    /// Creates a betting round.
    ///
    /// - `actors` are the players who can act this street (live: not folded, not
    ///   all-in).
    /// - `current_bet` is the opening bet (the big blind pre-flop, else 0).
    /// - `big_blind_amount` seeds `last_raise_increment`.
    /// - `committed_seed` records chips already in this street (the blinds
    ///   pre-flop; empty otherwise).
    /// - `bb_option` is the big blind's id pre-flop on an unraised pot, else `None`.
    pub fn new(
        actors: &[Uuid],
        current_bet: u32,
        big_blind_amount: u32,
        committed_seed: HashMap<Uuid, u32>,
        bb_option: Option<Uuid>,
    ) -> Self {
        Self {
            current_bet,
            last_raise_increment: big_blind_amount,
            committed_this_round: committed_seed,
            needs_to_act: actors.iter().copied().collect(),
            acted_at_bet: HashMap::new(),
            bb_option_pending: bb_option.is_some(),
            big_blind: bb_option,
        }
    }

    /// Chips a player has committed this street.
    pub fn committed(&self, id: Uuid) -> u32 {
        self.committed_this_round.get(&id).copied().unwrap_or(0)
    }

    /// Chips a player still owes to call this street.
    pub fn owed(&self, id: Uuid) -> u32 {
        amount_owed(self.current_bet, self.committed(id))
    }

    /// Whether the player still owes an action this street.
    pub fn needs_to_act(&self, id: Uuid) -> bool {
        self.needs_to_act.contains(&id)
    }

    /// Whether the player may raise (vs. being restricted to call/fold by an
    /// incomplete all-in).
    pub fn may_raise(&self, id: Uuid) -> bool {
        self.acted_at_bet.get(&id).is_none_or(|acted_at| {
            self.current_bet.saturating_sub(*acted_at) >= self.last_raise_increment
        })
    }

    /// Whether betting for this street is complete.
    pub fn is_closed(&self) -> bool {
        self.needs_to_act.is_empty() && !self.bb_option_pending
    }

    pub(crate) fn references_only(&self, players: &HashSet<Uuid>) -> bool {
        self.committed_this_round
            .keys()
            .chain(self.needs_to_act.iter())
            .chain(self.acted_at_bet.keys())
            .all(|id| players.contains(id))
            && self.big_blind.is_none_or(|id| players.contains(&id))
            && self
                .acted_at_bet
                .values()
                .all(|acted_at| *acted_at <= self.current_bet)
    }

    pub(crate) fn needs_action_only_from(&self, players: &HashSet<Uuid>) -> bool {
        self.needs_to_act.iter().all(|id| players.contains(id))
    }

    pub(crate) fn commitments_within(&self, contributed: &HashMap<Uuid, u32>) -> bool {
        self.committed_this_round
            .iter()
            .all(|(id, amount)| *amount <= contributed.get(id).copied().unwrap_or(0))
    }

    /// Applies a resolved action, updating committed chips, the current bet, the
    /// minimum raise, whose action remains, and the big-blind option.
    ///
    /// `live_after` is the set of players who can still act *after* this action
    /// (i.e. excluding anyone who just folded or went all-in). It is used to
    /// reopen action on a full raise.
    pub fn apply_action(
        &mut self,
        id: Uuid,
        resolved: &Resolved,
        live_after: &HashSet<Uuid>,
    ) -> Result<(), ActionError> {
        let committed = self.committed(id);
        let new_committed = committed
            .checked_add(resolved.paid)
            .ok_or(ActionError::ChipAmountOverflow)?;
        self.committed_this_round.insert(id, new_committed);

        // The actor has now acted.
        self.needs_to_act.remove(&id);

        if let Some(target) = resolved.raised_to {
            if target > self.current_bet {
                let increment = target - self.current_bet;
                self.current_bet = target;

                if resolved.is_full_raise {
                    self.last_raise_increment = increment;
                    // Full reopening: everyone else still able to act must respond
                    // with full rights.
                    self.needs_to_act = live_after.iter().copied().filter(|p| *p != id).collect();
                } else {
                    // Incomplete all-in: players who already acted must respond
                    // to the extra chips. Whether cumulative short raises have
                    // restored their raise rights is derived by `may_raise`.
                    for &p in live_after {
                        if p != id && self.acted_at_bet.contains_key(&p) {
                            self.needs_to_act.insert(p);
                        }
                    }
                }
            }
        }

        // Record the level this action reached or called. Later incomplete raises
        // are measured cumulatively from here.
        self.acted_at_bet.insert(id, self.current_bet);

        // Any raise pre-flop, or the big blind acting on its option, ends the option.
        if resolved.raised_to.is_some() || self.big_blind == Some(id) {
            self.bb_option_pending = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn owed_is_the_delta() {
        assert_eq!(amount_owed(2, 1), 1);
        assert_eq!(amount_owed(2, 2), 0);
        assert_eq!(amount_owed(2, 5), 0); // saturating
    }

    #[test]
    fn call_pays_only_the_delta_after_blind() {
        // Posted small blind of 1, current bet is big blind 2: owe 1, not 2.
        let r = resolve_action(PlayerAction::Call, 100, 1, 2, 2).unwrap();
        assert_eq!(r.paid, 1);
        assert!(!r.all_in);
    }

    #[test]
    fn call_then_raise_then_call_accumulates() {
        // Already committed 2; a raise made current_bet 6: owe 4, not 6.
        let r = resolve_action(PlayerAction::Call, 100, 2, 6, 4).unwrap();
        assert_eq!(r.paid, 4);
    }

    #[test]
    fn short_call_is_all_in_and_does_not_change_bet() {
        // Owe 10 but only have 4 chips: all-in for 4, bet unchanged.
        let r = resolve_action(PlayerAction::Call, 4, 0, 10, 10).unwrap();
        assert_eq!(r.paid, 4);
        assert!(r.all_in);
        assert_eq!(r.raised_to, None);
    }

    #[test]
    fn check_illegal_when_owing() {
        assert_eq!(
            resolve_action(PlayerAction::Check, 100, 0, 2, 2),
            Err(ActionError::CannotCheck)
        );
    }

    #[test]
    fn raise_below_min_is_rejected() {
        // current_bet 10, min increment 10 => min raise-to 20. Raising to 15 is illegal.
        assert_eq!(
            resolve_action(PlayerAction::RaiseTo(15), 100, 0, 10, 10),
            Err(ActionError::RaiseTooSmall)
        );
    }

    #[test]
    fn raise_at_min_is_a_full_raise() {
        let r = resolve_action(PlayerAction::RaiseTo(20), 100, 0, 10, 10).unwrap();
        assert_eq!(r.paid, 20);
        assert!(r.is_full_raise);
        assert_eq!(r.raised_to, Some(20));
    }

    #[test]
    fn all_in_below_min_raises_bet_without_reopening() {
        // current_bet 10, min raise-to 20. All-in for 15 total raises the bet to
        // 15 but is not a full raise.
        let r = resolve_action(PlayerAction::AllIn, 15, 0, 10, 10).unwrap();
        assert_eq!(r.paid, 15);
        assert_eq!(r.raised_to, Some(15));
        assert!(!r.is_full_raise);
    }

    #[test]
    fn all_in_above_min_is_a_full_raise() {
        let r = resolve_action(PlayerAction::AllIn, 25, 0, 10, 10).unwrap();
        assert_eq!(r.raised_to, Some(25));
        assert!(r.is_full_raise);
    }

    #[test]
    fn opening_bet_minimum_is_big_blind() {
        // Post-flop, current_bet 0, last_raise_increment seeded to big blind (2):
        // min raise-to is 2; a bet to 1 is illegal.
        assert_eq!(
            resolve_action(PlayerAction::RaiseTo(1), 100, 0, 0, 2),
            Err(ActionError::RaiseTooSmall)
        );
        assert!(resolve_action(PlayerAction::RaiseTo(2), 100, 0, 0, 2).is_ok());
    }

    // --- BettingRound state machine ---

    /// Applies an action by id, recomputing the live set from a mutable closure.
    fn step(
        round: &mut BettingRound,
        actor: Uuid,
        action: PlayerAction,
        chips: u32,
        live_after: &HashSet<Uuid>,
    ) -> Resolved {
        let resolved = resolve_action(
            action,
            chips,
            round.committed(actor),
            round.current_bet,
            round.last_raise_increment,
        )
        .expect("legal action in test");
        round
            .apply_action(actor, &resolved, live_after)
            .expect("legal action in test");
        resolved
    }

    #[test]
    fn call_around_closes_preflop_with_bb_option() {
        let (utg, sb, bb) = (id(1), id(2), id(3));
        let actors = [utg, sb, bb];
        let committed = HashMap::from([(sb, 1), (bb, 2)]);
        let mut round = BettingRound::new(&actors, 2, 2, committed, Some(bb));

        let live = HashSet::from([utg, sb, bb]);
        step(&mut round, utg, PlayerAction::Call, 100, &live);
        assert!(!round.is_closed());
        step(&mut round, sb, PlayerAction::Call, 100, &live);
        assert!(!round.is_closed(), "bb still has the option");
        step(&mut round, bb, PlayerAction::Check, 100, &live);
        assert!(round.is_closed(), "bb checked its option -> round closes");
    }

    #[test]
    fn raise_reopens_action_for_earlier_callers() {
        let (utg, sb, bb) = (id(1), id(2), id(3));
        let actors = [utg, sb, bb];
        let committed = HashMap::from([(sb, 1), (bb, 2)]);
        let mut round = BettingRound::new(&actors, 2, 2, committed, Some(bb));
        let live = HashSet::from([utg, sb, bb]);

        step(&mut round, utg, PlayerAction::Call, 100, &live); // utg calls 2
        step(&mut round, sb, PlayerAction::RaiseTo(6), 100, &live); // sb raises to 6
                                                                    // utg must act again; bb option is gone.
        assert!(round.needs_to_act(utg));
        assert!(round.needs_to_act(bb));
        assert!(!round.is_closed());
        step(&mut round, bb, PlayerAction::Call, 100, &live);
        step(&mut round, utg, PlayerAction::Call, 100, &live);
        assert!(round.is_closed());
    }

    #[test]
    fn incomplete_all_in_requires_response_but_forbids_reraise() {
        let (a, b, c) = (id(1), id(2), id(3));
        let actors = [a, b, c];
        // Post-flop: current_bet 0, increment = big blind 2.
        let mut round = BettingRound::new(&actors, 0, 2, HashMap::new(), None);
        let live = HashSet::from([a, b, c]);

        step(&mut round, a, PlayerAction::RaiseTo(10), 100, &live); // A bets 10 (full)
        step(&mut round, b, PlayerAction::Call, 100, &live); // B calls 10
                                                             // C all-in for 15 total: increment 5 < 10 => incomplete raise.
        let live_after_c = HashSet::from([a, b]); // C now all-in
        step(&mut round, c, PlayerAction::AllIn, 15, &live_after_c);

        assert_eq!(round.current_bet, 15);
        // A and B must respond (owe 5 more) but may not re-raise.
        assert!(round.needs_to_act(a));
        assert!(round.needs_to_act(b));
        assert!(!round.may_raise(a));
        assert!(!round.may_raise(b));
    }

    #[test]
    fn cumulative_incomplete_all_ins_reopen_raising() {
        let (a, b, c, d) = (id(1), id(2), id(3), id(4));
        let actors = [a, b, c, d];
        let mut round = BettingRound::new(&actors, 0, 2, HashMap::new(), None);
        let all_live = HashSet::from([a, b, c, d]);

        step(&mut round, a, PlayerAction::RaiseTo(10), 100, &all_live);
        step(&mut round, b, PlayerAction::Call, 100, &all_live);
        step(
            &mut round,
            c,
            PlayerAction::AllIn,
            15,
            &HashSet::from([a, b, d]),
        );
        assert!(!round.may_raise(a));
        step(
            &mut round,
            d,
            PlayerAction::AllIn,
            20,
            &HashSet::from([a, b]),
        );

        assert_eq!(round.current_bet, 20);
        assert!(round.may_raise(a));
        assert!(round.may_raise(b));
    }

    #[test]
    fn player_who_has_not_acted_keeps_raise_rights_after_short_all_in() {
        let (a, b, c) = (id(1), id(2), id(3));
        let actors = [a, b, c];
        let mut round = BettingRound::new(&actors, 0, 2, HashMap::new(), None);
        let all_live = HashSet::from([a, b, c]);

        step(&mut round, a, PlayerAction::RaiseTo(10), 100, &all_live);
        step(
            &mut round,
            b,
            PlayerAction::AllIn,
            15,
            &HashSet::from([a, c]),
        );

        assert!(round.needs_to_act(c));
        assert!(round.may_raise(c));
    }

    #[test]
    fn chip_math_is_safe_at_u32_max() {
        let actions = legal_actions(u32::MAX, u32::MAX, u32::MAX - 1, 10, true);
        assert!(!actions
            .iter()
            .any(|action| matches!(action, LegalAction::AllIn(_))));
        assert!(!actions
            .iter()
            .any(|action| matches!(action, LegalAction::RaiseTo { .. })));

        assert_eq!(
            resolve_action(PlayerAction::AllIn, u32::MAX, u32::MAX, u32::MAX - 1, 10,),
            Err(ActionError::ChipAmountOverflow)
        );
    }

    #[test]
    fn restored_round_rejects_ghost_player_ids() {
        let a = id(1);
        let ghost = id(2);
        let mut round = BettingRound::new(&[a], 0, 2, HashMap::new(), None);
        round.needs_to_act.insert(ghost);

        assert!(!round.references_only(&HashSet::from([a])));
    }
}
