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
        self.deck.shuffle();

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
            cards_to_rank.push(hand.cards[0].clone());
            cards_to_rank.push(hand.cards[1].clone());

            let hand_rank = rank_hand(cards_to_rank);
            // todo: remove after testing
            println!("{} has {}", player.name, hand_rank);

            if best_hand.is_empty() {
                best_hand.push(hand_rank);
                leading_players.insert(player.clone(), hand_rank);
            } else if &hand_rank == best_hand.last().unwrap() {
                // todo: Add logic to check for a kicker (high card) when players are tied with
                // matching Pairs, Two Pairs, Three of a Kinds, or Four of a Kinds on the table but one has a higher card in their hand.
                // Be sure to make sure that a hand is not unintentionally outranking an equal hand based on its suit in the rank_hand() comparison!
                // something like this: if hand_rank.len() < 5 {}
                best_hand.push(hand_rank);
                leading_players.insert(player.clone(), hand_rank);
            } else if &hand_rank > best_hand.last().unwrap() {
                best_hand.clear();
                best_hand.push(hand_rank);
                leading_players.clear();
                leading_players.insert(player.clone(), hand_rank);
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
        } else {
            panic!("Error: No winning player was determined.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cards::card;
    use cards::card::{Card, Rank, Suit};

    /// Tests rank_all_hands().
    ///
    /// Tests that a single winner is correctly chosen.
    #[test]
    fn rank_all_hands_identifies_winner() {
        let mut game = TexasHoldEm::new();

        let two_of_diamonds = card!(Two, Diamond);
        let two_of_hearts = card!(Two, Heart);
        let three_of_clubs = card!(Three, Club);
        let four_of_diamonds = card!(Four, Diamond);
        let five_of_clubs = card!(Five, Club);
        let six_of_diamonds = card!(Six, Diamond);
        let eight_of_spades = card!(Eight, Spade);
        let nine_of_clubs = card!(Nine, Club);
        let nine_of_hearts = card!(Nine, Heart);
        let ten_of_spades = card!(Ten, Spade);
        let jack_of_clubs = card!(Jack, Club);
        let queen_of_hearts = card!(Queen, Heart);
        let king_of_clubs = card!(King, Club);
        let ace_of_hearts = card!(Ace, Heart);
        let ace_of_spades = card!(Ace, Spade);

        let flush = HandRank::Flush([
            three_of_clubs,
            five_of_clubs,
            nine_of_clubs,
            jack_of_clubs,
            king_of_clubs,
        ]);

        let table_cards: Vec<Card> = vec![
            two_of_diamonds,
            three_of_clubs,
            eight_of_spades,
            jack_of_clubs,
            king_of_clubs,
        ];

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        let player1_cards: Vec<Card> = vec![five_of_clubs, nine_of_clubs];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.clone(), player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        let player2_cards: Vec<Card> = vec![two_of_hearts, ten_of_spades];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.clone(), player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        let player3_cards: Vec<Card> = vec![four_of_diamonds, queen_of_hearts];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.clone(), player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        let player4_cards: Vec<Card> = vec![ace_of_hearts, ace_of_spades];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.clone(), player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        let player5_cards: Vec<Card> = vec![six_of_diamonds, nine_of_hearts];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.clone(), player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 1);
        assert!(leading_players.contains_key(&player1));
        assert_eq!(*leading_players.get(&player1).unwrap(), flush);
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that a single winner is correctly chosen.
    #[test]
    fn rank_all_hands_identifies_push_with_winning_table_flush() {
        let mut game = TexasHoldEm::new();

        let two_of_diamonds = card!(Two, Diamond);
        let two_of_hearts = card!(Two, Heart);
        let three_of_clubs = card!(Three, Club);
        let four_of_diamonds = card!(Four, Diamond);
        let five_of_clubs = card!(Five, Club);
        let six_of_diamonds = card!(Six, Diamond);
        let eight_of_spades = card!(Eight, Spade);
        let nine_of_clubs = card!(Nine, Club);
        let nine_of_hearts = card!(Nine, Heart);
        let ten_of_spades = card!(Ten, Spade);
        let jack_of_clubs = card!(Jack, Club);
        let queen_of_hearts = card!(Queen, Heart);
        let king_of_clubs = card!(King, Club);
        let ace_of_hearts = card!(Ace, Heart);
        let ace_of_spades = card!(Ace, Spade);

        let flush = HandRank::Flush([
            three_of_clubs,
            five_of_clubs,
            nine_of_clubs,
            jack_of_clubs,
            king_of_clubs,
        ]);

        let table_cards: Vec<Card> = vec![
            three_of_clubs,
            five_of_clubs,
            nine_of_clubs,
            jack_of_clubs,
            king_of_clubs,
        ];

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        let player1_cards: Vec<Card> = vec![two_of_diamonds, eight_of_spades];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.clone(), player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        let player2_cards: Vec<Card> = vec![two_of_hearts, ten_of_spades];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.clone(), player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        let player3_cards: Vec<Card> = vec![four_of_diamonds, queen_of_hearts];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.clone(), player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        let player4_cards: Vec<Card> = vec![ace_of_hearts, ace_of_spades];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.clone(), player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        let player5_cards: Vec<Card> = vec![six_of_diamonds, nine_of_hearts];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.clone(), player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 5);
        assert_eq!(*leading_players.get(&player1).unwrap(), flush);
        assert_eq!(*leading_players.get(&player2).unwrap(), flush);
        assert_eq!(*leading_players.get(&player3).unwrap(), flush);
        assert_eq!(*leading_players.get(&player4).unwrap(), flush);
        assert_eq!(*leading_players.get(&player5).unwrap(), flush);
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that a single winner is correctly chosen.
    #[test]
    fn rank_all_hands_identifies_push_with_equal_winning_hand_flushes() {
        let mut game = TexasHoldEm::new();

        let two_of_diamonds = card!(Two, Diamond);
        let two_of_hearts = card!(Two, Heart);
        let three_of_clubs = card!(Three, Club);
        let four_of_diamonds = card!(Four, Diamond);
        let five_of_clubs = card!(Five, Club);
        let six_of_diamonds = card!(Six, Diamond);
        let seven_of_clubs = card!(Seven, Club);
        let eight_of_spades = card!(Eight, Spade);
        let nine_of_clubs = card!(Nine, Club);
        let nine_of_hearts = card!(Nine, Heart);
        let jack_of_clubs = card!(Jack, Club);
        let queen_of_hearts = card!(Queen, Heart);
        let king_of_clubs = card!(King, Club);
        let ace_of_hearts = card!(Ace, Heart);
        let ace_of_spades = card!(Ace, Spade);

        let flush1 = HandRank::Flush([
            three_of_clubs,
            five_of_clubs,
            nine_of_clubs,
            jack_of_clubs,
            king_of_clubs,
        ]);

        let flush2 = HandRank::Flush([
            three_of_clubs,
            seven_of_clubs,
            nine_of_clubs,
            jack_of_clubs,
            king_of_clubs,
        ]);

        let table_cards: Vec<Card> = vec![
            three_of_clubs,
            two_of_diamonds,
            nine_of_clubs,
            jack_of_clubs,
            king_of_clubs,
        ];

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        let player1_cards: Vec<Card> = vec![five_of_clubs, eight_of_spades];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.clone(), player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        let player2_cards: Vec<Card> = vec![two_of_hearts, seven_of_clubs];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.clone(), player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        let player3_cards: Vec<Card> = vec![four_of_diamonds, queen_of_hearts];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.clone(), player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        let player4_cards: Vec<Card> = vec![ace_of_hearts, ace_of_spades];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.clone(), player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        let player5_cards: Vec<Card> = vec![six_of_diamonds, nine_of_hearts];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.clone(), player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 2);
        assert!(leading_players.contains_key(&player1));
        assert!(leading_players.contains_key(&player2));
        assert_eq!(*leading_players.get(&player1).unwrap(), flush1);
        assert_eq!(*leading_players.get(&player2).unwrap(), flush2);
        assert_eq!(
            *leading_players.get(&player1).unwrap(),
            *leading_players.get(&player2).unwrap()
        );
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that all players push when the winning hand is on the table.
    #[test]
    fn rank_all_hands_identifies_push_with_winning_table_straight() {
        let mut game = TexasHoldEm::new();

        let two_of_diamonds = card!(Two, Diamond);
        let three_of_clubs = card!(Three, Club);
        let four_of_hearts = card!(Four, Heart);
        let five_of_diamonds = card!(Five, Diamond);
        let six_of_clubs = card!(Six, Club);
        let seven_of_spades = card!(Seven, Spade);
        let five_of_clubs = card!(Five, Club);
        let nine_of_spades = card!(Nine, Spade);
        let ten_of_diamonds = card!(Ten, Diamond);
        let jack_of_clubs = card!(Jack, Club);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_diamonds = card!(King, Diamond);
        let ace_of_hearts = card!(Ace, Heart);

        let straight = HandRank::Straight([
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ]);

        let table_cards: Vec<Card> = vec![
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ];

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        let player1_cards: Vec<Card> = vec![three_of_clubs, four_of_hearts];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.clone(), player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        let player2_cards: Vec<Card> = vec![five_of_diamonds, six_of_clubs];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.clone(), player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        let player3_cards: Vec<Card> = vec![seven_of_spades, nine_of_spades];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.clone(), player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        let player4_cards: Vec<Card> = vec![two_of_diamonds, five_of_clubs];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.clone(), player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        let player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.clone(), player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 5);
        assert_eq!(*leading_players.get(&player1).unwrap(), straight);
        assert_eq!(*leading_players.get(&player2).unwrap(), straight);
        assert_eq!(*leading_players.get(&player3).unwrap(), straight);
        assert_eq!(*leading_players.get(&player4).unwrap(), straight);
        assert_eq!(*leading_players.get(&player5).unwrap(), straight);
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that multiple equal hands result in a push for all involved players.
    #[test]
    fn rank_all_hands_identifies_push_with_equal_winning_hand_straights() {
        let mut game = TexasHoldEm::new();

        let two_of_diamonds = card!(Two, Diamond);
        let three_of_clubs = card!(Three, Club);
        let four_of_hearts = card!(Four, Heart);
        let five_of_diamonds = card!(Five, Diamond);
        let seven_of_spades = card!(Seven, Spade);
        let five_of_clubs = card!(Five, Club);
        let nine_of_spades = card!(Nine, Spade);
        let ten_of_diamonds = card!(Ten, Diamond);
        let ten_of_hearts = card!(Ten, Heart);
        let jack_of_clubs = card!(Jack, Club);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_diamonds = card!(King, Diamond);
        let ace_of_hearts = card!(Ace, Heart);

        let straight1 = HandRank::Straight([
            ten_of_diamonds,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ]);

        let straight2 = HandRank::Straight([
            ten_of_hearts,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ]);

        let table_cards: Vec<Card> = vec![
            three_of_clubs,
            jack_of_clubs,
            queen_of_spades,
            king_of_diamonds,
            ace_of_hearts,
        ];

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        let player1_cards: Vec<Card> = vec![four_of_hearts, ten_of_diamonds];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.clone(), player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        let player2_cards: Vec<Card> = vec![five_of_diamonds, ten_of_hearts];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.clone(), player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        let player3_cards: Vec<Card> = vec![seven_of_spades, nine_of_spades];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.clone(), player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        let player4_cards: Vec<Card> = vec![two_of_diamonds, five_of_clubs];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.clone(), player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        let player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.clone(), player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 2);
        assert!(leading_players.contains_key(&player1));
        assert!(leading_players.contains_key(&player2));
        assert_eq!(*leading_players.get(&player1).unwrap(), straight1);
        assert_eq!(*leading_players.get(&player2).unwrap(), straight2);
        assert_eq!(
            *leading_players.get(&player1).unwrap(),
            *leading_players.get(&player2).unwrap()
        );
    }
}
