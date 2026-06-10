//! PokerStars-format hand-history renderer.
//!
//! A [`HandHistory`] is a [`GameObserver`] that turns the engine's public
//! `GameEvent` stream into a PokerStars hand history written to **stdout**, one
//! line at a time (flushed, so redirecting stdout to a file yields a clean,
//! parseable history). In the default text card mode the cards are PokerStars
//! codes (`As`, `Td`) so standard tools can parse the output; in glyph mode the
//! same layout prints with Unicode card glyphs as flair.
//!
//! The human's turn prompt is *not* part of the history — it is written to stderr
//! by the agent (see [`crate::agents`]).

use std::collections::HashMap;
use std::io::{self, Stdout, Write};

use chrono::Local;

use casino_poker::agent::Street;
use casino_poker::casino_cards::card::Card;
use casino_poker::events::{ActionView, Blind, GameEvent, GameObserver, PotKind, SeatInfo};
use casino_poker::hand_rankings::ComparableHand;

use crate::render::cards_to_string;

/// Site token at the head of every hand. Standard parsers detect the format by
/// this name, so it defaults to `PokerStars`; rename to brand your own histories
/// (at the cost of stock tools auto-detecting them).
const SITE: &str = "PokerStars";
const TABLE: &str = "Casino";
/// Cosmetic timezone label on the header timestamp. The time itself is the local
/// clock, so this won't necessarily match; parsers tolerate the token.
const TIMEZONE: &str = "ET";

/// Renders the engine's `GameEvent`s as a PokerStars hand history to a writer —
/// stdout in production (see [`HandHistory::stdout`]), or an in-memory buffer in
/// tests.
pub struct HandHistory<W: Write = Stdout> {
    out: W,
    // The button seat (1-based) and roster for the current hand, kept for the
    // SUMMARY's position tags. (Hand number and blinds are written from the
    // `HandStarted` event directly and not retained.)
    button_seat: usize,
    seats: Vec<SeatInfo>,
    // The community cards dealt so far this hand.
    board: Vec<Card>,
    // Per-seat tracking (keyed by player name) for the SUMMARY section.
    fold_street: HashMap<String, Street>,
    shown: HashMap<String, (Vec<Card>, ComparableHand)>,
    collected: HashMap<String, u32>,
    /// Awarded chips per pot, in award order, for the SUMMARY pot breakdown.
    awards: Vec<(Option<PotKind>, u32)>,
}

impl HandHistory<Stdout> {
    /// A renderer that writes the hand history to stdout.
    pub fn stdout() -> Self {
        Self::new(io::stdout())
    }
}

impl<W: Write> HandHistory<W> {
    /// A renderer that writes the hand history to `out`.
    pub fn new(out: W) -> Self {
        Self {
            out,
            button_seat: 0,
            seats: Vec::new(),
            board: Vec::new(),
            fold_street: HashMap::new(),
            shown: HashMap::new(),
            collected: HashMap::new(),
            awards: Vec::new(),
        }
    }
}

impl<W: Write> GameObserver for HandHistory<W> {
    fn notify(&mut self, event: &GameEvent) {
        match event {
            GameEvent::HandStarted {
                hand_number,
                button_seat,
                small_blind,
                big_blind,
                seats,
            } => self.start_hand(*hand_number, *button_seat, *small_blind, *big_blind, seats),

            GameEvent::BlindPosted {
                player,
                blind,
                amount,
                all_in,
            } => {
                let kind = match blind {
                    Blind::Small => "small",
                    Blind::Big => "big",
                };
                self.line(&format!(
                    "{player}: posts {kind} blind {amount}{}",
                    all_in_suffix(*all_in)
                ));
            }

            GameEvent::HoleCardsDealt { hero } => {
                self.line("*** HOLE CARDS ***");
                if let Some((name, cards)) = hero {
                    self.line(&format!("Dealt to {name} [{}]", cards_to_string(cards)));
                }
            }

            GameEvent::ActionTaken {
                player,
                street,
                action,
            } => self.action(player, *street, action),

            GameEvent::StreetDealt { street, board, .. } => {
                self.board = board.clone();
                self.line(&street_marker(*street, board));
            }

            GameEvent::UncalledBetReturned { player, amount } => {
                self.line(&format!("Uncalled bet ({amount}) returned to {player}"));
            }

            GameEvent::Showdown { board, .. } => {
                self.board = board.clone();
                self.line("*** SHOW DOWN ***");
            }

            GameEvent::ShowdownReveal { player, hole, hand } => {
                self.shown.insert(player.clone(), (hole.clone(), *hand));
                self.line(&format!(
                    "{player}: shows [{}] ({})",
                    cards_to_string(hole),
                    hand.describe()
                ));
            }

            GameEvent::PotAwarded {
                player,
                amount,
                pot,
                ..
            } => {
                *self.collected.entry(player.clone()).or_insert(0) += amount;
                self.awards.push((*pot, *amount));
                self.line(&format!(
                    "{player} collected {amount} from {}",
                    pot_name(*pot)
                ));
            }

            GameEvent::HandComplete => self.summary(),
        }
    }
}

