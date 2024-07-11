use std::collections::{HashMap, HashSet};

use cards::card::Card;
use cards::deck::Deck;
use cards::hand::Hand;

use crate::hand_rankings::{get_high_card_value, rank_hand, HandRank};
use crate::player::Player;

pub const MINIMUM_TABLE_BUY_IN_CHIPS_AMOUNT: u32 = 100;
const MAXIMUM_PLAYERS_COUNT: usize = 10;

/// The core of the Texas hold 'em game.
///
/// Only a single table is implemented currently.
///
/// A maximum of 10 players are allowed at a table.
pub struct TexasHoldEm {
    pub game_over: bool,
    deck: Deck,
    players: HashSet<Player>,
    seats: Vec<Player>,
    small_blind: u32,
    big_blind: u32,
    limit: bool,
}

impl TexasHoldEm {
    /// Create a new game that internally contains a deck and players.
    pub fn new(small_blind: u32, big_blind: u32, limit: bool) -> Self {
        Self {
            game_over: false,
            deck: Deck::new(),
            players: HashSet::new(),
            seats: Vec::new(),
            small_blind,
            big_blind,
            limit,
        }
    }

    /// Play the game.
    pub fn play(&mut self) {
        let mut dealer: usize = 0;

        while !self.game_over {
            self.play_round(dealer);
            self.check_for_game_over();

            dealer = (dealer + 1) % self.seats.len();
        }

        println!("Game over. Thanks for playing!");
    }

    /// End the game.
    pub fn end_game(&mut self) {
        self.game_over = true;
    }

    // Create a new player with zero chips.
    pub fn new_player(&mut self, name: &str) -> Player {
        Player::new(name)
    }

    // Create a new player with a defined amount of chips.
    pub fn new_player_with_chips(&mut self, name: &str, chips: u32) -> Player {
        Player::new_with_chips(name, chips)
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

        self.seats.push(player.clone());
        self.players.insert(player);
        Ok(())
    }

    /// Remove a player from the game.
    pub fn remove_player(&mut self, player: &mut Player) -> Option<Player> {
        if self.players.is_empty() {
            eprintln!("Unable to remove player. The table is empty.");
            return None;
        }

        if self.players.get(player).is_none() {
            eprintln!(
                "Unable to remove player. {} is not at the table.",
                player.name
            );
            return None;
        } else {
            // Remove player from seat
            self.seats.retain(|x| *x != *player);
        }

        // Remove and return player
        self.players.take(player)
    }

