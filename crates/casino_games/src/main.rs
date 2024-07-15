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
    println!("1. Texas hold 'em");
    println!("Enter the number of the game you would like to play.");
    print!("Game: ");
    io::stdout().flush().expect("Failed to flush stdout.");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    let trimmed_input = input.trim();

    if trimmed_input.to_lowercase() == "q" || trimmed_input.to_lowercase() == "quit" {
        println!("Quitting game.");
        process::exit(0);
    }

    match trimmed_input.to_lowercase().as_str() {
        "q" => {
            println!("Quitting game.");
            process::exit(0);
        }
        "quit" => {
            println!("Quitting game.");
            process::exit(0);
        }
        "1" => {
            println!();
            texas_hold_em_game::play_game();
        }
        _ => println!("Invalid input. Please enter the number of a game listed above or enter 'q' to quit the game.\n"),
    }
}
