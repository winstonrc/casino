use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::process;
use std::thread::sleep;
use std::time::Duration;

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
                println!("Required chips amount: {MINIMUM_CHIPS_BUY_IN_AMOUNT}");
                println!(
                    "Additional chips needed: {}",
                    MINIMUM_CHIPS_BUY_IN_AMOUNT - player.chips
                );

                buy_chips_prompt(player);
            }
        }

        match self.game.add_player(player.clone()) {
            Ok(()) => {}
            Err("The player does not have enough chips to play at this table.") => {
                eprintln!("The player does not have enough chips to play at this table.");
            }
            Err(_) => {
                eprintln!("Unable to add player to the table. Reason unknown.");
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
                    "q" | "quit" => {
                        println!("Quitting game.");
                        process::exit(0);
                    }
                    "n" | "no" => {
                        self.game.end_game();
                        println!("Game ended.\n");
                        break;
                    }
                    "y" | "yes" | "" => {
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
            eprintln!(
                "Unable to get user's hand with the identifier: {}",
                self.user.identifier
            );
        }

        // Play the round
        let mut round_over = false;
        while !round_over {
            // Pre-flop betting round
            let mut starting_better_seat_index: usize = self.game.get_under_the_gun_seat_index();
            let mut starting_bet_amount: u32 = self.game.get_big_blind_amount();
            (round_over, player_hands, burned_cards) = self.run_betting_round(
                starting_better_seat_index,
                starting_bet_amount,
                player_hands,
                &table_cards,
                burned_cards
            );
            if round_over {
                break;
            }

            // These remain the same for the following betting rounds
            starting_better_seat_index = self.game.get_small_blind_seat_index();
            starting_bet_amount = 0;

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
            (round_over, player_hands, burned_cards) = self.run_betting_round(
                starting_better_seat_index,
                starting_bet_amount,
                player_hands,
                &table_cards,
                burned_cards
            );
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
            (round_over, player_hands, burned_cards) = self.run_betting_round(
                starting_better_seat_index,
                starting_bet_amount,
                player_hands,
                &table_cards,
                burned_cards
            );
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
            (round_over, player_hands, burned_cards) = self.run_betting_round(
                starting_better_seat_index,
                starting_bet_amount,
                player_hands,
                &table_cards,
                burned_cards
            );
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
    // todo: Replace all these variables with a game state
    fn run_betting_round(
        &mut self,
        starting_better_seat_index: usize,
        starting_bet_amount: u32,
        mut player_hands: HashMap<Uuid, Hand>,
        table_cards: &Hand,
        mut burned_cards: Hand
    ) -> (bool, HashMap<Uuid, Hand>, Hand) {
        // Betting begins with the first player to the left of the dealer, aka the small blind
        let mut current_player_seat_index = starting_better_seat_index;
        let mut current_table_bet: u32 = starting_bet_amount;
        let mut active_players: HashSet<Uuid> = player_hands.keys().copied().collect();

        // Last player to raise needs to be set to the big blind if the first player to bet is under the gun.
        // This is because we can view the betting at the start of the betting round as the small blind raising and then the big blind
        // raising by a larger amount that the small blind will have to action upon after the betting has rotated back to them.
        let mut last_player_to_raise_identifier: Option<Uuid> = None;
        if current_player_seat_index == self.game.get_under_the_gun_seat_index() {
            if let Some(big_blind) = self
                .game
                .get_player_at_seat(self.game.get_big_blind_seat_index())
            {
                last_player_to_raise_identifier = Some(big_blind.identifier);
            }
        }

        let mut last_action: Option<PlayerAction> = None;
        let mut first_player_who_checked: Option<Uuid> = None;
        let mut can_player_check_as_action: bool = current_table_bet == 0;

        while active_players.len() > 1 {
            if let Some(current_player) = self.game.get_player_at_seat(current_player_seat_index) {
                if let Some(identifier) = last_player_to_raise_identifier {
                    if current_player.identifier == identifier {
                        break;
                    }
                }

                if let Some(first_player_to_check_identifier) = first_player_who_checked {
                    if current_player.identifier == first_player_to_check_identifier
                        && last_player_to_raise_identifier.is_none()
                    {
                        break;
                    }
                }

                if active_players.contains(&current_player.identifier) {
                    let action: PlayerAction = 
                    // todo: Implement side-pot logic so that the player can go all-in properly.
                    // Right now, the current player is forced to fold to keep the game moving along.
                    if current_player.chips < current_table_bet {
                        println!(
                            "{} doesn't have enough chips to continue betting.",
                            current_player.name
                        );
                        PlayerAction::Fold()
                    } else if current_player.identifier == self.user.identifier
                    {
                        println!("It's your turn.");
                        println!("The current bet is {current_table_bet} chips.");
                        user_bet_prompt(
                            current_table_bet,
                            current_player.chips,
                            &last_action,
                            can_player_check_as_action,
                        )
                    } else {
                        println!("It's {}'s turn.", current_player.name);
                        computer_action(
                            current_table_bet,
                            current_player.chips,
                            &last_action,
                            can_player_check_as_action,
                            table_cards,
                        )
                    };

                    if action != PlayerAction::Fold() && action != PlayerAction::Check() {
                        can_player_check_as_action = false;
                    }

                    last_action = Some(action.clone());

                    match action {
                        PlayerAction::Call() => {
                            println!(
                                "{} calls with {} chips.",
                                current_player.name, current_table_bet
                            );
                            current_player.subtract_chips(current_table_bet);
                            self.game.add_chips_to_main_pot(current_table_bet);
                        }
                        PlayerAction::Check() => {
                            println!("{} checks.", current_player.name);
                            if first_player_who_checked.is_none() {
                                first_player_who_checked = Some(current_player.identifier);
                            }
                        }
                        PlayerAction::Fold() => {
                            println!("{} folds.", current_player.name);
                            
                            let hand = player_hands.get(&current_player.identifier);
                            if let Some(hand) = hand {
                                if let (Some(card1), Some(card2)) = (hand.cards.first(), hand.cards.last()) {
                                burned_cards.push(*card1);
                                burned_cards.push(*card2);
                                }
                            }
                            player_hands.remove(&current_player.identifier);
                            active_players.remove(&current_player.identifier);
                        }
                        PlayerAction::Raise(bet) => {
                            let total_bet = current_table_bet + bet;

                            println!("{} raises by {bet} chips.", current_player.name);
                            if total_bet == current_player.chips {
                                println!("{} is all in.", current_player.name);
                            }

                            last_player_to_raise_identifier = Some(current_player.identifier);
                            current_player.subtract_chips(total_bet);
                            self.game.add_chips_to_main_pot(total_bet);
                            current_table_bet += bet;

                            println!("The current bet is now {current_table_bet}.");
                        }
                    }
                } else {
                    current_player_seat_index =
                        self.game.rotate_current_player(current_player_seat_index);
                    continue;
                }

                // Move to the next player
                current_player_seat_index =
                    self.game.rotate_current_player(current_player_seat_index);
            }
        }

        let round_over = player_hands.len() == 1;

        (round_over, player_hands, burned_cards)
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

        println!("\nWelcome {name}! Are you happy with this name?");
        print!("yes/no [Y/n]: ");
        io::stdout().flush().expect("Failed to flush stdout.");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let trimmed_input = input.trim();

        println!();

        match trimmed_input.to_lowercase().as_str() {
            "q" | "quit" => {
                println!("Quitting game.");
                process::exit(0);
            }
            "n" | "no" => {
                continue;
            }
            "y" | "yes" | "" => {
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
            "q" | "quit" => {
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
                    io::stdin().read_line(&mut input).expect("Failed to read line");

                    let trimmed_input = input.trim();

                    if trimmed_input.to_lowercase() == "q" || trimmed_input.to_lowercase() == "quit" {
                        println!("Quitting game.");
                        process::exit(0);
                    }

                    let mut numbers = trimmed_input.split_whitespace();

                    // Attempt to parse the first number
                    let first_number: Result<u32, _> = numbers.next().unwrap_or("").parse();

                    // Attempt to parse the second number
                    let second_number: Result<u32, _> = numbers.next().unwrap_or("").parse();

                    // Check if parsing was successful for both numbers
                    if let (Ok(small_blind), Ok(big_blind)) = (first_number, second_number) {
                        return (small_blind, big_blind);
                    }
                }
            }
            _ => println!(
                "Invalid input. Please enter the number of a table listed above or enter 'q' to quit the game.\n"
            ),
        }
    }
}

fn buy_chips_prompt(player: &mut Player) {
    println!("How many chips would you like to buy?");

    loop {
        print!("Amount ({CURRENCY}) of chips to buy: ");
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
                println!("You purchased {CURRENCY} {chips} worth of chips.\n");
                break;
            }
            Err(_) => println!("Error: Not a valid number."),
        }
    }
}

