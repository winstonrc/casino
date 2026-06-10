//! The Texas Hold'em game engine.
//!
//! The engine owns the full hand lifecycle and all money: per-player
//! contributions, the board, who has folded or gone all-in, betting, and pot
//! distribution. Callers (a terminal UI, tests, a future network layer) drive it
//! with a thin loop — deal, run each betting street, then award the pots — and
//! supply a [`PokerAgent`] per player to decide actions.
//!
//! Side pots are computed at showdown from total contributions (see [`crate::pot`]).

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use casino_cards::card::Card;
use casino_cards::deck::Deck;
use casino_cards::hand::Hand;

use crate::agent::{AgentError, PlayerView, PokerAgent, Street};
use crate::betting::{legal_actions, resolve_action, BettingRound, PlayerAction, Resolved};
use crate::events::{ActionView, Blind, GameEvent, GameObserver, NullObserver, PotKind};
use crate::hand_rankings::{evaluate, ComparableHand};
use crate::player::Player;
use crate::pot::{build_pots, distribute_pots, refund_uncalled};

/// The result of running a betting street.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoundOutcome {
    /// Betting completed normally; continue to the next street or showdown.
    Continue,
    /// Only one player remains unfolded; skip remaining streets and award.
    HandOver,
    /// A player asked to quit the game.
    Quit,
}

