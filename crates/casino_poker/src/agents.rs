//! Reusable, I/O-free computer opponents.
//!
//! These implement [`PokerAgent`] purely from a [`PlayerView`], so they work
//! with any front-end (terminal, TUI, GUI, tests).
//! They do no I/O and add no artificial delay — pacing is the front-end's job.

use rand::Rng;

use crate::agent::{AgentError, PlayerView, PokerAgent};
use crate::betting::{LegalAction, PlayerAction};
use crate::hand_rankings::{evaluate, HandCategory};
use casino_cards::card::Card;

/// A "loose" opponent that plays legal but unsophisticated poker (it ignores its
/// cards). Useful as a low-variance baseline or seated alongside stronger agents
/// so a table doesn't feel uniform.
pub struct RandomAgent;

impl PokerAgent for RandomAgent {
    fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError> {
        let mut rng = rand::thread_rng();
        let roll: f64 = rng.gen();

        let can_check = matches_any(view, |a| matches!(a, LegalAction::Check));
        let can_call = matches_any(view, |a| matches!(a, LegalAction::Call(_)));
        let raise_min = raise_min(view);

        if can_check {
            if let Some(min) = raise_min {
                if roll < 0.2 {
                    return Ok(PlayerAction::RaiseTo(min));
                }
            }
            return Ok(PlayerAction::Check);
        }

        if can_call {
            if let Some(min) = raise_min {
                if roll < 0.12 {
                    return Ok(PlayerAction::RaiseTo(min));
                }
            }
            if roll < 0.6 {
                return Ok(PlayerAction::Call);
            }
            return Ok(PlayerAction::Fold);
        }

        // Can't even call: occasionally shove, otherwise fold.
        let can_all_in = matches_any(view, |a| matches!(a, LegalAction::AllIn(_)));
        if can_all_in && roll < 0.3 {
            return Ok(PlayerAction::AllIn);
        }
        Ok(PlayerAction::Fold)
    }
}

/// An opponent that reads its cards: it estimates hand strength and weighs it
/// against the pot odds. The natural place a future model-backed agent would slot
/// in.
pub struct HeuristicAgent;

impl PokerAgent for HeuristicAgent {
    fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError> {
        let mut rng = rand::thread_rng();
        let roll: f64 = rng.gen();

        let strength = estimate_strength(view);
        // Price of calling: chips owed as a fraction of the pot after calling.
        let pot_odds = if view.amount_owed == 0 {
            0.0
        } else {
            view.amount_owed as f64 / (view.pot_total + view.amount_owed) as f64
        };

        let can_check = matches_any(view, |a| matches!(a, LegalAction::Check));
        let can_call = matches_any(view, |a| matches!(a, LegalAction::Call(_)));
        let raise_min = raise_min(view);
        let can_all_in = matches_any(view, |a| matches!(a, LegalAction::AllIn(_)));

        if can_check {
            // Free to continue: value-bet strong hands, occasionally semi-bluff.
            if let Some(min) = raise_min {
                if strength > 0.78 || (strength > 0.5 && roll < 0.35) {
                    return Ok(PlayerAction::RaiseTo(min));
                }
            }
            return Ok(PlayerAction::Check);
        }

        // Facing a bet.
        if let Some(min) = raise_min {
            if strength > 0.85 && roll < 0.7 {
                return Ok(PlayerAction::RaiseTo(min));
            }
        }

        // Call when estimated strength beats the price, with a little slack.
        if strength + 0.05 >= pot_odds {
            if can_call {
                return Ok(PlayerAction::Call);
            }
            if can_all_in && strength > 0.7 {
                return Ok(PlayerAction::AllIn);
            }
        }

        Ok(PlayerAction::Fold)
    }
}

fn matches_any(view: &PlayerView, predicate: impl Fn(&LegalAction) -> bool) -> bool {
    view.legal_actions.iter().any(predicate)
}

fn raise_min(view: &PlayerView) -> Option<u32> {
    view.legal_actions.iter().find_map(|a| match *a {
        LegalAction::RaiseTo { min, .. } => Some(min),
        _ => None,
    })
}

/// Estimates hand strength in `0.0..=1.0`. Pre-flop uses a simple hole-card
/// heuristic; post-flop it evaluates the made hand against the board.
fn estimate_strength(view: &PlayerView) -> f64 {
    if view.board.is_empty() {
        preflop_strength(view.hole)
    } else {
        let hand = evaluate(&view.hole, &view.board);
        let base = match hand.category {
            // High-card strength scales with the top card.
            HandCategory::HighCard => 0.10 + (hand.tiebreak[0] as f64 / 14.0) * 0.20,
            HandCategory::Pair => 0.55,
            HandCategory::TwoPair => 0.72,
            HandCategory::ThreeOfAKind => 0.82,
            HandCategory::Straight => 0.88,
            HandCategory::Flush => 0.92,
            HandCategory::FullHouse => 0.96,
            HandCategory::FourOfAKind => 0.99,
            HandCategory::StraightFlush => 1.0,
        };
        base.clamp(0.0, 1.0)
    }
}

fn preflop_strength(hole: [Card; 2]) -> f64 {
    let hi = hole[0].rank.value().max(hole[1].rank.value()) as f64;
    let lo = hole[0].rank.value().min(hole[1].rank.value()) as f64;
    let is_pair = hole[0].rank == hole[1].rank;
    let is_suited = hole[0].suit == hole[1].suit;

    let mut strength = (hi + lo) / 56.0; // ~0.14..=0.5 from the card ranks
    if is_pair {
        strength += 0.35 + hi / 100.0; // pairs are strong, higher pairs stronger
    } else {
        if is_suited {
            strength += 0.06;
        }
        if (hi - lo) <= 2.0 {
            strength += 0.05; // connected cards make straights
        }
    }
    strength.clamp(0.0, 1.0)
}
