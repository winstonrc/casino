use std::collections::HashMap;
use std::process;

use casino_poker::agent::PokerAgent;
use casino_poker::agents::{HeuristicAgent, RandomAgent};
use casino_poker::casino_cards::card::set_glyph_display;
use casino_poker::games::texas_hold_em::{RoundOutcome, TexasHoldEm};
use casino_poker::uuid::Uuid;

use crate::agents::HumanAgent;
use crate::hand_history::HandHistory;
use crate::persistence::{self, Profile};
use crate::prompts;
use crate::render;

const MINIMUM_CHIPS_BUY_IN_AMOUNT: u32 = 100;
// 10 is the recommended maximum number of players at a table, so it is the default.
const MAXIMUM_PLAYERS_COUNT: usize = 10;
const OPPONENT_COUNT: usize = 5;

/// Terminal front-end for Texas Hold'em: owns the engine and the per-player
/// agents, and drives the hand/tournament loop. The hand history is rendered to
/// stdout by the engine's [`HandHistory`] observer; between-hand chrome goes to
/// stderr via the `render` helpers.
pub struct TexasHoldEmGame {
    game: TexasHoldEm,
    agents: HashMap<Uuid, Box<dyn PokerAgent>>,
    user_id: Uuid,
    profile: Profile,
}

impl TexasHoldEmGame {
    fn new(game: TexasHoldEm, user_id: Uuid, profile: Profile) -> Self {
        Self {
            game,
            agents: HashMap::new(),
            user_id,
            profile,
        }
    }

    /// Current chips of the user, or 0 if they have been removed from the table.
    fn user_chips(&self) -> u32 {
        self.game.player(&self.user_id).map_or(0, |p| p.chips)
    }

    /// Sync the profile's chip count from the engine and write it to disk.
    fn persist(&mut self) {
        self.profile.chips = self.user_chips();
        persistence::save(&self.profile);
    }

    /// Runs the tournament until the game ends or the user quits.
    fn play_tournament(&mut self) {
        while !self.game.check_for_game_over() {
            render::render_leaderboard(&self.game);

            let chips_before = self.user_chips();
            let outcome = self.play_round();

            if outcome == RoundOutcome::Quit {
                eprintln!("Quitting game. Your progress is saved.");
                self.persist();
                return;
            }

            // Record stats for the completed hand.
            self.profile.hands_played += 1;
            if self.user_chips() > chips_before {
                self.profile.hands_won += 1;
            }
            self.persist();

            for name in self.game.remove_losers() {
                render::render_removed(&name);
            }
            self.prune_agents();

            if self.game.player(&self.user_id).is_none() {
                eprintln!("\nYou are out of chips. Thanks for playing!");
                self.persist();
                return;
            }
            if self.game.check_for_game_over() {
                render::render_game_over(&self.game);
                self.persist();
                return;
            }
            if !prompts::prompt_play_another_hand() {
                self.game.end_game();
                self.persist();
                eprintln!("Progress saved. See you next time!");
                return;
            }
        }
    }

    /// Plays a single hand: deal, run each street's betting, then award the pots.
    /// The board and all narration are rendered by the engine's observer.
    fn play_round(&mut self) -> RoundOutcome {
        use casino_poker::agent::Street;

        self.game.begin_hand();

        for street in [Street::Preflop, Street::Flop, Street::Turn, Street::River] {
            match street {
                Street::Preflop => {}
                Street::Flop => self.game.deal_flop(),
                Street::Turn => self.game.deal_turn(),
                Street::River => self.game.deal_river(),
            }

            match self.game.run_betting_round(street, &mut self.agents) {
                RoundOutcome::Continue => {}
                RoundOutcome::HandOver => break,
                RoundOutcome::Quit => return RoundOutcome::Quit,
            }
        }

        self.game.award_pots();
        self.game.end_hand();
        RoundOutcome::Continue
    }

    /// Drops agents for players who are no longer seated (e.g. busted out).
    fn prune_agents(&mut self) {
        let seated: Vec<Uuid> = self.game.seats().to_vec();
        self.agents.retain(|id, _| seated.contains(id));
    }
}

pub fn play_game() {
    eprintln!("**********************");
    eprintln!("* ♠ Texas hold 'em ♠ *");
    eprintln!("**********************");

    let (small_blind_amount, big_blind_amount) = prompts::choose_table();

    let mut game = TexasHoldEm::new(
        MINIMUM_CHIPS_BUY_IN_AMOUNT,
        MAXIMUM_PLAYERS_COUNT,
        small_blind_amount,
        big_blind_amount,
    );

    // Set up the user, resuming a saved profile when one exists.
    let mut profile = prompts::load_or_create_profile();
    profile.glyph_cards = prompts::choose_card_style(profile.glyph_cards);
    set_glyph_display(profile.glyph_cards);
    eprintln!(
        "Your progress was saved at {}.\n",
        persistence::save_location()
    );

    let mut user = game.new_player(&profile.name);
    user.add_chips(profile.chips);
    while user.chips < MINIMUM_CHIPS_BUY_IN_AMOUNT {
        eprintln!("You need at least {MINIMUM_CHIPS_BUY_IN_AMOUNT} chips to play at this table.");
        prompts::buy_chips_prompt(&mut user);
    }
    profile.chips = user.chips;

    let user_name = profile.name.clone();
    let user_chips = user.chips;
    let user_id = user.identifier;
    match game.add_player(user) {
        Ok(()) => render::render_buy_in(&user_name, user_chips),
        Err(reason) => {
            eprintln!("Unable to seat you: {reason}");
            process::exit(1);
        }
    }

    // Seat the computer opponents.
    for i in 1..=OPPONENT_COUNT {
        let opponent =
            game.new_player_with_chips(&format!("Player {}", i + 1), MINIMUM_CHIPS_BUY_IN_AMOUNT);
        let name = opponent.name.clone();
        let chips = opponent.chips;
        if game.add_player(opponent).is_ok() {
            render::render_buy_in(&name, chips);
        }
    }

    // Shuffle seating so the dealer button doesn't always start with the user.
    game.randomize_seats();

    // Narrate the hand as a PokerStars history on stdout, from the user's seat,
    // and save a parseable copy to a per-session log file (created on the first
    // hand, so abandoning setup leaves nothing behind).
    let log_path = persistence::session_history_path();
    if let Some(path) = &log_path {
        eprintln!("Saving this session's hand history to {}.", path.display());
    }
    game.set_hero(user_id);
    game.set_observer(Box::new(HandHistory::stdout(log_path)));

    let mut texas_hold_em = TexasHoldEmGame::new(game, user_id, profile);

    // Register an agent for every seated player: the user is human, most opponents
    // play the strength-aware heuristic, and one is a "loose" random player.
    let first_opponent = texas_hold_em
        .game
        .seats()
        .iter()
        .copied()
        .find(|id| *id != user_id);
    for id in texas_hold_em.game.seats().to_vec() {
        let agent: Box<dyn PokerAgent> = if id == user_id {
            Box::new(HumanAgent)
        } else if Some(id) == first_opponent {
            Box::new(RandomAgent)
        } else {
            Box::new(HeuristicAgent)
        };
        texas_hold_em.agents.insert(id, agent);
    }

    texas_hold_em.play_tournament();
}
