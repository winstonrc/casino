//! Small rendering helpers. The in-hand narration is the PokerStars hand history
//! (see [`crate::hand_history`], which writes to stdout); the helpers here are
//! between-hand chrome (leaderboard, buy-ins, game over) written to **stderr** so
//! they never pollute the captured history.

use casino_poker::casino_cards::card::Card;
use casino_poker::games::texas_hold_em::TexasHoldEm;

/// Formats a list of cards as a space-separated string, honoring the current card
/// display style (PokerStars codes or glyphs).
pub fn cards_to_string(cards: &[Card]) -> String {
    cards
        .iter()
        .map(|card| card.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

// --- Between-hand chrome (UI-side; written to stderr, not the history) ---

/// Print the leaderboard, highest chip count first.
pub fn render_leaderboard(game: &TexasHoldEm) {
    let mut players: Vec<_> = game
        .seats()
        .iter()
        .filter_map(|id| game.player(id))
        .collect();
    players.sort_by_key(|p| std::cmp::Reverse(p.chips));

    eprintln!("***************");
    eprintln!("* LEADERBOARD *");
    eprintln!("***************");
    for player in &players {
        let plural = if player.chips == 1 { "" } else { "s" };
        eprintln!("{}: {} chip{plural}", player.name, player.chips);
    }
    eprintln!();
}

pub fn render_buy_in(name: &str, chips: u32) {
    eprintln!("{name} bought in with {chips} chips. Good luck!");
}

pub fn render_removed(name: &str) {
    eprintln!("{name} is out of chips and was removed from the game.");
}

/// Print the game-over message appropriate to how many players remain.
pub fn render_game_over(game: &TexasHoldEm) {
    if game.seats().is_empty() {
        eprintln!("No players remaining. Game over!");
    } else {
        eprintln!("One player remaining. Game over!");
    }
}
