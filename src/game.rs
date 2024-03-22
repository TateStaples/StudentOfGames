use std::hash::Hash;

pub trait HasTurnOrder: Eq + Clone + Copy + std::fmt::Debug {
    fn prev(&self) -> Self;
    fn next(&self) -> Self;
}

pub trait Game: Eq + Hash + Clone + std::fmt::Debug + Send {
    type PlayerId: HasTurnOrder;  // The playerId enum must have .next() and .prev()
    type Action: Eq + Clone + Copy + std::fmt::Debug + Into<usize> + From<usize>;
    type ActionIterator: Iterator<Item = Self::Action>;
    type PublicInformation: PartialEq + Clone + std::fmt::Debug + Send;

    const MAX_TURNS: usize;
    const NAME: &'static str;
    const NUM_PLAYERS: usize;
    const DIMS: &'static [i64];

    fn new() -> Self;  // Default initialization
    fn player(&self) -> Self::PlayerId;  // The active player
    fn is_over(&self) -> bool;
    fn reward(&self, player_id: Self::PlayerId) -> f32;  // The reward for the player getting to active state. Typically terminal nodes
    fn iter_actions(&self) -> Self::ActionIterator;
    fn step(&mut self, action: &Self::Action) -> bool;  // Implement action and return whether done
    fn public_information(&self) -> Self::PublicInformation;  // The state features
    fn sample_state(public_information: Self::PublicInformation) -> Self;  // Sample a state from the public information
    fn print(&self);  // Output the game state
}

pub trait FixedSize<const A: usize, const S: usize> {
    const MAX_NUM_ACTIONS: usize = A;
    const HIDDEN_STATES: usize = S;
}

pub trait FixedGame<const A: usize, const S: usize>: Game + FixedSize<A, S>{}