/// The core of the Texas hold 'em game.
///
/// The game currently defaults to no-limit.
pub struct TexasHoldEm {
    game_over: bool,
    deck: Deck,
    players: HashMap<Uuid, Player>,
    seats: Vec<Uuid>,
    /// The player on the dealer button, tracked by id so the button survives
    /// players being removed between hands.
    dealer: Option<Uuid>,
    dealer_seat_index: usize,
    /// Total chips each player has put into the pot this hand (all streets). The
    /// sole input to side-pot construction.
    contributed: HashMap<Uuid, u32>,
    /// Players who have folded this hand.
    folded: HashSet<Uuid>,
    /// Players who are all-in this hand.
    all_in: HashSet<Uuid>,
    /// Each player's two hole cards for the current hand.
    player_hands: HashMap<Uuid, Hand>,
    /// The shared community cards.
    board: Hand,
    /// Burned and folded cards, returned to the deck at hand end.
    burned: Hand,
    minimum_chips_buy_in_amount: u32,
    maximum_players_count: usize,
    small_blind_amount: u32,
    big_blind_amount: u32,
    /// Receives the public narration of the hand. Defaults to a no-op sink.
    observer: Box<dyn GameObserver>,
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
            dealer: None,
            dealer_seat_index: 0,
            contributed: HashMap::new(),
            folded: HashSet::new(),
            all_in: HashSet::new(),
            player_hands: HashMap::new(),
            board: Hand::new(),
            burned: Hand::new(),
            minimum_chips_buy_in_amount,
            maximum_players_count,
            small_blind_amount,
            big_blind_amount,
            observer: Box::new(NullObserver),
        }
    }

    /// Set the observer that receives the hand's [`GameEvent`]s. Without one, the
    /// engine runs silently (the default [`NullObserver`]).
    pub fn set_observer(&mut self, observer: Box<dyn GameObserver>) {
        self.observer = observer;
    }

    /// Create a new player with zero chips.
    pub fn new_player(&self, name: &str) -> Player {
        Player::new(name)
    }

    /// Create a new player with a defined amount of chips.
    pub fn new_player_with_chips(&self, name: &str, chips: u32) -> Player {
        Player::new_with_chips(name, chips)
    }

    /// Add a player into the game.
    pub fn add_player(&mut self, player: Player) -> Result<(), &'static str> {
        if self.players.len() >= self.maximum_players_count {
            return Err("Unable to join the table. It is already at max capacity.");
        }

        if player.chips < self.minimum_chips_buy_in_amount {
            return Err("The player does not have enough chips to play at this table.");
        }

        self.seats.push(player.identifier);
        self.players.insert(player.identifier, player);
        Ok(())
    }

    /// Remove a player from the game.
    pub fn remove_player(&mut self, player_identifier: &Uuid) -> Option<Player> {
        self.players.get(player_identifier)?;
        self.seats.retain(|x| x != player_identifier);
        self.players.remove(player_identifier)
    }

    /// Remove players who have run out of chips and return their names (so the
    /// caller can announce them). The dealer button is tracked by id, so removing
    /// players (even the current button) does not seat a ghost.
    pub fn remove_losers(&mut self) -> Vec<String> {
        let broke: Vec<Uuid> = self
            .players
            .iter()
            .filter(|(_, p)| p.chips == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut removed = Vec::new();
        for id in broke {
            if let Some(player) = self.remove_player(&id) {
                removed.push(player.name);
            }
        }
        removed
    }

    /// Returns `true` and ends the game if one or zero players remain.
    pub fn check_for_game_over(&mut self) -> bool {
        if self.players.len() <= 1 {
            self.end_game();
        }

        self.game_over
    }

    /// End the game.
    pub fn end_game(&mut self) {
        self.game_over = true;
    }

    /// Shuffle the game's deck.
    pub fn shuffle_deck(&mut self) {
        self.deck.shuffle();
    }

    /// Rotate the dealer button clockwise to the next seated player.
    ///
    /// The button is tracked by player id. If the previous button player is still
    /// seated, it advances to the next seat; otherwise (first hand, or the button
    /// player busted) it falls to the first seat. This keeps `dealer_seat_index`
    /// always valid — a small simplification of formal dead-button rules.
    pub fn rotate_dealer(&mut self) {
        if self.seats.is_empty() {
            return;
        }
        let next = match self
            .dealer
            .and_then(|d| self.seats.iter().position(|s| *s == d))
        {
            Some(pos) => (pos + 1) % self.seats.len(),
            None => 0,
        };
        self.dealer_seat_index = next;
        self.dealer = Some(self.seats[next]);
    }

    /// Seat index of the small blind. Heads-up, the button posts the small blind.
    pub fn get_small_blind_seat_index(&self) -> usize {
        let len = self.seats.len();
        if len == 2 {
            self.dealer_seat_index
        } else {
            (self.dealer_seat_index + 1) % len
        }
    }

    /// Seat index of the big blind. Heads-up, the non-button player posts it.
    pub fn get_big_blind_seat_index(&self) -> usize {
        let len = self.seats.len();
        if len == 2 {
            (self.dealer_seat_index + 1) % len
        } else {
            (self.dealer_seat_index + 2) % len
        }
    }

    /// Seat index of the player to the left of the big blind (under the gun).
    pub fn get_under_the_gun_seat_index(&self) -> usize {
        (self.get_big_blind_seat_index() + 1) % self.seats.len()
    }

    pub fn get_small_blind_amount(&self) -> u32 {
        self.small_blind_amount
    }

    pub fn get_big_blind_amount(&self) -> u32 {
        self.big_blind_amount
    }

    /// Total chips across all pots (main + sides) for the current hand.
    pub fn pot_total(&self) -> u32 {
        self.contributed.values().sum()
    }

    /// The community cards.
    pub fn board(&self) -> &Hand {
        &self.board
    }

    /// A player's hole cards, if they are still in the hand.
    pub fn player_hand(&self, id: &Uuid) -> Option<&Hand> {
        self.player_hands.get(id)
    }

    /// A seated player by id.
    pub fn player(&self, id: &Uuid) -> Option<&Player> {
        self.players.get(id)
    }

    /// The seated players' ids, in seat order.
    pub fn seats(&self) -> &[Uuid] {
        &self.seats
    }

    /// Number of players still in the current hand (seated and not folded).
    fn live_count(&self) -> usize {
        self.seats
            .iter()
            .filter(|id| !self.folded.contains(id))
            .count()
    }

    /// Begin a hand: rotate the button, shuffle, post blinds, and deal hole cards.
    pub fn begin_hand(&mut self) {
        self.rotate_dealer();
        self.shuffle_deck();
        if let Some(dealer) = self
            .seats
            .get(self.dealer_seat_index)
            .and_then(|id| self.players.get(id))
        {
            let event = GameEvent::HandStarted {
                dealer: dealer.name.clone(),
            };
            self.observer.notify(&event);
        }
        self.post_blind(true);
        self.post_blind(false);
        self.deal_hands_to_all_players();
    }

    /// Post a blind, going all-in if the player cannot cover it. The blind is
    /// recorded in `contributed`, which feeds side-pot construction.
    fn post_blind(&mut self, is_small_blind: bool) {
        if self.seats.is_empty() {
            return;
        }
        let seat_index = if is_small_blind {
            self.get_small_blind_seat_index()
        } else {
            self.get_big_blind_seat_index()
        };
        let blind_amount = if is_small_blind {
            self.small_blind_amount
        } else {
            self.big_blind_amount
        };
        let id = self.seats[seat_index];
        let chips = self.players.get(&id).map_or(0, |p| p.chips);
        let posted = blind_amount.min(chips);

        if let Some(player) = self.players.get_mut(&id) {
            player.subtract_chips(posted);
        }
        *self.contributed.entry(id).or_insert(0) += posted;
        let all_in = posted < blind_amount;
        if all_in {
            self.all_in.insert(id);
        }

        if let Some(name) = self.players.get(&id).map(|p| p.name.clone()) {
            let event = GameEvent::BlindPosted {
                player: name,
                blind: if is_small_blind {
                    Blind::Small
                } else {
                    Blind::Big
                },
                amount: posted,
                all_in,
            };
            self.observer.notify(&event);
        }
    }

    /// Deal two hole cards to every seated player.
    fn deal_hands_to_all_players(&mut self) {
        let n = self.seats.len();
        if n == 0 {
            return;
        }
        let start = self.get_small_blind_seat_index();
        for offset in 0..n {
            let id = self.seats[(start + offset) % n];
            if let Some(hand) = self.deal_hand() {
                self.player_hands.insert(id, hand);
            }
        }
    }

    /// Burn a card, then deal the three flop cards to the board.
    pub fn deal_flop(&mut self) {
        if let Some(card) = self.deal_card() {
            self.burned.push(card);
        }
        for _ in 0..3 {
            if let Some(card) = self.deal_card() {
                self.board.push(card);
            }
        }
        self.emit_street_dealt(Street::Flop);
    }

    /// Burn a card, then deal the turn card to the board.
    pub fn deal_turn(&mut self) {
        self.deal_single_board_card();
        self.emit_street_dealt(Street::Turn);
    }

    /// Burn a card, then deal the river card to the board.
    pub fn deal_river(&mut self) {
        self.deal_single_board_card();
        self.emit_street_dealt(Street::River);
    }

    fn deal_single_board_card(&mut self) {
        if let Some(card) = self.deal_card() {
            self.burned.push(card);
        }
        if let Some(card) = self.deal_card() {
            self.board.push(card);
        }
    }

    fn emit_street_dealt(&mut self, street: Street) {
        let event = GameEvent::StreetDealt {
            street,
            board: self.board.cards.clone(),
            pot: self.pot_total(),
        };
        self.observer.notify(&event);
    }

    /// Deal a hand of two cards.
    fn deal_hand(&mut self) -> Option<Hand> {
        let mut hand = Hand::new();
        hand.push(self.deal_card()?);
        hand.push(self.deal_card()?);
        Some(hand)
    }

    /// Deal a single card.
    pub fn deal_card(&mut self) -> Option<Card> {
        self.deck.deal_face_up()
    }

    /// Run a betting round for the given street, asking each player's agent to act.
    ///
    /// Returns [`RoundOutcome::HandOver`] if all but one player folds,
    /// [`RoundOutcome::Quit`] if a player quits, and [`RoundOutcome::Continue`]
    /// when betting completes normally.
    pub fn run_betting_round(
        &mut self,
        street: Street,
        agents: &mut HashMap<Uuid, Box<dyn PokerAgent>>,
    ) -> RoundOutcome {
        let n = self.seats.len();
        if n == 0 {
            return RoundOutcome::HandOver;
        }

        let actors: Vec<Uuid> = self
            .seats
            .iter()
            .copied()
            .filter(|id| !self.folded.contains(id) && !self.all_in.contains(id))
            .collect();

        let (current_bet, committed_seed, bb_option, mut seat) = if street == Street::Preflop {
            let bb_id = self.seats[self.get_big_blind_seat_index()];
            let bb_option = if self.all_in.contains(&bb_id) {
                None
            } else {
                Some(bb_id)
            };
            // Pre-flop, committed-this-street equals the blinds posted so far.
            (
                self.big_blind_amount,
                self.contributed.clone(),
                bb_option,
                self.first_to_act_preflop_seat(),
            )
        } else {
            (0, HashMap::new(), None, self.first_to_act_postflop_seat())
        };

        let mut round = BettingRound::new(
            &actors,
            current_bet,
            self.big_blind_amount,
            committed_seed,
            bb_option,
        );

        loop {
            if self.live_count() <= 1 {
                return RoundOutcome::HandOver;
            }
            if round.is_closed() {
                break;
            }

            let id = self.seats[seat];
            if self.folded.contains(&id) || self.all_in.contains(&id) || !round.needs_to_act(id) {
                seat = (seat + 1) % n;
                continue;
            }

            let view = self.build_view(id, street, &round);
            let action = match agents.get_mut(&id).map(|agent| agent.decide(&view)) {
                Some(Ok(action)) => action,
                Some(Err(AgentError::Quit)) | Some(Err(AgentError::Eof)) => {
                    return RoundOutcome::Quit
                }
                None => PlayerAction::Fold,
            };

            let chips = self.players.get(&id).map_or(0, |p| p.chips);
            let resolved = resolve_action(
                action,
                chips,
                round.committed(id),
                round.current_bet,
                round.last_raise_increment,
            )
            .unwrap_or_else(|_| {
                // An agent should only return legal actions; treat anything else
                // as a fold rather than panicking.
                resolve_action(
                    PlayerAction::Fold,
                    chips,
                    round.committed(id),
                    round.current_bet,
                    round.last_raise_increment,
                )
                .expect("fold is always legal")
            });

            if let Some(player) = self.players.get_mut(&id) {
                player.subtract_chips(resolved.paid);
            }
            *self.contributed.entry(id).or_insert(0) += resolved.paid;
            self.announce_action(id, &resolved, round.current_bet);

            if resolved.folded {
                self.folded.insert(id);
                if let Some(hand) = self.player_hands.remove(&id) {
                    for card in hand.cards {
                        self.burned.push(card);
                    }
                }
            }
            if resolved.all_in {
                self.all_in.insert(id);
            }

            let live_after: HashSet<Uuid> = self
                .seats
                .iter()
                .copied()
                .filter(|p| !self.folded.contains(p) && !self.all_in.contains(p))
                .collect();
            round.apply_action(id, &resolved, &live_after);

            seat = (seat + 1) % n;
        }

        RoundOutcome::Continue
    }

    /// Seat index of the first player to act pre-flop (under the gun, or the
    /// button heads-up).
    fn first_to_act_preflop_seat(&self) -> usize {
        if self.seats.len() == 2 {
            self.dealer_seat_index
        } else {
            self.get_under_the_gun_seat_index()
        }
    }

    /// Seat index of the first player to act post-flop (small blind, or the
    /// non-button player heads-up).
    fn first_to_act_postflop_seat(&self) -> usize {
        if self.seats.len() == 2 {
            self.get_big_blind_seat_index()
        } else {
            self.get_small_blind_seat_index()
        }
    }

    /// Build the read-only snapshot handed to an agent on its turn.
    fn build_view(&self, id: Uuid, street: Street, round: &BettingRound) -> PlayerView {
        let hand = self
            .player_hands
            .get(&id)
            .expect("acting player has a hand");
        let hole = [hand.cards[0], hand.cards[1]];
        let chips = self.players.get(&id).map_or(0, |p| p.chips);
        let committed = round.committed(id);
        let legal = legal_actions(
            chips,
            committed,
            round.current_bet,
            round.last_raise_increment,
            round.may_raise(id),
        );

        PlayerView {
            you: id,
            name: self
                .players
                .get(&id)
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            street,
            hole,
            board: self.board.cards.clone(),
            chips,
            amount_owed: round.owed(id),
            current_bet: round.current_bet,
            min_raise_to: round.current_bet + round.last_raise_increment,
            pot_total: self.pot_total(),
            players_remaining: self.live_count(),
            legal_actions: legal,
            big_blind: self.big_blind_amount,
        }
    }

    /// Emit an [`ActionTaken`](GameEvent::ActionTaken) event for a resolved action.
    /// `current_bet` is the bet *before* this action, so a raise off a bet of zero
    /// is an opening bet.
    fn announce_action(&mut self, id: Uuid, resolved: &Resolved, current_bet: u32) {
        let Some(name) = self.players.get(&id).map(|p| p.name.clone()) else {
            return;
        };
        let all_in = resolved.all_in;
        let action = if resolved.folded {
            ActionView::Folded
        } else if let Some(to) = resolved.raised_to {
            if current_bet == 0 {
                ActionView::Bet { amount: to, all_in }
            } else {
                ActionView::Raised { to, all_in }
            }
        } else if resolved.paid == 0 {
            ActionView::Checked
        } else {
            ActionView::Called {
                amount: resolved.paid,
                all_in,
            }
        };
        self.observer.notify(&GameEvent::ActionTaken {
            player: name,
            action,
        });
    }

    /// Award the pot(s) at the end of a hand: refund any uncalled bet, build the
    /// main and side pots, and pay the best eligible hand(s).
    pub fn award_pots(&mut self) {
        if let Some((id, refund)) = refund_uncalled(&mut self.contributed, &self.folded) {
            if refund > 0 {
                if let Some(player) = self.players.get_mut(&id) {
                    player.add_chips(refund);
                }
                if let Some(player) = self.players.get(&id).map(|p| p.name.clone()) {
                    self.observer.notify(&GameEvent::UncalledBetReturned {
                        player,
                        amount: refund,
                    });
                }
            }
        }

        let live: Vec<Uuid> = self
            .seats
            .iter()
            .copied()
            .filter(|id| !self.folded.contains(id))
            .collect();
        let pots = build_pots(&self.contributed, &self.folded);
        let total: u32 = pots.iter().map(|p| p.amount).sum();

        // Uncontested: everyone else folded.
        if live.len() <= 1 {
            if let Some(&winner) = live.first() {
                if let Some(player) = self.players.get_mut(&winner) {
                    player.add_chips(total);
                }
                if let Some(player) = self.players.get(&winner).map(|p| p.name.clone()) {
                    self.observer.notify(&GameEvent::PotAwarded {
                        player,
                        amount: total,
                        hand: None,
                        pot: None,
                    });
                }
            }
            return;
        }

        let mut evaluated: HashMap<Uuid, ComparableHand> = HashMap::new();
        for &id in &live {
            if let Some(hand) = self.player_hands.get(&id) {
                evaluated.insert(
                    id,
                    evaluate(&[hand.cards[0], hand.cards[1]], &self.board.cards),
                );
            }
        }
        for &id in &live {
            if let (Some(name), Some(cards), Some(category)) = (
                self.players.get(&id).map(|p| p.name.clone()),
                self.player_hands.get(&id).map(|h| h.cards.clone()),
                evaluated.get(&id).map(|h| h.category),
            ) {
                self.observer.notify(&GameEvent::ShowdownReveal {
                    player: name,
                    cards,
                    hand: category,
                });
            }
        }

        let awards = distribute_pots(&pots, &evaluated, &self.seats, self.dealer_seat_index);
        // Only label pots when the hand actually split into side pots; a single
        // pot needs no "main pot" qualifier.
        let labelled = awards.len() > 1;
        for award in &awards {
            let kind = if award.index == 0 {
                PotKind::Main
            } else {
                PotKind::Side(award.index as u8)
            };
            let pot = labelled.then_some(kind);
            for &(id, amount) in &award.payouts {
                if amount == 0 {
                    continue;
                }
                if let Some(player) = self.players.get_mut(&id) {
                    player.add_chips(amount);
                }
                let category = evaluated.get(&id).map(|h| h.category);
                if let Some(player) = self.players.get(&id).map(|p| p.name.clone()) {
                    self.observer.notify(&GameEvent::PotAwarded {
                        player,
                        amount,
                        hand: category,
                        pot,
                    });
                }
            }
        }
    }

    /// Return every card from hands, board, and burn pile to the deck and clear
    /// the per-hand state, readying the engine for the next hand.
    pub fn end_hand(&mut self) {
        for (_, hand) in self.player_hands.drain() {
            for card in hand.cards {
                let _ = self.deck.insert_at_top(card);
            }
        }
        for card in self.board.cards.drain(..) {
            let _ = self.deck.insert_at_top(card);
        }
        for card in self.burned.cards.drain(..) {
            let _ = self.deck.insert_at_top(card);
        }
        self.contributed.clear();
        self.folded.clear();
        self.all_in.clear();
    }

    /// Number of cards currently in the deck (used in tests).
    #[cfg(test)]
    fn deck_len(&self) -> usize {
        self.deck.len()
    }
}

