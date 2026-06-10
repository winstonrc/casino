//! The human, terminal-driven agent. (Computer agents live in
//! `casino_poker::agents`.)
//!
//! All prompts are written to **stderr**, keeping stdout a clean PokerStars hand
//! history (see [`crate::hand_history`]); input is read from stdin.

use std::io::{self, Write};

use casino_poker::agent::{AgentError, PlayerAction, PlayerView, PokerAgent};
use casino_poker::betting::LegalAction;

use crate::prompts::read_line;
use crate::render::cards_to_string;

/// A human player driven by stdin prompts.
pub struct HumanAgent;

impl PokerAgent for HumanAgent {
    fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError> {
        let can_check = view
            .legal_actions
            .iter()
            .any(|a| matches!(a, LegalAction::Check));
        let call_amount = view.legal_actions.iter().find_map(|a| match *a {
            LegalAction::Call(amount) => Some(amount),
            _ => None,
        });
        let raise_range = view.legal_actions.iter().find_map(|a| match *a {
            LegalAction::RaiseTo { min, max } => Some((min, max)),
            _ => None,
        });
        let all_in_total = view.legal_actions.iter().find_map(|a| match *a {
            LegalAction::AllIn(total) => Some(total),
            _ => None,
        });

        eprintln!("\n-- Your turn --");
        eprintln!("Your hand: {}", cards_to_string(&view.hole));
        if !view.board.is_empty() {
            eprintln!("Board: {}", cards_to_string(&view.board));
        }
        eprintln!(
            "Pot: {} | Your chips: {} | To call: {}",
            view.pot_total, view.chips, view.amount_owed
        );

        let action = loop {
            // Each action can be chosen by a single-letter shortcut or its full
            // word. Check uses `x` so `c` is unambiguously call.
            let mut menu: Vec<String> = vec!["(f)old".to_string()];
            if can_check {
                menu.push("(x) check".to_string());
            } else if let Some(amount) = call_amount {
                menu.push(format!("(c)all {amount}"));
            }
            if let Some((min, max)) = raise_range {
                // Raise amounts are the total to commit this street ("raise to").
                menu.push(format!("(r)aise to {min}-{max}"));
            }
            if let Some(total) = all_in_total {
                menu.push(format!("(a)ll-in ({total})"));
            }
            eprint!("Action ({}): ", menu.join(", "));
            io::stderr().flush().expect("Failed to flush stderr.");

            let input = read_line()?;
            let lowered = input.trim().to_lowercase();
            let mut tokens = lowered.split_whitespace();

            match tokens.next() {
                Some("q") | Some("quit") => return Err(AgentError::Quit),
                Some("f") | Some("fold") => break PlayerAction::Fold,
                Some("x") | Some("check") if can_check => break PlayerAction::Check,
                Some("c") | Some("call") if call_amount.is_some() => break PlayerAction::Call,
                // Typing call when the stack can't fully cover the bet is a short all-in.
                Some("c") | Some("call") if all_in_total.is_some() && !can_check => {
                    break PlayerAction::AllIn
                }
                // `c` when checking is free: there's nothing to call.
                Some("c") | Some("call") if can_check => {
                    eprintln!("Nothing to call — type 'x' to check.");
                }
                Some("a") | Some("all") | Some("allin") | Some("all-in")
                    if all_in_total.is_some() =>
                {
                    break PlayerAction::AllIn
                }
                Some("r") | Some("raise") if raise_range.is_some() => {
                    let (min, max) = raise_range.unwrap();
                    // Accept both "raise 50" and "raise to 50".
                    let mut amount_token = tokens.next();
                    if amount_token == Some("to") {
                        amount_token = tokens.next();
                    }
                    let to = match amount_token.and_then(|s| s.parse::<u32>().ok()) {
                        Some(to) => to,
                        None => {
                            eprint!("Raise to how much? ");
                            io::stderr().flush().expect("Failed to flush stderr.");
                            match read_line()?.trim().parse::<u32>() {
                                Ok(to) => to,
                                Err(_) => {
                                    eprintln!("Please enter a whole number.");
                                    continue;
                                }
                            }
                        }
                    };
                    if to < min || to > max {
                        eprintln!("You can raise to between {min} and {max} chips.");
                        continue;
                    }
                    break PlayerAction::RaiseTo(to);
                }
                _ => eprintln!("Invalid action. Try again, or type 'quit'."),
            }
        };
        // Blank line (stderr only) separates the prompt block from the history
        // that resumes on stdout, bracketing it with the blank before "-- Your turn --".
        eprintln!();
        Ok(action)
    }
}
