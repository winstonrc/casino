use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::process;
use rand::prelude::*;

use casino_poker::casino_cards::hand::Hand;
use casino_poker::games::texas_hold_em::{PlayerAction, TexasHoldEm};
use casino_poker::player::Player;
use casino_poker::uuid::Uuid;

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
        let mut player_hands = self.game.deal_hands_to_all_players();

        // Print the user's hand
        if let Some(user_hand) = player_hands.get(&self.user.identifier) {
            println!("Your hand: {}\n", user_hand.to_symbols());
        } else {
            eprintln!("Unable to get user's hand with the identifier: {}", self.user.identifier);
        }

        // Play the round
        let mut round_over = false;
        while !round_over {
            // Pre-flop betting round
            let big_blind_seat_index = self.game.get_big_blind_seat_index();
            let first_better = self.game.rotate_current_player(big_blind_seat_index);
            (round_over, player_hands) = self.run_betting_round(first_better, player_hands, &table_cards);
            if round_over {
                break;
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

            let starting_better_seat_index = self.game.get_small_blind_seat_index();

            // Flop betting round
            (round_over, player_hands) = self.run_betting_round(starting_better_seat_index, player_hands, &table_cards);
            if round_over {
                break;
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
            (round_over, player_hands) = self.run_betting_round(starting_better_seat_index, player_hands, &table_cards);
            if round_over {
                break;
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
            (round_over, player_hands) = self.run_betting_round(starting_better_seat_index, player_hands, &table_cards);
            if round_over {
                break;
            }

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

    /// Runs a betting round for all players currently playing.
    /// Returns a tuple indicating whether the round is over and the remaining players' hands.
    ///
    /// The round is over if only one player remains.
    /// The round continues if more than one player remains.
    fn run_betting_round(&mut self, starting_better_seat_index: usize, mut player_hands: HashMap<Uuid, Hand>, table_cards: &Hand) -> (bool, HashMap<Uuid, Hand>) {
        // Betting begins with the first player to the left of the dealer, aka the small blind
        let mut current_player_seat_index = starting_better_seat_index;
        let mut current_table_bet: u32 = 0;
        let mut active_players: HashSet<Uuid> = player_hands.keys().cloned().collect();
        let mut last_player_to_raise: Option<Uuid> = None;
        let mut last_action: Option<PlayerAction> = None;
        let mut can_player_check_as_action: bool = true;
        let mut first_player_who_checked: Option<Uuid> = None;

        while active_players.len() > 1 {
            if let Some(current_player) = self.game.get_player_at_seat(current_player_seat_index) {
                if let Some(last_player_to_raise_identifier) = last_player_to_raise {
                    if current_player.identifier == last_player_to_raise_identifier {
                        break;
                    }
                }

                if let Some(first_player_to_check_identifier) = first_player_who_checked {
                    if current_player.identifier == first_player_to_check_identifier {
                        break;
                    }
                }

                if active_players.contains(&current_player.identifier) {
                    let action: PlayerAction = if current_player.identifier == self.user.identifier {
                        println!("It's your turn.");
                        user_bet_prompt(current_table_bet, current_player.chips, last_action, can_player_check_as_action)
                    } else {
                        println!("It's {}'s turn.", current_player.name);
                        computer_action(current_table_bet, current_player.chips, last_action, can_player_check_as_action, table_cards)
                    };

                    if action != PlayerAction::Fold() && action != PlayerAction::Check() {
                        can_player_check_as_action = false;
                    }

                    last_action = Some(action.clone());

                    match action {
                        PlayerAction::Call() => {
                            println!("{} calls.", current_player.name);
                            current_player.subtract_chips(current_table_bet);
                            self.game.add_chips_to_main_pot(current_table_bet);
                        },
                        PlayerAction::Check() => {
                            println!("{} checks.", current_player.name);
                            if first_player_who_checked.is_none() {
                                first_player_who_checked = Some(current_player.identifier);
                            }
                        },
                        PlayerAction::Fold() => {
                            println!("{} folds.", current_player.name);
                            player_hands.remove(&current_player.identifier);
                            active_players.remove(&current_player.identifier);
                        },
                        PlayerAction::Raise(bet) => {
                            let total_bet = current_table_bet + bet;
                            if total_bet > current_player.chips {
                                panic!("Player does not have enough chips")
                            }
    
                            println!("{} raises by {} chips.", current_player.name, bet);
                            if total_bet == current_player.chips {
                                println!("{} is all in.", current_player.name);
                            }
    
                            last_player_to_raise = Some(current_player.identifier);
                            current_player.subtract_chips(total_bet);
                            self.game.add_chips_to_main_pot(total_bet);
                            current_table_bet += bet;
                        },
                    }
                } else {
                    current_player_seat_index = self.game.rotate_current_player(current_player_seat_index);
                    continue;
                }
    
                // Move to the next player
                current_player_seat_index = self.game.rotate_current_player(current_player_seat_index);
            }
        }
        
        let round_over = player_hands.len() == 1;

        (round_over, player_hands)
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

/// 
// Replace with better AI logic
fn computer_action(current_table_bet: u32, player_chips: u32, last_action: Option<PlayerAction>, can_player_check_as_action: bool, _table_cards: &Hand) -> PlayerAction {
    if player_chips >= current_table_bet {
        if current_table_bet <= player_chips / 10 {
            let mut rng = rand::thread_rng();
            let random_num: f64 = rng.gen();

            match last_action {
                None => {
                    if random_num >= 0.66 {
                        return PlayerAction::Check();
                    } else if random_num >= 0.33 {
                        let raise_amount: u32 = player_chips / 10;
                        return PlayerAction::Raise(raise_amount);
                    } else {
                        return PlayerAction::Fold();
                    }
                },
                Some(action) => match action {
                    PlayerAction::Check() => {
                        if random_num >= 0.66 {
                            return PlayerAction::Check();
                        } else if random_num >= 0.33 {
                            let raise_amount: u32 = player_chips / 10;
                            return PlayerAction::Raise(raise_amount);
                        } else {
                            return PlayerAction::Fold();
                        }
                    }
                    PlayerAction::Call() => {
                        if random_num >= 0.66 {
                            return PlayerAction::Call();
                        } else if random_num >= 0.33 {
                            let raise_amount: u32 = player_chips / 10;
                            return PlayerAction::Raise(raise_amount);
                        } else {
                            return PlayerAction::Fold();
                        }
                    }
                    PlayerAction::Raise(_) => {
                        if random_num >= 0.66 {
                            return PlayerAction::Call();
                        } else if random_num >= 0.33 {
                            let raise_amount: u32 = player_chips / 10;
                            return PlayerAction::Raise(raise_amount);
                        } else {
                            return PlayerAction::Fold();
                        }
                    }
                    // It's possible that the previous and first player to bet folded,
                    // in which case, you could theoretically still check.
                    // Normally however, you wouldn't be able to check if bets have already been made.
                    // In normal play, the first player is likely not going to fold if they can check.
                    // But it has been done before (*cough cough* Thien).
                    PlayerAction::Fold() => {
                        if can_player_check_as_action {
                            if random_num >= 0.66 {
                                return PlayerAction::Check();
                            } else if random_num >= 0.33 {
                                let raise_amount: u32 = player_chips / 10;
                                return PlayerAction::Raise(raise_amount);
                            } else {
                                return PlayerAction::Fold();
                            }
                        } else {
                            if random_num >= 0.66 {
                                return PlayerAction::Call();
                            } else if random_num >= 0.33 {
                                let raise_amount: u32 = player_chips / 10;
                                return PlayerAction::Raise(raise_amount);
                            } else {
                                return PlayerAction::Fold();
                            }
                        }
                    }
                }
            }            
        } else {
            PlayerAction::Fold()
        }
    } else {
        PlayerAction::Fold()
    }
}

/// Prompt the user for their desired action when it's their turn in the betting round.
fn user_bet_prompt(current_table_bet: u32, player_chips: u32, last_action: Option<PlayerAction>, can_player_check_as_action: bool) -> PlayerAction {
    loop {
        let mut actions: Vec<&str> = Vec::new();
        actions.push("Fold");

        match last_action.clone() {
            None => {
                actions.push("Check");
                
                if player_chips > current_table_bet {
                    actions.push("Raise");
                }
            },
            Some(action) => match action {
                PlayerAction::Check() => {
                    actions.push("Check");
                    actions.push("Raise");
                }
                PlayerAction::Call() => {
                    if player_chips >= current_table_bet {
                        actions.push("Call");
                    }

                    if player_chips > current_table_bet {
                        actions.push("Raise");
                    }
                }
                PlayerAction::Raise(_) => {
                    if player_chips >= current_table_bet {
                        actions.push("Call");
                    }

                    if player_chips > current_table_bet {
                        actions.push("Raise");
                    }
                }
                PlayerAction::Fold() => {
                    if can_player_check_as_action {
                        actions.push("Check");
                        
                        if player_chips > current_table_bet {
                            actions.push("Raise");
                        }
                    } else {
                        if player_chips >= current_table_bet {
                            actions.push("Call");
                        }
    
                        if player_chips > current_table_bet {
                            actions.push("Raise");
                        }
                    }
                }
            }
        }

        println!("Select an action: ");
        if !actions.is_empty() {
            let actions_string = actions.join(", ") + ".";
            println!("{}", actions_string);
        } else {
            eprintln!("No valid actions available.");
        }
        
        print!("Action: ");
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
            "call" => {
                if player_chips < current_table_bet {
                    println!("You do not have enough chips to call.");
                    continue;
                }

                return PlayerAction::Call();
            }
            "check" => {
                if current_table_bet > 0 {
                    println!("You cannot check. A bet has already been made.");
                    continue;
                }

                return PlayerAction::Check();
            }
            "fold" => {
                return PlayerAction::Fold();
            }
            "raise" => {
                loop {
                    if player_chips < current_table_bet {
                        println!("You do not have enough chips to raise.");
                        continue;
                    }

                    println!("How much would you like to raise by?");
                    let mut raise_string = String::new();
                    io::stdin()
                        .read_line(&mut raise_string)
                        .expect("Failed to read line");

                    let raise: u32 = match raise_string.trim().parse() {
                        Ok(num) => num,
                        Err(_) => {
                            eprintln!("Invalid input. Please enter a valid number.");
                            continue;
                        }
                    };
                    
                    println!("Are you happy raising by {}?", raise);
                    print!("yes/no [Y/n]: ");
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
                            continue;
                        }
                        "no" => {
                            continue;
                        }
                        "y" => {
                            return PlayerAction::Raise(raise);
                        }
                        "yes" => {
                            return PlayerAction::Raise(raise);
                        }
                        _ => println!("Invalid input. Please enter 'y' or 'n' or enter 'q' to quit the game."),
                    }
                }
            }
            _ => println!(
                "Invalid input. Please enter a valid option or enter 'q' to quit.\n"
            ),
        }
    }
}
