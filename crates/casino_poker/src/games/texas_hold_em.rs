use std::collections::HashMap;

use uuid::Uuid;

use casino_cards::card::Card;
use casino_cards::deck::Deck;
use casino_cards::hand::Hand;

use crate::hand_rankings::{get_high_card_value, rank_hand, HandRank};
use crate::player::Player;

/// The actions a Player can choose from on their turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlayerAction {
    /// Match the current bet.
    Call(),
    /// Make a bet of zero.
    /// Must be invoked by the first player betting before subsequent players are allowed to perform the action.
    Check(),
    /// Discard current hand and remove self from the current round.
    Fold(),
    /// Raise the current bet to a higher amount.
    Raise(u32),
}

/// The core of the Texas hold 'em game.
///
/// The game currently defaults to no-limit.
pub struct TexasHoldEm {
    game_over: bool,
    deck: Deck,
    players: HashMap<Uuid, Player>,
    seats: Vec<Uuid>,
    dealer_seat_index: usize,
    main_pot: Pot,
    side_pots: Vec<Pot>,
    minimum_chips_buy_in_amount: u32,
    maximum_players_count: usize,
    small_blind_amount: u32,
    big_blind_amount: u32,
}

impl TexasHoldEm {
    /// Create a new game that internally contains a deck and players.
    pub fn new(
        minimum_chips_buy_in_amount: u32,
        maximum_players_count: usize,
        small_blind_amount: u32,
        big_blind_amount: u32,
    ) -> Self {
        Self {
            game_over: false,
            deck: Deck::new(),
            players: HashMap::new(),
            seats: Vec::new(),
            dealer_seat_index: 0,
            main_pot: Pot::new(0, HashMap::new()),
            side_pots: Vec::new(),
            minimum_chips_buy_in_amount,
            maximum_players_count,
            small_blind_amount,
            big_blind_amount,
        }
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
        if self.players.len() > self.maximum_players_count {
            return Err("Unable to join the table. It is already at max capacity.");
        }

        if player.chips < self.minimum_chips_buy_in_amount {
            println!("The player does not have enough chips to play at this table.");
            println!("Current chips amount: {}", player.chips);
            println!(
                "Required chips amount: {}",
                self.minimum_chips_buy_in_amount
            );
            println!(
                "Additional chips needed: {}",
                self.minimum_chips_buy_in_amount - player.chips
            );
            return Err("The player does not have enough chips to play at this table.");
        }

        println!(
            "{} bought in with {} chips. Good luck!",
            &player.name, &player.chips
        );

        self.seats.push(player.identifier);
        self.players.insert(player.identifier, player);
        Ok(())
    }

    /// Remove a player from the game.
    pub fn remove_player(&mut self, player_identifier: &Uuid) -> Option<Player> {
        if self.players.is_empty() {
            eprintln!("Unable to remove player. The table is empty.");
            return None;
        }

        if self.players.get(&player_identifier).is_none() {
            eprintln!(
                "Unable to remove player. The identifier {} is not at the table.",
                player_identifier
            );
            return None;
        } else {
            // Remove player from seat
            self.seats.retain(|x| x != player_identifier);
        }

        // Remove and return player
        self.players.remove(&player_identifier)
    }

    /// Simulates a tournament consisting of multiple rounds without betting or folding.
    pub fn play_tournament(&mut self) {
        while !self.game_over {
            self.print_leaderboard();
            self.simulate_round();
            self.remove_losers();
            self.check_for_game_over();
        }
    }

    pub fn remove_losers(&mut self) {
        for (identifier, player) in self.players.clone() {
            if player.chips == 0 {
                println!(
                    "{} is out of chips and was removed from the game.",
                    player.name
                );
                self.remove_player(&identifier);
            }
        }
    }

    pub fn check_for_game_over(&mut self) -> bool {
        if self.players.is_empty() {
            println!("No players remaining. Game over!");
            self.end_game();
        }

        if self.players.len() == 1 {
            println!("One player remaining. Game over!");
            self.end_game();
        }

        self.game_over
    }

    /// End the game.
    pub fn end_game(&mut self) {
        self.game_over = true;
    }

    /// Print statistics about the players currently seated at the table.
    /// Players are printed from the highest to lowest amount of chips.
    pub fn print_leaderboard(&self) {
        // Step 1: Create a vector of (player_identifier, chips) tuples
        let mut player_stats: Vec<(&Uuid, &Player)> = self
            .seats
            .iter()
            .filter_map(|player_identifier| {
                self.players
                    .get(player_identifier)
                    .map(|player| (player_identifier, player))
            })
            .collect();

        // Step 2: Sort players by the number of chips (descending order)
        player_stats.sort_by(|(_, player1), (_, player2)| player2.chips.cmp(&player1.chips));

        println!("***************");
        println!("* LEADERBOARD *");
        println!("***************");

        // Step 3: Print the sorted list
        for &(_player_identifier, player) in &player_stats {
            println!(
                "{}: {} chip{}",
                player.name,
                player.chips,
                if player.chips == 1 { "" } else { "s" }
            );
        }
        println!();
    }