/// Determine the available actions the computer player can make and select one.
// Replace with better AI logic
fn computer_action(
    current_table_bet: u32,
    player_chips: u32,
    last_action: &Option<PlayerAction>,
    check_action_is_available: bool,
    _table_cards: &Hand,
) -> PlayerAction {
    let mut rng = rand::thread_rng();
    let random_num: f64 = rng.gen();

    // Simulate the computer making a decision
    let delay_seconds = rng.gen_range(1..=4);
    let delay_duration = Duration::from_secs(delay_seconds);
    sleep(delay_duration);

    let raise_amount: u32 = if player_chips >= (current_table_bet * 3) + current_table_bet {
        if random_num < 0.5 {
            current_table_bet * 3
        } else {
            current_table_bet * 2
        }
    } else if player_chips >= (current_table_bet * 2) + current_table_bet {
        current_table_bet * 2
    } else {
        0
    };

    match last_action {
        None => {
            // This logic defers in the first betting round and the later betting rounds.
            // In the first betting round, the minimum bet is the big blind amount, and the first better is the player under the gun.
            // This means that the first better (under the gun) cannot check but can call instead.
            // In later betting rounds, the betting starts at 0, and the first better (small blind) can check.
            // Technically the small blind is always the first better, but posting the small blind and big blind are automatic bets,
            // so we consider the under the gun player the first better in the first betting round.
            if check_action_is_available {
                if random_num <= 0.66 || raise_amount == 0 {
                    PlayerAction::Check()
                } else {
                    PlayerAction::Raise(raise_amount)
                }
            } else if random_num <= 0.33 {
                PlayerAction::Call()
            } else if random_num <= 0.5 && raise_amount > 0 {
                PlayerAction::Raise(raise_amount)
            } else {
                PlayerAction::Fold()
            }
        }
        Some(action) => match action {
            PlayerAction::Check() => {
                if random_num <= 0.75 || raise_amount == 0 {
                    PlayerAction::Check()
                } else {
                    PlayerAction::Raise(raise_amount)
                }
            }
            PlayerAction::Call() => {
                if random_num <= 0.4 {
                    PlayerAction::Call()
                } else if random_num <= 0.55 && raise_amount > 0 {
                    PlayerAction::Raise(raise_amount)
                } else {
                    PlayerAction::Fold()
                }
            }
            PlayerAction::Raise(_) => {
                if random_num <= 0.4 {
                    PlayerAction::Call()
                } else if random_num <= 0.5 && raise_amount > 0 {
                    PlayerAction::Raise(raise_amount)
                } else {
                    PlayerAction::Fold()
                }
            }
            // It's possible that the previous / first player to bet folded, and checking is still an option for the next player.
            // In normal play, the first player is likely going to check rather than fold every time, but it is a possibility.
            // (*cough* @thien *cough*).
            PlayerAction::Fold() => {
                if check_action_is_available {
                    if random_num <= 0.75 || raise_amount == 0 {
                        PlayerAction::Check()
                    } else {
                        PlayerAction::Raise(raise_amount)
                    }
                } else if random_num <= 0.33 {
                    PlayerAction::Call()
                } else if random_num <= 0.5 && raise_amount > 0 {
                    PlayerAction::Raise(raise_amount)
                } else {
                    PlayerAction::Fold()
                }
            }
        },
    }
}

