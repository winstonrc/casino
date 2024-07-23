use std::io::{self, Write};
use std::process;

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
    println!("Texas hold 'em");
    println!("Which game would you like to play?");
    print!("Game: ");
    io::stdout().flush().expect("Failed to flush stdout.");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let trimmed_input = input
        .trim()
        .to_lowercase()
        .replace("'", "")
        .replace(" ", "");

    match trimmed_input.as_str() {
        "q" => {
            println!("Quitting game.");
            process::exit(0);
        }
        "quit" => {
            println!("Quitting game.");
            process::exit(0);
        }
        "texasholdem" | "holdem" => {
            println!();
            texas_hold_em_game::play_game();
        }
        _ => println!(
            "Invalid input. Please enter the name of a game listed above or enter 'q' to quit.\n"
        ),
    }
}