impl<W: Write> HandHistory<W> {
    /// Writes one history line and flushes (so a redirected stdout file stays
    /// current and correctly ordered).
    fn line(&mut self, text: &str) {
        let _ = writeln!(self.out, "{text}");
        let _ = self.out.flush();
    }

    fn start_hand(
        &mut self,
        hand_number: u32,
        button_seat: usize,
        small_blind: u32,
        big_blind: u32,
        seats: &[SeatInfo],
    ) {
        self.button_seat = button_seat;
        self.seats = seats.to_vec();
        self.board.clear();
        self.fold_street.clear();
        self.shown.clear();
        self.collected.clear();
        self.awards.clear();

        let timestamp = Local::now().format("%Y/%m/%d %H:%M:%S");
        self.line(&format!(
            "{SITE} Hand #{hand_number}: Hold'em No Limit ({small_blind}/{big_blind}) - {timestamp} {TIMEZONE}"
        ));
        self.line(&format!(
            "Table '{TABLE}' {}-max Seat #{button_seat} is the button",
            seats.len()
        ));
        for seat in seats {
            self.line(&format!(
                "Seat {}: {} ({} in chips)",
                seat.seat_no, seat.name, seat.stack
            ));
        }
    }

    fn action(&mut self, player: &str, street: Street, action: &ActionView) {
        let text = match action {
            ActionView::Folded => {
                self.fold_street.insert(player.to_string(), street);
                format!("{player}: folds")
            }
            ActionView::Checked => format!("{player}: checks"),
            ActionView::Called { amount, all_in } => {
                format!("{player}: calls {amount}{}", all_in_suffix(*all_in))
            }
            ActionView::Bet { amount, all_in } => {
                format!("{player}: bets {amount}{}", all_in_suffix(*all_in))
            }
            ActionView::Raised { by, to, all_in } => {
                format!("{player}: raises {by} to {to}{}", all_in_suffix(*all_in))
            }
        };
        self.line(&text);
    }

    fn summary(&mut self) {
        self.line("*** SUMMARY ***");
        self.line(&self.total_pot_line());
        if !self.board.is_empty() {
            self.line(&format!("Board [{}]", cards_to_string(&self.board)));
        }
        let seats = self.seats.clone();
        for seat in &seats {
            let line = format!(
                "Seat {}: {}{}",
                seat.seat_no,
                seat.name,
                self.seat_tag(seat)
            );
            self.line(&line);
        }
    }

    /// The `Total pot …` line, with a `Main pot/Side pot` breakdown when side pots
    /// were formed.
    fn total_pot_line(&self) -> String {
        let total: u32 = self.awards.iter().map(|(_, amount)| amount).sum();
        let has_side_pots = self
            .awards
            .iter()
            .any(|(pot, _)| !matches!(pot, None | Some(PotKind::Main)));
        if !has_side_pots {
            return format!("Total pot {total} | Rake 0");
        }
        let main: u32 = self
            .awards
            .iter()
            .filter(|(pot, _)| matches!(pot, Some(PotKind::Main)))
            .map(|(_, amount)| amount)
            .sum();
        let mut breakdown = format!("Main pot {main}.");
        let mut sides: Vec<(u8, u32)> = Vec::new();
        for (pot, amount) in &self.awards {
            if let Some(PotKind::Side(n)) = pot {
                if let Some(entry) = sides.iter_mut().find(|(idx, _)| idx == n) {
                    entry.1 += amount;
                } else {
                    sides.push((*n, *amount));
                }
            }
        }
        sides.sort_by_key(|(n, _)| *n);
        let single_side = sides.len() == 1;
        for (n, amount) in sides {
            if single_side {
                breakdown.push_str(&format!(" Side pot {amount}."));
            } else {
                breakdown.push_str(&format!(" Side pot-{n} {amount}."));
            }
        }
        format!("Total pot {total} {breakdown} | Rake 0")
    }

    /// The position tag and result clause for a seat's SUMMARY line.
    fn seat_tag(&self, seat: &SeatInfo) -> String {
        let mut tag = String::new();
        if let Some(position) = self.position(seat.seat_no) {
            tag.push_str(&format!(" ({position})"));
        }
        let won = self.collected.get(&seat.name).copied().unwrap_or(0);
        if let Some((hole, hand)) = self.shown.get(&seat.name) {
            let cards = cards_to_string(hole);
            if won > 0 {
                tag.push_str(&format!(
                    " showed [{cards}] and won ({won}) with {}",
                    hand.describe()
                ));
            } else {
                tag.push_str(&format!(
                    " showed [{cards}] and lost with {}",
                    hand.describe()
                ));
            }
        } else if won > 0 {
            tag.push_str(&format!(" collected ({won})"));
        } else if let Some(street) = self.fold_street.get(&seat.name) {
            tag.push_str(&format!(" folded {}", fold_phrase(*street)));
        }
        tag
    }

