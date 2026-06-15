use std::cmp::Ordering;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

use serde::{Deserialize, Serialize};
use strum::EnumIter;

/// Whether `Card`'s `Display` uses the single Unicode playing-card glyphs.
/// Defaults to `false` (portable rank+suit text).
static GLYPH_DISPLAY: AtomicBool = AtomicBool::new(false);

/// Chooses how `Card` renders via its [`Display`](std::fmt::Display) impl:
/// `true` for the single Unicode playing-card glyphs (e.g. `🂡` — pretty but tiny
/// or missing in some terminals), `false` for parseable PokerStars codes (e.g.
/// `As`). Applies process-wide and affects everything that prints a `Card` or
/// `Hand`.
pub fn set_glyph_display(enabled: bool) {
    GLYPH_DISPLAY.store(enabled, AtomicOrdering::Relaxed);
}

/// Returns whether glyph display is currently enabled (see [`set_glyph_display`]).
pub fn glyph_display_enabled() -> bool {
    GLYPH_DISPLAY.load(AtomicOrdering::Relaxed)
}

/// Creates a new card.
///
/// Requires the Rank and Suit to be provided as an ident.
///
/// Sets face_up to true by default.
#[macro_export]
macro_rules! card {
    ($rank:ident, $suit:ident) => {
        Card {
            rank: Rank::$rank,
            suit: Suit::$suit,
            face_up: true,
        }
    };
}

/// Numerical values for the ranks.
///
/// Ranks contain Two through Ace (Ace-high by default).
#[derive(
    Clone, Copy, Debug, Deserialize, EnumIter, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[repr(u8)]
pub enum Rank {
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
    Ace = 14,
}

impl Rank {
    pub fn value(&self) -> u8 {
        *self as u8
    }

    /// The single-character PokerStars rank code: `2`–`9`, `T` (Ten), `J`, `Q`,
    /// `K`, `A`. Used by `Card`'s text `Display` so output is parseable by
    /// standard hand-history tools.
    pub fn code(&self) -> char {
        match self {
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
        }
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rank = match self {
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
        };

        write!(f, "{}", rank)
    }
}

/// A suit can be a Club, Diamond, Heart, or Spade, which are ranked from lowest to highest value.
///
/// Suit values are based on the values for the game Bridge.
#[derive(
    Clone, Copy, Debug, Deserialize, EnumIter, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum Suit {
    Club = 0,
    Diamond = 1,
    Heart = 2,
    Spade = 3,
}

impl Suit {
    pub fn value(&self) -> u8 {
        *self as u8
    }

    /// The single-character PokerStars suit code: `c`, `d`, `h`, `s` (lowercase).
    pub fn code(&self) -> char {
        match self {
            Suit::Club => 'c',
            Suit::Diamond => 'd',
            Suit::Heart => 'h',
            Suit::Spade => 's',
        }
    }
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suit = match self {
            Suit::Club => '♣',
            Suit::Diamond => '♦',
            Suit::Heart => '♥',
            Suit::Spade => '♠',
        };

        write!(f, "{}", suit)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
    pub face_up: bool,
}

