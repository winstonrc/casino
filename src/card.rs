use std::cmp::Ordering;
use std::fmt;

use strum::EnumIter;

/// Creates a new card.
///
/// Requires the Rank and Suit to be provided as an ident.
#[macro_export]
macro_rules! card {
    ($rank:ident, $suit:ident) => {
        Card {
            rank: Rank::$rank,
            suit: Suit::$suit,
        }
    };
}

/// Creates a new card.
///
/// Requires the Rank and Suit to be provided as an expr.
#[macro_export]
macro_rules! card_from_expr {
    ($rank:expr, $suit:expr) => {
        Card {
            rank: $rank,
            suit: $suit,
        }
    };
}

/// Numerical values for the ranks.
///
/// Ranks contain Two through Ace (high by default).
#[derive(Clone, Copy, Debug, EnumIter, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
}

impl From<Rank> for char {
    fn from(value: Rank) -> Self {
        match value {
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

/// A suit can be a Club, Diamond, Heart, or Spade, which are ranked from lowest to highest value.
///
/// Suit values are based on the values for the game Bridge.
#[derive(Clone, Copy, Debug, EnumIter, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
}

impl From<Suit> for char {
    fn from(suit: Suit) -> Self {
        match suit {
            Suit::Club => '♣',
            Suit::Diamond => '♦',
            Suit::Heart => '♥',
            Suit::Spade => '♠',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

impl Card {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Self { rank, suit }
    }

    pub fn to_symbol(self) -> char {
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
        let rank_ordering = self.rank.partial_cmp(&other.rank);
        match rank_ordering {
            Some(Ordering::Equal) => self.suit.partial_cmp(&other.suit),
            _ => rank_ordering,
        }
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", char::from(self.rank), char::from(self.suit))
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
    fn cards_have_correct_string_values() {
        let two_of_clubs_card = card!(Two, Club);
        assert_eq!(two_of_clubs_card.to_string(), "2♣");
        assert_eq!(two_of_clubs_card.to_symbol(), '🃒');

        let seven_of_diamonds_card = card!(Seven, Diamond);
        assert_eq!(seven_of_diamonds_card.to_string(), "7♦");
        assert_eq!(seven_of_diamonds_card.to_symbol(), '🃇');

        let king_of_hearts_card = card!(King, Heart);
        assert_eq!(king_of_hearts_card.to_string(), "K♥");
        assert_eq!(king_of_hearts_card.to_symbol(), '🂾');

        let ace_of_spades_card = card!(Ace, Spade);
        assert_eq!(ace_of_spades_card.to_string(), "A♠");
        assert_eq!(ace_of_spades_card.to_symbol(), '🂡');
    }
}
