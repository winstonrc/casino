use uuid::Uuid;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
        self.chips += amount;
    }

    pub fn subtract_chips(&mut self, amount: u32) {
        self.chips -= amount;
    }
}
