use std::io::{self, Write};
use std::process;

mod agents;
mod hand_history;
mod persistence;
mod prompts;
mod render;
mod texas_hold_em_game;

fn main() {
    // The menu is interactive chrome, so it goes to stderr; stdout is reserved for
    // the PokerStars hand history emitted during play.
    eprintln!("Welcome to the casino!");
    eprintln!("Enter 'q' at anytime to quit.\n");

    loop {
        select_game();
    }
}

fn select_game() {
    eprintln!("Games");
    eprintln!("1. Texas hold 'em");
    eprintln!("Select a game by number, or press Enter to play Texas hold 'em.");
    eprint!("Game: ");
    io::stderr().flush().expect("Failed to flush stderr.");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let trimmed_input = input.trim().to_lowercase().replace(['\'', ' '], "");

    match trimmed_input.as_str() {
        "q" | "quit" => {
            eprintln!("Quitting game.");
            process::exit(0);
        }
        // Default (Enter), the number, or the name all launch the only game.
        "" | "1" | "texasholdem" | "holdem" => {
            eprintln!();
            texas_hold_em_game::play_game();
        }
        _ => eprintln!("Invalid input. Enter 1 (or press Enter) to play, or 'q' to quit.\n"),
    }
}
