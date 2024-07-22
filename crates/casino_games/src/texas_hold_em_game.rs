use std::io::{self, Write};
use std::process;

use casino_poker::games::texas_hold_em::TexasHoldEm;
use casino_poker::player::Player;

const MINIMUM_CHIPS_BUY_IN_AMOUNT: u32 = 100;
// 10 is the recommended maximum number of players at a table, so it is the default.
const MAXIMUM_PLAYERS_COUNT: usize = 10;
const CURRENCY: &str = "USD";

pub fn play_game() {
    println!("**********************");
    println!("* ♠ Texas hold 'em ♠ *");
    println!("**********************");

    let (small_blind_amount, big_blind_amount) = choose_table();

    let mut texas_hold_em_1_3_no_limit = TexasHoldEm::new(
        MINIMUM_CHIPS_BUY_IN_AMOUNT,
        MAXIMUM_PLAYERS_COUNT,
        small_blind_amount,
        big_blind_amount,
    );

    let user_name = get_player_name_prompt();
    let mut player1 = texas_hold_em_1_3_no_limit.new_player(&user_name);
    add_player_prompt(&mut texas_hold_em_1_3_no_limit, &mut player1);
    let mut player2 = texas_hold_em_1_3_no_limit.new_player_with_chips("Player 2", 100);
    add_player_prompt(&mut texas_hold_em_1_3_no_limit, &mut player2);
    let mut player3 = texas_hold_em_1_3_no_limit.new_player_with_chips("Player 3", 100);
    add_player_prompt(&mut texas_hold_em_1_3_no_limit, &mut player3);
    let mut player4 = texas_hold_em_1_3_no_limit.new_player_with_chips("Player 4", 100);
    add_player_prompt(&mut texas_hold_em_1_3_no_limit, &mut player4);
    let mut player5 = texas_hold_em_1_3_no_limit.new_player_with_chips("Player 5", 100);
    add_player_prompt(&mut texas_hold_em_1_3_no_limit, &mut player5);
    let mut player6 = texas_hold_em_1_3_no_limit.new_player_with_chips("Player 6", 100);
    add_player_prompt(&mut texas_hold_em_1_3_no_limit, &mut player6);

    println!();

    play_tournament(&mut texas_hold_em_1_3_no_limit);
}

fn choose_table() -> (u32, u32) {
    loop {
        println!("Tables");
        println!("1. 1/3 No-limit");
        println!("2. 2/5 No-limit");
        println!("3. Custom");
        println!("Enter the table number.");
        print!("Table: ");
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
                return (1, 3);
            }
            "2" => {
                return (2, 5);
            }
            "3" => {
                loop {
                    println!("Enter the amounts for the small and big blinds.");
                    println!("Format: <small_blind> <big_blind>");

                    print!("Amounts: ");
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

                    let mut numbers = trimmed_input.split_whitespace();
                    
                    // Attempt to parse the first number
                    let number1: Result<u32, _> = numbers.next().unwrap_or("").parse();
                    
                    // Attempt to parse the second number
                    let number2: Result<u32, _> = numbers.next().unwrap_or("").parse();

                    // Check if parsing was successful for both numbers
                    if let (Ok(small_blind), Ok(big_blind)) = (number1, number2) {
                        return (small_blind, big_blind);
                    } else {
                        println!("Error: Please enter two valid numbers.");
                    }
                };
            }
            _ => println!("Invalid input. Please enter the number of a table listed above or enter 'q' to quit the game.\n"),            
        }
    }
}

fn get_player_name_prompt() -> String {
    loop {
        print!("Enter your name: ");
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

        let mut name = String::from(trimmed_input);

        if name.as_str() == "" {
            name = String::from("Player 1");
        }

        println!("\nWelcome {}! Are you happy with this name?", name);
        print!("yes/no [Y/n]: ");
        io::stdout().flush().expect("Failed to flush stdout.");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let trimmed_input = input.trim();

        println!();

        match trimmed_input.to_lowercase().as_str() {
            "q" => {
                println!("Quitting game.");
                process::exit(0);
            }
            "quit" => {
                println!("Quitting game.");
                process::exit(0);
            }
            "n" => {
                continue;
            }
            "no" => {
                continue;
            }
            "y" => {
                break name;
            }
            "yes" => {
                break name;
            }
            "" => {
                break name;
            }
            _ => println!("Invalid input. Please enter 'y' or 'n' or enter 'q' to quit the game."),
        }
    }
}

fn add_player_prompt(game: &mut TexasHoldEm, player: &mut Player) {
    if player.chips < MINIMUM_CHIPS_BUY_IN_AMOUNT {
        while player.chips < MINIMUM_CHIPS_BUY_IN_AMOUNT {
            println!("You do not have enough chips to play at this table.");
            println!("Current chips amount: {}", player.chips);
            println!(
                "Required chips amount: {}",
                MINIMUM_CHIPS_BUY_IN_AMOUNT
            );
            println!(
                "Additional chips needed: {}",
                MINIMUM_CHIPS_BUY_IN_AMOUNT - player.chips
            );

            buy_chips_prompt(player);
        }
    }

    match game.add_player(player.clone()) {
        Ok(()) => {}
        Err("The player does not have enough chips to play at this table.") => {
            eprintln!("The player does not have enough chips to play at this table.")
        }
        Err(_) => {
            eprintln!("Unable to add player to the table. Reason unknown.");
        }
    }
}

fn buy_chips_prompt(player: &mut Player) {
    println!("How many chips would you like to buy?");

    loop {
        print!("Amount ({}) of chips to buy: ", CURRENCY);
        io::stdout().flush().expect("Failed to flush stdout.");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let trimmed_input = input.trim();

        if trimmed_input.to_lowercase() == "q" || trimmed_input.to_lowercase() == "quit" {
            println!("No chips were purchased. Quitting game.");
            process::exit(0);
        }

        match trimmed_input.parse::<u32>() {
            Ok(chips) => {
                player.add_chips(chips);
                println!("You purchased {} {} worth of chips.\n", CURRENCY, chips);
                break;
            }
            Err(_) => println!("Error: Not a valid number."),
        }
    }
}

fn play_tournament(game: &mut TexasHoldEm) {
    while !game.check_for_game_over() {
        game.print_leaderboard();
        game.play_round();
        game.remove_losers();
        game.check_for_game_over();
        if game.check_for_game_over() {
            process::exit(0);
        }

        loop {
            println!("\nPlay another hand?");
            print!("yes/no [Y/n]: ");
            io::stdout().flush().expect("Failed to flush stdout.");

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");
            let trimmed_input = input.trim();

            match trimmed_input.to_lowercase().as_str() {
                "q" => {
                    println!("Quitting game.");
                    process::exit(0);
                }
                "quit" => {
                    println!("Quitting game.");
                    process::exit(0);
                }
                "n" => {
                    game.end_game();
                    println!("Game ended.\n");
                    break;
                }
                "no" => {
                    game.end_game();
                    println!("Game ended.\n");
                    break;
                }
                "y" => {
                    println!();
                    break;
                }
                "yes" => {
                    println!();
                    break;
                }
                "" => {
                    println!();
                    break;
                }
                _ => println!(
                    "Invalid input. Please enter 'y' or 'n' or enter 'q' to quit the game."
                ),
            }
        
        }

        game.check_for_game_over();
    }
}