    pub fn check_for_game_over(&mut self) {
        if self.players.is_empty() {
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
    pub fn play_round(&mut self, dealer: usize) {
        self.deck.shuffle();

        // todo: implement small blind & big blind

        let mut table_cards = Hand::new();
        let mut burned_cards = Hand::new();

        let player_hands = self.deal_hands_to_all_players(dealer);

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
            if let (Some(card1), Some(card2)) = (hand.cards.first(), hand.cards.last()) {
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

    /// Deals a single card.
    fn deal_card(&mut self) -> Option<Card> {
        // todo: change to deck.deal_face_down for all other players after testing is completed.
        if let Some(card) = self.deck.deal_face_up() {
            return Some(card);
        }

        None
    }

    /// Deal a hand of two cards.
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

    /// Deal hands of two cards to every player starting with the player to the left of the dealer.
    fn deal_hands_to_all_players(&mut self, dealer: usize) -> HashMap<Player, Hand> {
        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        let num_players = self.seats.len();
        let mut current_player = (dealer + 1) % num_players;

        println!();

        // Deal cards to player starting to the left of the dealer
        while current_player != dealer {
            if let Some(hand) = self.deal_hand() {
                // todo: Update to only show hand of user after testing is complete.
                println!(
                    "Hand dealt to {}: {}",
                    self.seats[current_player].name,
                    hand.to_symbols()
                );
                player_hands.insert(self.seats[current_player].clone(), hand);
            }

            // Move to the next player
            current_player = (current_player + 1) % num_players;
        }

        // Deal cards to the dealer
        if let Some(hand) = self.deal_hand() {
            // todo: update to only show hand of user
            println!(
                "Hand dealt to {}: {}",
                self.seats[dealer].name,
                hand.to_symbols()
            );
            player_hands.insert(self.seats[dealer].clone(), hand);
        }

        println!();

        player_hands
    }

    /// Rank the provided hands to determine which hands are the best.
    fn rank_all_hands(
        &self,
        player_hands: &HashMap<Player, Hand>,
        table_cards: &Hand,
    ) -> HashMap<Player, Vec<HandRank>> {
        let mut leading_players: HashMap<Player, Vec<HandRank>> = HashMap::new();
        let mut best_hand: Vec<(HandRank, &Hand)> = Vec::new();

        for (player, hand) in player_hands.iter() {
            let mut cards_to_rank: Vec<Card> = table_cards.get_cards().clone();
            cards_to_rank.push(hand.cards[0]);
            cards_to_rank.push(hand.cards[1]);

            let hand_rank = rank_hand(cards_to_rank);
            // todo: remove after testing
            println!("{} has {}", player.name, hand_rank);

            let mut hand_rank_vec = Vec::new();
            hand_rank_vec.push(hand_rank);

            if best_hand.is_empty() {
                best_hand.push((hand_rank, hand));
                leading_players.insert(player.clone(), hand_rank_vec);
                continue;
            }

            if let Some((best_hand_rank, best_hand_cards)) = best_hand.last() {
                if hand_rank == *best_hand_rank {
                    // If hand ranks are equal and are made up of less than 5 cards then check for a kicker (high card).
                    // If the kicker is on the table, then that should be used.
                    if hand_rank.len() < 5 {
                        let mut current_cards_and_table_cards: Vec<Card> =
                            table_cards.get_cards().clone();

                        current_cards_and_table_cards.push(hand.cards[0]);
                        current_cards_and_table_cards.push(hand.cards[1]);

                        let mut cards_not_used_in_current_hand_rank: Vec<Card> = Vec::new();
                        for card in current_cards_and_table_cards {
                            // Check to see that the kicker is not part of the hand rank.
                            if !hand_rank.contains(&card) {
                                cards_not_used_in_current_hand_rank.push(card);
                            }
                        }
                        let current_hand_kicker =
                            get_high_card_value(&cards_not_used_in_current_hand_rank).unwrap();

                        let mut best_hand_cards_and_table_cards: Vec<Card> =
                            table_cards.get_cards().clone();

                        best_hand_cards_and_table_cards.push(best_hand_cards.cards[0]);
                        best_hand_cards_and_table_cards.push(best_hand_cards.cards[1]);

                        let mut cards_not_used_in_best_hand_rank: Vec<Card> = Vec::new();
                        for card in best_hand_cards_and_table_cards {
                            // Check to see that the kicker is not part of the hand rank.
                            if !best_hand_rank.contains(&card) {
                                cards_not_used_in_best_hand_rank.push(card);
                            }
                        }
                        let best_hand_kicker =
                            get_high_card_value(&cards_not_used_in_best_hand_rank).unwrap();

                        // If there is a tie, but the best hand has a higher kicker, add that kicker to the best hand, so that it is returned when the best hand is declared
                        if let Some((leading_player, leading_hand_vec)) =
                            leading_players.iter().next()
                        {
                            if leading_hand_vec.len() < 2 {
                                leading_players
                                    .entry(leading_player.clone())
                                    .or_default()
                                    .push(HandRank::HighCard(best_hand_kicker));
                            }
                        }

                        // If the kicker is equal, then both hands are equal.
                        // Otherwise, one of the hands must be greater and there will only be one leading player.
                        if current_hand_kicker.rank == best_hand_kicker.rank {
                            best_hand.push((hand_rank, hand));
                            hand_rank_vec.push(HandRank::HighCard(current_hand_kicker));
                            leading_players.insert(player.clone(), hand_rank_vec);
                        } else if current_hand_kicker.rank > best_hand_kicker.rank {
                            best_hand.clear();
                            best_hand.push((hand_rank, hand));
                            leading_players.clear();
                            hand_rank_vec.push(HandRank::HighCard(current_hand_kicker));
                            leading_players.insert(player.clone(), hand_rank_vec);
                        }
                    } else {
                        // If hands are still equal after considering the kicker, push the new hand.
                        best_hand.push((hand_rank, hand));
                        leading_players.insert(player.clone(), hand_rank_vec);
                    }
                } else if hand_rank > *best_hand_rank {
                    best_hand.clear();
                    best_hand.push((hand_rank, hand));
                    leading_players.clear();
                    leading_players.insert(player.clone(), hand_rank_vec);
                }
            }
        }

        leading_players
    }

    fn determine_round_result(&self, leading_players: &HashMap<Player, Vec<HandRank>>) {
        if leading_players.len() == 1 {
            let (winning_player, winning_hand_rank_vec): (&Player, &Vec<HandRank>) =
                leading_players.iter().next().unwrap();

            if winning_hand_rank_vec.len() > 1 {
                println!(
                    "{} wins with {} and {}",
                    winning_player.name, winning_hand_rank_vec[0], winning_hand_rank_vec[1]
                );
            } else {
                println!(
                    "{} wins with {}",
                    winning_player.name,
                    winning_hand_rank_vec.last().unwrap()
                );
            }
        } else if leading_players.len() > 1 {
            for (player, tied_hand_rank) in leading_players.iter() {
                if tied_hand_rank.len() > 1 {
                    println!(
                        "{} pushes with {} and {}",
                        player.name, tied_hand_rank[0], tied_hand_rank[1]
                    );
                } else {
                    println!(
                        "{} pushes with {}",
                        player.name,
                        tied_hand_rank.last().unwrap()
                    );
                }
            }
        } else {
            panic!("Error: No winning player was determined.");
        }
    }
}

impl Default for TexasHoldEm {
    fn default() -> Self {
        Self {
            game_over: false,
            deck: Deck::new(),
            players: HashSet::new(),
            seats: Vec::new(),
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
        assert_eq!(leading_players.get(&player1).unwrap()[0], flush);
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that a single winner is correctly chosen when the winning hand ranks combines the table and hand
    /// but one player has a higher kicker (high card) than the other.
    #[test]
    fn rank_all_hands_identifies_winner_based_on_kicker_with_hand_winner() {
        let mut game = TexasHoldEm::new();

        let two_of_diamonds = card!(Two, Diamond);
        let two_of_hearts = card!(Two, Heart);
        let six_of_diamonds = card!(Six, Diamond);
        let nine_of_clubs = card!(Nine, Club);
        let nine_of_hearts = card!(Nine, Heart);
        let nine_of_spades = card!(Nine, Spade);
        let ten_of_spades = card!(Ten, Spade);
        let jack_of_clubs = card!(Jack, Club);
        let ace_of_spades = card!(Ace, Spade);

        let two_pair1 = HandRank::TwoPair([
            nine_of_clubs,
            nine_of_hearts,
            two_of_diamonds,
            two_of_hearts,
        ]);

        let table_cards: Vec<Card> = vec![
            two_of_diamonds,
            two_of_hearts,
            nine_of_clubs,
            ten_of_spades,
            jack_of_clubs,
        ];

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        let player1_cards: Vec<Card> = vec![nine_of_hearts, ace_of_spades];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.clone(), player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        let player2_cards: Vec<Card> = vec![nine_of_spades, six_of_diamonds];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.clone(), player2_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 1);
        assert!(leading_players.contains_key(&player1));
        assert_eq!(leading_players.get(&player1).unwrap()[0], two_pair1);
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that a single winner is correctly chosen when the winning hand ranks is on the table
    /// but one player has a higher kicker (high card) than the other.
    #[test]
    fn rank_all_hands_identifies_winner_based_on_kicker_with_table_winner() {
        let mut game = TexasHoldEm::new();

        let two_of_diamonds = card!(Two, Diamond);
        let two_of_hearts = card!(Two, Heart);
        let six_of_diamonds = card!(Six, Diamond);
        let nine_of_clubs = card!(Nine, Club);
        let nine_of_hearts = card!(Nine, Heart);
        let ten_of_hearts = card!(Ten, Heart);
        let ten_of_spades = card!(Ten, Spade);
        let jack_of_clubs = card!(Jack, Club);
        let ace_of_spades = card!(Ace, Spade);

        let two_pair1 = HandRank::TwoPair([
            nine_of_clubs,
            nine_of_hearts,
            two_of_diamonds,
            two_of_hearts,
        ]);

        let table_cards: Vec<Card> = vec![
            two_of_diamonds,
            two_of_hearts,
            nine_of_clubs,
            nine_of_hearts,
            jack_of_clubs,
        ];

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        let player1_cards: Vec<Card> = vec![ten_of_spades, ace_of_spades];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.clone(), player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        let player2_cards: Vec<Card> = vec![six_of_diamonds, ten_of_hearts];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.clone(), player2_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 1);
        assert!(leading_players.contains_key(&player1));
        assert_eq!(leading_players.get(&player1).unwrap()[0], two_pair1);
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
        assert_eq!(leading_players.get(&player1).unwrap()[0], flush);
        assert_eq!(leading_players.get(&player2).unwrap()[0], flush);
        assert_eq!(leading_players.get(&player3).unwrap()[0], flush);
        assert_eq!(leading_players.get(&player4).unwrap()[0], flush);
        assert_eq!(leading_players.get(&player5).unwrap()[0], flush);
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
        assert_eq!(leading_players.get(&player1).unwrap()[0], flush1);
        assert_eq!(leading_players.get(&player2).unwrap()[0], flush2);
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
        assert_eq!(leading_players.get(&player1).unwrap()[0], straight);
        assert_eq!(leading_players.get(&player2).unwrap()[0], straight);
        assert_eq!(leading_players.get(&player3).unwrap()[0], straight);
        assert_eq!(leading_players.get(&player4).unwrap()[0], straight);
        assert_eq!(leading_players.get(&player5).unwrap()[0], straight);
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
        let five_of_clubs = card!(Five, Club);
        let five_of_diamonds = card!(Five, Diamond);
        let seven_of_spades = card!(Seven, Spade);
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
        assert_eq!(leading_players.get(&player1).unwrap()[0], straight1);
        assert_eq!(leading_players.get(&player2).unwrap()[0], straight2);
        assert_eq!(
            *leading_players.get(&player1).unwrap(),
            *leading_players.get(&player2).unwrap()
        );
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that multiple equal hands result in a push for all involved players.
    #[test]
    fn rank_all_hands_identifies_higher_straight_beats_ace_low_straight() {
        let mut game = TexasHoldEm::new();

        let two_of_diamonds = card!(Two, Diamond);
        let three_of_clubs = card!(Three, Club);
        let four_of_hearts = card!(Four, Heart);
        let five_of_clubs = card!(Five, Club);
        let five_of_diamonds = card!(Five, Diamond);
        let six_of_spades = card!(Six, Spade);
        let nine_of_spades = card!(Nine, Spade);
        let ten_of_diamonds = card!(Ten, Diamond);
        let ten_of_hearts = card!(Ten, Heart);
        let jack_of_clubs = card!(Jack, Club);
        let jack_of_hearts = card!(Jack, Heart);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_spades = card!(Queen, Spade);
        let king_of_diamonds = card!(King, Diamond);
        let ace_of_hearts = card!(Ace, Heart);

        let straight = HandRank::Straight([
            two_of_diamonds,
            three_of_clubs,
            four_of_hearts,
            five_of_diamonds,
            six_of_spades,
        ]);

        let table_cards: Vec<Card> = vec![
            two_of_diamonds,
            three_of_clubs,
            four_of_hearts,
            five_of_diamonds,
            king_of_diamonds,
        ];

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        let player1_cards: Vec<Card> = vec![ten_of_diamonds, ace_of_hearts];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.clone(), player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        let player2_cards: Vec<Card> = vec![six_of_spades, jack_of_clubs];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.clone(), player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        let player3_cards: Vec<Card> = vec![nine_of_spades, ten_of_hearts];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.clone(), player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        let player4_cards: Vec<Card> = vec![five_of_clubs, queen_of_spades];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.clone(), player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        let player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.clone(), player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 1);
        assert!(!leading_players.contains_key(&player1));
        assert!(leading_players.contains_key(&player2));
        assert_eq!(leading_players.get(&player2).unwrap()[0], straight);
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that hand ranking correctly updates the leader for a pair that is higher than the previously set high pair.
    #[test]
    fn rank_all_hands_identifies_higher_pair_as_winner_over_previous_high_pair() {
        let mut game = TexasHoldEm::new();

        let two_of_spades = card!(Two, Spade);
        let four_of_clubs = card!(Four, Club);
        let five_of_diamonds = card!(Five, Diamond);
        let six_of_clubs = card!(Six, Club);
        let seven_of_hearts = card!(Seven, Heart);
        let nine_of_clubs = card!(Nine, Club);
        let nine_of_spades = card!(Nine, Spade);
        let ten_of_clubs = card!(Ten, Club);
        let ten_of_diamonds = card!(Ten, Diamond);
        let ten_of_spades = card!(Ten, Spade);
        let jack_of_clubs = card!(Jack, Club);
        let jack_of_spades = card!(Jack, Spade);
        let queen_of_diamonds = card!(Queen, Diamond);
        let queen_of_hearts = card!(Queen, Heart);
        let king_of_diamonds = card!(King, Diamond);
        let ace_of_hearts = card!(Ace, Heart);
        let ace_of_spades = card!(Ace, Spade);

        let pair = HandRank::Pair([ace_of_hearts, ace_of_spades]);

        let table_cards: Vec<Card> = vec![
            queen_of_diamonds,
            jack_of_clubs,
            five_of_diamonds,
            two_of_spades,
            ace_of_spades,
        ];

        let mut player_hands: HashMap<Player, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player2 = game.new_player_with_chips("Player 2", 100);
        let player2_cards: Vec<Card> = vec![jack_of_spades, nine_of_spades];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.clone(), player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        let player3_cards: Vec<Card> = vec![nine_of_clubs, four_of_clubs];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.clone(), player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        let player4_cards: Vec<Card> = vec![six_of_clubs, ten_of_spades];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.clone(), player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        let player5_cards: Vec<Card> = vec![seven_of_hearts, queen_of_hearts];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.clone(), player5_hand);

        let player6 = game.new_player_with_chips("Player 6", 100);
        let player6_cards: Vec<Card> = vec![ten_of_diamonds, ten_of_clubs];
        let player6_hand = Hand::new_from_cards(player6_cards);
        player_hands.insert(player6.clone(), player6_hand);

        let player1 = game.new_player_with_chips("Player 1", 100);
        let player1_cards: Vec<Card> = vec![king_of_diamonds, ace_of_hearts];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.clone(), player1_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 1);
        assert!(leading_players.contains_key(&player1));
        assert_eq!(leading_players.get(&player1).unwrap()[0], pair);
    }
}
