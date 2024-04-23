use uuid::Uuid;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Player {
    pub identifier: Uuid,
    pub name: String,
    pub chips: u32,
}

impl Player {
    pub fn new(name: &str) -> Self {
        let identifier = Uuid::new_v4();
        let chips: u32 = 0;

        Self {
            identifier,
            name: name.to_string(),
            chips,
        }
    }

    pub fn new_with_chips(name: &str, chips: u32) -> Self {
        let identifier = Uuid::new_v4();

        Self {
            identifier,
            name: name.to_string(),
            chips,
        }
    }

    pub fn update_chips(&mut self, chips: u32) {
        self.chips += chips;
    }
}
