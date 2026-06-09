use std::collections::HashMap;
use std::io::{self, Write};
use std::process;
use std::thread::sleep;
use std::time::Duration;

use rand::prelude::*;

use casino_poker::agent::{AgentError, LegalAction, PlayerAction, PlayerView, PokerAgent, Street};
use casino_poker::casino_cards::card::{set_glyph_display, Card};
use casino_poker::games::texas_hold_em::{RoundOutcome, TexasHoldEm};
use casino_poker::hand_rankings::{evaluate, HandCategory};
use casino_poker::player::Player;
use casino_poker::uuid::Uuid;

use crate::persistence::{self, Profile};

const MINIMUM_CHIPS_BUY_IN_AMOUNT: u32 = 100;
// 10 is the recommended maximum number of players at a table, so it is the default.
const MAXIMUM_PLAYERS_COUNT: usize = 10;
const CURRENCY: &str = "USD";
const OPPONENT_COUNT: usize = 5;

/// Terminal front-end for Texas Hold'em: owns the engine and the per-player
/// agents, and drives the hand/tournament loop.
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
            self.game.print_leaderboard();

            let chips_before = self.user_chips();
            let outcome = self.play_round();

            if outcome == RoundOutcome::Quit {
                println!("Quitting game. Your progress is saved.");
                self.persist();
                return;
            }

            // Record stats for the completed hand.
            self.profile.hands_played += 1;
            if self.user_chips() > chips_before {
                self.profile.hands_won += 1;
            }
            self.persist();

            self.game.remove_losers();
            self.prune_agents();

            if self.game.player(&self.user_id).is_none() {
                println!("\nYou are out of chips. Thanks for playing!");
                self.persist();
                return;
            }
            if self.game.check_for_game_over() {
                self.persist();
                return;
            }
            if !prompt_play_another_hand() {
                self.game.end_game();
                self.persist();
                println!("Progress saved. See you next time!");
                return;
            }
        }
    }

    /// Plays a single hand: deal, run each street's betting, then award the pots.
    fn play_round(&mut self) -> RoundOutcome {
        self.game.begin_hand();

        if let Some(hand) = self.game.player_hand(&self.user_id) {
            println!("\nYour hand: {}\n", hand.to_symbols());
        }

        for street in [Street::Preflop, Street::Flop, Street::Turn, Street::River] {
            match street {
                Street::Preflop => {}
                Street::Flop => {
                    self.game.deal_flop();
                    self.print_board("FLOP");
                }
                Street::Turn => {
                    self.game.deal_turn();
                    self.print_board("TURN");
                }
                Street::River => {
                    self.game.deal_river();
                    self.print_board("RIVER");
                }
            }

            match self.game.run_betting_round(street, &mut self.agents) {
                RoundOutcome::Continue => {}
                RoundOutcome::HandOver => break,
                RoundOutcome::Quit => return RoundOutcome::Quit,
            }
        }

        println!();
        self.game.award_pots();
        self.game.end_hand();
        RoundOutcome::Continue
    }

    fn print_board(&self, label: &str) {
        println!("** {label} **");
        println!("Board: {}", self.game.board().to_symbols());
        println!("Pot: {}\n", self.game.pot_total());
    }

    /// Drops agents for players who are no longer seated (e.g. busted out).
    fn prune_agents(&mut self) {
        let seated: Vec<Uuid> = self.game.seats().to_vec();
        self.agents.retain(|id, _| seated.contains(id));
    }
}