impl Card {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Self {
            rank,
            suit,
            face_up: true,
        }
    }

    pub fn value(&self) -> u8 {
        match self.rank {
            Rank::Ace => 1,
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 10,
            Rank::Queen => 10,
            Rank::King => 10,
        }
    }

    /// Returns the single Unicode playing-card glyph for this card (e.g. `🂡`),
    /// or the card-back glyph (`🂠`) if it is face down.
    ///
    /// These glyphs render inconsistently across terminals and fonts — often
    /// tiny or missing — so for portable, parseable output prefer the
    /// [`Display`](std::fmt::Display) form, which is the PokerStars code (e.g.
    /// `As`).
    pub fn glyph(&self) -> char {
        if !self.face_up {
            return '🂠';
        }

        match (self.rank, self.suit) {
            (Rank::Two, Suit::Club) => '🃒',
            (Rank::Three, Suit::Club) => '🃓',
            (Rank::Four, Suit::Club) => '🃔',
            (Rank::Five, Suit::Club) => '🃕',
            (Rank::Six, Suit::Club) => '🃖',
            (Rank::Seven, Suit::Club) => '🃗',
            (Rank::Eight, Suit::Club) => '🃘',
            (Rank::Nine, Suit::Club) => '🃙',
            (Rank::Ten, Suit::Club) => '🃚',
            (Rank::Jack, Suit::Club) => '🃛',
            (Rank::Queen, Suit::Club) => '🃝',
            (Rank::King, Suit::Club) => '🃞',
            (Rank::Ace, Suit::Club) => '🃑',
            (Rank::Two, Suit::Diamond) => '🃂',
            (Rank::Three, Suit::Diamond) => '🃃',
            (Rank::Four, Suit::Diamond) => '🃄',
            (Rank::Five, Suit::Diamond) => '🃅',
            (Rank::Six, Suit::Diamond) => '🃆',
            (Rank::Seven, Suit::Diamond) => '🃇',
            (Rank::Eight, Suit::Diamond) => '🃈',
            (Rank::Nine, Suit::Diamond) => '🃉',
            (Rank::Ten, Suit::Diamond) => '🃊',
            (Rank::Jack, Suit::Diamond) => '🃋',
            (Rank::Queen, Suit::Diamond) => '🃍',
            (Rank::King, Suit::Diamond) => '🃎',
            (Rank::Ace, Suit::Diamond) => '🃁',
            (Rank::Two, Suit::Heart) => '🂲',
            (Rank::Three, Suit::Heart) => '🂳',
            (Rank::Four, Suit::Heart) => '🂴',
            (Rank::Five, Suit::Heart) => '🂵',
            (Rank::Six, Suit::Heart) => '🂶',
            (Rank::Seven, Suit::Heart) => '🂷',
            (Rank::Eight, Suit::Heart) => '🂸',
            (Rank::Nine, Suit::Heart) => '🂹',
            (Rank::Ten, Suit::Heart) => '🂺',
            (Rank::Jack, Suit::Heart) => '🂻',
            (Rank::Queen, Suit::Heart) => '🂽',
            (Rank::King, Suit::Heart) => '🂾',
            (Rank::Ace, Suit::Heart) => '🂱',
            (Rank::Two, Suit::Spade) => '🂢',
            (Rank::Three, Suit::Spade) => '🂣',
            (Rank::Four, Suit::Spade) => '🂤',
            (Rank::Five, Suit::Spade) => '🂥',
            (Rank::Six, Suit::Spade) => '🂦',
            (Rank::Seven, Suit::Spade) => '🂧',
            (Rank::Eight, Suit::Spade) => '🂨',
            (Rank::Nine, Suit::Spade) => '🂩',
            (Rank::Ten, Suit::Spade) => '🂪',
            (Rank::Jack, Suit::Spade) => '🂫',
            (Rank::Queen, Suit::Spade) => '🂭',
            (Rank::King, Suit::Spade) => '🂮',
            (Rank::Ace, Suit::Spade) => '🂡',
        }
    }
}

/// The error returned when a [`Card`] cannot be parsed from its string code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseCardError;

impl fmt::Display for ParseCardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid card code (expected a rank 2-9/T/J/Q/K/A followed by a suit c/d/h/s, e.g. \"As\")"
        )
    }
}

impl std::error::Error for ParseCardError {}

impl std::str::FromStr for Card {
    type Err = ParseCardError;

    /// Parses a PokerStars-style code — the inverse of the text [`Display`] form —
    /// e.g. `"As"`, `"Td"`, `"9h"`. The rank is `2`–`9`/`T`/`J`/`Q`/`K`/`A`
    /// (uppercase) and the suit is `c`/`d`/`h`/`s` (lowercase); the parsed card is
    /// face up. The face-down `"??"` form, and any malformed input, return an error.
    ///
    /// [`Display`]: std::fmt::Display
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let rank_ch = chars.next().ok_or(ParseCardError)?;
        let suit_ch = chars.next().ok_or(ParseCardError)?;
        if chars.next().is_some() {
            return Err(ParseCardError);
        }

        let rank = match rank_ch {
            '2' => Rank::Two,
            '3' => Rank::Three,
            '4' => Rank::Four,
            '5' => Rank::Five,
            '6' => Rank::Six,
            '7' => Rank::Seven,
            '8' => Rank::Eight,
            '9' => Rank::Nine,
            'T' => Rank::Ten,
            'J' => Rank::Jack,
            'Q' => Rank::Queen,
            'K' => Rank::King,
            'A' => Rank::Ace,
            _ => return Err(ParseCardError),
        };
        let suit = match suit_ch {
            'c' => Suit::Club,
            'd' => Suit::Diamond,
            'h' => Suit::Heart,
            's' => Suit::Spade,
            _ => return Err(ParseCardError),
        };

