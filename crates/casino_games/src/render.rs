//! Terminal rendering: an in-hand [`GameObserver`] plus between-hand/private
//! rendering helpers that read engine state directly.

use std::thread::sleep;
use std::time::Duration;

use casino_poker::agent::Street;
use casino_poker::casino_cards::card::Card;
use casino_poker::events::{ActionView, Blind, GameEvent, GameObserver, PotKind};
use casino_poker::games::texas_hold_em::TexasHoldEm;
use casino_poker::uuid::Uuid;

/// Renders the engine's in-hand [`GameEvent`]s to stdout. Owned by the engine via
/// `set_observer`. It paces opponent actions slightly so they're readable as they
/// scroll by (the user's own actions are not delayed).
pub struct TerminalRenderer {
    user_name: String,
}

impl TerminalRenderer {
    pub fn new(user_name: String) -> Self {
        Self { user_name }
    }
}

impl GameObserver for TerminalRenderer {
    fn notify(&mut self, event: &GameEvent) {
        match event {
            GameEvent::HandStarted { dealer } => println!("{dealer} is the dealer."),
            GameEvent::BlindPosted {
                player,
                blind,
                amount,
                all_in,
            } => {
                let kind = match blind {
                    Blind::Small => "small",
                    Blind::Big => "big",
                };
                let plural = if *amount == 1 { "" } else { "s" };
                let allin = if *all_in { " (all in)" } else { "" };
                println!("{player} posts the {kind} blind: {amount} chip{plural}{allin}.");
            }
            GameEvent::ActionTaken { player, action } => {
                println!("{}", describe_action(player, action));
                if player != &self.user_name {
                    sleep(Duration::from_millis(500));
                }
            }
            GameEvent::StreetDealt { street, board, pot } => {
                println!("** {} **", street_label(*street));
                println!("Board: {}", cards_to_string(board));
                println!("Pot: {pot}\n");
            }
            GameEvent::UncalledBetReturned { player, amount } => {
                println!("{player} gets back {amount} uncalled.");
            }
            GameEvent::ShowdownReveal {
                player,
                cards,
                hand,
            } => {
                println!("{player} shows {} ({hand}).", cards_to_string(cards));
            }
            GameEvent::PotAwarded {
                player,
                amount,
                hand,
                pot,
            } => {
                let from = match pot {
                    Some(PotKind::Main) => " from the main pot".to_string(),
                    Some(PotKind::Side(n)) => format!(" from side pot {n}"),
                    None => String::new(),
                };
                match hand {
                    Some(category) => {
                        println!("{player} wins {amount} chips{from} with {category}.")
                    }
                    None => println!("{player} wins {amount} chips{from}."),
                }
            }
        }
    }
}

fn describe_action(player: &str, action: &ActionView) -> String {
    let all_in_note = |all_in: bool| if all_in { " and is all in" } else { "" };
    match action {
        ActionView::Folded => format!("{player} folds."),
        ActionView::Checked => format!("{player} checks."),
        ActionView::Called { amount, all_in } => {
            format!("{player} calls {amount}{}.", all_in_note(*all_in))
        }
        ActionView::Bet { amount, all_in } => {
            format!("{player} bets {amount}{}.", all_in_note(*all_in))
        }
        ActionView::Raised { to, all_in } => {
            format!("{player} raises to {to}{}.", all_in_note(*all_in))
        }
    }
}

fn street_label(street: Street) -> &'static str {
    match street {
        // `StreetDealt` is only emitted for the flop, turn, and river, so the
        // pre-flop arm is never actually rendered.
        Street::Preflop => "PRE-FLOP",
        Street::Flop => "FLOP",
        Street::Turn => "TURN",
        Street::River => "RIVER",
    }
}

/// Formats a list of cards as a space-separated string, honoring the current card
/// display style (text or glyphs).
pub fn cards_to_string(cards: &[Card]) -> String {
    cards
        .iter()
        .map(|card| card.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

// --- Between-hand and private rendering (UI-side; reads engine getters) ---

/// Print the leaderboard, highest chip count first.
pub fn render_leaderboard(game: &TexasHoldEm) {
    let mut players: Vec<_> = game
        .seats()
        .iter()
        .filter_map(|id| game.player(id))
        .collect();
    players.sort_by_key(|p| std::cmp::Reverse(p.chips));

    println!("***************");
    println!("* LEADERBOARD *");
    println!("***************");
    for player in &players {
        let plural = if player.chips == 1 { "" } else { "s" };
        println!("{}: {} chip{plural}", player.name, player.chips);
    }
    println!();
}

/// Print the user's own (private) hole cards.
pub fn render_your_hand(game: &TexasHoldEm, user_id: &Uuid) {
    if let Some(hand) = game.player_hand(user_id) {
        println!("\nYour hand: {}\n", cards_to_string(&hand.cards));
    }
}

pub fn render_buy_in(name: &str, chips: u32) {
    println!("{name} bought in with {chips} chips. Good luck!");
}

pub fn render_removed(name: &str) {
    println!("{name} is out of chips and was removed from the game.");
}

/// Print the game-over message appropriate to how many players remain.
pub fn render_game_over(game: &TexasHoldEm) {
    if game.seats().is_empty() {
        println!("No players remaining. Game over!");
    } else {
        println!("One player remaining. Game over!");
    }
}