pub fn play_game() {
    println!("**********************");
    println!("* ♠ Texas hold 'em ♠ *");
    println!("**********************");

    let (small_blind_amount, big_blind_amount) = choose_table();

    let mut game = TexasHoldEm::new(
        MINIMUM_CHIPS_BUY_IN_AMOUNT,
        MAXIMUM_PLAYERS_COUNT,
        small_blind_amount,
        big_blind_amount,
    );

    // Set up the user, resuming a saved profile when one exists.
    let mut profile = load_or_create_profile();
    profile.glyph_cards = choose_card_style(profile.glyph_cards);
    set_glyph_display(profile.glyph_cards);
    println!(
        "Your progress was saved at {}.\n",
        persistence::save_location()
    );

    let mut user = game.new_player(&profile.name);
    user.add_chips(profile.chips);
    while user.chips < MINIMUM_CHIPS_BUY_IN_AMOUNT {
        println!("You need at least {MINIMUM_CHIPS_BUY_IN_AMOUNT} chips to play at this table.");
        buy_chips_prompt(&mut user);
    }
    profile.chips = user.chips;
    let user_id = user.identifier;
    if let Err(reason) = game.add_player(user) {
        eprintln!("Unable to seat you: {reason}");
        process::exit(1);
    }

    // Seat the computer opponents.
    for i in 1..=OPPONENT_COUNT {
        let opponent =
            game.new_player_with_chips(&format!("Player {}", i + 1), MINIMUM_CHIPS_BUY_IN_AMOUNT);
        let _ = game.add_player(opponent);
    }

    println!();

    let mut texas_hold_em = TexasHoldEmGame::new(game, user_id, profile);

    // Register an agent for every seated player: the user is human, most
    // opponents play the strength-aware heuristic, and one is a "loose" random
    // player for variety.
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
            // One loose opponent keeps games from feeling uniform.
            Box::new(RandomAgent)
        } else {
            Box::new(HeuristicAgent)
        };
        texas_hold_em.agents.insert(id, agent);
    }

    texas_hold_em.play_tournament();
}

/// A human player driven by stdin prompts.
struct HumanAgent;

impl PokerAgent for HumanAgent {
    fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError> {
        let can_check = view
            .legal_actions
            .iter()
            .any(|a| matches!(a, LegalAction::Check));
        let call_amount = view.legal_actions.iter().find_map(|a| match *a {
            LegalAction::Call(amount) => Some(amount),
            _ => None,
        });
        let raise_range = view.legal_actions.iter().find_map(|a| match *a {
            LegalAction::RaiseTo { min, max } => Some((min, max)),
            _ => None,
        });
        let all_in_total = view.legal_actions.iter().find_map(|a| match *a {
            LegalAction::AllIn(total) => Some(total),
            _ => None,
        });

        println!("-- Your turn --");
        println!("Your hand: {}", cards_to_string(&view.hole));
        if !view.board.is_empty() {
            println!("Board: {}", cards_to_string(&view.board));
        }
        println!(
            "Pot: {} | Your chips: {} | To call: {}",
            view.pot_total, view.chips, view.amount_owed
        );

        loop {
            // Each action can be chosen by a single-letter shortcut or its full
            // word. Check uses `x` so `c` is unambiguously call.
            let mut menu: Vec<String> = vec!["(f)old".to_string()];
            if can_check {
                menu.push("(x) check".to_string());
            } else if let Some(amount) = call_amount {
                menu.push(format!("(c)all {amount}"));
            }
            if let Some((min, max)) = raise_range {
                // Raise amounts are the total to commit this street ("raise to").
                menu.push(format!("(r)aise to {min}-{max}"));
            }
            if let Some(total) = all_in_total {
                menu.push(format!("(a)ll-in ({total})"));
            }
            print!("Action ({}): ", menu.join(", "));
            io::stdout().flush().expect("Failed to flush stdout.");

            let input = read_line()?;
            let lowered = input.trim().to_lowercase();
            let mut tokens = lowered.split_whitespace();

            match tokens.next() {
                Some("q") | Some("quit") => return Err(AgentError::Quit),
                Some("f") | Some("fold") => return Ok(PlayerAction::Fold),
                Some("x") | Some("check") if can_check => return Ok(PlayerAction::Check),
                Some("c") | Some("call") if call_amount.is_some() => return Ok(PlayerAction::Call),
                // Typing call when the stack can't fully cover the bet is a short all-in.
                Some("c") | Some("call") if all_in_total.is_some() && !can_check => {
                    return Ok(PlayerAction::AllIn)
                }
                // `c` when checking is free: there's nothing to call.
                Some("c") | Some("call") if can_check => {
                    println!("Nothing to call — type 'x' to check.");
                }
                Some("a") | Some("all") | Some("allin") | Some("all-in")
                    if all_in_total.is_some() =>
                {
                    return Ok(PlayerAction::AllIn)
                }
                Some("r") | Some("raise") if raise_range.is_some() => {
                    let (min, max) = raise_range.unwrap();
                    // Accept both "raise 50" and "raise to 50".
                    let mut amount_token = tokens.next();
                    if amount_token == Some("to") {
                        amount_token = tokens.next();
                    }
                    let to = match amount_token.and_then(|s| s.parse::<u32>().ok()) {
                        Some(to) => to,
                        None => {
                            print!("Raise to how much? ");
                            io::stdout().flush().expect("Failed to flush stdout.");
                            match read_line()?.trim().parse::<u32>() {
                                Ok(to) => to,
                                Err(_) => {
                                    println!("Please enter a whole number.");
                                    continue;
                                }
                            }
                        }
                    };
                    if to < min || to > max {
                        println!("You can raise to between {min} and {max} chips.");
                        continue;
                    }
                    return Ok(PlayerAction::RaiseTo(to));
                }
                _ => println!("Invalid action. Try again, or type 'quit'."),
            }
        }
    }
}