impl Default for TexasHoldEm {
    fn default() -> Self {
        Self::new(100, 10, 2, 5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use casino_cards::card::{Rank, Suit};

    fn card(rank: Rank, suit: Suit) -> Card {
        Card::new(rank, suit)
    }

    /// Seats `count` players with the given chip stacks and returns their ids in
    /// seat order.
    fn seat_players(game: &mut TexasHoldEm, chips: &[u32]) -> Vec<Uuid> {
        let mut ids = Vec::new();
        for (i, &c) in chips.iter().enumerate() {
            let player = Player::new_with_chips(&format!("P{i}"), c);
            let id = player.identifier;
            game.seats.push(id);
            game.players.insert(id, player);
            ids.push(id);
        }
        ids
    }

    #[test]
    fn award_three_way_all_in_with_side_pot() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        // Stacks already moved into the pot via contributions.
        let ids = seat_players(&mut game, &[0, 0, 40]); // A=0,B=0,C kept 40 back after refund
        let (a, b, c) = (ids[0], ids[1], ids[2]);
        game.contributed = HashMap::from([(a, 20), (b, 60), (c, 60)]);

        // Give each a hand on a shared board. C best, B middle, A worst.
        game.board = Hand::new_from_cards(vec![
            card(Rank::Two, Suit::Club),
            card(Rank::Seven, Suit::Diamond),
            card(Rank::Nine, Suit::Heart),
            card(Rank::Jack, Suit::Spade),
            card(Rank::King, Suit::Club),
        ]);
        game.player_hands.insert(
            a,
            Hand::new_from_cards(vec![
                card(Rank::Three, Suit::Club),
                card(Rank::Four, Suit::Diamond),
            ]),
        );
        game.player_hands.insert(
            b,
            Hand::new_from_cards(vec![
                card(Rank::Queen, Suit::Club),
                card(Rank::Queen, Suit::Diamond),
            ]),
        );
        game.player_hands.insert(
            c,
            Hand::new_from_cards(vec![
                card(Rank::Ace, Suit::Club),
                card(Rank::Ace, Suit::Diamond),
            ]),
        );

        game.award_pots();

        // C wins main (60) + side (80) = 140; started this assertion with 40 in stack.
        assert_eq!(game.players[&c].chips, 40 + 140);
        assert_eq!(game.players[&b].chips, 0);
        assert_eq!(game.players[&a].chips, 0);
    }

    #[test]
    fn award_short_stack_wins_main_other_takes_side() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[0, 0, 0]);
        let (a, b, c) = (ids[0], ids[1], ids[2]);
        game.contributed = HashMap::from([(a, 20), (b, 60), (c, 60)]);

