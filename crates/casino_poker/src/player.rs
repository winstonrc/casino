//! Player identity and chip-stack types.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A player seated at a table.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Player {
    /// Stable identity used throughout the engine.
    pub identifier: Uuid,
    /// Display name.
    pub name: String,
    /// Chips remaining in the player's stack.
    pub chips: u32,
    /// Whether the player remains active in the surrounding session.
    pub active: bool,
}

/// A stable, lightweight reference to a player: their `Uuid` plus display `name`.
///
/// Carried by [`GameEvent`](crate::events::GameEvent)s and table views so a consumer
/// (renderer, logger, or AI agent) can key off the stable `id` while still having the
/// name for display. Names are *not* guaranteed unique or stable across sessions; the
/// `id` is the reliable key.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct PlayerRef {
    /// Stable player identity.
    pub id: Uuid,
    /// Display name captured when the reference was created.
    pub name: String,
}

/// Renders as the player's name, so hand-history sites can interpolate a `PlayerRef`
/// directly (`{player}`) just as they did a bare name.
impl fmt::Display for PlayerRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl Player {
    /// Creates an active player with an empty chip stack.
    pub fn new(name: &str) -> Self {
        let identifier = Uuid::new_v4();
        let chips: u32 = 0;

        Self {
            identifier,
            name: name.to_string(),
            chips,
            active: true,
        }
    }

    /// Creates an active player with the given chip stack.
    pub fn new_with_chips(name: &str, chips: u32) -> Self {
        let identifier = Uuid::new_v4();

        Self {
            identifier,
            name: name.to_string(),
            chips,
            active: true,
        }
    }

    /// A stable [`PlayerRef`] (id + name) for this player, for stamping onto events
    /// and views.
    pub fn to_ref(&self) -> PlayerRef {
        PlayerRef {
            id: self.identifier,
            name: self.name.clone(),
        }
    }

    /// Adds chips, saturating at [`u32::MAX`].
    pub fn add_chips(&mut self, amount: u32) {
        self.chips = self.chips.saturating_add(amount);
    }

    /// Subtracts chips from the player, saturating at zero.
    ///
    /// Saturating (rather than wrapping/panicking) is important for the all-in
    /// path, where a player commits exactly their remaining stack and any
    /// off-by-one in the betting math must not underflow a `u32`.
    pub fn subtract_chips(&mut self, amount: u32) {
        self.chips = self.chips.saturating_sub(amount);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_references_and_chip_mutators_preserve_contracts() {
        let empty = Player::new("Alice");
        assert_eq!(empty.name, "Alice");
        assert_eq!(empty.chips, 0);
        assert!(empty.active);

        let mut player = Player::new_with_chips("Bob", 10);
        let reference = player.to_ref();
        assert_eq!(reference.id, player.identifier);
        assert_eq!(reference.name, "Bob");
        assert_eq!(reference.to_string(), "Bob");

        player.add_chips(u32::MAX);
        assert_eq!(player.chips, u32::MAX);
        player.subtract_chips(u32::MAX);
        player.subtract_chips(1);
        assert_eq!(player.chips, 0);
    }
}
