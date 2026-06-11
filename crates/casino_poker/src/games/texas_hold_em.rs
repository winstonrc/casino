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

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use casino_cards::card::Card;
use casino_cards::deck::Deck;
use casino_cards::hand::Hand;

use crate::agent::{AgentError, PlayerView, PokerAgent, Street};
use crate::betting::{legal_actions, resolve_action, BettingRound, PlayerAction, Resolved};
use crate::events::{ActionView, Blind, GameEvent, GameObserver, NullObserver, PotKind, SeatInfo};
use crate::hand_rankings::{evaluate, ComparableHand};
use crate::player::Player;
use crate::pot::{build_pots, distribute_pots, refund_uncalled, Pot};

/// The result of running a betting street.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RoundOutcome {
    /// Betting completed normally; continue to the next street or showdown.
    Continue,
    /// Only one player remains unfolded; skip remaining streets and award.
    HandOver,
    /// A player asked to quit the game.
    Quit,
}

/// One step of driving a betting street without blocking on the caller.
///
/// The engine yields this from [`TexasHoldEm::begin_betting_round`] and
/// [`TexasHoldEm::submit_action`]: it either pauses to ask a specific player to act
/// (handing back an owned, serializable [`PlayerView`]) or reports that the street
/// has finished. This is the resumable seam a non-blocking front-end (a network
/// server, an async UI) drives; the terminal's blocking
/// [`run_betting_round`](TexasHoldEm::run_betting_round) is a thin wrapper over it.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BettingStep {
    /// Paused: `player` must act. `view` is everything they need to decide.
    AwaitingAction { player: Uuid, view: PlayerView },
    /// The street is over; the outcome mirrors [`run_betting_round`]'s return.
    ///
    /// [`run_betting_round`]: TexasHoldEm::run_betting_round
    RoundComplete(RoundOutcome),
}

/// The in-progress state of a single betting street, retained on the engine so the
/// street can be paused (awaiting a player's action) and resumed across calls.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ActiveBettingRound {
    street: Street,
    round: BettingRound,
    /// Index into `seats` of the next seat to consider.
    seat: usize,
    /// The player whose action was requested while paused, if any.
    awaiting: Option<Uuid>,
}

/// A serializable, read-only snapshot of the **public** table state — the
/// safe-to-broadcast counterpart to the per-player [`PlayerView`]. It deliberately
/// omits every player's hole cards, so it can be sent to spectators or to all
/// seats at once without leaking hidden information. Produced by
/// [`TexasHoldEm::table`].
///
/// `#[non_exhaustive]`: the engine produces these, so new fields can be added in a
/// minor release without breaking downstream readers.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TableView {
    /// Each seat, in seat order.
    pub seats: Vec<SeatView>,
    /// Seat index of the dealer button, or `None` before the first hand.
    pub button_seat: Option<usize>,
    /// The street currently being bet, or `None` between streets/hands.
    pub street: Option<Street>,
    /// The amount to match on the current street (`0` when no round is active).
    pub current_bet: u32,
    /// The player whose action the engine is awaiting, if any.
    pub to_act: Option<Uuid>,
    /// The shared board cards (0, 3, 4, or 5).
    pub board: Vec<Card>,
    /// Total chips across all pots.
    pub pot_total: u32,
    /// The live side-pot structure built from current contributions.
    pub pots: Vec<Pot>,
}

/// One seat's public state within a [`TableView`]. Contains **no** hole cards.
///
/// `#[non_exhaustive]`: more per-seat fields can be added in a minor release.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SeatView {
    pub id: Uuid,
    pub name: String,
    pub chips: u32,
    /// Chips this player has committed on the current street.
    pub committed_this_street: u32,
    pub folded: bool,
    pub all_in: bool,
}

/// The default observer for a freshly-deserialized engine: a silent sink. A
/// restored engine produces no narration until [`TexasHoldEm::set_observer`] is
/// called to re-attach a real observer.
fn default_observer() -> Box<dyn GameObserver> {
    Box::new(NullObserver)
}

/// The default RNG for a freshly-deserialized engine: a fresh entropy seed. The
/// RNG is not serialized, so reproducibility is not preserved across a save/load
/// (the in-flight hand's deck order *is* serialized); call
/// [`TexasHoldEm::reseed`] after restore to re-establish determinism.
fn default_rng() -> StdRng {
    StdRng::from_entropy()
}

