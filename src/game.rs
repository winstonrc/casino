use std::collections::{HashMap, HashSet};

use cards::card::Card;
use cards::deck::Deck;
use cards::hand::Hand;
use uuid::Uuid;

use crate::hand_rankings::rank_hand;
use crate::player::Player;

pub struct Game {
    deck: Deck,
    players: HashSet<Player>,
    game_over: bool,
}

impl Game {
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

    pub fn add_player(&mut self, player_name: &str) {
        // Default buy-in of 100 chips
        self.add_player_with_chips(player_name, 100);
    }

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
    pub fn remove_player(&mut self, player: &Player) {
        self.players.remove(player);
    }

    pub fn is_game_over(&self) -> bool {
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

    pub fn play(&mut self) {
        while !self.game_over {
            self.play_round();
            self.game_over = self.is_game_over();

            // todo: remove after testing
            self.game_over = true;
        }
    }

    fn play_round(&mut self) {
        self.deck.shuffle();
        println!("Deck shuffled.");

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();

        for player in self.players.clone() {
            println!();
            let hand = self.deal_hand();
            println!("Hand dealt to {}.", player.name);

            player_hands.insert(player, hand.clone());

            // todo: refactor hand ranking logic
            let mut cards_to_rank: Vec<&Card> = Vec::new();
            cards_to_rank.push(&hand.cards[0]);
            cards_to_rank.push(&hand.cards[1]);
            let hand_rank = rank_hand(cards_to_rank);
            // todo: remove after testing
            println!("{:?}", hand_rank);
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
