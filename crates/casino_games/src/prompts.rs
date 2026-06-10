//! Terminal input prompts and profile selection.

use std::io::{self, Write};
use std::process;

use casino_poker::agent::AgentError;
use casino_poker::player::Player;

use crate::persistence::{self, Profile};

const CURRENCY: &str = "USD";

/// Reads a line from stdin, returning [`AgentError::Eof`] at end of input.
pub fn read_line() -> Result<String, AgentError> {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => Err(AgentError::Eof),
        Ok(_) => Ok(input),
        Err(_) => Err(AgentError::Eof),
    }
}

pub fn get_player_name_prompt() -> String {
    loop {
        eprint!("Enter your name: ");
        io::stderr().flush().expect("Failed to flush stderr.");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            process::exit(0);
        }
        let trimmed_input = input.trim();

        if trimmed_input.eq_ignore_ascii_case("q") || trimmed_input.eq_ignore_ascii_case("quit") {
            eprintln!("Quitting game.");
            process::exit(0);
        }

        let name = if trimmed_input.is_empty() {
            String::from("Player 1")
        } else {
            String::from(trimmed_input)
        };

        eprintln!("\nWelcome {name}! Are you happy with this name?");
        eprint!("yes/no [Y/n]: ");
        io::stderr().flush().expect("Failed to flush stderr.");

        let mut confirm = String::new();
        if io::stdin().read_line(&mut confirm).is_err() {
            process::exit(0);
        }

        match confirm.trim().to_lowercase().as_str() {
            "q" | "quit" => {
                eprintln!("Quitting game.");
                process::exit(0);
            }
            "n" | "no" => continue,
            _ => return name,
        }
    }
}

pub fn choose_table() -> (u32, u32) {
    loop {
        eprintln!("Tables");
        eprintln!("1. 1/3 No-limit");
        eprintln!("2. 2/5 No-limit");
        eprintln!("3. Custom");
        eprint!("Table: ");
        io::stderr().flush().expect("Failed to flush stderr.");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            process::exit(0);
        }
        let trimmed_input = input.trim();

        match trimmed_input.to_lowercase().as_str() {
            "q" | "quit" => {
                eprintln!("Quitting game.");
                process::exit(0);
            }
            "1" => return (1, 3),
            "2" => return (2, 5),
            "3" => loop {
                eprintln!("Enter the small and big blind amounts.");
                eprint!("Format <small_blind> <big_blind>: ");
                io::stderr().flush().expect("Failed to flush stderr.");

                let mut custom = String::new();
                if io::stdin().read_line(&mut custom).is_err() {
                    process::exit(0);
                }
                if custom.trim().eq_ignore_ascii_case("q") {
                    process::exit(0);
                }

                let mut numbers = custom.split_whitespace();
                let small: Result<u32, _> = numbers.next().unwrap_or("").parse();
                let big: Result<u32, _> = numbers.next().unwrap_or("").parse();
                if let (Ok(small), Ok(big)) = (small, big) {
                    if small > 0 && big > small {
                        return (small, big);
                    }
                }
                eprintln!("Please enter two numbers where the big blind exceeds the small blind.");
            },
            _ => eprintln!("Invalid input. Enter 1, 2, 3, or 'q' to quit.\n"),
        }
    }
}

pub fn buy_chips_prompt(player: &mut Player) {
    eprintln!("How many chips would you like to buy?");
    loop {
        eprint!("Amount ({CURRENCY}) of chips to buy: ");
        io::stderr().flush().expect("Failed to flush stderr.");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            process::exit(0);
        }
        let trimmed_input = input.trim();

        if trimmed_input.eq_ignore_ascii_case("q") || trimmed_input.eq_ignore_ascii_case("quit") {
            eprintln!("No chips were purchased. Quitting game.");
            process::exit(0);
        }

        match trimmed_input.parse::<u32>() {
            Ok(chips) => {
                player.add_chips(chips);
                eprintln!("You purchased {CURRENCY} {chips} worth of chips.\n");
                return;
            }
            Err(_) => eprintln!("Error: Not a valid number."),
        }
    }
}

/// Asks how cards should be rendered, defaulting to the current preference so
/// pressing Enter keeps it. Glyphs look nicer where supported; text is portable.
pub fn choose_card_style(current: bool) -> bool {
    let current_label = if current { "glyphs" } else { "text" };
    eprintln!(
        "Card display: (t)ext like As (parseable), or (g)lyphs like 🂡 (nicer, but tiny in some terminals)."
    );
    eprint!("Choose [t/g] (Enter keeps {current_label}): ");
    io::stderr().flush().expect("Failed to flush stderr.");

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return current;
    }
    match input.trim().to_lowercase().as_str() {
        "q" | "quit" => {
            eprintln!("Quitting game.");
            process::exit(0);
        }
        "g" | "glyph" | "glyphs" => true,
        "t" | "text" => false,
        _ => current,
    }
}

/// Loads the saved profile (offering to resume it) or creates a fresh one.
pub fn load_or_create_profile() -> Profile {
    if let Some(profile) = persistence::load() {
        eprintln!(
            "Welcome back, {}! You have {} chips ({} hands played, {} won).",
            profile.name, profile.chips, profile.hands_played, profile.hands_won
        );
        eprint!("Resume this profile? [Y/n]: ");
        io::stderr().flush().expect("Failed to flush stderr.");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            match input.trim().to_lowercase().as_str() {
                "n" | "no" => {} // fall through to create a new profile
                "q" | "quit" => {
                    eprintln!("Quitting game.");
                    process::exit(0);
                }
                _ => return profile,
            }
        }
    }

    let name = get_player_name_prompt();
    Profile::new(&name, 0)
}

pub fn prompt_play_another_hand() -> bool {
    eprint!("\nPlay another hand? [Y/n]: ");
    io::stderr().flush().expect("Failed to flush stderr.");

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    !matches!(
        input.trim().to_lowercase().as_str(),
        "q" | "quit" | "n" | "no"
    )
}