    /// Simulates a single round with no betting or folding.
    pub fn simulate_round(&mut self) {
        // Pre-round
        self.rotate_dealer();
        self.shuffle_deck();
        self.add_players_to_main_pot();
        self.print_dealer();
        self.post_blind(true);
        self.post_blind(false);

        println!();

        // Initializing these as Hand because it is a Vec<Card> that can print as symbols if needed
        let mut table_cards = Hand::new();
        let mut burned_cards = Hand::new();
        let player_hands = self.deal_hands_to_all_players();

        // Flop
        if let Some(card) = self.deal_card() {
            burned_cards.push(card);
        }

        for _ in 0..3 {
            if let Some(card) = self.deal_card() {
                table_cards.push(card);
            }
        }

        // Turn
        if let Some(card) = self.deal_card() {
            burned_cards.push(card);
        }

        if let Some(card) = self.deal_card() {
            table_cards.push(card);
        }

        // River
        if let Some(card) = self.deal_card() {
            burned_cards.push(card);
        }

        if let Some(card) = self.deal_card() {
            table_cards.push(card);
        }

        println!("Table cards:");
        println!("{}", table_cards.to_symbols());
        println!();

        // Determine winners
        let winning_players = self.rank_all_hands(&player_hands, &table_cards);
        self.determine_round_result(&winning_players);

        // Post-round
        self.reset_deck(player_hands, table_cards, burned_cards);
        self.reset_pots();
    }

    /// Shuffle the game's deck.
    /// This is required at the start of every round.
    pub fn shuffle_deck(&mut self) {
        self.deck.shuffle();
    }

    /// Rotate the dealer button clockwise to the next player.
    /// This must happen before the start of the next round.
    /// This will also update the small blind and big blind players.
    pub fn rotate_dealer(&mut self) {
        self.dealer_seat_index = (self.dealer_seat_index + 1) % self.seats.len();
    }

    /// Print the name of the player that has the dealer button for the round.
    pub fn print_dealer(&self) {
        if let Some(dealer_identifier) = self.seats.get(self.dealer_seat_index) {
            if let Some(dealer) = self.players.get(dealer_identifier) {
                println!("{} is the dealer.", dealer.name);
            } else {
                eprintln!(
                    "Error: Unable to find the dealer with the id {}",
                    dealer_identifier
                );
            }
        } else {
            eprintln!(
                "Error: Unable to find the dealer at seat {}",
                self.dealer_seat_index
            );
        }
    }

    /// Add all players at the table to the main betting pot.
    pub fn add_players_to_main_pot(&mut self) {
        for (identifier, player) in self.players.clone() {
            self.main_pot.add_player(identifier, player);
        }
    }

    /// Get the seat index of the small blind player.
    /// This must happen before the start of the next round.
    /// This must happen after rotate_dealer() is executed.
    pub fn get_small_blind_seat_index(&self) -> usize {
        (self.dealer_seat_index + 1) % self.seats.len()
    }

    /// Get the seat index of the small blind player.
    /// This must happen before the start of the next round.
    /// This must happen after rotate_dealer() is executed.
    pub fn get_big_blind_seat_index(&self) -> usize {
        (self.dealer_seat_index + 2) % self.seats.len()
    }

    /// Get the seat index of the player to the left of the big blind.
    /// Aka under the gun.
    pub fn get_under_the_gun_seat_index(&self) -> usize {
        self.rotate_current_player(self.get_big_blind_seat_index())
    }

    pub fn subtract_chips_from_player(&mut self, player_identifier: &Uuid, amount: u32) {
        if let Some(player) = self.players.get_mut(player_identifier) {
            player.subtract_chips(amount);
        }
    }

    pub fn add_chips_to_main_pot(&mut self, amount: u32) {
        self.main_pot.add_chips(amount);
    }

