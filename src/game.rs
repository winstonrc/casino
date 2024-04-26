use std::collections::{HashMap, HashSet};

use cards::card::Card;
use cards::deck::Deck;
use cards::hand::Hand;

use crate::hand_rankings::{rank_hand, HandRank};
use crate::player::Player;

pub const MINIMUM_TABLE_BUY_IN_CHIPS_AMOUNT: u32 = 100;
const MAXIMUM_PLAYERS_COUNT: usize = 10;

/// The core of the Texas hold 'em game.
///
/// Only a single table is implemented currently.
///
/// A maximum of 10 players are allowed at a table.
pub struct TexasHoldEm {
    deck: Deck,
    players: HashSet<Player>,

    pub game_over: bool,
}

impl TexasHoldEm {
    /// Create a new game that internally contains a deck and players.
    pub fn new() -> Self {
        Self {
            deck: Deck::new(),
            players: HashSet::new(),
            game_over: false,
        }
    }

    /// Play the game.
    pub fn play(&mut self) {
        while !self.game_over {
            self.play_round();
            self.check_for_game_over();
        }

        println!("Game over. Thanks for playing!");
    }

    /// End the game.
    pub fn end_game(&mut self) {
        self.game_over = true;
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
    pub fn add_player(&mut self, player: Player) -> Result<(), &'static str> {
        if self.players.len() > MAXIMUM_PLAYERS_COUNT {
            return Err("Unable to join the table. It is already at max capacity.");
        }

        if player.chips < MINIMUM_TABLE_BUY_IN_CHIPS_AMOUNT {
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
            return Err("You do not have enough chips to play at this table.");
        }

        println!(
            "{} bought in with {} chips. Good luck!",
            &player.name, &player.chips
        );
        self.players.insert(player);
        Ok(())
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

    pub fn check_for_game_over(&mut self) {
        if self.players.len() == 0 {
            println!("No players remaining. Game over!");
            self.game_over = true;
        }

        if self.players.len() == 1 {
            println!("One player remaining. Game over!");
            self.game_over = true;
        }
    }

    // todo: implement betting system
    // todo: implement folding
    // todo: add hand timer
    /// Play a single round.
    pub fn play_round(&mut self) {
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

            // todo: remove after implementing round_over trigger
            round_over = true;
        }

        // Post-round
        let leading_players = self.rank_all_hands(&player_hands, &table_cards);
        self.determine_round_result(&leading_players);

        // Return cards from hands to deck
        for (_player, hand) in player_hands.iter() {
            if let (Some(card1), Some(card2)) = (hand.cards.get(0), hand.cards.get(1)) {
                self.deck.insert_at_top(*card1).unwrap();
                self.deck.insert_at_top(*card2).unwrap();
            }
        }

        // Return cards from the table to the deck
        for card in table_cards.get_cards() {
            self.deck.insert_at_top(*card).unwrap();
        }

        // Return cards from the burned pile to the deck
        for card in burned_cards.get_cards() {
            self.deck.insert_at_top(*card).unwrap();
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

    fn determine_round_result(&self, leading_players: &HashMap<Player, HandRank>) {
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
