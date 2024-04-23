use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::process;

use cards::card::Card;
use cards::deck::Deck;
use cards::hand::Hand;

use crate::hand_rankings::{rank_hand, HandRank};
use crate::player::Player;

const CURRENCY: &str = "USD";
const MAXIMUM_PLAYERS_COUNT: usize = 10;
const MINIMUM_TABLE_BUY_IN_CHIPS_AMOUNT: u32 = 100;

/// The core of the Texas hold 'em game.
///
/// Only a single table is implemented currently.
///
/// A maximum of 10 players are allowed at a table.
pub struct Game {
    deck: Deck,
    players: HashSet<Player>,
    game_over: bool,
}

impl Game {
    /// Create a new game that internally contains a deck and players.
    pub fn new() -> Self {
        let deck = Deck::new();
        let players = HashSet::new();
        let game_over = false;

        Self {
            deck,
            players,
            game_over,
        }
    }

    /// Play the game.
    pub fn play(&mut self) {
        while !self.game_over {
            self.play_round();

            loop {
                println!("Play another round?");
                print!("Yes (Y) / No (n): ");
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
                    "n" => {
                        self.game_over = true;
                        break;
                    }
                    "y" => {
                        break;
                    }
                    "" => {
                        break;
                    }
                    _ => println!(
                        "Invalid input. Please enter 'y' or 'n' or enter 'q' to quit the game."
                    ),
                }
            }

            self.game_over = self.is_game_over();
        }