    /// Post the blind amount for either the small blind or the big blind.
    /// Create a side pot if the player could not post the full blind amount.
    pub fn post_blind(&mut self, is_small_blind: bool) {
        let seat_index = if is_small_blind {
            self.get_small_blind_seat_index()
        } else {
            self.get_big_blind_seat_index()
        };

        if let Some(player_identifier) = self.seats.get(seat_index) {
            if let Some(player) = self.players.get_mut(player_identifier) {
                let blind_amount = if is_small_blind {
                    self.small_blind_amount
                } else {
                    self.big_blind_amount
                };

                if player.chips >= blind_amount {
                    println!(
                        "{} posted the {} blind with {} chip{}.",
                        player.name,
                        if is_small_blind { "small" } else { "big" },
                        blind_amount,
                        if blind_amount == 1 { "" } else { "s" }
                    );

                    player.subtract_chips(blind_amount);
                    self.main_pot.add_chips(blind_amount);
                } else if player.chips > 0 {
                    let partial_blind_amount = player.chips;
                    player.subtract_chips(partial_blind_amount);
                    self.main_pot.add_chips(partial_blind_amount);

                    // todo: Should this be cloning the main pot's players?
                    // What if the small blind didn't have enough chips to cover.
                    // They probably shouldn't be included if this were triggered again for the big blind.
                    // Handling side pot creation
                    let side_pot_players: HashMap<Uuid, Player> = self
                        .main_pot
                        .players
                        .clone()
                        .iter()
                        .filter(|(&id, _)| id != *player_identifier)
                        .map(|(&id, player)| (id, player.clone()))
                        .collect();
                    let side_pot_amount = blind_amount - partial_blind_amount;
                    let side_pot = Pot::new(side_pot_amount, side_pot_players);
                    self.side_pots.push(side_pot);

                    println!(
                        "{} posted {} to cover part of the {} blind. The remaining {} has gone into a side pot.",
                        player.name,
                        partial_blind_amount,
                        if is_small_blind { "small" } else { "big" },
                        side_pot_amount
                    );
                } else {
                    eprintln!(
                        "Error: The player has no chips and should not be playing this hand."
                    );
                }
            } else {
                eprintln!(
                    "Error: Unable to find player with the id {}",
                    player_identifier
                );
            }
        } else {
            eprintln!("Error: Unable to find player at seat {}", seat_index);
        }
    }

    pub fn get_small_blind_amount(&self) -> u32 {
        self.small_blind_amount
    }

    pub fn get_big_blind_amount(&self) -> u32 {
        self.big_blind_amount
    }

    /// Deal hands of two cards to every player starting with the player to the left of the dealer.
    pub fn deal_hands_to_all_players(&mut self) -> HashMap<Uuid, Hand> {
        let mut player_hands: HashMap<Uuid, Hand> = HashMap::new();
        let mut current_player_seat_index = self.get_small_blind_seat_index();

        // Deal cards to player starting to the left of the dealer
        while current_player_seat_index != self.dealer_seat_index {
            if let Some(current_player_identifier) = self.seats.get(current_player_seat_index) {
                if let Some(current_player) = self.players.get(current_player_identifier).cloned() {
                    if let Some(hand) = self.deal_hand() {
                        println!("Hand dealt to {}.", current_player.name);
                        player_hands.insert(current_player.identifier, hand);
                    } else {
                        eprintln!("Error: Unable to deal hand.");
                    }
                } else {
                    eprintln!(
                        "Error: Unable to find player with the id {}",
                        current_player_identifier
                    )
                }
            } else {
                eprintln!(
                    "Error: Unable to find player at the seat {}",
                    current_player_seat_index
                )
            }

            // Move to the next player
            current_player_seat_index = (current_player_seat_index + 1) % self.seats.len();
        }

        // Deal cards to the dealer
        if let Some(dealer_identifier) = self.seats.get(self.dealer_seat_index) {
            if let Some(dealer) = self.players.get(dealer_identifier).cloned() {
                if let Some(hand) = self.deal_hand() {
                    println!("Hand dealt to {}.", dealer.name);
                    player_hands.insert(dealer.identifier, hand);
                } else {
                    eprintln!("Error: Unable to deal hand.")
                }
            } else {
                eprintln!(
                    "Error: Unable to find player with the id {}",
                    dealer_identifier
                )
            }
        } else {
            eprintln!(
                "Error: Unable to find player at the seat {}",
                self.dealer_seat_index
            )
        }

        println!();

        player_hands
    }

    /// Get a mutable reference to a Player via their seat index.
    pub fn get_player_at_seat(&mut self, seat_index: usize) -> Option<&mut Player> {
        if let Some(player_identifier) = self.seats.get(seat_index) {
            if let Some(player) = self.players.get_mut(player_identifier) {
                return Some(player);
            }
        }

        None
    }

