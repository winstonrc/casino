//! The Texas Hold'em game engine.
//!
//! The engine owns the full hand lifecycle and all money: per-player
//! contributions, the board, who has folded or gone all-in, betting, and pot
//! distribution. Callers (a terminal UI, tests, a future network layer) drive it
//! with a thin loop — deal, run each betting street, then award the pots — and
//! supply a [`PokerAgent`](crate::agent::PokerAgent) per player to decide actions.
//!
//! Side pots are computed at showdown from total contributions (see [`crate::pot`]).

use std::collections::{HashMap, HashSet};
use std::fmt;

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use casino_cards::card::Card;
use casino_cards::deck::Deck;
use casino_cards::hand::Hand;

use crate::agent::{AgentError, PlayerView, PokerAgent, Street};
use crate::betting::{
    legal_actions, resolve_action, ActionError, BettingRound, PlayerAction, Resolved,
};
use crate::events::{ActionView, Blind, GameEvent, GameObserver, NullObserver, PotKind, SeatInfo};
use crate::hand_rankings::{evaluate, ComparableHand};
use crate::player::Player;
use crate::pot::{build_pots, distribute_pots, refund_uncalled, Pot};

/// Maximum number of players supported by one Texas Hold'em table.
pub const MAX_PLAYERS: usize = 10;

/// Why a new hand could not be started.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum HandStartError {
    /// A previous hand has not ended or been aborted.
    HandAlreadyInProgress,
    /// Fewer than two players are seated.
    NotEnoughPlayers {
        /// Number of seated players.
        seated: usize,
    },
    /// More players are seated than the table supports.
    TooManyPlayers {
        /// Number of seated players.
        seated: usize,
        /// Maximum supported by this table.
        maximum: usize,
    },
    /// The supplied deck cannot finish the hand.
    InsufficientCards {
        /// Cards available.
        available: usize,
        /// Cards required.
        required: usize,
    },
    /// The supplied deck contains the same card more than once.
    DuplicateCards,
    /// Seats and player records are inconsistent.
    InvalidRoster,
    /// Restored in-progress hand state is internally inconsistent.
    InvalidHandState,
    /// Total table chips exceed [`u32::MAX`].
    ChipTotalTooLarge,
}

impl fmt::Display for HandStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HandAlreadyInProgress => f.write_str("a hand is already in progress"),
            Self::NotEnoughPlayers { seated } => {
                write!(f, "at least 2 players are required, found {seated}")
            }
            Self::TooManyPlayers { seated, maximum } => {
                write!(f, "at most {maximum} players are supported, found {seated}")
            }
            Self::InsufficientCards {
                available,
                required,
            } => write!(
                f,
                "the deck has {available} cards but {required} are required"
            ),
            Self::DuplicateCards => f.write_str("the deck contains duplicate cards"),
            Self::InvalidRoster => {
                f.write_str("the seat roster contains duplicate, missing, or unseated players")
            }
            Self::InvalidHandState => {
                f.write_str("the saved hand state is internally inconsistent")
            }
            Self::ChipTotalTooLarge => {
                f.write_str("total chips at the table exceed the supported u32 bankroll")
            }
        }
    }
}

impl std::error::Error for HandStartError {}

/// The result of running a betting street.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RoundOutcome {
    /// Betting completed normally; continue to the next street or showdown.
    Continue,
    /// Only one player remains unfolded; skip remaining streets and award.
    HandOver,
    /// A player asked to quit the game.
    Quit,
    /// The agent's input ended unexpectedly.
    Eof,
}

/// Stable identity for one pending decision.
///
/// It survives serialization and changes only when the engine advances to a new
/// player decision, allowing retrying or asynchronous clients to reject stale input.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct DecisionId(u64);

/// One identified player decision yielded by either state machine.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PendingAction {
    /// Stable identity for this decision.
    pub decision_id: DecisionId,
    /// Player expected to act.
    pub player: Uuid,
    /// Player-specific decision snapshot.
    pub view: PlayerView,
}

/// Why a submitted action was rejected.
///
/// Rejection is transactional: the engine is not mutated and the same
/// [`PendingAction`] remains current. Decision identity is checked before player
/// identity, so a submission with both values wrong reports `StaleDecision`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum ActionSubmissionError {
    /// The engine is not awaiting an action.
    NoActionPending,
    /// Restored game state failed validation.
    InvalidState(HandStartError),
    /// An action was submitted for a player other than the one on the clock.
    WrongPlayer {
        /// Player expected by the engine.
        expected: Uuid,
        /// Player supplied by the caller.
        submitted: Uuid,
    },
    /// The decision has already advanced or belongs to another prompt.
    StaleDecision {
        /// Current decision identity.
        expected: DecisionId,
        /// Identity supplied by the caller.
        submitted: DecisionId,
    },
    /// The poker action is not legal in the current betting state.
    IllegalAction(ActionError),
}

impl fmt::Display for ActionSubmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoActionPending => f.write_str("no action is pending"),
            Self::InvalidState(error) => write!(f, "invalid game state: {error}"),
            Self::WrongPlayer {
                expected,
                submitted,
            } => write!(f, "expected player {expected}, received {submitted}"),
            Self::StaleDecision {
                expected,
                submitted,
            } => write!(
                f,
                "expected decision {:?}, received {:?}",
                expected, submitted
            ),
            Self::IllegalAction(error) => write!(f, "illegal action: {error}"),
        }
    }
}

impl std::error::Error for ActionSubmissionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IllegalAction(error) => Some(error),
            Self::InvalidState(error) => Some(error),
            _ => None,
        }
    }
}

/// Failure from an agent-driven blocking wrapper.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum PlayError {
    /// No agent was registered for the player on the clock.
    MissingAgent(Uuid),
    /// The hand or betting round could not start.
    HandStart(HandStartError),
    /// An agent-selected action could not be submitted.
    Submission(ActionSubmissionError),
}

impl fmt::Display for PlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAgent(player) => write!(f, "no agent registered for player {player}"),
            Self::HandStart(error) => write!(f, "hand could not start: {error}"),
            Self::Submission(error) => write!(f, "action submission failed: {error}"),
        }
    }
}

impl std::error::Error for PlayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Submission(error) => Some(error),
            Self::HandStart(error) => Some(error),
            Self::MissingAgent(_) => None,
        }
    }
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
    /// Paused on one identified decision.
    AwaitingAction(PendingAction),
    /// The street is over; the outcome mirrors [`run_betting_round`]'s return.
    ///
    /// [`run_betting_round`]: TexasHoldEm::run_betting_round
    RoundComplete(RoundOutcome),
    /// The requested or restored street state is invalid; no action was applied.
    CannotStart(HandStartError),
}

/// One step of driving a whole *hand* without blocking — the hand-level sibling of
/// [`BettingStep`]. The engine owns the deal→bet→deal→award sequencing: it pauses
/// only to ask a player to act, and otherwise advances streets and awards the pot
/// itself.
///
/// Yielded by [`TexasHoldEm::drive_hand`] and [`TexasHoldEm::submit_hand_action`];
/// the blocking [`play_hand`](TexasHoldEm::play_hand) is a thin wrapper over them.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HandStep {
    /// Paused on one identified decision.
    AwaitingAction(PendingAction),
    /// The hand is fully played, awarded, and ended.
    HandComplete,
    /// A fresh hand could not start; the engine was not mutated.
    CannotStart(HandStartError),
}

/// The result of driving a whole hand with [`play_hand`](TexasHoldEm::play_hand):
/// completion or a resumable interruption, preserving whether the player chose to
/// quit or their input ended unexpectedly.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HandOutcome {
    /// The hand completed and all pots were awarded.
    Complete,
    /// An agent asked to quit, leaving the hand resumable.
    Quit,
    /// Agent input ended, leaving the hand resumable.
    Eof,
}

/// The in-progress state of a single betting street, retained on the engine so the
/// street can be paused (awaiting a player's action) and resumed across calls.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ActiveBettingRound {
    street: Street,
    round: BettingRound,
    /// Index into `seats` of the next seat to consider.
    seat: usize,
    /// The identified decision currently awaiting input, if any.
    awaiting: Option<(Uuid, DecisionId)>,
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
    /// Stable player identity.
    pub id: Uuid,
    /// Display name.
    pub name: String,
    /// Chips remaining in the stack.
    pub chips: u32,
    /// Chips this player has committed on the current street.
    pub committed_this_street: u32,
    /// Cumulative chips this seat has put into the pot across **all** streets of
    /// the current hand. Distinct from the per-street [`committed_this_street`]:
    /// this never resets mid-hand, accumulating every street's commitment until
    /// the hand ends, and so reflects the seat's total stake in the live pot.
    ///
    /// [`committed_this_street`]: SeatView::committed_this_street
    pub contributed_this_hand: u32,
    /// Whether the player has folded this hand.
    pub folded: bool,
    /// Whether the player has committed their entire stack.
    pub all_in: bool,
}

/// Everything **one** player's client needs to render and (re)join a game: the
/// public [`TableView`], that player's own private state, and the hand's narration
/// so far. Produced by [`TexasHoldEm::client_view`].
///
/// Leak-safe by construction: `hole` and `pending_action` carry only the requesting
/// player's cards, while `recent_events` redacts another configured hero's private
/// deal. Public showdown reveals remain visible.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClientView {
    /// Public table state (no hole cards).
    pub table: TableView,
    /// The player this view is for.
    pub you: Uuid,
    /// Your two hole cards, or `None` if you have folded/mucked or hold no hand.
    pub hole: Option<[Card; 2]>,
    /// Your pending decision, when it is your turn.
    pub pending_action: Option<PendingAction>,
    /// This hand's narration so far, in order, for catch-up rendering.
    pub recent_events: Vec<GameEvent>,
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
    /// Receives perspective-aware hand narration. Defaults to a no-op sink. When a
    /// hero is configured this stream contains that player's private deal; use
    /// [`public_events`](Self::public_events) for broadcast. Not serialized (a trait
    /// object); re-attach with [`set_observer`] after restore.
    ///
    /// [`set_observer`]: TexasHoldEm::set_observer
    #[serde(skip, default = "default_observer")]
    observer: Box<dyn GameObserver>,
    /// The in-progress betting street, if one is paused or being driven. Holds the
    /// per-street state so a street can be advanced action-by-action; `None`
    /// between streets and hands. See [`TexasHoldEm::begin_betting_round`].
    betting: Option<ActiveBettingRound>,
    /// Most recently completed street when its successor has not started yet.
    #[serde(default)]
    completed_betting_street: Option<Street>,
    /// Monotonic source for decision identities. Incremented only for a new prompt.
    #[serde(default)]
    next_decision_id: u64,
    /// The RNG driving shuffles and seat randomization. Not serialized; restored
    /// engines re-seed from entropy (see [`default_rng`] and [`reseed`]).
    ///
    /// [`reseed`]: TexasHoldEm::reseed
    #[serde(skip, default = "default_rng")]
    rng: StdRng,
    /// The current hand's narration, in emission order: every [`GameEvent`] sent to
    /// the observer is also recorded here. Cleared when a hand begins, so it always
    /// holds "this hand so far". Lets a (re)connecting front-end catch up — see
    /// [`replay_log`](TexasHoldEm::replay_log) and [`client_view`](TexasHoldEm::client_view).
    #[serde(default)]
    event_log: Vec<GameEvent>,
}