        println!("Game over. Thanks for playing!");
    }

    // Create a new player with zero chips.
    pub fn new_player(&mut self, name: &str) -> Player {
        let player = Player::new(name);

        player
    }

    // Create a new player with a defined amount of chips.
    pub fn new_player_with_chips(&mut self, name: &str, chips: u32) -> Player {
        let player = Player::new_with_chips(name, chips);

        player
    }

    /// Add a player into the game.
    pub fn add_player(&mut self, player: &mut Player) -> bool {
        if self.players.len() > MAXIMUM_PLAYERS_COUNT {
            println!("Unable to join the table. It is already at max capacity.");
            return false;
        }

        if player.chips < MINIMUM_TABLE_BUY_IN_CHIPS_AMOUNT {
            while player.chips < MINIMUM_TABLE_BUY_IN_CHIPS_AMOUNT {
                println!("You do not have enough chips to play at this table.");
                println!("Current chips amount: {}", player.chips);
                println!(
                    "Required chips amount: {}",
                    MINIMUM_TABLE_BUY_IN_CHIPS_AMOUNT
                );

                println!(
                    "Additional chips needed: {}",
                    MINIMUM_TABLE_BUY_IN_CHIPS_AMOUNT - player.chips
                );

                println!();

                self.buy_chips(player);
            }
        }

        println!(
            "{} bought in with {} chips. Good luck!",
            &player.name, &player.chips
        );
        self.players.insert(player.clone());
        true
    }

    /// Remove a player from the game.
    pub fn remove_player(&mut self, player: &mut Player) -> Option<Player> {
        if self.players.len() < 1 {
            eprintln!("Unable to remove player. The table is empty.");
            return None;
        }

        if self.players.get(player).is_none() {
            eprintln!(
                "Unable to remove player. {} is not at the table.",
                player.name
            );
            return None;
        }

        self.players.take(player)
    }

    /// Buy chips.
    fn buy_chips(&mut self, player: &mut Player) {
        println!("How many chips would you like to buy?");
        println!("Enter 'q' at anytime to quit.\n");

        loop {
            print!("Please enter your desired amount in {}: ", CURRENCY);
            io::stdout().flush().expect("Failed to flush stdout.");

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read line");

            let trimmed_input = input.trim();

            if trimmed_input.to_lowercase() == "q" {
                println!("No chips were purchased. Quitting game.");
                process::exit(0);
            }

            match trimmed_input.parse::<u32>() {
                Ok(number) => {
                    println!("You purchased {} {} worth of chips.", CURRENCY, number);
                    player.update_chips(number);
                    break;
                }
                Err(_) => println!("Error: Not a valid number."),
            }
        }
    }

    fn is_game_over(&self) -> bool {
        if self.game_over == true {
            return true;
        }
        if self.players.len() == 0 {
            println!("No players remaining. Game over!");

            return true;
        }

        if self.players.len() == 1 {
            println!("One player remaining. Game over!");

            return true;
        }

        false
    }

    // todo: implement betting system
    // todo: implement folding
    // todo: add hand timer
    fn play_round(&mut self) {
        let mut round_over = false;
        while !round_over {
            println!("The deck contains {} cards", self.deck.len());
            self.deck.shuffle();
            println!();
            println!("Deck shuffled.");

            // todo: determine player seat position & dealing order
            // todo: implement dealer, small blind, big blind, and dealing order

            let mut table_cards = Hand::new();
            let mut burned_cards = Hand::new();
            let mut player_hands: HashMap<Player, Hand> = HashMap::new();

            for player in self.players.clone() {
                if let Some(hand) = self.deal_hand() {
                    // todo: update to only show hand of user
                    println!("Hand dealt to {}: {}", player.name, hand.to_symbols());
                    player_hands.insert(player.clone(), hand);
                }
            }
            println!();

            // Pre-flop betting round
            let mut pre_flop_betting_round_over = false;
            while !pre_flop_betting_round_over {
                // bet
                // todo: remove after implementing pre_flop_betting_round_over trigger
                pre_flop_betting_round_over = true;
            }

            // Flop
            if let Some(card) = self.deal_card() {
                burned_cards.push(card);
            }

            for _ in 0..3 {
                if let Some(card) = self.deal_card() {
                    table_cards.push(card);
                }
            }

            println!("Table cards: {}", table_cards.to_symbols());
            println!();

            // Flop betting round
            let mut flop_betting_round_over = false;
            while !flop_betting_round_over {
                // bet
                // todo: remove after implementing flop_betting_round_over trigger
                flop_betting_round_over = true;
            }

            // Turn
            if let Some(card) = self.deal_card() {
                burned_cards.push(card);
            }

            if let Some(card) = self.deal_card() {
                table_cards.push(card);
            }

            println!("Table cards: {}", table_cards.to_symbols());
            println!();

            // Turn betting round
            let mut turn_betting_round_over = false;
            while !turn_betting_round_over {
                // bet
                // todo: remove after implementing turn_betting_round_over trigger
                turn_betting_round_over = true;
            }

            // River
            if let Some(card) = self.deal_card() {
                burned_cards.push(card);
            }

            if let Some(card) = self.deal_card() {
                table_cards.push(card);
            }

            println!("Table cards: {}", table_cards.to_symbols());
            println!();

            // River betting round
            let mut river_betting_round_over = false;
            while !river_betting_round_over {
                // bet
                // todo: remove after implementing river_betting_round_over trigger
                river_betting_round_over = true;
            }

            let leading_players = self.rank_all_hands(&player_hands, &table_cards);

            self.determine_winners(&leading_players);

            // Return cards from hands to deck
            for (_player, hand) in player_hands.iter() {
                if let (Some(card1), Some(card2)) = (hand.cards.get(0), hand.cards.get(1)) {
                    self.deck.insert_at_top(*card1).unwrap();
                    self.deck.insert_at_top(*card2).unwrap();
                }
            }

            // Return cards from table to deck
            for card in table_cards.get_cards() {
                self.deck.insert_at_top(*card).unwrap();
            }

            // Return cards from burned piles to deck
            for card in burned_cards.get_cards() {
                self.deck.insert_at_top(*card).unwrap();
            }

            // todo: remove after implementing round_over trigger
            round_over = true;
        }
    }

    fn deal_card(&mut self) -> Option<Card> {
        if let Some(card) = self.deck.deal() {
            return Some(card);
        }

        None
    }

    fn deal_hand(&mut self) -> Option<Hand> {
        let mut hand = Hand::new();

        if let Some(card1) = self.deal_card() {
            hand.push(card1);
        } else {
            return None;
        }

        if let Some(card2) = self.deal_card() {
            hand.push(card2);
        } else {
            return None;
        }

        Some(hand)
    }

    fn rank_all_hands(
        &self,
        player_hands: &HashMap<Player, Hand>,
        table_cards: &Hand,
    ) -> HashMap<Player, HandRank> {
        let mut leading_players: HashMap<Player, HandRank> = HashMap::new();
        let mut best_hand: Vec<HandRank> = Vec::new();
        for (player, hand) in player_hands.iter() {
            let mut cards_to_rank: Vec<Card> = table_cards.get_cards().clone();
            cards_to_rank.push(hand.cards[0]);
            cards_to_rank.push(hand.cards[1]);

            let hand_rank = rank_hand(cards_to_rank);
            // todo: remove after testing
            println!("{} has {}", player.name, hand_rank);

            // todo: Add logic to check for a kicker (high card) when players are tied with
            // matching Pairs, Two Pairs, Three of a Kinds, or Four of a Kinds on the table but one has a higher card in their hand.
            // Be sure to make sure that a hand is not unintentionally outranking an equal hand based on its suit in the rank_hand() comparison!
            if best_hand.is_empty() {
                best_hand.push(hand_rank);
                leading_players.insert(player.clone(), hand_rank);
            } else if hand_rank > best_hand[best_hand.len() - 1] {
                best_hand.clear();
                best_hand.push(hand_rank);
                leading_players.clear();
                leading_players.insert(player.clone(), hand_rank);
            } else if hand_rank == best_hand[best_hand.len() - 1] {
                best_hand.push(hand_rank);
                leading_players.insert(player.clone(), hand_rank);
            } else {
                continue;
            }
        }

        leading_players
    }

    fn determine_winners(&self, leading_players: &HashMap<Player, HandRank>) {
        if leading_players.len() == 1 {
            let (winning_player, winning_hand_rank): (&Player, &HandRank) =
                leading_players.iter().next().unwrap().clone();

            println!("{} wins with {}", winning_player.name, winning_hand_rank);
        } else if leading_players.len() > 1 {
            for (player, tied_hand_rank) in leading_players.iter() {
                println!("{} pushes with {}", player.name, tied_hand_rank);
            }
        }
    }
}