/// A "loose" opponent that plays legal but unsophisticated poker (it ignores its
/// cards). One is seated alongside the [`HeuristicAgent`]s for variety so the
/// table doesn't feel uniform.
struct RandomAgent;

impl PokerAgent for RandomAgent {
    fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError> {
        let mut rng = rand::thread_rng();
        // A short pause so the action is readable as it scrolls by.
        sleep(Duration::from_millis(rng.gen_range(300..1000)));

        let roll: f64 = rng.gen();
        let can_check = view
            .legal_actions
            .iter()
            .any(|a| matches!(a, LegalAction::Check));
        let can_call = view
            .legal_actions
            .iter()
            .any(|a| matches!(a, LegalAction::Call(_)));
        let raise_min = view.legal_actions.iter().find_map(|a| match *a {
            LegalAction::RaiseTo { min, .. } => Some(min),
            _ => None,
        });

        if can_check {
            if let Some(min) = raise_min {
                if roll < 0.2 {
                    return Ok(PlayerAction::RaiseTo(min));
                }
            }
            return Ok(PlayerAction::Check);
        }

        if can_call {
            if let Some(min) = raise_min {
                if roll < 0.12 {
                    return Ok(PlayerAction::RaiseTo(min));
                }
            }
            if roll < 0.6 {
                return Ok(PlayerAction::Call);
            }
            return Ok(PlayerAction::Fold);
        }

        // Can't even call: occasionally shove, otherwise fold.
        let can_all_in = view
            .legal_actions
            .iter()
            .any(|a| matches!(a, LegalAction::AllIn(_)));
        if can_all_in && roll < 0.3 {
            return Ok(PlayerAction::AllIn);
        }
        Ok(PlayerAction::Fold)
    }
}

/// Asks how cards should be rendered, defaulting to the current preference so
/// pressing Enter keeps it. Glyphs look nicer where supported; text is portable.
fn choose_card_style(current: bool) -> bool {
    let current_label = if current { "glyphs" } else { "text" };
    println!(
        "Card display: (t)ext like A♠, or (g)lyphs like 🂡 (nicer, but tiny in some terminals)."
    );
    print!("Choose [t/g] (Enter keeps {current_label}): ");
    io::stdout().flush().expect("Failed to flush stdout.");

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return current;
    }
    match input.trim().to_lowercase().as_str() {
        "q" | "quit" => {
            println!("Quitting game.");
            process::exit(0);
        }
        "g" | "glyph" | "glyphs" => true,
        "t" | "text" => false,
        _ => current,
    }
}

/// Loads the saved profile (offering to resume it) or creates a fresh one.
fn load_or_create_profile() -> Profile {
    if let Some(profile) = persistence::load() {
        println!(
            "Welcome back, {}! You have {} chips ({} hands played, {} won).",
            profile.name, profile.chips, profile.hands_played, profile.hands_won
        );
        print!("Resume this profile? [Y/n]: ");
        io::stdout().flush().expect("Failed to flush stdout.");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            match input.trim().to_lowercase().as_str() {
                "n" | "no" => {} // fall through to create a new profile
                "q" | "quit" => {
                    println!("Quitting game.");
                    process::exit(0);
                }
                _ => return profile,
            }
        }
    }

    let name = get_player_name_prompt();
    Profile::new(&name, 0)
}