    /// Rotate the current player's seat index.
    pub fn rotate_current_player(&self, current_player_seat_index: usize) -> usize {
        (current_player_seat_index + 1) % self.seats.len()
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

    /// Deals a single card.
    pub fn deal_card(&mut self) -> Option<Card> {
        // todo: change to deck.deal_face_down for all other players after testing is completed.
        if let Some(card) = self.deck.deal_face_up() {
            return Some(card);
        }

        None
    }

    /// Rank the provided hands to determine which hands are the best.
    pub fn rank_all_hands(
        &self,
        player_hands: &HashMap<Uuid, Hand>,
        table_cards: &Hand,
    ) -> HashMap<Uuid, Vec<HandRank>> {
        let mut winning_players: HashMap<Uuid, Vec<HandRank>> = HashMap::new();
        let mut best_hand: Vec<(HandRank, &Hand)> = Vec::new();

        for (player_identifier, hand) in player_hands.iter() {
            if let Some(player) = self.players.get(player_identifier) {
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
                    winning_players.insert(player.identifier, hand_rank_vec);
                    continue;
                }

                if let Some((best_hand_rank, best_hand_cards)) = best_hand.last() {
                    match hand_rank.cmp(best_hand_rank) {
                        std::cmp::Ordering::Equal => {
                            // If hand ranks are equal and are made up of less than 5 cards then check for a kicker (high card).
                            if hand_rank.len() < 5 {
                                let mut current_cards_and_table_cards =
                                    table_cards.get_cards().clone();
                                current_cards_and_table_cards.push(hand.cards[0]);
                                current_cards_and_table_cards.push(hand.cards[1]);

                                // Get the kicker for current hand rank
                                let mut cards_not_used_in_current_hand_rank = Vec::new();
                                for card in current_cards_and_table_cards {
                                    // Check to see that the kicker is not part of the current hand rank.
                                    if !hand_rank.contains(&card) {
                                        cards_not_used_in_current_hand_rank.push(card);
                                    }
                                }
                                let current_hand_kicker =
                                    get_high_card_value(&cards_not_used_in_current_hand_rank)
                                        .unwrap();

                                // Get the kicker for the best hand rank
                                let mut best_hand_cards_and_table_cards =
                                    table_cards.get_cards().clone();
                                best_hand_cards_and_table_cards.push(best_hand_cards.cards[0]);
                                best_hand_cards_and_table_cards.push(best_hand_cards.cards[1]);

                                let mut cards_not_used_in_best_hand_rank = Vec::new();
                                for card in best_hand_cards_and_table_cards {
                                    // Check to see that the kicker is not part of the best hand rank.
                                    if !best_hand_rank.contains(&card) {
                                        cards_not_used_in_best_hand_rank.push(card);
                                    }
                                }
                                let best_hand_kicker =
                                    get_high_card_value(&cards_not_used_in_best_hand_rank).unwrap();

                                // If there is a tie, but the best hand has a higher kicker, add that kicker to the best hand.
                                if let Some((leading_player, leading_hand_vec)) =
                                    winning_players.iter().next()
                                {
                                    if leading_hand_vec.len() < 2 {
                                        winning_players
                                            .entry(*leading_player)
                                            .or_default()
                                            .push(HandRank::HighCard(best_hand_kicker));
                                    }
                                }

                                // Compare the kickers to determine the best hand.
                                match current_hand_kicker.rank.cmp(&best_hand_kicker.rank) {
                                    std::cmp::Ordering::Equal => {
                                        best_hand.push((hand_rank, hand));
                                        hand_rank_vec.push(HandRank::HighCard(current_hand_kicker));
                                        winning_players.insert(player.identifier, hand_rank_vec);
                                    }
                                    std::cmp::Ordering::Greater => {
                                        best_hand.clear();
                                        best_hand.push((hand_rank, hand));
                                        winning_players.clear();
                                        hand_rank_vec.push(HandRank::HighCard(current_hand_kicker));
                                        winning_players.insert(player.identifier, hand_rank_vec);
                                    }
                                    std::cmp::Ordering::Less => {
                                        // Do nothing, as the best hand remains unchanged.
                                    }
                                }
                            } else {
                                // If the hand uses too many cards to consider a kicker, push the new hand.
                                best_hand.push((hand_rank, hand));
                                winning_players.insert(player.identifier, hand_rank_vec);
                            }
                        }
                        std::cmp::Ordering::Greater => {
                            best_hand.clear();
                            best_hand.push((hand_rank, hand));
                            winning_players.clear();
                            winning_players.insert(player.identifier, hand_rank_vec);
                        }
                        std::cmp::Ordering::Less => {
                            // Do nothing, as the best hand remains unchanged.
                        }
                    }
                }
            } else {
                eprintln!(
                    "Error: Unable to find player with the id {}",
                    player_identifier
                )
            }
        }

        winning_players
    }