    /// The PokerStars position label for a 1-based seat number, if it is the
    /// button, small blind, or big blind.
    fn position(&self, seat_no: usize) -> Option<&'static str> {
        let len = self.seats.len();
        if len == 0 {
            return None;
        }
        let button = self.button_seat;
        let (small, big) = if len == 2 {
            // Heads-up: the button is the small blind.
            (button, wrap_seat(button, 1, len))
        } else {
            (wrap_seat(button, 1, len), wrap_seat(button, 2, len))
        };
        if seat_no == button {
            Some("button")
        } else if seat_no == small {
            Some("small blind")
        } else if seat_no == big {
            Some("big blind")
        } else {
            None
        }
    }
}

/// `" and is all-in"` when the action committed the player's whole stack.
fn all_in_suffix(all_in: bool) -> &'static str {
    if all_in {
        " and is all-in"
    } else {
        ""
    }
}

/// Maps a pot to its PokerStars `collected from …` phrase.
fn pot_name(pot: Option<PotKind>) -> String {
    match pot {
        None => "pot".to_string(),
        Some(PotKind::Main) => "main pot".to_string(),
        Some(PotKind::Side(1)) => "side pot".to_string(),
        Some(PotKind::Side(n)) => format!("side pot-{n}"),
    }
}

/// The PokerStars summary phrase for the street a player folded on.
fn fold_phrase(street: Street) -> &'static str {
    match street {
        Street::Preflop => "before Flop",
        Street::Flop => "on the Flop",
        Street::Turn => "on the Turn",
        Street::River => "on the River",
    }
}

/// The PokerStars street marker, splitting the cumulative board so the turn and
/// river cards appear in their own brackets.
fn street_marker(street: Street, board: &[Card]) -> String {
    match street {
        Street::Preflop => String::new(), // never emitted for pre-flop
        Street::Flop => format!("*** FLOP *** [{}]", cards_to_string(&board[..3])),
        Street::Turn => format!(
            "*** TURN *** [{}] [{}]",
            cards_to_string(&board[..3]),
            cards_to_string(&board[3..4])
        ),
        Street::River => format!(
            "*** RIVER *** [{}] [{}]",
            cards_to_string(&board[..4]),
            cards_to_string(&board[4..5])
        ),
    }
}