/// An opponent that actually reads its cards: it estimates hand strength and
/// weighs it against the pot odds to decide. More realistic than [`RandomAgent`],
/// and the natural place a future model-backed agent would slot in.
struct HeuristicAgent;

impl PokerAgent for HeuristicAgent {
    fn decide(&mut self, view: &PlayerView) -> Result<PlayerAction, AgentError> {
        let mut rng = rand::thread_rng();
        sleep(Duration::from_millis(rng.gen_range(300..900)));

        let strength = estimate_strength(view);
        // Price of calling: chips owed as a fraction of the pot after calling.
        let pot_odds = if view.amount_owed == 0 {
            0.0
        } else {
            view.amount_owed as f64 / (view.pot_total + view.amount_owed) as f64
        };

        let can_check = matches_any(view, |a| matches!(a, LegalAction::Check));
        let can_call = matches_any(view, |a| matches!(a, LegalAction::Call(_)));
        let raise_min = view.legal_actions.iter().find_map(|a| match *a {
            LegalAction::RaiseTo { min, .. } => Some(min),
            _ => None,
        });
        let can_all_in = matches_any(view, |a| matches!(a, LegalAction::AllIn(_)));
        let roll: f64 = rng.gen();

        if can_check {
            // Free to continue: value-bet strong hands, occasionally semi-bluff.
            if let Some(min) = raise_min {
                if strength > 0.78 || (strength > 0.5 && roll < 0.35) {
                    return Ok(PlayerAction::RaiseTo(min));
                }
            }
            return Ok(PlayerAction::Check);
        }

        // Facing a bet.
        if let Some(min) = raise_min {
            if strength > 0.85 && roll < 0.7 {
                return Ok(PlayerAction::RaiseTo(min));
            }
        }

        // Call when estimated strength beats the price, with a little slack.
        if strength + 0.05 >= pot_odds {
            if can_call {
                return Ok(PlayerAction::Call);
            }
            if can_all_in && strength > 0.7 {
                return Ok(PlayerAction::AllIn);
            }
        }

        Ok(PlayerAction::Fold)
    }
}

fn matches_any(view: &PlayerView, predicate: impl Fn(&LegalAction) -> bool) -> bool {
    view.legal_actions.iter().any(predicate)
}

/// Estimates hand strength in `0.0..=1.0`. Pre-flop uses a simple hole-card
/// heuristic; post-flop it evaluates the made hand against the board.
fn estimate_strength(view: &PlayerView) -> f64 {
    if view.board.is_empty() {
        preflop_strength(view.hole)
    } else {
        let hand = evaluate(&view.hole, &view.board);
        let base = match hand.category {
            // High-card strength scales with the top card.
            HandCategory::HighCard => 0.10 + (hand.tiebreak[0] as f64 / 14.0) * 0.20,
            HandCategory::Pair => 0.55,
            HandCategory::TwoPair => 0.72,
            HandCategory::ThreeOfAKind => 0.82,
            HandCategory::Straight => 0.88,
            HandCategory::Flush => 0.92,
            HandCategory::FullHouse => 0.96,
            HandCategory::FourOfAKind => 0.99,
            HandCategory::StraightFlush => 1.0,
        };
        base.clamp(0.0, 1.0)
    }
}

fn preflop_strength(hole: [Card; 2]) -> f64 {
    let hi = hole[0].rank.value().max(hole[1].rank.value()) as f64;
    let lo = hole[0].rank.value().min(hole[1].rank.value()) as f64;
    let is_pair = hole[0].rank == hole[1].rank;
    let is_suited = hole[0].suit == hole[1].suit;

    let mut strength = (hi + lo) / 56.0; // ~0.14..=0.5 from the card ranks
    if is_pair {
        strength += 0.35 + hi / 100.0; // pairs are strong, higher pairs stronger
    } else {
        if is_suited {
            strength += 0.06;
        }
        if (hi - lo) <= 2.0 {
            strength += 0.05; // connected cards make straights
        }
    }
    strength.clamp(0.0, 1.0)
}