impl TexasHoldEm {
    /// Create a new game that internally contains a deck and players.
    ///
    /// # Panics
    ///
    /// Panics unless `maximum_players_count` is between 2 and [`MAX_PLAYERS`].
    pub fn new(
        minimum_chips_buy_in_amount: u32,
        maximum_players_count: usize,
        small_blind_amount: u32,
        big_blind_amount: u32,
    ) -> Self {
        assert!(
            (2..=MAX_PLAYERS).contains(&maximum_players_count),
            "maximum_players_count must be between 2 and {MAX_PLAYERS}"
        );
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
            completed_betting_street: None,
            next_decision_id: 0,
            rng: default_rng(),
            event_log: Vec::new(),
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

    /// Designate the private perspective used by newly emitted observer events and
    /// [`replay_log`](Self::replay_log). Replay retains recorded hole cards only
    /// when they belong to the current hero, so changing perspective cannot expose
    /// the previous hero's cards. Filtered public/client event access never exposes
    /// private cards to another player.
    pub fn set_hero(&mut self, hero: Uuid) {
        self.hero = Some(hero);
    }

    /// Set the observer that receives the hand's [`GameEvent`]s. Without one, the
    /// engine runs silently (the default [`NullObserver`]).
    pub fn set_observer(&mut self, observer: Box<dyn GameObserver>) {
        self.observer = observer;
    }

    /// Emit a game event: record it in the per-hand [`event_log`] **and** notify the
    /// observer. All gameplay narration goes through here so the log is a faithful,
    /// in-order copy of what the observer saw.
    fn emit(&mut self, event: GameEvent) {
        self.event_log.push(event.clone());
        self.observer.notify(&event);
    }

    /// Re-send the current hand's recorded events to the observer, in order. A
    /// front-end that attaches a fresh observer to a *restored* game calls this once
    /// (after [`set_observer`](Self::set_observer)) to replay the hand-so-far —
    /// header, blinds, every action, the board — exactly as it originally happened.
    /// A no-op when the log is empty. Does **not** re-record (so repeated calls don't
    /// grow the log).
    pub fn replay_log(&mut self) {
        // Clone so we can borrow the observer mutably while reading the log.
        let events = self.events_for(self.hero);
        for event in &events {
            self.observer.notify(event);
        }
    }

    /// Return an owned, public copy of this hand's events in emission order.
    ///
    /// The optional private hero payload is always redacted; publicly revealed
    /// showdown cards remain. This is the stream to broadcast or pass to agents.
    pub fn public_events(&self) -> Vec<GameEvent> {
        self.events_for(None)
    }

    fn events_for(&self, player_id: Option<Uuid>) -> Vec<GameEvent> {
        self.event_log
            .iter()
            .cloned()
            .map(|event| match event {
                GameEvent::HoleCardsDealt { hero } => {
                    let hero = hero.filter(|(player, _)| Some(player.id) == player_id);
                    GameEvent::HoleCardsDealt { hero }
                }
                event => event,
            })
            .collect()
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
    ///
    /// Players may only join between hands. The table's total bankroll is capped
    /// at [`u32::MAX`] so every possible single-player payout remains representable
    /// by [`Player::chips`].
    pub fn add_player(&mut self, player: Player) -> Result<(), &'static str> {
        if self.hand_in_progress() {
            return Err("Unable to join the table while a hand is in progress.");
        }
        if self.players.len() >= self.maximum_players_count {
            return Err("Unable to join the table. It is already at max capacity.");
        }

        if player.chips < self.minimum_chips_buy_in_amount {
            return Err("The player does not have enough chips to play at this table.");
        }
        if self.players.contains_key(&player.identifier) {
            return Err("A player with that identifier is already seated.");
        }
        if self
            .total_table_chips()
            .and_then(|total| total.checked_add(player.chips))
            .is_none()
        {
            return Err("The table cannot hold more than u32::MAX chips.");
        }

        self.seats.push(player.identifier);
        self.players.insert(player.identifier, player);
        Ok(())
    }

    /// Shuffle the seating order so the dealer button (and therefore the blinds)
    /// don't always start with the first player added. Call once after all players
    /// are seated and before the first hand.
    pub fn randomize_seats(&mut self) -> bool {
        if self.hand_in_progress() {
            return false;
        }
        self.seats.shuffle(&mut self.rng);
        self.sync_dealer_seat_index();
        true
    }

    /// Remove a player from the game. Returns `None` while a hand is in progress.
    pub fn remove_player(&mut self, player_identifier: &Uuid) -> Option<Player> {
        if self.hand_in_progress() {
            return None;
        }
        self.players.get(player_identifier)?;
        self.seats.retain(|x| x != player_identifier);
        let removed = self.players.remove(player_identifier);
        self.sync_dealer_seat_index();
        removed
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
    fn shuffle_deck(&mut self) {
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
    /// rotates on from here. Returns `false` during a hand or when the player is
    /// not seated.
    ///
    /// [`begin_hand`]: Self::begin_hand
    pub fn set_dealer(&mut self, player: Uuid) -> bool {
        if self.hand_in_progress() {
            return false;
        }
        if let Some(pos) = self.seats.iter().position(|s| *s == player) {
            self.dealer_seat_index = pos;
            self.dealer = Some(player);
            true
        } else {
            false
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
        if self
            .total_table_chips()
            .and_then(|total| total.checked_add(amount))
            .is_none()
        {
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
    /// The button is tracked by player id. Returns `false` during a hand or on an
    /// empty table. If the previous button player is still
    /// seated, it advances to the next seat; otherwise (first hand, or the button
    /// player busted) it falls to the first seat. This keeps `dealer_seat_index`
    /// always valid — a small simplification of formal dead-button rules.
    pub fn rotate_dealer(&mut self) -> bool {
        if self.hand_in_progress() || self.seats.is_empty() {
            return false;
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
        true
    }

    fn sync_dealer_seat_index(&mut self) {
        self.dealer_seat_index = self
            .dealer
            .and_then(|dealer| self.seats.iter().position(|id| *id == dealer))
            .unwrap_or(0);
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

    /// Configured small-blind amount.
    pub fn get_small_blind_amount(&self) -> u32 {
        self.small_blind_amount
    }

    /// Configured big-blind amount.
    pub fn get_big_blind_amount(&self) -> u32 {
        self.big_blind_amount
    }

    /// Total chips across all pots (main + sides) for the current hand.
    pub fn pot_total(&self) -> u32 {
        self.contributed
            .values()
            .try_fold(0u32, |total, amount| total.checked_add(*amount))
            .unwrap_or(u32::MAX)
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
        self.betting
            .as_ref()
            .and_then(|b| b.awaiting.map(|(player, _)| player))
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

    /// The public per-seat roster in seat order (no hole cards): identity, stack,
    /// this-street commitment, and fold/all-in status. Shared by [`table`](Self::table)
    /// and the decide-side [`PlayerView`] so an agent sees the same opponents the
    /// spectator view does.
    fn seat_views(&self) -> Vec<SeatView> {
        self.seats
            .iter()
            .map(|id| {
                let player = self.players.get(id);
                SeatView {
                    id: *id,
                    name: player.map(|p| p.name.clone()).unwrap_or_default(),
                    chips: player.map_or(0, |p| p.chips),
                    committed_this_street: self.committed_this_street(id),
                    contributed_this_hand: self.contributed.get(id).copied().unwrap_or(0),
                    folded: self.folded.contains(id),
                    all_in: self.all_in.contains(id),
                }
            })
            .collect()
    }

    /// A serializable snapshot of the public table state (no hole cards), for
    /// spectator/lobby rendering or broadcasting to all seats. See [`TableView`].
    pub fn table(&self) -> TableView {
        TableView {
            seats: self.seat_views(),
            button_seat: self.button_seat(),
            street: self.betting.as_ref().map(|b| b.street),
            current_bet: self.current_bet(),
            to_act: self.to_act(),
            board: self.board.cards.clone(),
            pot_total: self.pot_total(),
            pots: self.pots(),
        }
    }

    /// The identified decision currently on the clock, or `None` when no action is
    /// pending. This is read-only: repeated calls return the same [`DecisionId`] and
    /// do not advance or reset the round.
    ///
    /// This is the reconnection seam: after deserializing a game paused mid-street,
    /// call it to re-derive the awaited player's prompt (to render their UI or feed
    /// their agent) before resuming with [`submit_action`].
    ///
    /// [`begin_betting_round`]: Self::begin_betting_round
    /// [`submit_action`]: Self::submit_action
    pub fn pending_action(&self) -> Option<PendingAction> {
        self.validate_roster_and_bankroll().ok()?;
        let active = self.betting.as_ref()?;
        let (player, decision_id) = active.awaiting?;
        Some(PendingAction {
            decision_id,
            player,
            view: self.build_view(player, active.street, &active.round),
        })
    }

    /// Build the [`ClientView`] for one player — the public table, that player's own
    /// private state, and the hand's events so far — for (re)joining a game. See
    /// [`ClientView`] for the hidden-information guarantees.
    pub fn client_view(&self, player_id: Uuid) -> ClientView {
        let pending_action = self
            .pending_action()
            .filter(|pending| pending.player == player_id);
        let hole = self
            .player_hands
            .get(&player_id)
            .and_then(|hand| <&[Card; 2]>::try_from(hand.cards.as_slice()).ok())
            .copied();
        ClientView {
            table: self.table(),
            you: player_id,
            hole,
            pending_action,
            recent_events: self.events_for(Some(player_id)),
        }
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
    ///
    /// Validation is transactional: on error the engine is not mutated.
    pub fn begin_hand(&mut self) -> Result<(), HandStartError> {
        self.validate_hand_start(&self.deck)?;
        self.rotate_dealer();
        self.shuffle_deck();
        self.start_hand();
        Ok(())
    }

    /// Begin a hand from a caller-supplied, pre-ordered deck **without shuffling**,
    /// so an external harness can script exact hole/board cards (for tests,
    /// replays, or puzzle setups).
    ///
    /// Dealing pops from the **tail** of the deck: `deal_*` and the hole-card deal
    /// take the last card first. So lay the deck out in **reverse** deal order —
    /// the final cards in the `Vec` are dealt first, and burn cards (one before
    /// each of the flop/turn/river) must be included in that ordering.
    pub fn begin_hand_with_deck(&mut self, deck: Deck) -> Result<(), HandStartError> {
        self.validate_hand_start(&deck)?;
        self.rotate_dealer();
        self.deck = deck;
        self.start_hand();
        Ok(())
    }

    fn validate_hand_start(&self, deck: &Deck) -> Result<(), HandStartError> {
        if self.hand_in_progress() {
            return Err(HandStartError::HandAlreadyInProgress);
        }
        let seated = self.seats.len();
        if seated < 2 {
            return Err(HandStartError::NotEnoughPlayers { seated });
        }
        if seated > self.maximum_players_count || seated > MAX_PLAYERS {
            return Err(HandStartError::TooManyPlayers {
                seated,
                maximum: self.maximum_players_count.min(MAX_PLAYERS),
            });
        }
        self.validate_roster_and_bankroll()?;
        if !self.board.cards.is_empty()
            || !self.burned.cards.is_empty()
            || !self.contributed.is_empty()
            || !self.folded.is_empty()
            || !self.all_in.is_empty()
        {
            return Err(HandStartError::InvalidHandState);
        }

        // Two hole cards per player, five board cards, and three burn cards.
        let required = seated * 2 + 8;
        if deck.len() < required {
            return Err(HandStartError::InsufficientCards {
                available: deck.len(),
                required,
            });
        }
        let unique: HashSet<_> = deck.iter().map(|card| (card.rank, card.suit)).collect();
        if unique.len() != deck.len() {
            return Err(HandStartError::DuplicateCards);
        }
        Ok(())
    }

    fn total_table_chips(&self) -> Option<u32> {
        self.players
            .values()
            .try_fold(0u32, |total, player| total.checked_add(player.chips))
    }

    fn validate_roster_and_bankroll(&self) -> Result<(), HandStartError> {
        let seated: HashSet<Uuid> = self.seats.iter().copied().collect();
        if seated.len() != self.seats.len()
            || seated.len() != self.players.len()
            || seated.iter().any(|id| !self.players.contains_key(id))
        {
            return Err(HandStartError::InvalidRoster);
        }
        let bankroll = self
            .players
            .values()
            .try_fold(0u64, |total, player| {
                total.checked_add(u64::from(player.chips))
            })
            .and_then(|total| {
                self.contributed
                    .values()
                    .try_fold(total, |sum, amount| sum.checked_add(u64::from(*amount)))
            });
        if bankroll.is_none_or(|total| total > u64::from(u32::MAX)) {
            return Err(HandStartError::ChipTotalTooLarge);
        }
        if self.next_decision_id == u64::MAX {
            return Err(HandStartError::InvalidHandState);
        }
        let state_ids_are_seated = self
            .contributed
            .keys()
            .chain(self.folded.iter())
            .chain(self.all_in.iter())
            .chain(self.player_hands.keys())
            .all(|id| seated.contains(id));
        if !state_ids_are_seated {
            return Err(HandStartError::InvalidRoster);
        }
        if let Some(active) = &self.betting {
            let actionable: HashSet<Uuid> = seated
                .iter()
                .copied()
                .filter(|id| !self.folded.contains(id) && !self.all_in.contains(id))
                .collect();
            if active.seat >= self.seats.len()
                || active.awaiting.is_none()
                || active.awaiting.is_some_and(|(player, decision)| {
                    !actionable.contains(&player)
                        || !self.player_hands.contains_key(&player)
                        || !active.round.needs_to_act(player)
                        || decision.0 != self.next_decision_id
                })
                || !active.round.references_only(&seated)
                || !active.round.needs_action_only_from(&actionable)
                || !active.round.commitments_within(&self.contributed)
                || !self.street_matches_board(active.street)
            {
                return Err(HandStartError::InvalidHandState);
            }
        }
        if self.completed_betting_street.is_some_and(|street| {
            self.betting.is_some() || !self.completed_street_matches_board(street)
        }) {
            return Err(HandStartError::InvalidHandState);
        }
        if self.player_hands.values().any(|hand| hand.cards.len() != 2) {
            return Err(HandStartError::InvalidHandState);
        }
        if self.hand_in_progress() && !matches!(self.board.cards.len(), 0 | 3 | 4 | 5) {
            return Err(HandStartError::InvalidHandState);
        }
        if self.hand_in_progress()
            && self
                .seats
                .iter()
                .filter(|id| !self.folded.contains(id))
                .any(|id| !self.player_hands.contains_key(id))
        {
            return Err(HandStartError::InvalidHandState);
        }
        if self.hand_in_progress() && self.deck.len() < self.cards_needed_to_finish_hand() {
            return Err(HandStartError::InvalidHandState);
        }

        let mut seen = HashSet::new();
        let cards = self
            .deck
            .iter()
            .chain(
                self.player_hands
                    .values()
                    .flat_map(|hand| hand.cards.iter()),
            )
            .chain(self.board.cards.iter())
            .chain(self.burned.cards.iter());
        if cards
            .map(|card| (card.rank, card.suit))
            .any(|card| !seen.insert(card))
        {
            return Err(HandStartError::InvalidHandState);
        }
        Ok(())
    }

    fn cards_needed_to_finish_hand(&self) -> usize {
        match self.board.cards.len() {
            0 => 8,
            3 => 4,
            4 => 2,
            5 => 0,
            _ => usize::MAX,
        }
    }

    fn street_matches_board(&self, street: Street) -> bool {
        matches!(
            (street, self.board.cards.len()),
            (Street::Preflop, 0) | (Street::Flop, 3) | (Street::Turn, 4) | (Street::River, 5)
        )
    }

    fn completed_street_matches_board(&self, street: Street) -> bool {
        matches!(
            (street, self.board.cards.len()),
            (Street::Preflop, 0) | (Street::Flop, 3) | (Street::Turn, 4) | (Street::River, 5)
        )
    }

    /// The shared body of [`begin_hand`]/[`begin_hand_with_deck`] that runs after
    /// the button is set and the deck is in place: number the hand, announce it,
    /// post the blinds, and deal hole cards.
    ///
    /// [`begin_hand`]: Self::begin_hand
    /// [`begin_hand_with_deck`]: Self::begin_hand_with_deck
    fn start_hand(&mut self) {
        // Fresh hand: drop the previous hand's narration so the log holds only this
        // hand. (Resume goes through `drive_hand`, not here, so it keeps the log.)
        self.event_log.clear();
        self.completed_betting_street = None;
        self.hand_number += 1;
        // Emitted before blinds are posted, so the seat stacks are pre-blind.
        let seats: Vec<SeatInfo> = self
            .seats
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                self.players.get(id).map(|p| SeatInfo {
                    seat_no: i + 1,
                    player: p.to_ref(),
                    stack: p.chips,
                })
            })
            .collect();
        if !seats.is_empty() {
            self.emit(GameEvent::HandStarted {
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
            let player = self.players.get(&id)?.to_ref();
            let hand = self.player_hands.get(&id)?;
            Some((player, hand.cards.clone()))
        });
        self.emit(GameEvent::HoleCardsDealt { hero });
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
        let all_in = self
            .players
            .get(&id)
            .is_some_and(|player| player.chips == 0);
        if all_in {
            self.all_in.insert(id);
        }

        if let Some(player) = self.players.get(&id).map(|p| p.to_ref()) {
            let event = GameEvent::BlindPosted {
                player,
                blind: if is_small_blind {
                    Blind::Small
                } else {
                    Blind::Big
                },
                amount: posted,
                all_in,
            };
            self.emit(event);
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
    fn deal_flop(&mut self) {
        self.completed_betting_street = None;
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
    fn deal_turn(&mut self) {
        self.completed_betting_street = None;
        self.deal_single_board_card();
        self.emit_street_dealt(Street::Turn);
    }

    /// Burn a card, then deal the river card to the board.
    fn deal_river(&mut self) {
        self.completed_betting_street = None;
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
        self.emit(event);
    }

    /// Deal a hand of two cards.
    fn deal_hand(&mut self) -> Option<Hand> {
        let mut hand = Hand::new();
        hand.push(self.deal_card()?);
        hand.push(self.deal_card()?);
        Some(hand)
    }

    /// Deal a single card.
    fn deal_card(&mut self) -> Option<Card> {
        self.deck.deal_face_up()
    }

    /// Run a betting round for the given street, asking each player's agent to act.
    ///
    /// A thin, blocking wrapper over [`begin_betting_round`](Self::begin_betting_round)
    /// and [`submit_action`](Self::submit_action): it pumps the resumable state
    /// machine, sourcing each requested action from that player's agent. Returns
    /// [`RoundOutcome::HandOver`] if all but one player folds,
    /// [`RoundOutcome::Quit`] if a player quits, [`RoundOutcome::Eof`] if input
    /// ends unexpectedly, and [`RoundOutcome::Continue`] when betting completes.
    ///
    /// Two resumption-related behaviors:
    /// - If a betting round is **already in progress** on entry (e.g. one restored
    ///   from a saved game), it is *continued* rather than restarted — so resuming a
    ///   hand mid-street finishes the exact same street.
    /// - On [`RoundOutcome::Quit`] or [`RoundOutcome::Eof`] the in-progress round
    ///   is left intact, so the caller can serialize and resume the player on the
    ///   clock later. Use [`abort_betting_round`](Self::abort_betting_round)
    ///   explicitly to discard it.
    pub fn run_betting_round(
        &mut self,
        street: Street,
        agents: &mut HashMap<Uuid, Box<dyn PokerAgent>>,
    ) -> Result<RoundOutcome, PlayError> {
        let mut step = match self.pending_action() {
            // A round is already active (a resumed hand): continue from the player
            // on the clock instead of restarting the street.
            Some(pending) => BettingStep::AwaitingAction(pending),
            None => self.begin_betting_round(street),
        };
        loop {
            match step {
                BettingStep::AwaitingAction(pending) => {
                    let action = match agents
                        .get_mut(&pending.player)
                        .map(|agent| agent.decide(&pending.view))
                    {
                        Some(Ok(action)) => action,
                        Some(Err(AgentError::Quit)) => {
                            // Leave the round paused on this player so the caller can
                            // serialize and resume here; don't abort.
                            return Ok(RoundOutcome::Quit);
                        }
                        Some(Err(AgentError::Eof)) => return Ok(RoundOutcome::Eof),
                        None => return Err(PlayError::MissingAgent(pending.player)),
                    };
                    step = self
                        .submit_action(pending.player, pending.decision_id, action)
                        .map_err(PlayError::Submission)?;
                }
                BettingStep::RoundComplete(outcome) => return Ok(outcome),
                BettingStep::CannotStart(error) => return Err(PlayError::HandStart(error)),
            }
        }
    }

    /// Drive a whole hand, returning the first [`HandStep`]: resume the hand in
    /// progress if there is one, otherwise begin a fresh hand. The engine owns the
    /// deal→bet→deal→award sequencing; the caller only supplies actions via
    /// [`submit_hand_action`](Self::submit_hand_action).
    ///
    /// Every value this and `submit_hand_action` *return* leaves the engine at an
    /// unambiguous boundary — `AwaitingAction` (a betting round is active),
    /// `HandComplete` (the hand is over), or `CannotStart` (validation failed
    /// without mutation) — never mid-deal. So a front-end that saves on each
    /// returned step can resume exactly, with no street ever re-dealt.
    pub fn drive_hand(&mut self) -> HandStep {
        if let Err(error) = self.validate_roster_and_bankroll() {
            return HandStep::CannotStart(error);
        }
        if !self.hand_in_progress() {
            if let Err(error) = self.begin_hand() {
                return HandStep::CannotStart(error);
            }
            let step = self.begin_betting_round(Street::Preflop);
            return self.drive_hand_from(Street::Preflop, step);
        }
        // Resume an in-progress hand.
        if let Some(pending) = self.pending_action() {
            return HandStep::AwaitingAction(pending);
        }
        if let Some(street) = self.completed_betting_street {
            return self
                .drive_hand_from(street, BettingStep::RoundComplete(RoundOutcome::Continue));
        }
        // Defensive legacy-save path: a hand is in progress but no betting round
        // or completed-street marker is active. Re-open the street implied by the
        // board; `begin_betting_round` never deals.
        let street = self.street_for_board();
        let step = self.begin_betting_round(street);
        self.drive_hand_from(street, step)
    }

    /// Submit the player, decision identity, and action yielded by
    /// [`drive_hand`](Self::drive_hand), returning the next [`HandStep`].
    ///
    /// A stale ID, wrong player, or illegal poker action returns an error without
    /// changing any engine state. When a valid action closes a round, the engine
    /// advances through dealing or awards before yielding again.
    pub fn submit_hand_action(
        &mut self,
        player: Uuid,
        decision_id: DecisionId,
        action: PlayerAction,
    ) -> Result<HandStep, ActionSubmissionError> {
        // The street being bet, captured before `submit_action` closes it (which
        // nulls `betting`, so `current_street()` would then read `None`).
        let street = self
            .current_street()
            .ok_or(ActionSubmissionError::NoActionPending)?;
        let step = self.submit_action(player, decision_id, action)?;
        Ok(self.drive_hand_from(street, step))
    }

    /// Blocking convenience over [`Self::drive_hand`]/[`Self::submit_hand_action`]:
    /// plays the hand to completion, sourcing each action from that player's agent. Mirrors
    /// [`run_betting_round`](Self::run_betting_round) one level up. `Quit` and
    /// `Eof` leave the hand paused and remain distinct resumable outcomes.
    pub fn play_hand(
        &mut self,
        agents: &mut HashMap<Uuid, Box<dyn PokerAgent>>,
    ) -> Result<HandOutcome, PlayError> {
        let mut step = self.drive_hand();
        loop {
            match step {
                HandStep::AwaitingAction(pending) => {
                    let action = match agents
                        .get_mut(&pending.player)
                        .map(|agent| agent.decide(&pending.view))
                    {
                        Some(Ok(action)) => action,
                        Some(Err(AgentError::Quit)) => return Ok(HandOutcome::Quit),
                        Some(Err(AgentError::Eof)) => return Ok(HandOutcome::Eof),
                        None => return Err(PlayError::MissingAgent(pending.player)),
                    };
                    step = self
                        .submit_hand_action(pending.player, pending.decision_id, action)
                        .map_err(PlayError::Submission)?;
                }
                HandStep::HandComplete => return Ok(HandOutcome::Complete),
                HandStep::CannotStart(error) => return Err(PlayError::HandStart(error)),
            }
        }
    }

    /// Advance the hand from a betting-round step: pause on a player, or deal the
    /// next street / award the pot as rounds close, until the next decision or the
    /// hand ends. `street` is the street the incoming `step` belongs to (tracked by
    /// the caller because `advance` nulls `betting` on completion).
    fn drive_hand_from(&mut self, mut street: Street, mut step: BettingStep) -> HandStep {
        loop {
            match step {
                BettingStep::AwaitingAction(pending) => {
                    return HandStep::AwaitingAction(pending);
                }
                BettingStep::RoundComplete(RoundOutcome::HandOver) => {
                    self.award_pots();
                    self.end_hand();
                    return HandStep::HandComplete;
                }
                BettingStep::RoundComplete(RoundOutcome::Quit) => {
                    // Unreachable: this path never sources agent quits.
                    unreachable!("drive_hand_from does not source Quit")
                }
                BettingStep::RoundComplete(RoundOutcome::Eof) => {
                    unreachable!("drive_hand_from does not source Eof")
                }
                BettingStep::RoundComplete(RoundOutcome::Continue) => match street {
                    Street::Preflop => {
                        self.deal_flop();
                        street = Street::Flop;
                        step = self.begin_betting_round(street);
                    }
                    Street::Flop => {
                        self.deal_turn();
                        street = Street::Turn;
                        step = self.begin_betting_round(street);
                    }
                    Street::Turn => {
                        self.deal_river();
                        street = Street::River;
                        step = self.begin_betting_round(street);
                    }
                    Street::River => {
                        self.award_pots();
                        self.end_hand();
                        return HandStep::HandComplete;
                    }
                },
                BettingStep::CannotStart(error) => return HandStep::CannotStart(error),
            }
        }
    }

    /// The street whose betting an in-progress hand should (re)open, inferred from
    /// the community cards already dealt. Only used on the defensive resume path in
    /// [`drive_hand`] (a hand in progress with no active betting round).
    fn street_for_board(&self) -> Street {
        match self.board.cards.len() {
            0..=2 => Street::Preflop,
            3 => Street::Flop,
            4 => Street::Turn,
            _ => Street::River,
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
    /// street's `deal_*`), so every live seat holds hole cards. If a decision is
    /// already pending, this method returns it unchanged rather than resetting the
    /// active street.
    ///
    /// If a round is already awaiting action, this method is idempotent: it returns
    /// the existing [`PendingAction`] without resetting commitments or allocating a
    /// new [`DecisionId`].
    pub fn begin_betting_round(&mut self, street: Street) -> BettingStep {
        if self.seats.is_empty() {
            return BettingStep::RoundComplete(RoundOutcome::HandOver);
        }
        if let Err(error) = self.validate_roster_and_bankroll() {
            return BettingStep::CannotStart(error);
        }
        if let Some(pending) = self.pending_action() {
            return BettingStep::AwaitingAction(pending);
        }
        if self.completed_betting_street == Some(street) {
            return BettingStep::RoundComplete(RoundOutcome::Continue);
        }
        if !self.hand_in_progress() || !self.street_matches_board(street) {
            return BettingStep::CannotStart(HandStartError::InvalidHandState);
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
        self.completed_betting_street = None;
        self.advance()
    }

    /// Submit the player, decision identity, and action yielded by
    /// [`begin_betting_round`](Self::begin_betting_round) or
    /// [`pending_action`](Self::pending_action).
    ///
    /// Validation happens before mutation. No pending action, a stale ID, the wrong
    /// player, or an illegal poker action returns [`ActionSubmissionError`] and
    /// preserves the current decision exactly.
    pub fn submit_action(
        &mut self,
        player: Uuid,
        decision_id: DecisionId,
        action: PlayerAction,
    ) -> Result<BettingStep, ActionSubmissionError> {
        self.validate_roster_and_bankroll()
            .map_err(ActionSubmissionError::InvalidState)?;
        let active = self
            .betting
            .as_ref()
            .ok_or(ActionSubmissionError::NoActionPending)?;
        let (expected_player, expected_decision) = active
            .awaiting
            .ok_or(ActionSubmissionError::NoActionPending)?;
        if decision_id != expected_decision {
            return Err(ActionSubmissionError::StaleDecision {
                expected: expected_decision,
                submitted: decision_id,
            });
        }
        if player != expected_player {
            return Err(ActionSubmissionError::WrongPlayer {
                expected: expected_player,
                submitted: player,
            });
        }

        let id = player;
        let chips = self.players.get(&id).map_or(0, |p| p.chips);
        let previous_bet = active.round.current_bet;
        let resolved = resolve_action(
            action,
            chips,
            active.round.committed(id),
            active.round.current_bet,
            active.round.last_raise_increment,
        )
        .map_err(ActionSubmissionError::IllegalAction)?;
        if resolved.raised_to.is_some() && !active.round.may_raise(id) {
            return Err(ActionSubmissionError::IllegalAction(
                ActionError::RaiseNotAllowed,
            ));
        }

        let live_after: HashSet<Uuid> = self
            .seats
            .iter()
            .copied()
            .filter(|p| {
                !self.folded.contains(p)
                    && !self.all_in.contains(p)
                    && (*p != id || (!resolved.folded && !resolved.all_in))
            })
            .collect();
        let mut next_round = active.round.clone();
        next_round
            .apply_action(id, &resolved, &live_after)
            .map_err(ActionSubmissionError::IllegalAction)?;
        let contribution = self.contributed.get(&id).copied().unwrap_or(0);
        let next_contribution =
            contribution
                .checked_add(resolved.paid)
                .ok_or(ActionSubmissionError::IllegalAction(
                    ActionError::ChipAmountOverflow,
                ))?;

        let mut active = self
            .betting
            .take()
            .expect("validated active betting round disappeared");
        active.awaiting = None;
        active.round = next_round;

        if let Some(player_state) = self.players.get_mut(&id) {
            player_state.subtract_chips(resolved.paid);
        }
        self.contributed.insert(id, next_contribution);
        self.announce_action(id, &resolved, previous_bet, active.street);

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

        active.seat = (active.seat + 1) % self.seats.len();
        self.betting = Some(active);
        Ok(self.advance())
    }

    /// Abandon the current hand without awarding it, restoring every contribution
    /// to its player's stack and returning all dealt cards to the deck. No
    /// [`GameEvent::HandComplete`] event is emitted.
    ///
    /// This leaves the engine between hands and immediately reusable for table
    /// mutations or a fresh [`begin_hand`](Self::begin_hand). It is a no-op when no
    /// hand is in progress.
    pub fn abort_betting_round(&mut self) {
        if !self.hand_in_progress() {
            return;
        }
        for (id, amount) in self.contributed.drain() {
            if let Some(player) = self.players.get_mut(&id) {
                player.add_chips(amount);
            }
        }
        self.clear_hand_state();
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
            let actionable: Vec<Uuid> = self
                .seats
                .iter()
                .copied()
                .filter(|id| !self.folded.contains(id) && !self.all_in.contains(id))
                .collect();
            if actionable.is_empty()
                || (actionable.len() == 1 && active.round.owed(actionable[0]) == 0)
            {
                self.completed_betting_street = Some(active.street);
                return BettingStep::RoundComplete(RoundOutcome::Continue);
            }
            if active.round.is_closed() {
                self.completed_betting_street = Some(active.street);
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
            self.next_decision_id = self
                .next_decision_id
                .checked_add(1)
                .expect("decision id space exhausted");
            let decision_id = DecisionId(self.next_decision_id);
            active.awaiting = Some((id, decision_id));
            self.betting = Some(active);
            return BettingStep::AwaitingAction(PendingAction {
                decision_id,
                player: id,
                view,
            });
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
            min_raise_to: round.current_bet.saturating_add(round.last_raise_increment),
            pot_total: self.pot_total(),
            players_remaining: self.live_count(),
            legal_actions: legal,
            big_blind: self.big_blind_amount,
            seats: self.seat_views(),
            button_seat: self.button_seat(),
        }
    }

    /// Emit an [`ActionTaken`](GameEvent::ActionTaken) event for a resolved action.
    /// `current_bet` is the bet *before* this action, so a raise off a bet of zero
    /// is an opening bet and a raise's `by` is the increment over that prior bet.
    fn announce_action(&mut self, id: Uuid, resolved: &Resolved, current_bet: u32, street: Street) {
        let Some(player) = self.players.get(&id).map(|p| p.to_ref()) else {
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
        self.emit(GameEvent::ActionTaken {
            player,
            street,
            action,
        });
    }

    /// Award the pot(s) at the end of a hand: refund any uncalled bet, build the
    /// main and side pots, and pay the best eligible hand(s).
    fn award_pots(&mut self) {
        if let Some((id, refund)) = refund_uncalled(&mut self.contributed, &self.folded) {
            if refund > 0 {
                if let Some(player) = self.players.get_mut(&id) {
                    player.add_chips(refund);
                }
                if let Some(player) = self.players.get(&id).map(|p| p.to_ref()) {
                    self.emit(GameEvent::UncalledBetReturned {
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
        let total: u64 = pots.iter().map(|p| p.amount).sum();
        let total = u32::try_from(total).expect("validated table bankroll must fit in u32");

        // Uncontested: everyone else folded.
        if live.len() <= 1 {
            if let Some(&winner) = live.first() {
                if let Some(player) = self.players.get_mut(&winner) {
                    player.add_chips(total);
                }
                if let Some(player) = self.players.get(&winner).map(|p| p.to_ref()) {
                    self.emit(GameEvent::PotAwarded {
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
        self.emit(GameEvent::Showdown {
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
            // Folded players' hands are drained into `burned` at fold time (see the
            // fold handling around line 1180), so any `player_hands` entry still
            // reachable here holds exactly its two dealt hole cards — which is what
            // makes the two-card indexing below safe.
            debug_assert!(hand.cards.len() == 2);
            let comparable = evaluate(&[hand.cards[0], hand.cards[1]], &self.board.cards);
            evaluated.insert(id, comparable);
            if let Some(player) = self.players.get(&id).map(|p| p.to_ref()) {
                self.emit(GameEvent::ShowdownReveal {
                    player,
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
                let amount =
                    u32::try_from(amount).expect("validated table bankroll must fit in u32");
                if let Some(player) = self.players.get_mut(&id) {
                    player.add_chips(amount);
                }
                let hand = evaluated.get(&id).copied();
                if let Some(player) = self.players.get(&id).map(|p| p.to_ref()) {
                    self.emit(GameEvent::PotAwarded {
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
    fn end_hand(&mut self) {
        self.emit(GameEvent::HandComplete);
        self.clear_hand_state();
    }

    fn clear_hand_state(&mut self) {
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
        self.betting = None;
        self.completed_betting_street = None;
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
    fn public_table_management_helpers_cover_their_contracts() {
        let mut game = TexasHoldEm::new(10, 3, 1, 2);
        assert_eq!(game.get_small_blind_amount(), 1);
        assert_eq!(game.get_big_blind_amount(), 2);

        let too_short = game.new_player_with_chips("Short", 9);
        assert!(game.add_player(too_short).is_err());
        game.set_min_buy_in(0);

        let first = game.new_player("First");
        let first_id = first.identifier;
        game.add_player(first).unwrap();
        assert_eq!(game.player(&first_id).unwrap().chips, 0);
        assert!(game.add_chips_to(&first_id, 25));
        assert_eq!(game.player(&first_id).unwrap().chips, 25);
        assert!(!game.add_chips_to(&Uuid::new_v4(), 1));

        let broke = game.new_player("Broke");
        let broke_id = broke.identifier;
        game.add_player(broke).unwrap();
        assert_eq!(game.seats(), &[first_id, broke_id]);
        assert!(!game.check_for_game_over());
        assert_eq!(game.remove_losers(), vec!["Broke"]);
        assert_eq!(game.seats(), &[first_id]);
        assert!(game.player(&broke_id).is_none());

        assert!(game.check_for_game_over());
        game.end_game();
        assert!(game.check_for_game_over());
        assert_eq!(game.remove_player(&first_id).unwrap().name, "First");
        assert!(game.remove_player(&first_id).is_none());
    }

    #[test]
    fn reseed_matches_a_fresh_seeded_engine() {
        let mut seeded = TexasHoldEm::new_seeded(0, 10, 1, 2, 99);
        let mut reseeded = TexasHoldEm::new(0, 10, 1, 2);
        reseeded.reseed(99);
        seat_players(&mut seeded, &[100, 100]);
        seat_players(&mut reseeded, &[100, 100]);

        seeded.begin_hand().unwrap();
        reseeded.begin_hand().unwrap();

        let seeded_hands: Vec<Vec<Card>> = seeded
            .seats()
            .iter()
            .map(|id| seeded.player_hand(id).unwrap().cards.clone())
            .collect();
        let reseeded_hands: Vec<Vec<Card>> = reseeded
            .seats()
            .iter()
            .map(|id| reseeded.player_hand(id).unwrap().cards.clone())
            .collect();
        assert_eq!(seeded_hands, reseeded_hands);
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
        game.begin_hand().unwrap();
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

    struct EofAgent;
    impl PokerAgent for EofAgent {
        fn decide(&mut self, _view: &PlayerView) -> Result<PlayerAction, AgentError> {
            Err(AgentError::Eof)
        }
    }

    struct IllegalAgent;
    impl PokerAgent for IllegalAgent {
        fn decide(&mut self, _view: &PlayerView) -> Result<PlayerAction, AgentError> {
            Ok(PlayerAction::Check)
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
        game.begin_hand().unwrap();
        for street in [Street::Preflop, Street::Flop, Street::Turn, Street::River] {
            match street {
                Street::Preflop => {}
                Street::Flop => game.deal_flop(),
                Street::Turn => game.deal_turn(),
                Street::River => game.deal_river(),
            }
            if game.run_betting_round(street, &mut agents).unwrap() == RoundOutcome::HandOver {
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
                BettingStep::AwaitingAction(pending) => {
                    let action = policy(&pending.view);
                    step = game
                        .submit_action(pending.player, pending.decision_id, action)
                        .unwrap();
                }
                BettingStep::RoundComplete(outcome) => return outcome,
                BettingStep::CannotStart(error) => {
                    panic!("test attempted to drive an invalid street: {error}")
                }
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
        wrapped.begin_hand().unwrap();
        let wrapped_outcome = wrapped.run_betting_round(Street::Preflop, &mut agents);

        let mut resumable = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut resumable, &stacks);
        resumable.begin_hand().unwrap();
        let resumable_outcome = drive_street(&mut resumable, Street::Preflop, calling_policy);

        assert_eq!(wrapped_outcome, Ok(resumable_outcome));
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
        game.begin_hand().unwrap();

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
        game.begin_hand().unwrap();

        let step = game.begin_betting_round(Street::Preflop);
        let BettingStep::AwaitingAction(pending) = &step else {
            panic!("expected to pause awaiting the first actor");
        };
        assert!(game.seats.contains(&pending.player));
        assert_eq!(pending.view.you, pending.player);
        assert!(
            !pending.view.legal_actions.is_empty(),
            "an acting player always has legal actions"
        );

        // Server-ready: the whole step round-trips through JSON.
        let json = serde_json::to_string(&step).expect("serialize");
        let back: BettingStep = serde_json::from_str(&json).expect("deserialize");
        let BettingStep::AwaitingAction(back_pending) = back else {
            panic!("expected AwaitingAction after round-trip");
        };
        assert_eq!(back_pending.player, pending.player);
        assert_eq!(back_pending.decision_id, pending.decision_id);
        assert_eq!(back_pending.view.legal_actions, pending.view.legal_actions);
    }

    #[test]
    fn abort_refunds_and_clears_an_in_progress_hand() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();

        let _ = game.begin_betting_round(Street::Preflop);
        assert!(game.betting.is_some(), "paused mid-street");
        assert_eq!(game.pot_total(), 3);
        game.abort_betting_round();

        assert!(game.betting.is_none());
        assert!(!game.hand_in_progress());
        assert_eq!(game.pot_total(), 0);
        assert_eq!(game.deck_len(), 52);
        assert!(ids.iter().all(|id| game.players[id].chips == 100));
        assert!(
            !matches!(game.event_log.last(), Some(GameEvent::HandComplete)),
            "an abandoned hand is not narrated as completed"
        );
        assert!(game.randomize_seats());
        game.begin_hand().unwrap();
    }

    #[test]
    fn end_hand_clears_an_in_progress_street() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        let _ = game.begin_betting_round(Street::Preflop);
        game.end_hand();
        assert!(game.betting.is_none());
    }

    #[test]
    fn submit_action_without_pending_is_rejected() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        let result = game.submit_action(Uuid::nil(), DecisionId(1), PlayerAction::Fold);
        assert!(matches!(
            result,
            Err(ActionSubmissionError::NoActionPending)
        ));
        assert!(game.betting.is_none());
    }

    #[test]
    fn rejected_submission_preserves_state() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 17);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(pending) = game.begin_betting_round(Street::Preflop) else {
            panic!("expected a pending decision");
        };
        let before = serde_json::to_string(&game).unwrap();

        let wrong_player = *game
            .seats()
            .iter()
            .find(|&&id| id != pending.player)
            .unwrap();
        assert!(matches!(
            game.submit_action(wrong_player, pending.decision_id, PlayerAction::Fold),
            Err(ActionSubmissionError::WrongPlayer { .. })
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);

        assert!(matches!(
            game.submit_action(
                pending.player,
                DecisionId(pending.decision_id.0 + 1),
                PlayerAction::Fold
            ),
            Err(ActionSubmissionError::StaleDecision { .. })
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
        assert!(matches!(
            game.submit_action(
                wrong_player,
                DecisionId(pending.decision_id.0 + 1),
                PlayerAction::Fold
            ),
            Err(ActionSubmissionError::StaleDecision { .. })
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
        assert_eq!(
            game.pending_action().unwrap().decision_id,
            pending.decision_id
        );
    }

    #[test]
    fn exact_blind_stacks_are_all_in_and_never_prompted() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[1, 2]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        game.begin_hand().unwrap();

        assert!(ids.iter().all(|id| game.is_all_in(id)));
        assert!(ids.iter().all(|id| game.player(id).unwrap().chips == 0));
        assert!(
            log.borrow()
                .iter()
                .filter(|event| matches!(event, GameEvent::BlindPosted { all_in: true, .. }))
                .count()
                == 2
        );
        assert!(matches!(
            game.begin_betting_round(Street::Preflop),
            BettingStep::RoundComplete(RoundOutcome::Continue)
        ));
        assert!(game.pending_action().is_none());
    }

    #[test]
    fn zero_stack_blinds_are_all_in_and_never_prompted() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[0, 0]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        game.begin_hand().unwrap();

        assert!(ids.iter().all(|id| game.is_all_in(id)));
        assert_eq!(
            log.borrow()
                .iter()
                .filter(|event| matches!(
                    event,
                    GameEvent::BlindPosted {
                        amount: 0,
                        all_in: true,
                        ..
                    }
                ))
                .count(),
            2
        );
        assert!(matches!(
            game.begin_betting_round(Street::Preflop),
            BettingStep::RoundComplete(RoundOutcome::Continue)
        ));
    }

    #[test]
    fn short_big_blind_keeps_configured_bring_in() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[100, 1]);
        game.begin_hand().unwrap();
        assert!(game.is_all_in(&ids[1]));

        let BettingStep::AwaitingAction(pending) = game.begin_betting_round(Street::Preflop) else {
            panic!("the button should still act");
        };
        assert_eq!(pending.view.current_bet, 2);
        assert_eq!(pending.view.amount_owed, 1);
    }

    #[test]
    fn multiway_short_big_blind_preserves_action_order_and_bring_in() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[100, 100, 1]);
        game.begin_hand().unwrap();
        assert!(game.is_all_in(&ids[2]));

        let BettingStep::AwaitingAction(under_the_gun) = game.begin_betting_round(Street::Preflop)
        else {
            panic!("expected under-the-gun action");
        };
        assert_eq!(under_the_gun.player, ids[0]);
        assert_eq!(under_the_gun.view.current_bet, 2);
        assert_eq!(under_the_gun.view.amount_owed, 2);

        let BettingStep::AwaitingAction(small_blind) = game
            .submit_action(
                under_the_gun.player,
                under_the_gun.decision_id,
                PlayerAction::Call,
            )
            .unwrap()
        else {
            panic!("expected small-blind action");
        };
        assert_eq!(small_blind.player, ids[1]);
        assert_eq!(small_blind.view.amount_owed, 1);
    }

    fn reach_incomplete_raise(game: &mut TexasHoldEm) -> (PendingAction, PendingAction) {
        game.begin_hand().unwrap();
        drive_street(game, Street::Preflop, calling_policy);
        game.deal_flop();

        let BettingStep::AwaitingAction(opener) = game.begin_betting_round(Street::Flop) else {
            panic!("expected opener");
        };
        let step = game
            .submit_action(opener.player, opener.decision_id, PlayerAction::RaiseTo(10))
            .unwrap();
        let BettingStep::AwaitingAction(caller) = step else {
            panic!("expected caller");
        };
        let step = game
            .submit_action(caller.player, caller.decision_id, PlayerAction::Call)
            .unwrap();
        let BettingStep::AwaitingAction(shover) = step else {
            panic!("expected shover");
        };
        let step = game
            .submit_action(shover.player, shover.decision_id, PlayerAction::AllIn)
            .unwrap();
        let BettingStep::AwaitingAction(reopened) = step else {
            panic!("prior actors must respond to the incomplete raise");
        };
        (reopened, shover)
    }

    #[test]
    fn incomplete_raise_rejects_reraise_without_mutation() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[17, 100, 100]);
        let (pending, _) = reach_incomplete_raise(&mut game);
        assert!(!pending
            .view
            .legal_actions
            .iter()
            .any(|action| matches!(action, crate::betting::LegalAction::RaiseTo { .. })));
        let before = serde_json::to_string(&game).unwrap();

        assert_eq!(
            game.submit_action(
                pending.player,
                pending.decision_id,
                PlayerAction::RaiseTo(30),
            )
            .unwrap_err(),
            ActionSubmissionError::IllegalAction(ActionError::RaiseNotAllowed)
        );
        assert_eq!(serde_json::to_string(&game).unwrap(), before);

        assert_eq!(
            game.submit_action(pending.player, pending.decision_id, PlayerAction::AllIn)
                .unwrap_err(),
            ActionSubmissionError::IllegalAction(ActionError::RaiseNotAllowed)
        );
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn repeated_begin_returns_the_same_pending_decision_without_resetting() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(first) = game.begin_betting_round(Street::Preflop) else {
            panic!("expected a decision");
        };
        let before = serde_json::to_string(&game).unwrap();
        let BettingStep::AwaitingAction(repeated) = game.begin_betting_round(Street::Preflop)
        else {
            panic!("expected the existing decision");
        };

        assert_eq!(repeated.decision_id, first.decision_id);
        assert_eq!(repeated.player, first.player);
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn repeated_begin_after_completion_does_not_reopen_the_street() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 33);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();
        assert_eq!(
            drive_street(&mut game, Street::Preflop, calling_policy),
            RoundOutcome::Continue
        );
        let before = serde_json::to_string(&game).unwrap();

        assert!(matches!(
            game.begin_betting_round(Street::Preflop),
            BettingStep::RoundComplete(RoundOutcome::Continue)
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn lone_actionable_player_is_not_prompted_when_nothing_is_owed() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 1, 2]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(pending) = game.begin_betting_round(Street::Preflop) else {
            panic!("the remaining player must respond to the blinds");
        };
        let step = game
            .submit_action(pending.player, pending.decision_id, PlayerAction::Call)
            .unwrap();
        assert!(matches!(
            step,
            BettingStep::RoundComplete(RoundOutcome::Continue)
        ));

        game.deal_flop();
        assert!(matches!(
            game.begin_betting_round(Street::Flop),
            BettingStep::RoundComplete(RoundOutcome::Continue)
        ));
    }

    #[test]
    fn completed_street_survives_serde_and_advances_instead_of_reopening() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 31);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        assert_eq!(
            drive_street(&mut game, Street::Preflop, calling_policy),
            RoundOutcome::Continue
        );
        assert_eq!(game.board().cards.len(), 0);

        let json = serde_json::to_string(&game).unwrap();
        let mut restored: TexasHoldEm = serde_json::from_str(&json).unwrap();
        let step = restored.drive_hand();

        assert_eq!(restored.board().cards.len(), 3);
        assert!(matches!(step, HandStep::AwaitingAction(_)));
        assert_eq!(restored.current_street(), Some(Street::Flop));
    }

    #[test]
    fn incomplete_raise_allows_short_calling_all_in() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[17, 17, 100]);
        let (pending, _) = reach_incomplete_raise(&mut game);

        let result = game.submit_action(pending.player, pending.decision_id, PlayerAction::AllIn);
        assert!(result.is_ok());
        assert!(game.is_all_in(&pending.player));
        assert_eq!(game.committed_this_street(&pending.player), 15);
        let stacks: u32 = game.players.values().map(|player| player.chips).sum();
        assert_eq!(stacks + game.pot_total(), 134);
    }

    #[test]
    fn cumulative_short_all_ins_reopen_raising_through_engine() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let total = 239;
        let ids = seat_players(&mut game, &[100, 100, 17, 22]);
        game.begin_hand().unwrap();
        drive_street(&mut game, Street::Preflop, calling_policy);
        game.deal_flop();

        let BettingStep::AwaitingAction(opener) = game.begin_betting_round(Street::Flop) else {
            panic!("expected opener");
        };
        assert_eq!(opener.player, ids[1]);
        let BettingStep::AwaitingAction(first_shover) = game
            .submit_action(opener.player, opener.decision_id, PlayerAction::RaiseTo(10))
            .unwrap()
        else {
            panic!("expected first short stack");
        };
        let BettingStep::AwaitingAction(second_shover) = game
            .submit_action(
                first_shover.player,
                first_shover.decision_id,
                PlayerAction::AllIn,
            )
            .unwrap()
        else {
            panic!("expected second short stack");
        };
        let BettingStep::AwaitingAction(not_yet_acted) = game
            .submit_action(
                second_shover.player,
                second_shover.decision_id,
                PlayerAction::AllIn,
            )
            .unwrap()
        else {
            panic!("expected the player who has not acted");
        };
        let BettingStep::AwaitingAction(reopened) = game
            .submit_action(
                not_yet_acted.player,
                not_yet_acted.decision_id,
                PlayerAction::Call,
            )
            .unwrap()
        else {
            panic!("expected action to return to the opener");
        };

        assert_eq!(reopened.player, opener.player);
        assert!(reopened
            .view
            .legal_actions
            .iter()
            .any(|action| matches!(action, crate::betting::LegalAction::RaiseTo { .. })));
        let stacks: u32 = game.players.values().map(|player| player.chips).sum();
        assert_eq!(stacks + game.pot_total(), total);
    }

    // --- Capability additions: serde, determinism, observability, robustness ---

    #[test]
    fn engine_round_trips_mid_hand_and_finishes_identically() {
        // Control: drive the whole preflop street, no serialization.
        let mut control = TexasHoldEm::new_seeded(0, 10, 1, 2, 7);
        seat_players(&mut control, &[100, 100, 100]);
        control.begin_hand().unwrap();
        let control_outcome = drive_street(&mut control, Street::Preflop, calling_policy);
        let control_snap = seat_snapshot(&control);

        // Subject: same seeded setup; take one action so the paused state is
        // genuinely mid-street, then serde round-trip the *whole engine*.
        let mut subject = TexasHoldEm::new_seeded(0, 10, 1, 2, 7);
        seat_players(&mut subject, &[100, 100, 100]);
        subject.begin_hand().unwrap();
        let mut step = subject.begin_betting_round(Street::Preflop);
        if let BettingStep::AwaitingAction(pending) = &step {
            let action = calling_policy(&pending.view);
            step = subject
                .submit_action(pending.player, pending.decision_id, action)
                .unwrap();
        }
        assert!(
            matches!(step, BettingStep::AwaitingAction(_)),
            "expected to be paused mid-street before serializing"
        );

        let json = serde_json::to_string(&subject).expect("serialize engine");
        let mut restored: TexasHoldEm = serde_json::from_str(&json).expect("deserialize engine");
        restored.set_observer(Box::new(crate::events::NullObserver));

        // Reconnection path: re-derive the awaited player's view from the RESTORED
        // engine (not the pre-serialization step) and finish the street.
        let mut outcome = None;
        while let Some(pending) = restored.pending_action() {
            if let BettingStep::RoundComplete(o) = restored
                .submit_action(
                    pending.player,
                    pending.decision_id,
                    calling_policy(&pending.view),
                )
                .unwrap()
            {
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
            g.begin_hand().unwrap();
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

        game.begin_hand_with_deck(Deck::new()).unwrap();

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
        game.begin_hand().unwrap();
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
    fn seat_view_contributed_this_hand_accumulates_across_streets() {
        // `contributed_this_hand` is the seat's running stake in the live pot; it
        // must grow across streets, while `committed_this_street` resets each street.
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();

        // Pre-flop: everyone calls/checks down to the big blind.
        drive_street(&mut game, Street::Preflop, calling_policy);

        // After the street settles, the running totals equal each seat's blind/call
        // and the per-street commitment has been cleared (no round in progress).
        let preflop: Vec<SeatView> = game.table().seats;
        for seat in &preflop {
            assert_eq!(
                seat.committed_this_street, 0,
                "committed_this_street resets once the street is no longer active"
            );
            assert_eq!(
                seat.contributed_this_hand,
                game.contributed.get(&seat.id).copied().unwrap_or(0),
                "SeatView mirrors the engine's per-hand contribution map"
            );
        }
        // The distinguishing property: every seat posted a blind or called preflop, so
        // its cumulative stake is non-zero even though `committed_this_street` reset to
        // 0 above. A field wrongly aliased to the per-street counter would read 0 here.
        for seat in &preflop {
            assert!(
                seat.contributed_this_hand > 0,
                "contributed_this_hand retains the preflop stake after the street settles"
            );
        }
        let preflop_total: u32 = preflop.iter().map(|s| s.contributed_this_hand).sum();

        // Second street: more chips go in, so the per-hand totals strictly grow.
        game.deal_flop();
        drive_street(&mut game, Street::Flop, calling_policy);

        let flop: Vec<SeatView> = game.table().seats;
        let flop_total: u32 = flop.iter().map(|s| s.contributed_this_hand).sum();
        assert!(
            flop_total >= preflop_total,
            "contributed_this_hand never shrinks within a hand"
        );
        for seat in &flop {
            assert_eq!(
                seat.contributed_this_hand,
                game.contributed.get(&seat.id).copied().unwrap_or(0),
                "post-flop contributions still mirror the engine map across streets"
            );
        }
        // The big blind covered the flop check, so the dealt blinds make the running
        // total strictly exceed any single street's commitment.
        assert!(
            preflop_total > 0,
            "blinds put chips into the per-hand total before any voluntary action"
        );
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
        game.begin_hand().unwrap();
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
    fn pending_action_re_derives_the_prompt_without_mutating() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 3);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();
        let step = game.begin_betting_round(Street::Preflop);

        let BettingStep::AwaitingAction(pending) = &step else {
            panic!("expected to pause awaiting an actor");
        };
        let p1 = game.pending_action().expect("a decision is pending");
        let p2 = game.pending_action().expect("still pending (no mutation)");
        assert_eq!(p1.player, pending.player);
        assert_eq!(p2.player, pending.player);
        assert_eq!(p1.decision_id, pending.decision_id);
        assert_eq!(p2.decision_id, pending.decision_id);
        assert_eq!(p1.view.legal_actions, pending.view.legal_actions);

        let json = serde_json::to_string(&game).unwrap();
        let mut restored: TexasHoldEm = serde_json::from_str(&json).unwrap();
        let restored_pending = restored.pending_action().unwrap();
        assert_eq!(restored_pending.decision_id, pending.decision_id);
        assert!(matches!(
            restored
                .submit_action(
                    restored_pending.player,
                    restored_pending.decision_id,
                    calling_policy(&restored_pending.view),
                )
                .unwrap(),
            BettingStep::AwaitingAction(_)
        ));
        assert!(matches!(
            restored.submit_action(
                restored_pending.player,
                restored_pending.decision_id,
                PlayerAction::Fold
            ),
            Err(ActionSubmissionError::StaleDecision { .. })
        ));

        // Between hands there is no pending decision.
        let mut idle = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut idle, &[100, 100]);
        assert!(idle.pending_action().is_none());
    }

    #[test]
    fn quit_preserves_the_round_and_run_betting_round_resumes_it() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 11);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();

        // The player on the clock quits: the round is left paused on them (not
        // aborted), exactly the state a front-end would serialize to resume later.
        let mut quitters: HashMap<Uuid, Box<dyn PokerAgent>> = HashMap::new();
        for &id in game.seats() {
            quitters.insert(id, Box::new(QuitAgent));
        }
        assert_eq!(
            game.run_betting_round(Street::Preflop, &mut quitters),
            Ok(RoundOutcome::Quit)
        );
        assert!(game.hand_in_progress(), "the hand must survive a quit");
        assert_eq!(game.current_street(), Some(Street::Preflop));
        assert!(
            game.pending_action().is_some(),
            "a player is still on the clock after a quit"
        );

        // Calling run_betting_round again *continues* the same round to completion
        // rather than restarting the street.
        let mut callers: HashMap<Uuid, Box<dyn PokerAgent>> = HashMap::new();
        for &id in game.seats() {
            callers.insert(id, Box::new(CallingAgent));
        }
        let outcome = game
            .run_betting_round(Street::Preflop, &mut callers)
            .unwrap();
        assert!(matches!(
            outcome,
            RoundOutcome::Continue | RoundOutcome::HandOver
        ));
        assert!(
            game.pending_action().is_none(),
            "the resumed round finished, so no one is on the clock"
        );
    }

    #[test]
    fn run_betting_round_preserves_pending_decision_on_failures() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 13);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(pending) = game.begin_betting_round(Street::Preflop) else {
            panic!("expected a pending decision");
        };
        let before = serde_json::to_string(&game).unwrap();

        assert_eq!(
            game.run_betting_round(Street::Preflop, &mut HashMap::new()),
            Err(PlayError::MissingAgent(pending.player))
        );
        assert_eq!(serde_json::to_string(&game).unwrap(), before);

        let mut eof_agents: HashMap<Uuid, Box<dyn PokerAgent>> =
            HashMap::from([(pending.player, Box::new(EofAgent) as Box<dyn PokerAgent>)]);
        assert_eq!(
            game.run_betting_round(Street::Preflop, &mut eof_agents),
            Ok(RoundOutcome::Eof)
        );
        assert_eq!(serde_json::to_string(&game).unwrap(), before);

        let mut illegal_agents: HashMap<Uuid, Box<dyn PokerAgent>> = HashMap::from([(
            pending.player,
            Box::new(IllegalAgent) as Box<dyn PokerAgent>,
        )]);
        assert_eq!(
            game.run_betting_round(Street::Preflop, &mut illegal_agents),
            Err(PlayError::Submission(ActionSubmissionError::IllegalAction(
                ActionError::CannotCheck
            )))
        );
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
        assert_eq!(
            game.pending_action().unwrap().decision_id,
            pending.decision_id
        );
    }

    #[test]
    fn blocking_wrappers_preserve_pending_decision_on_failures() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 41);
        seat_players(&mut game, &[100, 100, 100]);

        let missing = game.play_hand(&mut HashMap::new());
        let pending = game
            .pending_action()
            .expect("missing agent leaves prompt intact");
        assert_eq!(missing, Err(PlayError::MissingAgent(pending.player)));

        let before = serde_json::to_string(&game).unwrap();
        let mut illegal: HashMap<Uuid, Box<dyn PokerAgent>> = HashMap::new();
        illegal.insert(pending.player, Box::new(IllegalAgent));
        assert_eq!(
            game.play_hand(&mut illegal),
            Err(PlayError::Submission(ActionSubmissionError::IllegalAction(
                ActionError::CannotCheck
            )))
        );
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
        assert_eq!(
            game.pending_action().unwrap().decision_id,
            pending.decision_id
        );
    }

    #[test]
    fn quit_and_eof_are_distinct_resumable_outcomes() {
        for (agent, expected) in [
            (
                Box::new(QuitAgent) as Box<dyn PokerAgent>,
                HandOutcome::Quit,
            ),
            (Box::new(EofAgent) as Box<dyn PokerAgent>, HandOutcome::Eof),
        ] {
            let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 43);
            seat_players(&mut game, &[100, 100]);
            let first = game.drive_hand();
            let HandStep::AwaitingAction(pending) = first else {
                panic!("expected decision");
            };
            let mut agents = HashMap::from([(pending.player, agent)]);
            assert_eq!(game.play_hand(&mut agents), Ok(expected));
            assert_eq!(
                game.pending_action().unwrap().decision_id,
                pending.decision_id
            );
        }
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

    // --- Hand-level driver, replay_log, client_view ---

    /// An agent that always folds — drives a hand to a pre-flop finish.
    struct FoldAgent;
    impl PokerAgent for FoldAgent {
        fn decide(&mut self, _view: &PlayerView) -> Result<PlayerAction, AgentError> {
            Ok(PlayerAction::Fold)
        }
    }

    fn agents_all(
        game: &TexasHoldEm,
        mut make: impl FnMut() -> Box<dyn PokerAgent>,
    ) -> HashMap<Uuid, Box<dyn PokerAgent>> {
        game.seats().iter().map(|&id| (id, make())).collect()
    }

    #[test]
    fn play_hand_plays_a_full_hand_conserving_chips() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 50, 75]);
        let before: u32 = game.players.values().map(|p| p.chips).sum();
        let mut agents = agents_all(&game, || Box::new(CallingAgent));
        assert_eq!(game.play_hand(&mut agents), Ok(HandOutcome::Complete));
        let after: u32 = game.players.values().map(|p| p.chips).sum();
        assert_eq!(before, after, "chips conserved over a full driven hand");
        assert_eq!(game.deck_len(), 52, "all cards returned to the deck");
        assert!(!game.hand_in_progress(), "the hand is over");
    }

    #[test]
    fn play_hand_heads_up_completes() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        let before: u32 = game.players.values().map(|p| p.chips).sum();
        let mut agents = agents_all(&game, || Box::new(CallingAgent));
        assert_eq!(game.play_hand(&mut agents), Ok(HandOutcome::Complete));
        assert_eq!(before, game.players.values().map(|p| p.chips).sum::<u32>());
        assert_eq!(game.deck_len(), 52);
    }

    #[test]
    fn play_hand_runs_out_all_streets_when_all_in() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 50, 75]);
        let before: u32 = game.players.values().map(|p| p.chips).sum();
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        let mut agents = agents_all(&game, || Box::new(ShoveAgent));
        assert_eq!(game.play_hand(&mut agents), Ok(HandOutcome::Complete));

        let events = log.borrow();
        let streets = events
            .iter()
            .filter(|e| matches!(e, GameEvent::StreetDealt { .. }))
            .count();
        assert_eq!(
            streets, 3,
            "flop, turn, and river are each dealt once on a run-out"
        );
        // Once the flop is dealt everyone is all-in, so no further actions occur.
        let first_street = events
            .iter()
            .position(|e| matches!(e, GameEvent::StreetDealt { .. }))
            .unwrap();
        assert!(
            !events[first_street..]
                .iter()
                .any(|e| matches!(e, GameEvent::ActionTaken { .. })),
            "no actions after the all-in run-out begins"
        );
        assert!(matches!(events.last(), Some(GameEvent::HandComplete)));
        let after: u32 = game.players.values().map(|p| p.chips).sum();
        assert_eq!(before, after, "chips conserved through the run-out");
    }

    #[test]
    fn play_hand_awards_without_flop_when_all_fold_preflop() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100, 100]);
        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        let mut agents = agents_all(&game, || Box::new(FoldAgent));
        assert_eq!(game.play_hand(&mut agents), Ok(HandOutcome::Complete));
        let events = log.borrow();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, GameEvent::StreetDealt { .. })),
            "no community cards dealt when the hand ends pre-flop"
        );
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::PotAwarded { .. })));
        assert!(matches!(events.last(), Some(GameEvent::HandComplete)));
    }

    #[test]
    fn drive_hand_round_trips_mid_hand_and_finishes_identically() {
        // Drive a hand to completion with the call-down policy.
        fn finish(game: &mut TexasHoldEm) {
            let mut step = game.drive_hand();
            while let HandStep::AwaitingAction(pending) = step {
                step = game
                    .submit_hand_action(
                        pending.player,
                        pending.decision_id,
                        calling_policy(&pending.view),
                    )
                    .unwrap();
            }
        }
        let chips = |g: &TexasHoldEm| -> Vec<u32> {
            g.seats().iter().map(|id| g.players[id].chips).collect()
        };

        let mut control = TexasHoldEm::new_seeded(0, 10, 1, 2, 7);
        seat_players(&mut control, &[100, 100, 100]);
        finish(&mut control);
        let control_chips = chips(&control);

        // Subject: same seed; drive two actions (asserting the resume invariant),
        // round-trip the whole engine mid-hand, then finish.
        let mut subject = TexasHoldEm::new_seeded(0, 10, 1, 2, 7);
        seat_players(&mut subject, &[100, 100, 100]);
        let mut step = subject.drive_hand();
        assert!(
            subject.betting.is_some(),
            "betting is active right after drive_hand opens pre-flop"
        );
        for _ in 0..2 {
            if let HandStep::AwaitingAction(pending) = &step {
                let action = calling_policy(&pending.view);
                step = subject
                    .submit_hand_action(pending.player, pending.decision_id, action)
                    .unwrap();
                if matches!(step, HandStep::AwaitingAction(_)) {
                    assert!(
                        subject.betting.is_some(),
                        "resume invariant: betting=Some at every paused boundary"
                    );
                }
            }
        }
        let json = serde_json::to_string(&subject).expect("serialize");
        let mut restored: TexasHoldEm = serde_json::from_str(&json).expect("deserialize");
        restored.set_observer(Box::new(NullObserver));
        finish(&mut restored);
        assert_eq!(
            chips(&restored),
            control_chips,
            "a restored hand finishes to identical chips"
        );
    }

    #[test]
    fn drive_hand_resumes_without_redeal_from_a_betting_none_board() {
        // Craft the (today-unreachable) between-streets state and confirm the
        // defensive resume re-opens that street's betting without re-dealing.
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 5);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();
        game.deal_flop();
        assert!(game.pending_action().is_none());
        let deck_before = game.deck_len();
        let board_before = game.board().cards.clone();

        let step = game.drive_hand();
        assert!(matches!(step, HandStep::AwaitingAction(_)));
        assert_eq!(
            game.current_street(),
            Some(Street::Flop),
            "re-opens flop betting"
        );
        assert_eq!(game.board().cards, board_before, "no re-deal");
        assert_eq!(game.deck_len(), deck_before, "no cards drawn");
    }

    #[test]
    fn client_view_is_leak_safe() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 9);
        let ids = seat_players(&mut game, &[100, 100, 100]);
        // No hero set → the broadcast event stream carries no hole cards.
        let HandStep::AwaitingAction(pending) = game.drive_hand() else {
            panic!("expected a pre-flop decision");
        };
        let actor = pending.player;
        let other = *ids.iter().find(|&&id| id != actor).unwrap();

        let actor_cv = game.client_view(actor);
        assert!(actor_cv.pending_action.is_some());
        assert!(
            actor_cv.pending_action.is_some(),
            "the acting player gets their decision view"
        );
        assert!(actor_cv.hole.is_some());

        let other_cv = game.client_view(other);
        assert!(other_cv.pending_action.is_none());
        assert_ne!(
            actor_cv.hole, other_cv.hole,
            "each sees only their own cards"
        );

        assert!(
            actor_cv.recent_events.iter().all(|e| match e {
                GameEvent::HoleCardsDealt { hero } => hero.is_none(),
                GameEvent::ShowdownReveal { .. } => false,
                _ => true,
            }),
            "the broadcast stream leaks no hole cards pre-showdown with no hero"
        );
        assert!(!actor_cv.table.seats.is_empty());

        // After the actor folds, their hole is mucked.
        game.submit_hand_action(actor, pending.decision_id, PlayerAction::Fold)
            .unwrap();
        assert!(
            game.client_view(actor).hole.is_none(),
            "folded cards are mucked"
        );
    }

    #[test]
    fn public_and_client_events_redact_private_hero_cards() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 23);
        let ids = seat_players(&mut game, &[100, 100, 100]);
        let hero = ids[0];
        let other = ids[1];
        game.set_hero(hero);
        game.begin_hand().unwrap();

        let public = game.public_events();
        assert!(public
            .iter()
            .all(|event| !matches!(event, GameEvent::HoleCardsDealt { hero: Some(_) })));

        let hero_events = game.client_view(hero).recent_events;
        assert!(hero_events.iter().any(|event| matches!(
            event,
            GameEvent::HoleCardsDealt {
                hero: Some((player, cards))
            } if player.id == hero && cards.len() == 2
        )));

        let other_events = game.client_view(other).recent_events;
        assert!(other_events
            .iter()
            .all(|event| !matches!(event, GameEvent::HoleCardsDealt { hero: Some(_) })));
    }