        Ok(Card::new(rank, suit))
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> Ordering {
        let rank_ordering = self.rank.cmp(&other.rank);
        match rank_ordering {
            Ordering::Equal => self.suit.cmp(&other.suit),
            _ => rank_ordering,
        }
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Card {
    /// Renders the card as the PokerStars two-character code — rank code plus a
    /// lowercase suit letter, e.g. `As`, `Td` (a face-down card renders as `??`).
    /// This form is parseable by standard hand-history tools. If glyph display is
    /// enabled via [`set_glyph_display`], renders the single Unicode playing-card
    /// glyph instead.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if glyph_display_enabled() {
            write!(f, "{}", self.glyph())
        } else if self.face_up {
            write!(f, "{}{}", self.rank.code(), self.suit.code())
        } else {
            write!(f, "??")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use strum::IntoEnumIterator;

    #[test]
    fn correct_number_of_ranks() {
        assert_eq!(Rank::iter().len(), 13);
    }

    #[test]
    fn rank_values_are_correct() {
        assert_eq!(Rank::Two.value(), 2);
        assert_eq!(Rank::Three.value(), 3);
        assert_eq!(Rank::Four.value(), 4);
        assert_eq!(Rank::Five.value(), 5);
        assert_eq!(Rank::Six.value(), 6);
        assert_eq!(Rank::Seven.value(), 7);
        assert_eq!(Rank::Eight.value(), 8);
        assert_eq!(Rank::Nine.value(), 9);
        assert_eq!(Rank::Ten.value(), 10);
        assert_eq!(Rank::Jack.value(), 11);
        assert_eq!(Rank::Queen.value(), 12);
        assert_eq!(Rank::King.value(), 13);
        assert_eq!(Rank::Ace.value(), 14);

        assert!(Rank::Two < Rank::Three);
        assert!(Rank::Three < Rank::Four);
        assert!(Rank::Four < Rank::Five);
        assert!(Rank::Five < Rank::Six);
        assert!(Rank::Six < Rank::Seven);
        assert!(Rank::Seven < Rank::Eight);
        assert!(Rank::Eight < Rank::Nine);
        assert!(Rank::Nine < Rank::Ten);
        assert!(Rank::Ten < Rank::Jack);
        assert!(Rank::Jack < Rank::Queen);
        assert!(Rank::Queen < Rank::King);
        assert!(Rank::King < Rank::Ace);
    }

    #[test]
    fn correct_number_of_suits() {
        assert_eq!(Suit::iter().len(), 4);
    }

    #[test]
    fn suit_values_are_correct() {
        assert_eq!(Suit::Club.value(), 0);
        assert_eq!(Suit::Diamond.value(), 1);
        assert_eq!(Suit::Heart.value(), 2);
        assert_eq!(Suit::Spade.value(), 3);

        assert!(Suit::Club < Suit::Diamond);
        assert!(Suit::Diamond < Suit::Heart);
        assert!(Suit::Heart < Suit::Spade);
    }

    #[test]
    fn card_values_are_correct() {
        assert_eq!(card!(Ace, Club).value(), 1);
        assert_eq!(card!(Two, Club).value(), 2);
        assert_eq!(card!(Three, Club).value(), 3);
        assert_eq!(card!(Four, Club).value(), 4);
        assert_eq!(card!(Five, Club).value(), 5);
        assert_eq!(card!(Six, Club).value(), 6);
        assert_eq!(card!(Seven, Club).value(), 7);
        assert_eq!(card!(Eight, Club).value(), 8);
        assert_eq!(card!(Nine, Club).value(), 9);
        assert_eq!(card!(Ten, Club).value(), 10);
        assert_eq!(card!(Jack, Club).value(), 10);
        assert_eq!(card!(Queen, Club).value(), 10);
        assert_eq!(card!(King, Club).value(), 10);
    }

    #[test]
    fn cards_display_as_pokerstars_codes() {
        assert_eq!(card!(Two, Club).to_string(), "2c");
        assert_eq!(card!(Seven, Diamond).to_string(), "7d");
        assert_eq!(card!(Ten, Spade).to_string(), "Ts");
        assert_eq!(card!(King, Heart).to_string(), "Kh");
        assert_eq!(card!(Ace, Spade).to_string(), "As");

        let mut face_down = card!(Ace, Spade);
        face_down.face_up = false;
        assert_eq!(face_down.to_string(), "??");
    }

    #[test]
    fn cards_parse_from_their_codes() {
        use std::str::FromStr;

        assert_eq!(Card::from_str("As").unwrap(), card!(Ace, Spade));
        assert_eq!(Card::from_str("Td").unwrap(), card!(Ten, Diamond));
        assert_eq!(Card::from_str("9h").unwrap(), card!(Nine, Heart));
        assert_eq!(Card::from_str("2c").unwrap(), card!(Two, Club));

        // Round-trips with the text Display form.
        let king = card!(King, Heart);
        assert_eq!(Card::from_str(&king.to_string()).unwrap(), king);

        // Rejects the face-down form, wrong case, and malformed input.
        assert!(Card::from_str("??").is_err());
        assert!(Card::from_str("aS").is_err());
        assert!(Card::from_str("A").is_err());
        assert!(Card::from_str("Ass").is_err());
    }

    #[test]
    fn glyph_returns_unicode_playing_card() {
        assert_eq!(card!(Two, Club).glyph(), '🃒');
        assert_eq!(card!(Ace, Spade).glyph(), '🂡');

        let mut face_down = card!(Ace, Spade);
        face_down.face_up = false;
        assert_eq!(face_down.glyph(), '🂠');
    }
}