/// Reads a line from stdin, returning [`AgentError::Eof`] at end of input.
fn read_line() -> Result<String, AgentError> {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => Err(AgentError::Eof),
        Ok(_) => Ok(input),
        Err(_) => Err(AgentError::Eof),
    }
}

fn cards_to_string(cards: &[Card]) -> String {
    cards
        .iter()
        .map(|card| card.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn get_player_name_prompt() -> String {
    loop {
        print!("Enter your name: ");
        io::stdout().flush().expect("Failed to flush stdout.");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            process::exit(0);
        }
        let trimmed_input = input.trim();

        if trimmed_input.eq_ignore_ascii_case("q") || trimmed_input.eq_ignore_ascii_case("quit") {
            println!("Quitting game.");
            process::exit(0);
        }

        let name = if trimmed_input.is_empty() {
            String::from("Player 1")
        } else {
            String::from(trimmed_input)
        };

        println!("\nWelcome {name}! Are you happy with this name?");
        print!("yes/no [Y/n]: ");
        io::stdout().flush().expect("Failed to flush stdout.");

        let mut confirm = String::new();
        if io::stdin().read_line(&mut confirm).is_err() {
            process::exit(0);
        }

        match confirm.trim().to_lowercase().as_str() {
            "q" | "quit" => {
                println!("Quitting game.");
                process::exit(0);
            }
            "n" | "no" => continue,
            _ => return name,
        }
    }
}

fn choose_table() -> (u32, u32) {
    loop {
        println!("Tables");
        println!("1. 1/3 No-limit");
        println!("2. 2/5 No-limit");
        println!("3. Custom");
        print!("Table: ");
        io::stdout().flush().expect("Failed to flush stdout.");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            process::exit(0);
        }
        let trimmed_input = input.trim();

        match trimmed_input.to_lowercase().as_str() {
            "q" | "quit" => {
                println!("Quitting game.");
                process::exit(0);
            }
            "1" => return (1, 3),
            "2" => return (2, 5),
            "3" => loop {
                println!("Enter the small and big blind amounts.");
                print!("Format <small_blind> <big_blind>: ");
                io::stdout().flush().expect("Failed to flush stdout.");

                let mut custom = String::new();
                if io::stdin().read_line(&mut custom).is_err() {
                    process::exit(0);
                }
                if custom.trim().eq_ignore_ascii_case("q") {
                    process::exit(0);
                }

                let mut numbers = custom.split_whitespace();
                let small: Result<u32, _> = numbers.next().unwrap_or("").parse();
                let big: Result<u32, _> = numbers.next().unwrap_or("").parse();
                if let (Ok(small), Ok(big)) = (small, big) {
                    if small > 0 && big > small {
                        return (small, big);
                    }
                }
                println!("Please enter two numbers where the big blind exceeds the small blind.");
            },
            _ => println!("Invalid input. Enter 1, 2, 3, or 'q' to quit.\n"),
        }
    }
}

fn buy_chips_prompt(player: &mut Player) {
    println!("How many chips would you like to buy?");
    loop {
        print!("Amount ({CURRENCY}) of chips to buy: ");
        io::stdout().flush().expect("Failed to flush stdout.");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            process::exit(0);
        }
        let trimmed_input = input.trim();

        if trimmed_input.eq_ignore_ascii_case("q") || trimmed_input.eq_ignore_ascii_case("quit") {
            println!("No chips were purchased. Quitting game.");
            process::exit(0);
        }

        match trimmed_input.parse::<u32>() {
            Ok(chips) => {
                player.add_chips(chips);
                println!("You purchased {CURRENCY} {chips} worth of chips.\n");
                return;
            }
            Err(_) => println!("Error: Not a valid number."),
        }
    }
}

fn prompt_play_another_hand() -> bool {
    print!("\nPlay another hand? [Y/n]: ");
    io::stdout().flush().expect("Failed to flush stdout.");

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    !matches!(
        input.trim().to_lowercase().as_str(),
        "q" | "quit" | "n" | "no"
    )
}