/// Prompt the user for their desired action when it's their turn in the betting round.
fn user_bet_prompt(
    current_table_bet: u32,
    player_chips: u32,
    last_action: &Option<PlayerAction>,
    can_player_check_as_action: bool,
) -> PlayerAction {
    loop {
        let actions: Vec<&str> = get_available_actions(
            current_table_bet,
            player_chips,
            last_action,
            can_player_check_as_action,
        );

        println!("Select an action: ");
        if actions.is_empty() {
            eprintln!("No valid actions available.");
        } else {
            let actions_string = actions.join(", ") + ".";
            println!("{actions_string}");
        }

        print!("Action: ");
        io::stdout().flush().expect("Failed to flush stdout.");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let trimmed_input = input.trim().to_lowercase();

        let words: Vec<&str> = trimmed_input.split_whitespace().collect();

        match words.first().copied() {
            Some("q" | "quit") => {
                println!("Quitting game.");
                process::exit(0);
            }
            Some("call") => {
                return PlayerAction::Call();
            }
            Some("check") => {
                return PlayerAction::Check();
            }
            Some("fold") => {
                return PlayerAction::Fold();
            }
            Some("raise") => {
                let raise_arg = words.get(1).and_then(|s| s.parse::<u32>().ok());

                let raise_amount: u32;

                if let Some(amount) = raise_arg {
                    if player_chips < current_table_bet + amount {
                        println!("You do not have enough chips to raise by {amount}.");
                        continue;
                    }
                    raise_amount = amount;
                } else {
                    print!("Amount: ");
                    io::stdout().flush().expect("Failed to flush stdout.");
                    let mut raise_string = String::new();
                    io::stdin()
                        .read_line(&mut raise_string)
                        .expect("Failed to read line");

                    raise_amount = if let Ok(amount) = raise_string.trim().parse() {
                        if player_chips < current_table_bet + amount {
                            println!("You do not have enough chips to raise by {amount}.");
                            continue;
                        }

                        amount
                    } else {
                        eprintln!("Invalid input. Please enter a valid number.");
                        continue;
                    };
                }

                println!("Raise {raise_amount} chips?");
                print!("yes/no [Y/n]: ");
                io::stdout().flush().expect("Failed to flush stdout.");

                let mut confirm = String::new();
                io::stdin()
                    .read_line(&mut confirm)
                    .expect("Failed to read line");
                let trimmed_confirm = confirm.trim();

                match trimmed_confirm.to_lowercase().as_str() {
                    "q" | "quit" => {
                        println!("Quitting game.");
                        process::exit(0);
                    }
                    "n" | "no" => {
                        continue;
                    }
                    "y" | "yes" | "" => {
                        return PlayerAction::Raise(raise_amount);
                    }
                    _ => println!(
                        "Invalid input. Please enter 'y' or 'n' or enter 'q' to quit the game."
                    ),
                }
            }
            _ => println!("Invalid input. Please enter a valid option or enter 'q' to quit.\n"),
        }
    }
}

fn get_available_actions(
    current_table_bet: u32,
    player_chips: u32,
    last_action: &Option<PlayerAction>,
    can_player_check_as_action: bool,
) -> Vec<&'static str> {
    let mut actions: Vec<&str> = Vec::new();
    actions.push("Fold");

    match last_action {
        None => {
            actions.push("Check");

            if player_chips > current_table_bet {
                actions.push("Raise");
            }
        }
        Some(action) => match action {
            PlayerAction::Check() => {
                actions.push("Check");
                actions.push("Raise");
            }
            PlayerAction::Call() | PlayerAction::Raise(_) => {
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
        },
    }

    actions
}