    // todo: implement side pot logic
    /// Determine which player or players won the round and how the pot(s) should be divided.
    pub fn determine_round_result(&mut self, winning_players: &HashMap<Uuid, Vec<HandRank>>) {
        match winning_players.len() {
            1 => {
                if let Some((player_identifier, winning_hand_rank_vec)) =
                    winning_players.iter().next()
                {
                    if let Some(player) = self.players.get_mut(player_identifier) {
                        if winning_hand_rank_vec.len() > 1 {
                            println!(
                                "\n{} wins with {} and {}",
                                player.name, winning_hand_rank_vec[0], winning_hand_rank_vec[1]
                            );
                        } else {
                            println!(
                                "\n{} wins with {}",
                                player.name,
                                winning_hand_rank_vec.last().unwrap()
                            );
                        }

                        // Allocate winnings from the main pot to the winner.
                        let main_pot_chips: u32 = self.main_pot.distribute_all_chips();
                        player.add_chips(main_pot_chips);

                        println!(
                            "{} wins {} chip{}.",
                            player.name,
                            main_pot_chips,
                            if main_pot_chips == 1 { "" } else { "s" }
                        );
                    } else {
                        eprintln!(
                            "Error: Unable to get player with the id {}.",
                            player_identifier
                        );
                    }
                }
            }
            n if n > 1 => {
                // Divide the main pot equally for the multiple winners.
                // In the event of a pot that cannot be split equally, the additional chips are allocated
                // to each player starting with the first winning player to the left of the dealer.
                // The winning players are already ordered starting from the left of the dealer,
                // which helps when allocating uneven winnings.

                let player_count = match u32::try_from(winning_players.len()) {
                    Ok(number) => number,
                    Err(error) => {
                        panic!("Couldn't convert {} to u32: {error}", winning_players.len())
                    }
                };

                let main_pot_chips: u32 = self.main_pot.distribute_all_chips();
                let divided_chips_amount = main_pot_chips / player_count;
                let remainder_chips_amount = main_pot_chips % player_count;
                // Create a vector to store the total chips each player will receive.
                let mut total_chips = vec![divided_chips_amount; winning_players.len()];

                // Distribute the remainder starting from the first winning player to the left of the dealer.
                for i in 0..remainder_chips_amount {
                    total_chips[i as usize] += 1;
                }

                // Create a map to store the position of each player
                let mut player_positions = HashMap::new();
                for (index, &seat) in self.seats.iter().enumerate() {
                    player_positions.insert(seat, index);
                }

                // Sort the winning players based on their positions relative to the dealer
                let mut sorted_winning_players: Vec<_> = winning_players.iter().collect();
                sorted_winning_players.sort_by_key(|(player_id, _)| {
                    let pos = player_positions.get(player_id).unwrap();
                    (*pos + self.seats.len() - (self.dealer_seat_index + 1)) % self.seats.len()
                });

                // Allocate the calculated total amount of chips to each player and print the result.
                for (i, (player_identifier, tied_hand_rank)) in
                    sorted_winning_players.iter().enumerate()
                {
                    if let Some(player) = self.players.get_mut(player_identifier) {
                        if tied_hand_rank.len() > 1 {
                            println!(
                                "\n{} pushes with {} and {}",
                                player.name, tied_hand_rank[0], tied_hand_rank[1]
                            );
                        } else {
                            println!(
                                "\n{} pushes with {}",
                                player.name,
                                tied_hand_rank.last().unwrap()
                            );
                        }

                        // Allocate winnings from the main pot to the winner.
                        let chips_won = total_chips[i];
                        player.add_chips(chips_won);
                        println!(
                            "{} wins {} chip{}.",
                            player.name,
                            chips_won,
                            if chips_won == 1 { "" } else { "s" }
                        );
                    } else {
                        eprintln!(
                            "Error: Unable to get player with the id {}.",
                            player_identifier
                        );
                    }
                }
            }
            _ => {
                panic!("Error: No winning player was determined.");
            }
        }
    }

    /// Returns all the cards to the deck.
    pub fn reset_deck(
        &mut self,
        player_hands: HashMap<Uuid, Hand>,
        table_cards: Hand,
        burned_cards: Hand,
    ) {
        // Return cards from the players' hands to the deck
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

    /// Resets the main pot and all side pots to be empty.
    pub fn reset_pots(&mut self) {
        self.main_pot = Pot::new(0, HashMap::new());
        self.side_pots = Vec::new();
    }
}

impl Default for TexasHoldEm {
    fn default() -> Self {
        Self {
            game_over: false,
            deck: Deck::new(),
            players: HashMap::new(),
            seats: Vec::new(),
            dealer_seat_index: 0,
            main_pot: Pot::new(0, HashMap::new()),
            side_pots: Vec::new(),
            minimum_chips_buy_in_amount: 100,
            maximum_players_count: 10,
            small_blind_amount: 2,
            big_blind_amount: 5,
        }
    }
}

/// The Pot manages how many chips have been bet and who the winnings should be allocated to.
#[derive(Clone)]
struct Pot {
    amount: u32,
    players: HashMap<Uuid, Player>,
}

impl Pot {
    fn new(amount: u32, players: HashMap<Uuid, Player>) -> Self {
        Self { amount, players }
    }

