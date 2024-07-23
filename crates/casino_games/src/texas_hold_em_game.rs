use std::io::{self, Write};
use std::process;

use casino_poker::casino_cards::hand::Hand;
use casino_poker::games::texas_hold_em::TexasHoldEm;
use casino_poker::player::Player;

const MINIMUM_CHIPS_BUY_IN_AMOUNT: u32 = 100;
// 10 is the recommended maximum number of players at a table, so it is the default.
const MAXIMUM_PLAYERS_COUNT: usize = 10;
const CURRENCY: &str = "USD";

pub struct TexasHoldEmGame {
    game: TexasHoldEm,
    user: Player,
}

impl TexasHoldEmGame {
    fn new(game: TexasHoldEm, user: Player) -> Self {
        Self { game, user }
    }

    fn add_player_prompt(&mut self, player: &mut Player) {
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
    
                self.buy_chips_prompt(player);
            }
        }
    
        match self.game.add_player(player.clone()) {
            Ok(()) => {}
            Err("The player does not have enough chips to play at this table.") => {
                eprintln!("The player does not have enough chips to play at this table.")
            }
            Err(_) => {
                eprintln!("Unable to add player to the table. Reason unknown.");
            }
        }
    }
    
    fn buy_chips_prompt(&self, player: &mut Player) {
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

    fn play_tournament(&mut self) {
        while !self.game.check_for_game_over() {
            self.game.print_leaderboard();
            self.play_round();
            self.game.remove_losers();
            self.game.check_for_game_over();

            if self.game.check_for_game_over() {
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
                        self.game.end_game();
                        println!("Game ended.\n");
                        break;
                    }
                    "no" => {
                        self.game.end_game();
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

            self.game.check_for_game_over();
        }
    }

    // todo: implement betting system
    // todo: implement folding
    // todo: implement side pot correctly
    // todo: implement hand timer
    /// Play a single round.
    pub fn play_round(&mut self) {
        // Pre-round
        self.game.rotate_dealer();
        self.game.shuffle_deck();
        self.game.add_players_to_main_pot();
        self.game.print_dealer();
        self.game.post_blind(true);
        self.game.post_blind(false);

        println!();

        // Initializing these as Hand because it is a Vec<Card> that can print as symbols if needed
        let mut table_cards = Hand::new();
        let mut burned_cards = Hand::new();
        let player_hands = self.game.deal_hands_to_all_players();

        // Play the round
        let mut round_over = false;
        while !round_over {
            // Pre-flop betting round
            let mut pre_flop_betting_round_over = false;
            while !pre_flop_betting_round_over {
                // bet
                // todo: remove after implementing pre_flop_betting_round_over trigger
                pre_flop_betting_round_over = true;
            }

            // Flop
            if let Some(card) = self.game.deal_card() {
                burned_cards.push(card);
            }

            for _ in 0..3 {
                if let Some(card) = self.game.deal_card() {
                    table_cards.push(card);
                }
            }

            println!("** FLOP **");
            println!("Table cards:");
            println!("{}", table_cards.to_symbols());
            println!();

            // Flop betting round
            let mut flop_betting_round_over = false;
            while !flop_betting_round_over {
                // bet
                // todo: remove after implementing flop_betting_round_over trigger
                flop_betting_round_over = true;
            }

            // Turn
            if let Some(card) = self.game.deal_card() {
                burned_cards.push(card);
            }

            if let Some(card) = self.game.deal_card() {
                table_cards.push(card);
            }

            println!("** TURN **");
            println!("Table cards:");
            println!("{}", table_cards.to_symbols());
            println!();

            // Turn betting round
            let mut turn_betting_round_over = false;
            while !turn_betting_round_over {
                // bet
                // todo: remove after implementing turn_betting_round_over trigger
                turn_betting_round_over = true;
            }

            // River
            if let Some(card) = self.game.deal_card() {
                burned_cards.push(card);
            }

            if let Some(card) = self.game.deal_card() {
                table_cards.push(card);
            }

            println!("** RIVER **");
            println!("Table cards:");
            println!("{}", table_cards.to_symbols());
            println!();

            // River betting round
            let mut river_betting_round_over = false;
            while !river_betting_round_over {
                // bet
                // todo: remove after implementing river_betting_round_over trigger
                river_betting_round_over = true;
            }

            // todo: remove after implementing round_over trigger
            round_over = true;
        }

        // Determine winners
        let winning_players = self.game.rank_all_hands(&player_hands, &table_cards);
        self.game.determine_round_result(&winning_players);

        // Post-round
        self.game
            .reset_deck(player_hands, table_cards, burned_cards);
        self.game.reset_pots();
    }
}

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
    
    let mut texas_hold_em = TexasHoldEmGame::new(texas_hold_em_1_3_no_limit, player1.clone());

    texas_hold_em.add_player_prompt(&mut player1);
    let mut player2 = texas_hold_em.game.new_player_with_chips("Player 2", 100);
    texas_hold_em.add_player_prompt(&mut player2);
    let mut player3 = texas_hold_em.game.new_player_with_chips("Player 3", 100);
    texas_hold_em.add_player_prompt(&mut player3);
    let mut player4 = texas_hold_em.game.new_player_with_chips("Player 4", 100);
    texas_hold_em.add_player_prompt(&mut player4);
    let mut player5 = texas_hold_em.game.new_player_with_chips("Player 5", 100);
    texas_hold_em.add_player_prompt(&mut player5);
    let mut player6 = texas_hold_em.game.new_player_with_chips("Player 6", 100);
    texas_hold_em.add_player_prompt(&mut player6);

    println!();

    texas_hold_em.play_tournament();
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