/// The core of the Texas hold 'em game.
///
/// The game currently defaults to no-limit.
#[derive(Serialize, Deserialize)]
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
    /// Hands dealt so far this session, used to number hand histories.
    hand_number: u32,
    /// The perspective player whose hole cards appear in the hand history's
    /// `Dealt to …` line. `None` runs with no perspective (e.g. all-bot tables).
    hero: Option<Uuid>,
    /// Receives the public narration of the hand. Defaults to a no-op sink. Not
    /// serialized (a trait object); re-attach with [`set_observer`] after restore.
    ///
    /// [`set_observer`]: TexasHoldEm::set_observer
    #[serde(skip, default = "default_observer")]
    observer: Box<dyn GameObserver>,
    /// The in-progress betting street, if one is paused or being driven. Holds the
    /// per-street state so a street can be advanced action-by-action; `None`
    /// between streets and hands. See [`TexasHoldEm::begin_betting_round`].
    betting: Option<ActiveBettingRound>,
    /// The RNG driving shuffles and seat randomization. Not serialized; restored
    /// engines re-seed from entropy (see [`default_rng`] and [`reseed`]).
    ///
    /// [`reseed`]: TexasHoldEm::reseed
    #[serde(skip, default = "default_rng")]
    rng: StdRng,
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
            hand_number: 0,
            hero: None,
            observer: Box::new(NullObserver),
            betting: None,
            rng: default_rng(),
        }
    }

    /// Like [`new`](Self::new), but with a deterministic RNG seeded from `seed`, so
    /// shuffles and seat randomization are reproducible. Useful for replays,
    /// provably-fair deals, and tests. Note the seed is **not** persisted by
    /// serialization — call [`reseed`](Self::reseed) after a restore to re-establish
    /// determinism.
    pub fn new_seeded(
        minimum_chips_buy_in_amount: u32,
        maximum_players_count: usize,
        small_blind_amount: u32,
        big_blind_amount: u32,
        seed: u64,
    ) -> Self {
        let mut game = Self::new(
            minimum_chips_buy_in_amount,
            maximum_players_count,
            small_blind_amount,
            big_blind_amount,
        );
        game.rng = StdRng::seed_from_u64(seed);
        game
    }

    /// Re-seed the engine's RNG, making subsequent shuffles/seat randomization
    /// reproducible from `seed`. Call after deserializing a game if you need
    /// deterministic future deals.
    pub fn reseed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    /// Designate the perspective player for hand histories (their hole cards show
    /// in the `Dealt to …` line). Typically the human at a terminal.
    pub fn set_hero(&mut self, hero: Uuid) {
        self.hero = Some(hero);
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

    /// Shuffle the seating order so the dealer button (and therefore the blinds)
    /// don't always start with the first player added. Call once after all players
    /// are seated and before the first hand.
    pub fn randomize_seats(&mut self) {
        self.seats.shuffle(&mut self.rng);
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

    /// Shuffle the game's deck using the engine's RNG.
    pub fn shuffle_deck(&mut self) {
        self.deck.shuffle_with(&mut self.rng);
    }

    /// The player who held the dealer button on the last hand, or `None` before the
    /// first hand. Note this id may belong to a player since removed (the button is
    /// only advanced by [`begin_hand`]); callers that need a *seated* player should
    /// check membership in [`seats`]. Used to snapshot a tournament for resumption.
    ///
    /// [`begin_hand`]: Self::begin_hand
    /// [`seats`]: Self::seats
    pub fn dealer(&self) -> Option<Uuid> {
        self.dealer
    }

    /// Place the dealer button on a specific seated player. Used to restore a
    /// resumed tournament to the button it left off on; the next [`begin_hand`]
    /// rotates on from here. No-op if the player is not seated.
    ///
    /// [`begin_hand`]: Self::begin_hand
    pub fn set_dealer(&mut self, player: Uuid) {
        if let Some(pos) = self.seats.iter().position(|s| *s == player) {
            self.dealer_seat_index = pos;
            self.dealer = Some(player);
        }
    }

    /// Set the blind amounts for subsequent hands (e.g. rising tournament levels).
    ///
    /// Apply **between hands**. Returns `false` (a no-op) while a hand is in
    /// progress, because a street's bet sizing is seeded from the current blinds and
    /// changing them mid-hand would corrupt it; returns `true` when applied.
    pub fn set_blinds(&mut self, small_blind: u32, big_blind: u32) -> bool {
        if self.hand_in_progress() {
            return false;
        }
        self.small_blind_amount = small_blind;
        self.big_blind_amount = big_blind;
        true
    }

    /// Set the minimum buy-in required to seat new players. Affects only future
    /// [`add_player`](Self::add_player) calls.
    pub fn set_min_buy_in(&mut self, amount: u32) {
        self.minimum_chips_buy_in_amount = amount;
    }

    /// Add chips to a seated player's stack — a rebuy or top-up — without
    /// re-seating them (which would churn their id).
    ///
    /// Apply **between hands**. Returns `false` (a no-op) while a hand is in
    /// progress (to avoid corrupting live pot/contribution accounting) or if the
    /// player is not seated; returns `true` when the chips are added.
    pub fn add_chips_to(&mut self, id: &Uuid, amount: u32) -> bool {
        if self.hand_in_progress() {
            return false;
        }
        match self.players.get_mut(id) {
            Some(player) => {
                player.add_chips(amount);
                true
            }
            None => false,
        }
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
    /// Returns `0` on an empty table (there is no seat to index).
    pub fn get_small_blind_seat_index(&self) -> usize {
        let len = self.seats.len();
        if len == 0 {
            return 0;
        }
        if len == 2 {
            self.dealer_seat_index
        } else {
            (self.dealer_seat_index + 1) % len
        }
    }

    /// Seat index of the big blind. Heads-up, the non-button player posts it.
    /// Returns `0` on an empty table (there is no seat to index).
    pub fn get_big_blind_seat_index(&self) -> usize {
        let len = self.seats.len();
        if len == 0 {
            return 0;
        }
        if len == 2 {
            (self.dealer_seat_index + 1) % len
        } else {
            (self.dealer_seat_index + 2) % len
        }
    }

    /// Seat index of the player to the left of the big blind (under the gun).
    /// Returns `0` on an empty table (there is no seat to index).
    pub fn get_under_the_gun_seat_index(&self) -> usize {
        let len = self.seats.len();
        if len == 0 {
            return 0;
        }
        (self.get_big_blind_seat_index() + 1) % len
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

    /// The player the engine is currently awaiting an action from, or `None` when
    /// no betting round is paused on a decision (between streets/hands, or while
    /// the engine is skipping folded/all-in seats).
    pub fn to_act(&self) -> Option<Uuid> {
        self.betting.as_ref().and_then(|b| b.awaiting)
    }

    /// The amount to match on the current street, or `0` when no round is active.
    pub fn current_bet(&self) -> u32 {
        self.betting.as_ref().map_or(0, |b| b.round.current_bet)
    }

    /// Chips a player has committed on the current street (`0` when no round is
    /// active or the player has committed nothing).
    pub fn committed_this_street(&self, id: &Uuid) -> u32 {
        self.betting.as_ref().map_or(0, |b| b.round.committed(*id))
    }

    /// Whether the player has folded this hand.
    pub fn has_folded(&self, id: &Uuid) -> bool {
        self.folded.contains(id)
    }

    /// Whether the player is all-in this hand.
    pub fn is_all_in(&self, id: &Uuid) -> bool {
        self.all_in.contains(id)
    }

    /// The seat index of the dealer button, or `None` before the first hand (or if
    /// the button player has since been removed).
    pub fn button_seat(&self) -> Option<usize> {
        self.dealer
            .and_then(|d| self.seats.iter().position(|s| *s == d))
    }

    /// The live side-pot structure built from the current contributions (main pot
    /// at index 0). Lets a spectator/server render pots mid-hand, before showdown.
    pub fn pots(&self) -> Vec<Pot> {
        build_pots(&self.contributed, &self.folded)
    }

    /// A serializable snapshot of the public table state (no hole cards), for
    /// spectator/lobby rendering or broadcasting to all seats. See [`TableView`].
    pub fn table(&self) -> TableView {
        let seats = self
            .seats
            .iter()
            .map(|id| {
                let player = self.players.get(id);
                SeatView {
                    id: *id,
                    name: player.map(|p| p.name.clone()).unwrap_or_default(),
                    chips: player.map_or(0, |p| p.chips),
                    committed_this_street: self.committed_this_street(id),
                    folded: self.folded.contains(id),
                    all_in: self.all_in.contains(id),
                }
            })
            .collect();

        TableView {
            seats,
            button_seat: self.button_seat(),
            street: self.betting.as_ref().map(|b| b.street),
            current_bet: self.current_bet(),
            to_act: self.to_act(),
            board: self.board.cards.clone(),
            pot_total: self.pot_total(),
            pots: self.pots(),
        }
    }

    /// The player currently on the clock and the [`PlayerView`] they need to act,
    /// or `None` when no decision is pending. Unlike [`begin_betting_round`] /
    /// [`submit_action`], this is **read-only** — it does not advance or reset the
    /// round.
    ///
    /// This is the reconnection seam: after deserializing a game paused mid-street,
    /// call it to re-derive the awaited player's prompt (to render their UI or feed
    /// their agent) before resuming with [`submit_action`].
    ///
    /// [`begin_betting_round`]: Self::begin_betting_round
    /// [`submit_action`]: Self::submit_action
    pub fn current_view(&self) -> Option<(Uuid, PlayerView)> {
        let active = self.betting.as_ref()?;
        let id = active.awaiting?;
        Some((id, self.build_view(id, active.street, &active.round)))
    }

    /// Whether a hand is currently in progress (cards dealt but not yet ended).
    /// Mutators that must only run between hands gate on this — note `betting` is
    /// `None` *between streets* even though the hand is live, so checking it alone
    /// is insufficient. Front-ends restoring a saved game use this to decide whether
    /// to resume an in-progress hand or begin a fresh one.
    pub fn hand_in_progress(&self) -> bool {
        self.betting.is_some() || !self.player_hands.is_empty()
    }

    /// The street whose betting round is currently in progress, or `None` when no
    /// round is active (between streets/hands). Used when resuming a saved hand to
    /// pick up on the correct street without re-dealing it.
    pub fn current_street(&self) -> Option<Street> {
        self.betting.as_ref().map(|b| b.street)
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
        self.start_hand();
    }

    /// Begin a hand from a caller-supplied, pre-ordered deck **without shuffling**,
    /// so an external harness can script exact hole/board cards (for tests,
    /// replays, or puzzle setups).
    ///
    /// Dealing pops from the **tail** of the deck: `deal_*` and the hole-card deal
    /// take the last card first. So lay the deck out in **reverse** deal order —
    /// the final cards in the `Vec` are dealt first, and burn cards (one before
    /// each of the flop/turn/river) must be included in that ordering.
    pub fn begin_hand_with_deck(&mut self, deck: Deck) {
        self.rotate_dealer();
        self.deck = deck;
        self.start_hand();
    }

    /// The shared body of [`begin_hand`]/[`begin_hand_with_deck`] that runs after
    /// the button is set and the deck is in place: number the hand, announce it,
    /// post the blinds, and deal hole cards.
    ///
    /// [`begin_hand`]: Self::begin_hand
    /// [`begin_hand_with_deck`]: Self::begin_hand_with_deck
    fn start_hand(&mut self) {
        self.hand_number += 1;
        // Emitted before blinds are posted, so the seat stacks are pre-blind.
        let seats: Vec<SeatInfo> = self
            .seats
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                self.players.get(id).map(|p| SeatInfo {
                    seat_no: i + 1,
                    name: p.name.clone(),
                    stack: p.chips,
                })
            })
            .collect();
        if !seats.is_empty() {
            self.observer.notify(&GameEvent::HandStarted {
                hand_number: self.hand_number,
                button_seat: self.dealer_seat_index + 1,
                small_blind: self.small_blind_amount,
                big_blind: self.big_blind_amount,
                seats,
            });
        }
        self.post_blind(true);
        self.post_blind(false);
        self.deal_hands_to_all_players();
        // The marker always fires; carry the hero's cards for the `Dealt to …` line.
        let hero = self.hero.and_then(|id| {
            let name = self.players.get(&id)?.name.clone();
            let hand = self.player_hands.get(&id)?;
            Some((name, hand.cards.clone()))
        });
        self.observer.notify(&GameEvent::HoleCardsDealt { hero });
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
    /// A thin, blocking wrapper over [`begin_betting_round`](Self::begin_betting_round)
    /// and [`submit_action`](Self::submit_action): it pumps the resumable state
    /// machine, sourcing each requested action from that player's agent. Returns
    /// [`RoundOutcome::HandOver`] if all but one player folds,
    /// [`RoundOutcome::Quit`] if a player quits, and [`RoundOutcome::Continue`]
    /// when betting completes normally.
    ///
    /// Two resumption-related behaviors:
    /// - If a betting round is **already in progress** on entry (e.g. one restored
    ///   from a saved game), it is *continued* rather than restarted — so resuming a
    ///   hand mid-street finishes the exact same street.
    /// - On [`RoundOutcome::Quit`] the in-progress round is **left intact** (not
    ///   aborted), so the caller can serialize the engine and resume the player on
    ///   the clock later. Use [`abort_betting_round`](Self::abort_betting_round)
    ///   explicitly to discard it instead.
    pub fn run_betting_round(
        &mut self,
        street: Street,
        agents: &mut HashMap<Uuid, Box<dyn PokerAgent>>,
    ) -> RoundOutcome {
        let mut step = match self.current_view() {
            // A round is already active (a resumed hand): continue from the player
            // on the clock instead of restarting the street.
            Some((player, view)) => BettingStep::AwaitingAction { player, view },
            None => self.begin_betting_round(street),
        };
        loop {
            match step {
                BettingStep::AwaitingAction { player, view } => {
                    let action = match agents.get_mut(&player).map(|agent| agent.decide(&view)) {
                        Some(Ok(action)) => action,
                        Some(Err(AgentError::Quit)) | Some(Err(AgentError::Eof)) => {
                            // Leave the round paused on this player so the caller can
                            // serialize and resume here; don't abort.
                            return RoundOutcome::Quit;
                        }
                        None => PlayerAction::Fold,
                    };
                    step = self.submit_action(action);
                }
                BettingStep::RoundComplete(outcome) => return outcome,
            }
        }
    }

    /// Begin a betting street, returning the first [`BettingStep`].
    ///
    /// The non-blocking entry point: it sets up the street and advances to the
    /// first player who must act (or straight to completion). Drive the street by
    /// feeding each requested action back through
    /// [`submit_action`](Self::submit_action); abandon it with
    /// [`abort_betting_round`](Self::abort_betting_round).
    ///
    /// Precondition: a hand has been dealt up to this street (`begin_hand` plus the
    /// street's `deal_*`), so every live seat holds hole cards.
    pub fn begin_betting_round(&mut self, street: Street) -> BettingStep {
        let n = self.seats.len();
        if n == 0 {
            return BettingStep::RoundComplete(RoundOutcome::HandOver);
        }

        let actors: Vec<Uuid> = self
            .seats
            .iter()
            .copied()
            .filter(|id| !self.folded.contains(id) && !self.all_in.contains(id))
            .collect();

        let (current_bet, committed_seed, bb_option, seat) = if street == Street::Preflop {
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

        let round = BettingRound::new(
            &actors,
            current_bet,
            self.big_blind_amount,
            committed_seed,
            bb_option,
        );

        // Unconditional assignment: a leftover round can never survive into a new one.
        self.betting = Some(ActiveBettingRound {
            street,
            round,
            seat,
            awaiting: None,
        });
        self.advance()
    }

    /// Feed the awaited player's action into the active betting street, returning
    /// the next [`BettingStep`].
    ///
    /// A no-op returning `RoundComplete(Continue)` when no action is actually
    /// pending (no active round, or not awaiting one), so stale or duplicate input
    /// — the first thing a network layer hits — cannot panic the engine.
    pub fn submit_action(&mut self, action: PlayerAction) -> BettingStep {
        // No active round: stale or duplicate input (the first thing a network
        // client can get wrong). Define it as a no-op rather than panicking.
        let Some(mut active) = self.betting.take() else {
            return BettingStep::RoundComplete(RoundOutcome::Continue);
        };
        // Active but not paused on an action: same defensive no-op, round preserved.
        let Some(id) = active.awaiting.take() else {
            self.betting = Some(active);
            return BettingStep::RoundComplete(RoundOutcome::Continue);
        };

        let chips = self.players.get(&id).map_or(0, |p| p.chips);
        let resolved = resolve_action(
            action,
            chips,
            active.round.committed(id),
            active.round.current_bet,
            active.round.last_raise_increment,
        )
        .unwrap_or_else(|_| {
            // An agent should only return legal actions; treat anything else as a
            // fold rather than panicking.
            resolve_action(
                PlayerAction::Fold,
                chips,
                active.round.committed(id),
                active.round.current_bet,
                active.round.last_raise_increment,
            )
            .expect("fold is always legal")
        });

        if let Some(player) = self.players.get_mut(&id) {
            player.subtract_chips(resolved.paid);
        }
        *self.contributed.entry(id).or_insert(0) += resolved.paid;
        self.announce_action(id, &resolved, active.round.current_bet, active.street);

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

        // Recompute the live set *after* the fold/all-in updates above so the
        // reopening logic in `apply_action` sees who can still act.
        let live_after: HashSet<Uuid> = self
            .seats
            .iter()
            .copied()
            .filter(|p| !self.folded.contains(p) && !self.all_in.contains(p))
            .collect();
        active.round.apply_action(id, &resolved, &live_after);

        active.seat = (active.seat + 1) % self.seats.len();
        self.betting = Some(active);
        self.advance()
    }

    /// Discard any in-progress betting street, leaving the engine reusable. Used
    /// when a hand is abandoned (e.g. a player quits) without awarding pots.
    pub fn abort_betting_round(&mut self) {
        self.betting = None;
    }

    /// Advance the active betting street to the next player who must act, or to
    /// completion. On completion the active round is cleared (`betting` is left
    /// `None`); on a pause it is stored back with the awaited player recorded.
    fn advance(&mut self) -> BettingStep {
        let n = self.seats.len();
        let mut active = self
            .betting
            .take()
            .expect("advance called without an active betting round");

        loop {
            // Order is parity-critical: a single action can both close the round and
            // drop the table to one live (unfolded) player. Checking `live_count`
            // first yields `HandOver` (which skips remaining streets), matching the
            // original loop; flipping the two checks would wrongly return `Continue`.
            if self.live_count() <= 1 {
                return BettingStep::RoundComplete(RoundOutcome::HandOver);
            }
            if active.round.is_closed() {
                return BettingStep::RoundComplete(RoundOutcome::Continue);
            }

            let id = self.seats[active.seat];
            if self.folded.contains(&id)
                || self.all_in.contains(&id)
                || !active.round.needs_to_act(id)
            {
                active.seat = (active.seat + 1) % n;
                continue;
            }

            let view = self.build_view(id, active.street, &active.round);
            active.awaiting = Some(id);
            self.betting = Some(active);
            return BettingStep::AwaitingAction { player: id, view };
        }
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
    /// is an opening bet and a raise's `by` is the increment over that prior bet.
    fn announce_action(&mut self, id: Uuid, resolved: &Resolved, current_bet: u32, street: Street) {
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
                ActionView::Raised {
                    by: to.saturating_sub(current_bet),
                    to,
                    all_in,
                }
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
            street,
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

        // Re-show the final board (it has scrolled past during betting) before any
        // hand is revealed.
        self.observer.notify(&GameEvent::Showdown {
            board: self.board.cards.clone(),
            pot: total,
        });

        // Evaluate each live hand once, recording its value for pot distribution
        // and emitting its showdown reveal.
        let mut evaluated: HashMap<Uuid, ComparableHand> = HashMap::new();
        for &id in &live {
            let Some(hand) = self.player_hands.get(&id) else {
                continue;
            };
            let comparable = evaluate(&[hand.cards[0], hand.cards[1]], &self.board.cards);
            evaluated.insert(id, comparable);
            if let Some(name) = self.players.get(&id).map(|p| p.name.clone()) {
                self.observer.notify(&GameEvent::ShowdownReveal {
                    player: name,
                    hole: hand.cards.clone(),
                    hand: comparable,
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
                let hand = evaluated.get(&id).copied();
                if let Some(player) = self.players.get(&id).map(|p| p.name.clone()) {
                    self.observer.notify(&GameEvent::PotAwarded {
                        player,
                        amount,
                        hand,
                        pot,
                    });
                }
            }
        }
    }

    /// Return every card from hands, board, and burn pile to the deck and clear
    /// the per-hand state, readying the engine for the next hand.
    ///
    /// Called after `award_pots` on every completed hand, so it is also where the
    /// [`HandComplete`](GameEvent::HandComplete) signal is emitted (covering both
    /// the contested and uncontested award paths). A hand abandoned via a player
    /// quit never reaches here, and so produces no summary — intentionally.
    pub fn end_hand(&mut self) {
        self.observer.notify(&GameEvent::HandComplete);
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
        // Defensively drop any in-progress street so an abandoned/partial round
        // can never leak into the next hand.
        self.betting = None;
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
    fn set_dealer_places_the_button_and_begin_hand_rotates_on() {
        // The button-restore seam used by tournament resume: placing the dealer on
        // the *just-completed* hand's button must rotate to the correct next seat,
        // not one too far (the off-by-one guard).
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[100, 100, 100]);

        game.set_dealer(ids[1]);
        assert_eq!(game.dealer(), Some(ids[1]));

        // An unseated id is ignored.
        game.set_dealer(Uuid::nil());
        assert_eq!(game.dealer(), Some(ids[1]));

        // begin_hand rotates from the restored button to the next seat.
        game.begin_hand();
        assert_eq!(game.dealer(), Some(ids[2]));
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
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[0, 0]);
        let (a, b) = (ids[0], ids[1]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
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
        // No one reached a showdown, so neither a header nor a reveal is emitted.
        let events = log.borrow();
        assert!(
            !events.iter().any(|e| matches!(
                e,
                GameEvent::Showdown { .. } | GameEvent::ShowdownReveal { .. }
            )),
            "an uncontested hand has no showdown"
        );
    }

    #[test]
    fn showdown_event_reports_the_final_board_and_post_refund_pot() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[0, 0]);
        let (a, b) = (ids[0], ids[1]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        // B overbet: 60 vs A's 40, so 20 is refunded as uncalled before the
        // showdown header, which should report the post-refund pot of 80.
        game.contributed = HashMap::from([(a, 40), (b, 60)]);
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
                card(Rank::Queen, Suit::Club),
                card(Rank::Queen, Suit::Diamond),
            ]),
        );

        game.award_pots();

        let events = log.borrow();
        let showdown = events
            .iter()
            .find_map(|e| match e {
                GameEvent::Showdown { board, pot } => Some((board.clone(), *pot)),
                _ => None,
            })
            .expect("a contested hand emits a showdown header");
        assert_eq!(showdown.0.len(), 5, "the full board is reported");
        assert_eq!(showdown.1, 80, "pot is the post-refund total (40 + 40)");
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

    /// An agent that always asks to quit — exercises the quit path.
    struct QuitAgent;
    impl PokerAgent for QuitAgent {
        fn decide(&mut self, _view: &PlayerView) -> Result<PlayerAction, AgentError> {
            Err(AgentError::Quit)
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

    // --- Resumable betting API (begin_betting_round / submit_action / advance) ---

    /// A call-down decision made purely from the legal actions in the view, so it
    /// is independent of the (randomly dealt) hole cards. Mirrors `CallingAgent`.
    fn calling_policy(view: &PlayerView) -> PlayerAction {
        use crate::betting::LegalAction;
        if view
            .legal_actions
            .iter()
            .any(|a| matches!(a, LegalAction::Check))
        {
            PlayerAction::Check
        } else if view
            .legal_actions
            .iter()
            .any(|a| matches!(a, LegalAction::Call(_)))
        {
            PlayerAction::Call
        } else if view
            .legal_actions
            .iter()
            .any(|a| matches!(a, LegalAction::AllIn(_)))
        {
            PlayerAction::AllIn
        } else {
            PlayerAction::Fold
        }
    }

    /// Drive one betting street to completion through the resumable API, choosing
    /// each action with `policy`. Returns the street's `RoundOutcome`.
    fn drive_street(
        game: &mut TexasHoldEm,
        street: Street,
        mut policy: impl FnMut(&PlayerView) -> PlayerAction,
    ) -> RoundOutcome {
        let mut step = game.begin_betting_round(street);
        loop {
            match step {
                BettingStep::AwaitingAction { view, .. } => {
                    let action = policy(&view);
                    step = game.submit_action(action);
                }
                BettingStep::RoundComplete(outcome) => return outcome,
            }
        }
    }

    /// Per-seat (chips, contributed-this-hand) snapshot in seat order, for parity
    /// comparisons that don't depend on each game's random player ids.
    fn seat_snapshot(game: &TexasHoldEm) -> Vec<(u32, u32)> {
        game.seats
            .iter()
            .map(|id| {
                (
                    game.players[id].chips,
                    game.contributed.get(id).copied().unwrap_or(0),
                )
            })
            .collect()
    }

    #[test]
    fn resumable_street_matches_run_betting_round() {
        // Same setup driven two ways: the blocking wrapper vs. begin/submit. The
        // call-down policy ignores hole cards, so the random deal can't diverge them.
        let stacks = [100u32, 50, 75];

        let mut wrapped = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut wrapped, &stacks);
        let mut agents: HashMap<Uuid, Box<dyn PokerAgent>> = HashMap::new();
        for &id in wrapped.seats() {
            agents.insert(id, Box::new(CallingAgent));
        }
        wrapped.begin_hand();
        let wrapped_outcome = wrapped.run_betting_round(Street::Preflop, &mut agents);

        let mut resumable = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut resumable, &stacks);
        resumable.begin_hand();
        let resumable_outcome = drive_street(&mut resumable, Street::Preflop, calling_policy);

        assert_eq!(wrapped_outcome, resumable_outcome);
        assert_eq!(seat_snapshot(&wrapped), seat_snapshot(&resumable));
        // The street settled, so no round is left in progress.
        assert!(resumable.betting.is_none());
    }

    #[test]
    fn action_that_closes_and_leaves_one_live_is_hand_over() {
        // Ordering guard: the final fold both closes betting and drops the table to
        // one unfolded player. `live_count <= 1` is checked before `is_closed`, so
        // this must be `HandOver` (not `Continue`).
        use crate::betting::LegalAction;
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand();

        let mut opened = false;
        let outcome = drive_street(&mut game, Street::Preflop, |view| {
            if !opened {
                opened = true;
                // First actor min-raises to open the betting.
                if let Some(LegalAction::RaiseTo { min, .. }) = view
                    .legal_actions
                    .iter()
                    .find(|a| matches!(a, LegalAction::RaiseTo { .. }))
                {
                    return PlayerAction::RaiseTo(*min);
                }
            }
            // Everyone else folds.
            PlayerAction::Fold
        });

        assert_eq!(outcome, RoundOutcome::HandOver);
        assert!(game.betting.is_none());
    }

    #[test]
    fn begin_betting_round_on_empty_table_is_hand_over() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let step = game.begin_betting_round(Street::Preflop);
        assert!(matches!(
            step,
            BettingStep::RoundComplete(RoundOutcome::HandOver)
        ));
        assert!(game.betting.is_none());
    }

    #[test]
    fn awaiting_action_yields_the_first_actor_and_serializes() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand();

        let step = game.begin_betting_round(Street::Preflop);
        let BettingStep::AwaitingAction { player, view } = &step else {
            panic!("expected to pause awaiting the first actor");
        };
        assert!(game.seats.contains(player));
        assert_eq!(view.you, *player);
        assert!(
            !view.legal_actions.is_empty(),
            "an acting player always has legal actions"
        );

        // Server-ready: the whole step round-trips through JSON.
        let json = serde_json::to_string(&step).expect("serialize");
        let back: BettingStep = serde_json::from_str(&json).expect("deserialize");
        let BettingStep::AwaitingAction {
            player: p2,
            view: v2,
        } = back
        else {
            panic!("expected AwaitingAction after round-trip");
        };
        assert_eq!(p2, *player);
        assert_eq!(v2.legal_actions, view.legal_actions);
    }

    #[test]
    fn abort_and_end_hand_clear_an_in_progress_street() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand();

        let _ = game.begin_betting_round(Street::Preflop);
        assert!(game.betting.is_some(), "paused mid-street");
        game.abort_betting_round();
        assert!(game.betting.is_none());

        // end_hand also defensively clears a partial street.
        game.begin_hand();
        let _ = game.begin_betting_round(Street::Preflop);
        assert!(game.betting.is_some());
        game.end_hand();
        assert!(game.betting.is_none());
    }

    #[test]
    fn submit_action_without_pending_is_a_noop() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        // No active round: defined as a no-op, no panic, state untouched.
        let step = game.submit_action(PlayerAction::Fold);
        assert!(matches!(
            step,
            BettingStep::RoundComplete(RoundOutcome::Continue)
        ));
        assert!(game.betting.is_none());
    }

    // --- Capability additions: serde, determinism, observability, robustness ---

    #[test]
    fn engine_round_trips_mid_hand_and_finishes_identically() {
        // Control: drive the whole preflop street, no serialization.
        let mut control = TexasHoldEm::new_seeded(0, 10, 1, 2, 7);
        seat_players(&mut control, &[100, 100, 100]);
        control.begin_hand();
        let control_outcome = drive_street(&mut control, Street::Preflop, calling_policy);
        let control_snap = seat_snapshot(&control);

        // Subject: same seeded setup; take one action so the paused state is
        // genuinely mid-street, then serde round-trip the *whole engine*.
        let mut subject = TexasHoldEm::new_seeded(0, 10, 1, 2, 7);
        seat_players(&mut subject, &[100, 100, 100]);
        subject.begin_hand();
        let mut step = subject.begin_betting_round(Street::Preflop);
        if let BettingStep::AwaitingAction { view, .. } = &step {
            let action = calling_policy(view);
            step = subject.submit_action(action);
        }
        assert!(
            matches!(step, BettingStep::AwaitingAction { .. }),
            "expected to be paused mid-street before serializing"
        );

        let json = serde_json::to_string(&subject).expect("serialize engine");
        let mut restored: TexasHoldEm = serde_json::from_str(&json).expect("deserialize engine");
        restored.set_observer(Box::new(crate::events::NullObserver));

        // Reconnection path: re-derive the awaited player's view from the RESTORED
        // engine (not the pre-serialization step) and finish the street.
        let mut outcome = None;
        while let Some((_, view)) = restored.current_view() {
            if let BettingStep::RoundComplete(o) = restored.submit_action(calling_policy(&view)) {
                outcome = Some(o);
            }
        }
        let outcome = outcome.expect("restored engine completed the street");

        assert_eq!(outcome, control_outcome);
        assert_eq!(
            seat_snapshot(&restored),
            control_snap,
            "a restored engine must finish to the same chips as one never serialized"
        );
    }

    #[test]
    fn seeded_games_deal_identically() {
        let deal = |seed: u64| {
            let mut g = TexasHoldEm::new_seeded(0, 10, 1, 2, seed);
            seat_players(&mut g, &[100, 100, 100]);
            g.begin_hand();
            g.deal_flop();
            g
        };
        let a = deal(99);
        let b = deal(99);

        assert_eq!(
            a.board().cards,
            b.board().cards,
            "same seed must deal the same board"
        );
        for i in 0..a.seats().len() {
            let ha = a.player_hand(&a.seats()[i]).map(|h| h.cards.clone());
            let hb = b.player_hand(&b.seats()[i]).map(|h| h.cards.clone());
            assert_eq!(
                ha, hb,
                "seat {i} hole cards must match across identical seeds"
            );
        }
    }

    #[test]
    fn begin_hand_with_deck_deals_scripted_cards() {
        // Cards deal from the deck's TAIL (the next `deal` pops the last element),
        // and `Deck::new` lays cards out suit-major ending Spades 2..Ace, so the
        // very top is As, then Ks, Qs, Js. Heads-up, the button (= small blind =
        // seat 0) is dealt first, two cards each in seat order. Assert exact
        // per-seat hole cards to prove the tail-pop / seat-order contract.
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[100, 100]);

        game.begin_hand_with_deck(Deck::new());

        assert_eq!(
            game.player_hand(&ids[0]).unwrap().cards,
            vec![card(Rank::Ace, Suit::Spade), card(Rank::King, Suit::Spade)],
        );
        assert_eq!(
            game.player_hand(&ids[1]).unwrap().cards,
            vec![
                card(Rank::Queen, Suit::Spade),
                card(Rank::Jack, Suit::Spade)
            ],
        );
    }

    #[test]
    fn table_view_exposes_public_state_without_hole_cards() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 5);
        seat_players(&mut game, &[100, 50, 75]);
        game.begin_hand();
        let _ = game.begin_betting_round(Street::Preflop);

        let view = game.table();
        assert_eq!(view.seats.len(), 3);
        assert!(
            view.to_act.is_some(),
            "a player should be on the clock pre-flop"
        );
        assert_eq!(view.current_bet, game.get_big_blind_amount());
        assert!(view.board.is_empty(), "no board pre-flop");

        // Blinds are reflected in per-seat committed amounts and the pot.
        let committed: u32 = view.seats.iter().map(|s| s.committed_this_street).sum();
        assert_eq!(
            committed,
            game.get_small_blind_amount() + game.get_big_blind_amount()
        );
        assert_eq!(view.pot_total, committed);

        // The snapshot serializes, and `SeatView` structurally carries no hole
        // cards — the only cards it can ever hold are the public board (empty here).
        let json = serde_json::to_string(&view).expect("serialize table view");
        assert!(json.contains("committed_this_street"));
    }

    #[test]
    fn seat_index_getters_do_not_panic_when_empty() {
        let game = TexasHoldEm::new(0, 10, 1, 2);
        assert_eq!(game.get_small_blind_seat_index(), 0);
        assert_eq!(game.get_big_blind_seat_index(), 0);
        assert_eq!(game.get_under_the_gun_seat_index(), 0);
    }

    #[test]
    fn set_blinds_and_add_chips_are_noops_during_a_hand() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[100, 100]);
        game.begin_hand();
        let _ = game.begin_betting_round(Street::Preflop);

        let chips_before = game.player(&ids[0]).map(|p| p.chips);
        assert!(
            !game.set_blinds(5, 10),
            "set_blinds must report no-op mid-hand"
        );
        assert!(
            !game.add_chips_to(&ids[0], 1000),
            "add_chips_to must report no-op mid-hand"
        );
        assert_eq!(game.get_big_blind_amount(), 2, "blinds unchanged mid-hand");
        assert_eq!(
            game.player(&ids[0]).map(|p| p.chips),
            chips_before,
            "chips unchanged mid-hand"
        );
    }

    #[test]
    fn current_view_re_derives_the_prompt_without_mutating() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 3);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand();
        let step = game.begin_betting_round(Street::Preflop);

        let BettingStep::AwaitingAction { player, view } = &step else {
            panic!("expected to pause awaiting an actor");
        };
        // current_view re-derives the same prompt, read-only (callable repeatedly).
        let (id1, v1) = game.current_view().expect("a decision is pending");
        let (id2, _) = game.current_view().expect("still pending (no mutation)");
        assert_eq!(id1, *player);
        assert_eq!(id2, *player);
        assert_eq!(v1.you, view.you);
        assert_eq!(v1.legal_actions, view.legal_actions);

        // Between hands there is no pending decision.
        let mut idle = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut idle, &[100, 100]);
        assert!(idle.current_view().is_none());
    }

    #[test]
    fn quit_preserves_the_round_and_run_betting_round_resumes_it() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 11);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand();

        // The player on the clock quits: the round is left paused on them (not
        // aborted), exactly the state a front-end would serialize to resume later.
        let mut quitters: HashMap<Uuid, Box<dyn PokerAgent>> = HashMap::new();
        for &id in game.seats() {
            quitters.insert(id, Box::new(QuitAgent));
        }
        assert_eq!(
            game.run_betting_round(Street::Preflop, &mut quitters),
            RoundOutcome::Quit
        );
        assert!(game.hand_in_progress(), "the hand must survive a quit");
        assert_eq!(game.current_street(), Some(Street::Preflop));
        assert!(
            game.current_view().is_some(),
            "a player is still on the clock after a quit"
        );

        // Calling run_betting_round again *continues* the same round to completion
        // rather than restarting the street.
        let mut callers: HashMap<Uuid, Box<dyn PokerAgent>> = HashMap::new();
        for &id in game.seats() {
            callers.insert(id, Box::new(CallingAgent));
        }
        let outcome = game.run_betting_round(Street::Preflop, &mut callers);
        assert!(matches!(
            outcome,
            RoundOutcome::Continue | RoundOutcome::HandOver
        ));
        assert!(
            game.current_view().is_none(),
            "the resumed round finished, so no one is on the clock"
        );
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
        let showdown = position(|e| matches!(e, GameEvent::Showdown { .. })).unwrap();
        let first_reveal = position(|e| matches!(e, GameEvent::ShowdownReveal { .. })).unwrap();
        let last_reveal = last(|e| matches!(e, GameEvent::ShowdownReveal { .. })).unwrap();
        let first_award = position(|e| matches!(e, GameEvent::PotAwarded { .. })).unwrap();

        assert!(first_action < first_street, "actions precede the board");
        assert!(
            last_street < showdown,
            "the board is complete before the showdown header"
        );
        assert!(
            showdown < first_reveal,
            "the showdown header re-shows the board before any reveal"
        );
        assert!(
            last_reveal < first_award,
            "all hands are revealed before any payout"
        );
        assert!(
            first_award < position(|e| matches!(e, GameEvent::HandComplete)).unwrap(),
            "payouts precede the hand-complete signal"
        );
        assert!(
            matches!(events.last(), Some(GameEvent::HandComplete)),
            "the hand ends with the completion signal"
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
    fn showdown_reveals_each_live_hand_with_its_value() {
        use crate::hand_rankings::HandCategory;

        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[0, 0]);
        let (a, b) = (ids[0], ids[1]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        game.contributed = HashMap::from([(a, 10), (b, 10)]);

        // Board pair of Queens; A plays the board (pair), B has pocket fives (two pair).
        game.board = Hand::new_from_cards(vec![
            card(Rank::Queen, Suit::Club),
            card(Rank::Jack, Suit::Spade),
            card(Rank::Four, Suit::Diamond),
            card(Rank::Queen, Suit::Spade),
            card(Rank::Ten, Suit::Club),
        ]);
        game.player_hands.insert(
            a,
            Hand::new_from_cards(vec![
                card(Rank::Three, Suit::Heart),
                card(Rank::Two, Suit::Diamond),
            ]),
        );
        game.player_hands.insert(
            b,
            Hand::new_from_cards(vec![
                card(Rank::Five, Suit::Spade),
                card(Rank::Five, Suit::Heart),
            ]),
        );

        game.award_pots();

        let name_a = game.players[&a].name.clone();
        let name_b = game.players[&b].name.clone();
        let events = log.borrow();
        let category = |name: &str| {
            events.iter().find_map(|e| match e {
                GameEvent::ShowdownReveal { player, hand, .. } if player == name => {
                    Some(hand.category)
                }
                _ => None,
            })
        };
        assert_eq!(category(&name_a), Some(HandCategory::Pair));
        assert_eq!(category(&name_b), Some(HandCategory::TwoPair));
    }

    #[test]
    fn three_layer_all_in_labels_main_and_two_side_pots() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[0, 0, 0, 0]);
        let (a, b, c, d) = (ids[0], ids[1], ids[2], ids[3]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        // Three contribution levels => main pot + two side pots. The top level is
        // shared by C and D so nothing is refunded as uncalled.
        game.contributed = HashMap::from([(a, 20), (b, 40), (c, 60), (d, 60)]);

        // No board rank coincides with the pocket pairs, so the pocket pairs rank
        // A > K > Q > J cleanly.
        game.board = Hand::new_from_cards(vec![
            card(Rank::Two, Suit::Club),
            card(Rank::Five, Suit::Diamond),
            card(Rank::Seven, Suit::Heart),
            card(Rank::Nine, Suit::Spade),
            card(Rank::Ten, Suit::Club),
        ]);
        // A best (aces, eligible main only); B next (kings, main + side 1);
        // C (queens) beats D (jacks) for side 2.
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
                card(Rank::King, Suit::Club),
                card(Rank::King, Suit::Diamond),
            ]),
        );
        game.player_hands.insert(
            c,
            Hand::new_from_cards(vec![
                card(Rank::Queen, Suit::Club),
                card(Rank::Queen, Suit::Diamond),
            ]),
        );
        game.player_hands.insert(
            d,
            Hand::new_from_cards(vec![
                card(Rank::Jack, Suit::Club),
                card(Rank::Jack, Suit::Diamond),
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
                (80, Some(crate::events::PotKind::Main)), // 20 * 4, won by A
                (60, Some(crate::events::PotKind::Side(1))), // 20 * 3, won by B
                (40, Some(crate::events::PotKind::Side(2))), // 20 * 2, won by C
            ]
        );
    }

    #[test]
    fn split_pot_award_is_unlabelled() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[0, 0]);
        let (a, b) = (ids[0], ids[1]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        game.contributed = HashMap::from([(a, 10), (b, 10)]);

        // Both players play the same board (a chop): one pot, two winners.
        game.board = Hand::new_from_cards(vec![
            card(Rank::Ace, Suit::Spade),
            card(Rank::Ace, Suit::Heart),
            card(Rank::King, Suit::Spade),
            card(Rank::King, Suit::Heart),
            card(Rank::Queen, Suit::Diamond),
        ]);
        game.player_hands.insert(
            a,
            Hand::new_from_cards(vec![
                card(Rank::Three, Suit::Club),
                card(Rank::Two, Suit::Club),
            ]),
        );
        game.player_hands.insert(
            b,
            Hand::new_from_cards(vec![
                card(Rank::Three, Suit::Diamond),
                card(Rank::Two, Suit::Diamond),
            ]),
        );

        game.award_pots();

        let events = log.borrow();
        let pots: Vec<Option<crate::events::PotKind>> = events
            .iter()
            .filter_map(|e| match e {
                GameEvent::PotAwarded { amount, pot, .. } => Some((*amount, *pot)),
                _ => None,
            })
            .map(|(_, pot)| pot)
            .collect();
        assert_eq!(pots, vec![None, None], "a single split pot needs no label");
        assert_eq!(game.players[&a].chips, 10);
        assert_eq!(game.players[&b].chips, 10);
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