/// The 1-based seat `offset` positions clockwise from `from` (1-based) at a table
/// of `len` seats.
fn wrap_seat(from: usize, offset: usize, len: usize) -> usize {
    ((from - 1 + offset) % len) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    use casino_poker::casino_cards::card::{set_glyph_display, Card, Rank, Suit};
    use casino_poker::hand_rankings::evaluate;

    fn c(rank: Rank, suit: Suit) -> Card {
        Card::new(rank, suit)
    }

    fn seat(seat_no: usize, name: &str, stack: u32) -> SeatInfo {
        SeatInfo {
            seat_no,
            name: name.to_string(),
            stack,
        }
    }

    /// Drives a scripted heads-up hand to showdown and asserts the emitted
    /// PokerStars lines, including the `raises by to to` form and a SUMMARY.
    #[test]
    fn renders_a_pokerstars_hand_history() {
        set_glyph_display(false); // parseable card codes
        let board = [
            c(Rank::Five, Suit::Diamond),
            c(Rank::Nine, Suit::Club),
            c(Rank::King, Suit::Heart),
            c(Rank::Two, Suit::Spade),
            c(Rank::Seven, Suit::Club),
        ];
        let hero_hand = evaluate(
            &[c(Rank::Ace, Suit::Heart), c(Rank::King, Suit::Club)],
            &board,
        );
        let villain_hand = evaluate(
            &[c(Rank::Queen, Suit::Diamond), c(Rank::Jack, Suit::Diamond)],
            &board,
        );

        let mut hh = HandHistory::new(Vec::new());
        let events = [
            GameEvent::HandStarted {
                hand_number: 1,
                button_seat: 1,
                small_blind: 1,
                big_blind: 2,
                seats: vec![seat(1, "Hero", 200), seat(2, "Villain", 200)],
            },
            // Heads-up: the button posts the small blind.
            GameEvent::BlindPosted {
                player: "Hero".to_string(),
                blind: Blind::Small,
                amount: 1,
                all_in: false,
            },
            GameEvent::BlindPosted {
                player: "Villain".to_string(),
                blind: Blind::Big,
                amount: 2,
                all_in: false,
            },
            GameEvent::HoleCardsDealt {
                hero: Some((
                    "Hero".to_string(),
                    vec![c(Rank::Ace, Suit::Heart), c(Rank::King, Suit::Club)],
                )),
            },
            GameEvent::ActionTaken {
                player: "Hero".to_string(),
                street: Street::Preflop,
                action: ActionView::Raised {
                    by: 4,
                    to: 6,
                    all_in: false,
                },
            },
            GameEvent::ActionTaken {
                player: "Villain".to_string(),
                street: Street::Preflop,
                action: ActionView::Called {
                    amount: 4,
                    all_in: false,
                },
            },
            GameEvent::StreetDealt {
                street: Street::Flop,
                board: board[..3].to_vec(),
                pot: 12,
            },
            GameEvent::ActionTaken {
                player: "Villain".to_string(),
                street: Street::Flop,
                action: ActionView::Checked,
            },
            GameEvent::ActionTaken {
                player: "Hero".to_string(),
                street: Street::Flop,
                action: ActionView::Bet {
                    amount: 6,
                    all_in: false,
                },
            },
            GameEvent::ActionTaken {
                player: "Villain".to_string(),
                street: Street::Flop,
                action: ActionView::Folded,
            },
            GameEvent::UncalledBetReturned {
                player: "Hero".to_string(),
                amount: 6,
            },
            GameEvent::PotAwarded {
                player: "Hero".to_string(),
                amount: 12,
                hand: Some(hero_hand),
                pot: None,
            },
            GameEvent::HandComplete,
        ];
        let _ = villain_hand; // villain folded; not shown
        for event in &events {
            hh.notify(event);
        }

        let output = String::from_utf8(hh.out).unwrap();
        assert!(
            output.starts_with("PokerStars Hand #1: Hold'em No Limit (1/2) - "),
            "header:\n{output}"
        );
        for expected in [
            "Table 'Casino' 2-max Seat #1 is the button",
            "Seat 1: Hero (200 in chips)",
            "Hero: posts small blind 1",
            "Villain: posts big blind 2",
            "*** HOLE CARDS ***",
            "Dealt to Hero [Ah Kc]",
            "Hero: raises 4 to 6",
            "Villain: calls 4",
            "*** FLOP *** [5d 9c Kh]",
            "Hero: bets 6",
            "Villain: folds",
            "Uncalled bet (6) returned to Hero",
            "Hero collected 12 from pot",
            "*** SUMMARY ***",
            "Total pot 12 | Rake 0",
            "Board [5d 9c Kh]",
            "Seat 1: Hero (button) collected (12)",
            "Seat 2: Villain (big blind) folded on the Flop",
        ] {
            assert!(
                output.contains(expected),
                "missing {expected:?} in:\n{output}"
            );
        }
    }

    /// A showdown reveal names the hand PokerStars-style and the winner's summary
    /// line reports it.
    #[test]
    fn showdown_reveal_and_summary_name_the_hand() {
        set_glyph_display(false);
        let board = [
            c(Rank::Five, Suit::Diamond),
            c(Rank::Five, Suit::Club),
            c(Rank::King, Suit::Heart),
            c(Rank::Two, Suit::Spade),
            c(Rank::Seven, Suit::Club),
        ];
        let hand = evaluate(
            &[c(Rank::Ace, Suit::Heart), c(Rank::King, Suit::Club)],
            &board,
        );

        let mut hh = HandHistory::new(Vec::new());
        for event in [
            GameEvent::HandStarted {
                hand_number: 4,
                button_seat: 2,
                small_blind: 1,
                big_blind: 2,
                seats: vec![seat(1, "Hero", 200), seat(2, "Villain", 200)],
            },
            GameEvent::Showdown {
                board: board.to_vec(),
                pot: 40,
            },
            GameEvent::ShowdownReveal {
                player: "Hero".to_string(),
                hole: vec![c(Rank::Ace, Suit::Heart), c(Rank::King, Suit::Club)],
                hand,
            },
            GameEvent::PotAwarded {
                player: "Hero".to_string(),
                amount: 40,
                hand: Some(hand),
                pot: None,
            },
            GameEvent::HandComplete,
        ] {
            hh.notify(&event);
        }

        let output = String::from_utf8(hh.out).unwrap();
        assert!(
            output.contains("Hero: shows [Ah Kc] (two pair, Kings and Fives)"),
            "reveal in:\n{output}"
        );
        assert!(
            output.contains(
                "Seat 1: Hero (big blind) showed [Ah Kc] and won (40) with two pair, Kings and Fives"
            ),
            "summary in:\n{output}"
        );
    }
}