        // A (short, eligible only for main) has the best hand.
        game.board = Hand::new_from_cards(vec![
            card(Rank::Two, Suit::Club),
            card(Rank::Seven, Suit::Diamond),
            card(Rank::Nine, Suit::Heart),
            card(Rank::Jack, Suit::Spade),
            card(Rank::King, Suit::Club),
        ]);
        // A has pair of aces; B pair of queens; C only a high card. (No board pair
        // makes trips for anyone.)
        game.player_hands.insert(
            a,
            Hand::new_from_cards(vec![
                card(Rank::Ace, Suit::Club),
                card(Rank::Ace, Suit::Diamond),
            ]),
        );
        game.player_hands.insert(
            b,
            Hand::new_from_cards(vec![
                card(Rank::Queen, Suit::Heart),
                card(Rank::Queen, Suit::Spade),
            ]),
        );
        game.player_hands.insert(
            c,
            Hand::new_from_cards(vec![
                card(Rank::Three, Suit::Club),
                card(Rank::Four, Suit::Diamond),
            ]),
        );

        game.award_pots();

        assert_eq!(game.players[&a].chips, 60); // main pot only
        assert_eq!(game.players[&b].chips, 80); // side pot
        assert_eq!(game.players[&c].chips, 0);
    }

    #[test]
    fn uncontested_pot_goes_to_lone_live_player() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[0, 0]);
        let (a, b) = (ids[0], ids[1]);
        game.contributed = HashMap::from([(a, 2), (b, 10)]);
        game.folded.insert(a);
        // B has no hand evaluated need; wins uncontested.
        game.player_hands.insert(
            b,
            Hand::new_from_cards(vec![
                card(Rank::Ace, Suit::Club),
                card(Rank::King, Suit::Diamond),
            ]),
        );

        game.award_pots();
        // Refund returns B's uncalled 8; remaining pot of 4 (2+2) goes to B.
        assert_eq!(game.players[&b].chips, 8 + 4);
    }

    /// An agent that checks when it can, otherwise calls, otherwise shoves, and
    /// only folds as a last resort — drives a hand to showdown.
    struct CallingAgent;
    impl PokerAgent for CallingAgent {
        fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError> {
            use crate::betting::LegalAction;
            if view
                .legal_actions
                .iter()
                .any(|a| matches!(a, LegalAction::Check))
            {
                Ok(PlayerAction::Check)
            } else if view
                .legal_actions
                .iter()
                .any(|a| matches!(a, LegalAction::Call(_)))
            {
                Ok(PlayerAction::Call)
            } else if view
                .legal_actions
                .iter()
                .any(|a| matches!(a, LegalAction::AllIn(_)))
            {
                Ok(PlayerAction::AllIn)
            } else {
                Ok(PlayerAction::Fold)
            }
        }
    }

    /// An agent that always commits all its chips when able.
    struct ShoveAgent;
    impl PokerAgent for ShoveAgent {
        fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError> {
            use crate::betting::LegalAction;
            if view
                .legal_actions
                .iter()
                .any(|a| matches!(a, LegalAction::AllIn(_)))
            {
                Ok(PlayerAction::AllIn)
            } else if view
                .legal_actions
                .iter()
                .any(|a| matches!(a, LegalAction::Call(_)))
            {
                Ok(PlayerAction::Call)
            } else if view
                .legal_actions
                .iter()
                .any(|a| matches!(a, LegalAction::Check))
            {
                Ok(PlayerAction::Check)
            } else {
                Ok(PlayerAction::Fold)
            }
        }
    }

    fn play_full_hand(game: &mut TexasHoldEm, mut make_agent: impl FnMut() -> Box<dyn PokerAgent>) {
        let mut agents: HashMap<Uuid, Box<dyn PokerAgent>> = HashMap::new();
        for &id in game.seats() {
            agents.insert(id, make_agent());
        }
        game.begin_hand();
        for street in [Street::Preflop, Street::Flop, Street::Turn, Street::River] {
            match street {
                Street::Preflop => {}
                Street::Flop => game.deal_flop(),
                Street::Turn => game.deal_turn(),
                Street::River => game.deal_river(),
            }
            if game.run_betting_round(street, &mut agents) == RoundOutcome::HandOver {
                break;
            }
        }
        game.award_pots();
        game.end_hand();
    }

    #[test]
    fn chips_are_conserved_calling_down() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 50, 75]);
        let before: u32 = game.players.values().map(|p| p.chips).sum();
        play_full_hand(&mut game, || Box::new(CallingAgent));
        let after: u32 = game.players.values().map(|p| p.chips).sum();
        assert_eq!(
            before, after,
            "chips must be conserved over a called-down hand"
        );
        assert_eq!(game.deck_len(), 52);
    }

    #[test]
    fn chips_are_conserved_with_all_ins_and_side_pots() {
        // Unequal stacks shoving pre-flop forces a main pot and side pots.
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 50, 75]);
        let before: u32 = game.players.values().map(|p| p.chips).sum();
        play_full_hand(&mut game, || Box::new(ShoveAgent));
        let after: u32 = game.players.values().map(|p| p.chips).sum();
        assert_eq!(
            before, after,
            "chips must be conserved through all-ins and side pots"
        );
        assert_eq!(game.deck_len(), 52);
        // The whole table was all-in for differing amounts, so someone holds it all.
        let max_stack = game.players.values().map(|p| p.chips).max().unwrap();
        assert!(max_stack >= 100, "a winner should have gathered chips");
    }

    /// A `GameObserver` that records every event for assertions. Uses a shared
    /// `Rc<RefCell<…>>` so the test can read the log back after the engine takes
    /// ownership of the observer via `set_observer`.
    struct RecordingObserver {
        log: std::rc::Rc<std::cell::RefCell<Vec<GameEvent>>>,
    }
    impl GameObserver for RecordingObserver {
        fn notify(&mut self, event: &GameEvent) {
            self.log.borrow_mut().push(event.clone());
        }
    }

    #[test]
    fn observer_receives_events_in_order() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 50, 75]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        play_full_hand(&mut game, || Box::new(ShoveAgent));

        let events = log.borrow();
        let position = |pred: fn(&GameEvent) -> bool| events.iter().position(pred);
        let last = |pred: fn(&GameEvent) -> bool| events.iter().rposition(pred);

        // The hand opens with HandStarted, then exactly two blinds.
        assert!(matches!(events[0], GameEvent::HandStarted { .. }));
        assert!(matches!(events[1], GameEvent::BlindPosted { .. }));
        assert!(matches!(events[2], GameEvent::BlindPosted { .. }));

        // At least one action, then board, then showdown, then payouts.
        let first_action = position(|e| matches!(e, GameEvent::ActionTaken { .. })).unwrap();
        let first_street = position(|e| matches!(e, GameEvent::StreetDealt { .. })).unwrap();
        let last_street = last(|e| matches!(e, GameEvent::StreetDealt { .. })).unwrap();
        let first_reveal = position(|e| matches!(e, GameEvent::ShowdownReveal { .. })).unwrap();
        let last_reveal = last(|e| matches!(e, GameEvent::ShowdownReveal { .. })).unwrap();
        let first_award = position(|e| matches!(e, GameEvent::PotAwarded { .. })).unwrap();

        assert!(first_action < first_street, "actions precede the board");
        assert!(
            last_street < first_reveal,
            "the board is complete before showdown"
        );
        assert!(
            last_reveal < first_award,
            "all hands are revealed before any payout"
        );
        assert!(
            matches!(events.last(), Some(GameEvent::PotAwarded { .. })),
            "the hand ends with a payout"
        );
    }

    #[test]
    fn side_pot_awards_are_labelled_main_and_side() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[0, 0, 0]);
        let (a, b, c) = (ids[0], ids[1], ids[2]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        game.contributed = HashMap::from([(a, 20), (b, 60), (c, 60)]);

        // Short stack A wins the main pot; B takes the side pot.
        game.board = Hand::new_from_cards(vec![
            card(Rank::Two, Suit::Club),
            card(Rank::Seven, Suit::Diamond),
            card(Rank::Nine, Suit::Heart),
            card(Rank::Jack, Suit::Spade),
            card(Rank::King, Suit::Club),
        ]);
        game.player_hands.insert(
            a,
            Hand::new_from_cards(vec![
                card(Rank::Ace, Suit::Club),
                card(Rank::Ace, Suit::Diamond),
            ]),
        );
        game.player_hands.insert(
            b,
            Hand::new_from_cards(vec![
                card(Rank::Queen, Suit::Heart),
                card(Rank::Queen, Suit::Spade),
            ]),
        );
        game.player_hands.insert(
            c,
            Hand::new_from_cards(vec![
                card(Rank::Three, Suit::Club),
                card(Rank::Four, Suit::Diamond),
            ]),
        );

        game.award_pots();

        let events = log.borrow();
        let awards: Vec<(u32, Option<crate::events::PotKind>)> = events
            .iter()
            .filter_map(|e| match e {
                GameEvent::PotAwarded { amount, pot, .. } => Some((*amount, *pot)),
                _ => None,
            })
            .collect();
        assert_eq!(
            awards,
            vec![
                (60, Some(crate::events::PotKind::Main)),
                (80, Some(crate::events::PotKind::Side(1))),
            ]
        );
    }

    #[test]
    fn single_pot_award_is_unlabelled() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[0, 0]);
        let (a, b) = (ids[0], ids[1]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        game.contributed = HashMap::from([(a, 50), (b, 50)]);
        game.board = Hand::new_from_cards(vec![
            card(Rank::Two, Suit::Club),
            card(Rank::Seven, Suit::Diamond),
            card(Rank::Nine, Suit::Heart),
            card(Rank::Jack, Suit::Spade),
            card(Rank::King, Suit::Club),
        ]);
        game.player_hands.insert(
            a,
            Hand::new_from_cards(vec![
                card(Rank::Ace, Suit::Club),
                card(Rank::Ace, Suit::Diamond),
            ]),
        );
        game.player_hands.insert(
            b,
            Hand::new_from_cards(vec![
                card(Rank::Three, Suit::Club),
                card(Rank::Four, Suit::Diamond),
            ]),
        );

        game.award_pots();

        let events = log.borrow();
        let award = events
            .iter()
            .find_map(|e| match e {
                GameEvent::PotAwarded { pot, .. } => Some(*pot),
                _ => None,
            })
            .expect("a pot was awarded");
        assert_eq!(award, None, "a single pot needs no main/side label");
    }

    #[test]
    fn deck_returns_to_full_after_a_hand_with_a_fold() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand();
        game.deal_flop();
        game.deal_turn();
        game.deal_river();
        // Simulate a fold burning a player's cards.
        let folder = game.seats[0];
        if let Some(hand) = game.player_hands.remove(&folder) {
            for c in hand.cards {
                game.burned.push(c);
            }
        }
        game.folded.insert(folder);
        game.end_hand();
        assert_eq!(game.deck_len(), 52, "all cards must return to the deck");
    }
}
