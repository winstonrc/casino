use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Player {
    pub identifier: Uuid,
    pub name: String,
    pub chips: u32,
    pub active: bool,
}

impl Player {
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

    pub fn new_with_chips(name: &str, chips: u32) -> Self {
        let identifier = Uuid::new_v4();

        Self {
            identifier,
            name: name.to_string(),
            chips,
            active: true,
        }
    }

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