    fn add_player(&mut self, identifier: Uuid, player: Player) {
        self.players.insert(identifier, player);
    }

    fn add_chips(&mut self, chips: u32) {
        self.amount += chips;
    }

    fn distribute_all_chips(&mut self) -> u32 {
        let chips = self.amount;
        self.amount = 0;
        chips
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use casino_cards::card;
    use casino_cards::card::{Card, Rank, Suit};

    /// Tests rank_all_hands().
    ///
    /// Tests that a single winner is correctly chosen.
    #[test]
    fn rank_all_hands_identifies_winner() {
        let mut game = TexasHoldEm::new(100, 10, 1, 3);

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

        let mut player_hands: HashMap<Uuid, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        game.add_player(player1.clone()).unwrap();
        let player1_cards: Vec<Card> = vec![five_of_clubs, nine_of_clubs];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.identifier, player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        game.add_player(player2.clone()).unwrap();
        let player2_cards: Vec<Card> = vec![two_of_hearts, ten_of_spades];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.identifier, player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        game.add_player(player3.clone()).unwrap();
        let player3_cards: Vec<Card> = vec![four_of_diamonds, queen_of_hearts];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.identifier, player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        game.add_player(player4.clone()).unwrap();
        let player4_cards: Vec<Card> = vec![ace_of_hearts, ace_of_spades];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.identifier, player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        game.add_player(player5.clone()).unwrap();
        let player5_cards: Vec<Card> = vec![six_of_diamonds, nine_of_hearts];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.identifier, player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 1);
        assert!(leading_players.contains_key(&player1.identifier));
        assert_eq!(leading_players.get(&player1.identifier).unwrap()[0], flush);
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that a single winner is correctly chosen when the winning hand ranks combines the table and hand
    /// but one player has a higher kicker (high card) than the other.
    #[test]
    fn rank_all_hands_identifies_winner_based_on_kicker_with_hand_winner() {
        let mut game = TexasHoldEm::new(100, 10, 1, 3);

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

        let mut player_hands: HashMap<Uuid, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        game.add_player(player1.clone()).unwrap();
        let player1_cards: Vec<Card> = vec![nine_of_hearts, ace_of_spades];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.identifier, player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        game.add_player(player2.clone()).unwrap();
        let player2_cards: Vec<Card> = vec![nine_of_spades, six_of_diamonds];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.identifier, player2_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 1);
        assert!(leading_players.contains_key(&player1.identifier));
        assert_eq!(
            leading_players.get(&player1.identifier).unwrap()[0],
            two_pair1
        );
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that a single winner is correctly chosen when the winning hand ranks is on the table
    /// but one player has a higher kicker (high card) than the other.
    #[test]
    fn rank_all_hands_identifies_winner_based_on_kicker_with_table_winner() {
        let mut game = TexasHoldEm::new(100, 10, 1, 3);

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

        let mut player_hands: HashMap<Uuid, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        game.add_player(player1.clone()).unwrap();
        let player1_cards: Vec<Card> = vec![ten_of_spades, ace_of_spades];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.identifier, player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        game.add_player(player2.clone()).unwrap();
        let player2_cards: Vec<Card> = vec![six_of_diamonds, ten_of_hearts];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.identifier, player2_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 1);
        assert!(leading_players.contains_key(&player1.identifier));
        assert_eq!(
            leading_players.get(&player1.identifier).unwrap()[0],
            two_pair1
        );
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that a single winner is correctly chosen.
    #[test]
    fn rank_all_hands_identifies_push_with_winning_table_flush() {
        let mut game = TexasHoldEm::new(100, 10, 1, 3);

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

        let mut player_hands: HashMap<Uuid, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        game.add_player(player1.clone()).unwrap();
        let player1_cards: Vec<Card> = vec![two_of_diamonds, eight_of_spades];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.identifier, player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        game.add_player(player2.clone()).unwrap();
        let player2_cards: Vec<Card> = vec![two_of_hearts, ten_of_spades];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.identifier, player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        game.add_player(player3.clone()).unwrap();
        let player3_cards: Vec<Card> = vec![four_of_diamonds, queen_of_hearts];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.identifier, player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        game.add_player(player4.clone()).unwrap();
        let player4_cards: Vec<Card> = vec![ace_of_hearts, ace_of_spades];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.identifier, player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        game.add_player(player5.clone()).unwrap();
        let player5_cards: Vec<Card> = vec![six_of_diamonds, nine_of_hearts];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.identifier, player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 5);
        assert_eq!(leading_players.get(&player1.identifier).unwrap()[0], flush);
        assert_eq!(leading_players.get(&player2.identifier).unwrap()[0], flush);
        assert_eq!(leading_players.get(&player3.identifier).unwrap()[0], flush);
        assert_eq!(leading_players.get(&player4.identifier).unwrap()[0], flush);
        assert_eq!(leading_players.get(&player5.identifier).unwrap()[0], flush);
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that a single winner is correctly chosen.
    #[test]
    fn rank_all_hands_identifies_higher_flush_in_hand_wins() {
        let mut game = TexasHoldEm::new(100, 10, 1, 3);

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

        let winning_flush = HandRank::Flush([
            three_of_clubs,
            seven_of_clubs,
            nine_of_clubs,
            jack_of_clubs,
            king_of_clubs,
        ]);

        let mut player_hands: HashMap<Uuid, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        game.add_player(player1.clone()).unwrap();
        let player1_cards: Vec<Card> = vec![five_of_clubs, eight_of_spades];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.identifier, player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        game.add_player(player2.clone()).unwrap();
        let player2_cards: Vec<Card> = vec![two_of_hearts, seven_of_clubs];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.identifier, player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        game.add_player(player3.clone()).unwrap();
        let player3_cards: Vec<Card> = vec![four_of_diamonds, queen_of_hearts];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.identifier, player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        game.add_player(player4.clone()).unwrap();
        let player4_cards: Vec<Card> = vec![ace_of_hearts, ace_of_spades];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.identifier, player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        game.add_player(player5.clone()).unwrap();
        let player5_cards: Vec<Card> = vec![six_of_diamonds, nine_of_hearts];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.identifier, player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_ne!(flush1, flush2);
        assert_eq!(winning_flush, flush2);
        assert_eq!(leading_players.len(), 1);
        assert!(!leading_players.contains_key(&player1.identifier));
        assert!(leading_players.contains_key(&player2.identifier));
        assert_eq!(flush2, leading_players.get(&player2.identifier).unwrap()[0]);
        assert_eq!(
            winning_flush,
            leading_players.get(&player2.identifier).unwrap()[0]
        );
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that all players push when the winning hand is on the table.
    #[test]
    fn rank_all_hands_identifies_push_with_winning_table_straight() {
        let mut game = TexasHoldEm::new(100, 10, 1, 3);

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

        let mut player_hands: HashMap<Uuid, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        game.add_player(player1.clone()).unwrap();
        let player1_cards: Vec<Card> = vec![three_of_clubs, four_of_hearts];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.identifier, player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        game.add_player(player2.clone()).unwrap();
        let player2_cards: Vec<Card> = vec![five_of_diamonds, six_of_clubs];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.identifier, player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        game.add_player(player3.clone()).unwrap();
        let player3_cards: Vec<Card> = vec![seven_of_spades, nine_of_spades];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.identifier, player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        game.add_player(player4.clone()).unwrap();
        let player4_cards: Vec<Card> = vec![two_of_diamonds, five_of_clubs];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.identifier, player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        game.add_player(player5.clone()).unwrap();
        let player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.identifier, player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 5);
        assert_eq!(
            leading_players.get(&player1.identifier).unwrap()[0],
            straight
        );
        assert_eq!(
            leading_players.get(&player2.identifier).unwrap()[0],
            straight
        );
        assert_eq!(
            leading_players.get(&player3.identifier).unwrap()[0],
            straight
        );
        assert_eq!(
            leading_players.get(&player4.identifier).unwrap()[0],
            straight
        );
        assert_eq!(
            leading_players.get(&player5.identifier).unwrap()[0],
            straight
        );
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that multiple equal hands result in a push for all involved players.
    #[test]
    fn rank_all_hands_identifies_push_with_equal_winning_hand_straights() {
        let mut game = TexasHoldEm::new(100, 10, 1, 3);

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

        let mut player_hands: HashMap<Uuid, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        game.add_player(player1.clone()).unwrap();
        let player1_cards: Vec<Card> = vec![four_of_hearts, ten_of_diamonds];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.identifier, player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        game.add_player(player2.clone()).unwrap();
        let player2_cards: Vec<Card> = vec![five_of_diamonds, ten_of_hearts];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.identifier, player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        game.add_player(player3.clone()).unwrap();
        let player3_cards: Vec<Card> = vec![seven_of_spades, nine_of_spades];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.identifier, player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        game.add_player(player4.clone()).unwrap();
        let player4_cards: Vec<Card> = vec![two_of_diamonds, five_of_clubs];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.identifier, player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        game.add_player(player5.clone()).unwrap();
        let player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.identifier, player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 2);
        assert!(leading_players.contains_key(&player1.identifier));
        assert!(leading_players.contains_key(&player2.identifier));
        assert_eq!(
            leading_players.get(&player1.identifier).unwrap()[0],
            straight1
        );
        assert_eq!(
            leading_players.get(&player2.identifier).unwrap()[0],
            straight2
        );
        assert_eq!(
            *leading_players.get(&player1.identifier).unwrap(),
            *leading_players.get(&player2.identifier).unwrap()
        );
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that multiple equal hands result in a push for all involved players.
    #[test]
    fn rank_all_hands_identifies_higher_straight_beats_ace_low_straight() {
        let mut game = TexasHoldEm::new(100, 10, 1, 3);

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

        let mut player_hands: HashMap<Uuid, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player1 = game.new_player_with_chips("Player 1", 100);
        game.add_player(player1.clone()).unwrap();
        let player1_cards: Vec<Card> = vec![ten_of_diamonds, ace_of_hearts];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.identifier, player1_hand);

        let player2 = game.new_player_with_chips("Player 2", 100);
        game.add_player(player2.clone()).unwrap();
        let player2_cards: Vec<Card> = vec![six_of_spades, jack_of_clubs];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.identifier, player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        game.add_player(player3.clone()).unwrap();
        let player3_cards: Vec<Card> = vec![nine_of_spades, ten_of_hearts];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.identifier, player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        game.add_player(player4.clone()).unwrap();
        let player4_cards: Vec<Card> = vec![five_of_clubs, queen_of_spades];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.identifier, player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        game.add_player(player5.clone()).unwrap();
        let player5_cards: Vec<Card> = vec![jack_of_hearts, jack_of_spades];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.identifier, player5_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 1);
        assert!(!leading_players.contains_key(&player1.identifier));
        assert!(leading_players.contains_key(&player2.identifier));
        assert_eq!(
            leading_players.get(&player2.identifier).unwrap()[0],
            straight
        );
    }

