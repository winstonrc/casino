use uuid::Uuid;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Player {
    pub identifier: Uuid,
    pub name: String,
    pub chips: u32,
}