    #[test]
    fn accepted_decision_is_exactly_once_and_post_hand_input_is_rejected() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 29);
        seat_players(&mut game, &[100, 100]);
        let HandStep::AwaitingAction(first) = game.drive_hand() else {
            panic!("expected first decision");
        };
        let next = game
            .submit_hand_action(first.player, first.decision_id, PlayerAction::Call)
            .unwrap();
        let HandStep::AwaitingAction(_) = next else {
            panic!("expected another decision");
        };
        let before = serde_json::to_string(&game).unwrap();
        assert!(matches!(
            game.submit_hand_action(first.player, first.decision_id, PlayerAction::Call),
            Err(ActionSubmissionError::StaleDecision { .. })
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);

        let mut agents = agents_all(&game, || Box::new(CallingAgent));
        assert_eq!(game.play_hand(&mut agents), Ok(HandOutcome::Complete));
        let after = serde_json::to_string(&game).unwrap();
        assert!(matches!(
            game.submit_hand_action(first.player, first.decision_id, PlayerAction::Fold),
            Err(ActionSubmissionError::NoActionPending)
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), after);
    }

    #[test]
    fn replay_log_replays_real_events_in_order_idempotently() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 3);
        seat_players(&mut game, &[100, 100, 100]);
        let mut step = game.drive_hand();
        for _ in 0..2 {
            if let HandStep::AwaitingAction(pending) = &step {
                let action = calling_policy(&pending.view);
                step = game
                    .submit_hand_action(pending.player, pending.decision_id, action)
                    .unwrap();
            }
        }
        let expected = game.event_log.clone();
        let len_before = game.event_log.len();

        game.set_observer(Box::new(RecordingObserver { log: log.clone() }));
        game.replay_log();
        assert_eq!(
            *log.borrow(),
            expected,
            "replay re-sends the real events in order"
        );
        assert_eq!(
            game.event_log.len(),
            len_before,
            "replay does not grow the log"
        );
        game.replay_log();
        assert_eq!(
            game.event_log.len(),
            len_before,
            "a second replay still doesn't grow it"
        );
    }

    #[test]
    fn replay_and_event_copies_enforce_the_full_privacy_matrix() {
        fn dealt_hero(events: &[GameEvent]) -> Option<Uuid> {
            events.iter().find_map(|event| match event {
                GameEvent::HoleCardsDealt { hero } => hero.as_ref().map(|(player, _)| player.id),
                _ => None,
            })
        }

        let live_log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let replay_log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 37);
        let ids = seat_players(&mut game, &[100, 100, 100]);
        game.set_hero(ids[0]);
        game.set_observer(Box::new(RecordingObserver {
            log: live_log.clone(),
        }));
        let _ = game.drive_hand();

        assert_eq!(dealt_hero(&live_log.borrow()), Some(ids[0]));
        assert_eq!(dealt_hero(&game.public_events()), None);
        assert_eq!(
            dealt_hero(&game.client_view(ids[0]).recent_events),
            Some(ids[0])
        );
        assert_eq!(dealt_hero(&game.client_view(ids[1]).recent_events), None);

        game.set_hero(ids[1]);
        game.set_observer(Box::new(RecordingObserver {
            log: replay_log.clone(),
        }));
        game.replay_log();
        assert_eq!(
            dealt_hero(&replay_log.borrow()),
            None,
            "changing perspective must not replay the former hero's cards"
        );
    }

    #[test]
    fn event_log_holds_one_hand_and_clears_at_next_hand_start() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100, 100]);
        let mut agents = agents_all(&game, || Box::new(CallingAgent));
        game.play_hand(&mut agents).unwrap();
        // Between hands the log still holds the finished hand (incl. HandComplete).
        assert!(matches!(
            game.event_log.last(),
            Some(GameEvent::HandComplete)
        ));
        // The next hand clears it before re-populating.
        game.begin_hand().unwrap();
        assert!(
            matches!(game.event_log.first(), Some(GameEvent::HandStarted { .. })),
            "the log restarts with the new hand's header"
        );
        assert!(
            !game
                .event_log
                .iter()
                .any(|e| matches!(e, GameEvent::HandComplete)),
            "the previous hand's events were cleared"
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
                GameEvent::ShowdownReveal { player, hand, .. } if player.name == *name => {
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
        game.begin_hand().unwrap();
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

    #[test]
    fn live_hand_rejects_table_and_button_mutations() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();
        let seats_before = game.seats.clone();
        let dealer_before = game.dealer;

        let late = Player::new_with_chips("Late", 100);
        assert!(game.add_player(late).is_err());
        assert!(!game.randomize_seats());
        assert_eq!(game.remove_player(&ids[0]), None);
        assert!(!game.set_dealer(ids[1]));
        assert!(!game.rotate_dealer());
        assert_eq!(game.seats, seats_before);
        assert_eq!(game.dealer, dealer_before);
    }

    #[test]
    fn table_bankroll_cannot_exceed_u32() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        game.add_player(Player::new_with_chips("A", u32::MAX - 1))
            .unwrap();
        let b = Player::new_with_chips("B", 1);
        let b_id = b.identifier;
        game.add_player(b).unwrap();

        assert!(!game.add_chips_to(&b_id, 1));
        assert!(game.add_player(Player::new_with_chips("C", 1)).is_err());
        assert_eq!(game.total_table_chips(), Some(u32::MAX));
    }

    #[test]
    fn hand_start_validation_is_transactional() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        assert_eq!(
            game.begin_hand(),
            Err(HandStartError::NotEnoughPlayers { seated: 0 })
        );
        seat_players(&mut game, &[100, 100]);
        let before = serde_json::to_string(&game).unwrap();

        let short_deck = Deck::from_cards(vec![card(Rank::Ace, Suit::Spade); 11]);
        assert_eq!(
            game.begin_hand_with_deck(short_deck),
            Err(HandStartError::InsufficientCards {
                available: 11,
                required: 12,
            })
        );
        assert_eq!(serde_json::to_string(&game).unwrap(), before);

        let duplicate_deck = Deck::from_cards(vec![card(Rank::Ace, Suit::Spade); 12]);
        assert_eq!(
            game.begin_hand_with_deck(duplicate_deck),
            Err(HandStartError::DuplicateCards)
        );
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn drive_hand_reports_start_errors_without_mutating() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let before = serde_json::to_string(&game).unwrap();
        assert!(matches!(
            game.drive_hand(),
            HandStep::CannotStart(HandStartError::NotEnoughPlayers { seated: 0 })
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn wrong_street_is_rejected_without_mutation() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        let before = serde_json::to_string(&game).unwrap();

        assert!(matches!(
            game.begin_betting_round(Street::River),
            BettingStep::CannotStart(HandStartError::InvalidHandState)
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn restored_pending_actor_without_a_hand_is_rejected() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(pending) = game.begin_betting_round(Street::Preflop) else {
            panic!("expected a pending action");
        };
        game.player_hands.remove(&pending.player);

        assert!(game.pending_action().is_none());
        assert!(matches!(
            game.drive_hand(),
            HandStep::CannotStart(HandStartError::InvalidHandState)
        ));
    }

    #[test]
    fn restored_state_with_duplicate_cards_is_rejected_without_mutation() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        let duplicate = game.player_hands[&game.seats[0]].cards[0];
        game.board.push(duplicate);
        game.board.push(card(Rank::Two, Suit::Club));
        game.board.push(card(Rank::Three, Suit::Club));
        let before = serde_json::to_string(&game).unwrap();

        assert!(matches!(
            game.drive_hand(),
            HandStep::CannotStart(HandStartError::InvalidHandState)
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn restored_state_without_enough_cards_is_rejected_without_dealing() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        while game.deck.len() > 7 {
            game.deck.deal();
        }
        let before = serde_json::to_string(&game).unwrap();

        assert!(matches!(
            game.drive_hand(),
            HandStep::CannotStart(HandStartError::InvalidHandState)
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn client_view_does_not_panic_on_a_malformed_saved_hand() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        let ids = seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        game.player_hands.get_mut(&ids[0]).unwrap().cards.pop();

        let view = game.client_view(ids[0]);
        assert_eq!(view.you, ids[0]);
        assert_eq!(view.hole, None);
        assert!(view.pending_action.is_none());
    }

    #[test]
    fn invalid_restored_bankroll_rejects_submission_without_mutation() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(pending) = game.begin_betting_round(Street::Preflop) else {
            panic!("expected a pending action");
        };
        game.contributed.insert(pending.player, u32::MAX);
        let before = serde_json::to_string(&game).unwrap();

        assert!(matches!(
            game.submit_action(pending.player, pending.decision_id, PlayerAction::Fold),
            Err(ActionSubmissionError::InvalidState(
                HandStartError::ChipTotalTooLarge
            ))
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn invalid_completed_street_marker_is_rejected_without_dealing() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        game.completed_betting_street = Some(Street::Flop);
        let before = serde_json::to_string(&game).unwrap();

        assert!(matches!(
            game.drive_hand(),
            HandStep::CannotStart(HandStartError::InvalidHandState)
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn restored_round_without_pending_decision_is_rejected() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(_) = game.begin_betting_round(Street::Preflop) else {
            panic!("expected a pending action");
        };
        game.betting.as_mut().unwrap().awaiting = None;

        assert!(matches!(
            game.drive_hand(),
            HandStep::CannotStart(HandStartError::InvalidHandState)
        ));
    }

    #[test]
    fn exhausted_decision_ids_are_rejected_before_action_mutation() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(pending) = game.begin_betting_round(Street::Preflop) else {
            panic!("expected a pending action");
        };
        game.next_decision_id = u64::MAX;
        let before = serde_json::to_string(&game).unwrap();

        assert!(matches!(
            game.submit_action(pending.player, pending.decision_id, PlayerAction::Call),
            Err(ActionSubmissionError::InvalidState(
                HandStartError::InvalidHandState
            ))
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn exhausted_decision_ids_are_rejected_before_starting_a_round() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.next_decision_id = u64::MAX;
        let before = serde_json::to_string(&game).unwrap();

        assert!(matches!(
            game.drive_hand(),
            HandStep::CannotStart(HandStartError::InvalidHandState)
        ));
        assert_eq!(serde_json::to_string(&game).unwrap(), before);
    }

    #[test]
    fn invalid_pending_actor_is_rejected() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(pending) = game.begin_betting_round(Street::Preflop) else {
            panic!("expected a pending action");
        };
        game.folded.insert(pending.player);
        game.player_hands.remove(&pending.player);

        assert!(matches!(
            game.drive_hand(),
            HandStep::CannotStart(HandStartError::InvalidHandState)
        ));
    }

    #[test]
    fn street_commitment_cannot_exceed_hand_contribution() {
        let mut game = TexasHoldEm::new(0, 10, 1, 2);
        seat_players(&mut game, &[100, 100]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(pending) = game.begin_betting_round(Street::Preflop) else {
            panic!("expected a pending action");
        };
        game.contributed.insert(pending.player, 0);

        assert!(matches!(
            game.submit_action(pending.player, pending.decision_id, PlayerAction::Call),
            Err(ActionSubmissionError::InvalidState(
                HandStartError::InvalidHandState
            ))
        ));
    }

    #[test]
    fn raise_event_uses_the_pre_action_bet() {
        let mut game = TexasHoldEm::new_seeded(0, 10, 1, 2, 41);
        seat_players(&mut game, &[100, 100, 100]);
        game.begin_hand().unwrap();
        let BettingStep::AwaitingAction(pending) = game.begin_betting_round(Street::Preflop) else {
            panic!("expected a pending action");
        };

        game.submit_action(
            pending.player,
            pending.decision_id,
            PlayerAction::RaiseTo(6),
        )
        .unwrap();

        assert!(game.event_log.iter().any(|event| matches!(
            event,
            GameEvent::ActionTaken {
                action: ActionView::Raised { by: 4, to: 6, .. },
                ..
            }
        )));
    }
}