    /// Tests rank_all_hands().
    ///
    /// Tests that hand ranking correctly updates the leader for a pair that is higher than the previously set high pair.
    #[test]
    fn rank_all_hands_identifies_higher_pair_as_winner_over_previous_high_pair() {
        let mut game = TexasHoldEm::new(100, 10, 1, 3);

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

        let mut player_hands: HashMap<Uuid, Hand> = HashMap::new();
        let table_cards = Hand::new_from_cards(table_cards);

        let player2 = game.new_player_with_chips("Player 2", 100);
        game.add_player(player2.clone()).unwrap();
        let player2_cards: Vec<Card> = vec![jack_of_spades, nine_of_spades];
        let player2_hand = Hand::new_from_cards(player2_cards);
        player_hands.insert(player2.identifier, player2_hand);

        let player3 = game.new_player_with_chips("Player 3", 100);
        game.add_player(player3.clone()).unwrap();
        let player3_cards: Vec<Card> = vec![nine_of_clubs, four_of_clubs];
        let player3_hand = Hand::new_from_cards(player3_cards);
        player_hands.insert(player3.identifier, player3_hand);

        let player4 = game.new_player_with_chips("Player 4", 100);
        game.add_player(player4.clone()).unwrap();
        let player4_cards: Vec<Card> = vec![six_of_clubs, ten_of_spades];
        let player4_hand = Hand::new_from_cards(player4_cards);
        player_hands.insert(player4.identifier, player4_hand);

        let player5 = game.new_player_with_chips("Player 5", 100);
        game.add_player(player5.clone()).unwrap();
        let player5_cards: Vec<Card> = vec![seven_of_hearts, queen_of_hearts];
        let player5_hand = Hand::new_from_cards(player5_cards);
        player_hands.insert(player5.identifier, player5_hand);

        let player6 = game.new_player_with_chips("Player 6", 100);
        game.add_player(player6.clone()).unwrap();
        let player6_cards: Vec<Card> = vec![ten_of_diamonds, ten_of_clubs];
        let player6_hand = Hand::new_from_cards(player6_cards);
        player_hands.insert(player6.identifier, player6_hand);

        let player1 = game.new_player_with_chips("Player 1", 100);
        game.add_player(player1.clone()).unwrap();
        let player1_cards: Vec<Card> = vec![king_of_diamonds, ace_of_hearts];
        let player1_hand = Hand::new_from_cards(player1_cards);
        player_hands.insert(player1.identifier, player1_hand);

        let leading_players = game.rank_all_hands(&player_hands, &table_cards);

        assert_eq!(leading_players.len(), 1);
        assert!(leading_players.contains_key(&player1.identifier));
        assert_eq!(leading_players.get(&player1.identifier).unwrap()[0], pair);
    }
}
