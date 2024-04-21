use std::collections::{HashMap, HashSet};

use cards::card::Card;
use cards::deck::Deck;
use cards::hand::Hand;
use uuid::Uuid;

use crate::hand_rankings::{rank_hand, HandRank};
use crate::player::Player;

const DEFAULT_BUY_IN_CHIPS_AMOUNT: u32 = 100;

/// The core of the Texas hold 'em game.
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

    /// Add a player into the game with the default buy-in chips amount.
    pub fn add_player(&mut self, player_name: &str) {
        self.add_player_with_chips(player_name, DEFAULT_BUY_IN_CHIPS_AMOUNT);
    }

    /// Add a player into the game with a defined buy-in chips amount.
    pub fn add_player_with_chips(&mut self, player_name: &str, chips: u32) {
        let identifier = Uuid::new_v4();

        let player = Player {
            identifier,
            name: player_name.to_string(),
            chips,
        };

        println!(
            "{} bought in with {} chips. Good luck!",
            &player.name, &player.chips
        );
        self.players.insert(player);
    }

    // todo: implement
    /// Remove a player from the game.
    pub fn remove_player(&mut self, player: &Player) {
        self.players.remove(player);
    }

    /// Play the game.
    pub fn play(&mut self) {
        while !self.game_over {
            self.play_round();
            self.game_over = self.is_game_over();

            // todo: remove after implementing game over trigger
            self.game_over = true;
        }
    }

    fn is_game_over(&self) -> bool {
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

    fn play_round(&mut self) {
        self.deck.shuffle();
        println!("Deck shuffled.");

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        // todo: implement dealer, small blind, big blind, and dealing order
        for player in self.players.clone() {
            println!();
            let hand = self.deal_hand();
            println!("Hand dealt to {}.", player.name);

            player_hands.insert(player, hand.clone());
        }

        let table_cards: HashSet<Card> = HashSet::new();

        // todo: implement betting system
        // todo: implement folding
        let mut round_over = false;
        while !round_over {
            let mut leading_players: HashSet<Player> = HashSet::new();
            let mut best_hand: Option<HandRank> = None;
            for (player, hand) in player_hands.iter() {
                // todo: refactor hand ranking logic to consider cards on the table
                let mut cards_to_rank: HashSet<Card> = table_cards.clone();
                cards_to_rank.insert(hand.cards[0]);
                cards_to_rank.insert(hand.cards[1]);

                let hand_rank = rank_hand(cards_to_rank);
                // todo: remove after testing
                println!("{:?}", hand_rank);

                // todo: add logic to check for high card when players have equal results with the same 4 cards.
                // e.g. both are using a Tow Pair or Four of a Kind on the table but one has a higher card in their hand.
                if best_hand.is_none() {
                    best_hand = Some(hand_rank);
                    leading_players.insert(player.clone());
                } else if hand_rank > best_hand.unwrap() {
                    best_hand = Some(hand_rank);
                    leading_players.clear();
                    leading_players.insert(player.clone());
                } else if hand_rank == best_hand.unwrap() {
                    leading_players.insert(player.clone());
                } else {
                    continue;
                }
            }

            if leading_players.len() == 1 {
                let winning_player: Player = leading_players.iter().next().unwrap().clone();
                let winning_hand: Hand = player_hands.get(&winning_player).unwrap().clone();

                print!("{} wins with {}: ", winning_player.name, best_hand.unwrap());
                winning_hand.print_symbols();
            } else if leading_players.len() > 1 {
                for player in leading_players.iter() {
                    let player_hand: Hand = player_hands.get(&player).unwrap().clone();

                    print!("{} pushes with {}: ", player.name, best_hand.unwrap());
                    player_hand.print_symbols();
                }
            }

            // todo: remove after implementing round over trigger
            round_over = true;
        }
    }

    fn deal_hand(&mut self) -> Hand {
        let mut cards: Vec<Card> = Vec::new();
        let card1 = self.deck.deal().unwrap();
        cards.push(card1);

        let card2 = self.deck.deal().unwrap();
        cards.push(card2);

        let hand = Hand { cards };

        // todo: update to only show user's hand
        hand.print_symbols();

        hand
    }
}
