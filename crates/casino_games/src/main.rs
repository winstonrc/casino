use std::io::{self, Write};
use std::process;

mod persistence;
mod texas_hold_em_game;

fn main() {
    println!("Welcome to the casino!");
    println!("Enter 'q' at anytime to quit.\n");

    loop {
        select_game();
    }
}

fn select_game() {
    println!("Games");
    println!("1. Texas hold 'em");
    println!("Select a game by number, or press Enter to play Texas hold 'em.");
    print!("Game: ");
    io::stdout().flush().expect("Failed to flush stdout.");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let trimmed_input = input.trim().to_lowercase().replace(['\'', ' '], "");

    match trimmed_input.as_str() {
        "q" | "quit" => {
            println!("Quitting game.");
            process::exit(0);
        }
        // Default (Enter), the number, or the name all launch the only game.
        "" | "1" | "texasholdem" | "holdem" => {
            println!();
            texas_hold_em_game::play_game();
        }
        _ => println!("Invalid input. Enter 1 (or press Enter) to play, or 'q' to quit.\n"),
    }
}
