use std::hash::Hash;
use crate::types::{HasTurnOrder, PrivateObservation, PublicObservation};


pub trait Game: Eq + Hash + Clone + std::fmt::Debug + Send {
    type PlayerId: HasTurnOrder;  // The playerId enum must have .next() and .prev()
    type Action: Eq + Clone + Copy + std::fmt::Debug + Into<usize> + From<usize>;
    type ActionIterator: Iterator<Item = Self::Action>;

    const MAX_TURNS: usize;
    const NUM_PLAYERS: usize;
    
    fn new() -> Self;  // Default initialization
    fn player(&self) -> Self::PlayerId;  // The active player
    fn is_over(&self) -> bool;
    fn reward(&self, player_id: Self::PlayerId) -> f32;  // The reward for the player getting to active state.
    fn iter_actions(&self) -> Self::ActionIterator;
    fn step(&mut self, action: &Self::Action) -> (PublicObservation, PrivateObservation);  // Implement action and return whether done
    fn print(&self);  // Output the game state
}

pub trait FixedGame<const A: usize, const S: usize>: Game {
    const MAX_NUM_ACTIONS: usize = A;
    const HIDDEN_STATES: usize = S;
}


pub trait ImperfectGame: Game {
    fn sample_state(public_information: Vec<PublicObservation>) -> Self;  // Sample a state from the public information
    fn transition(&self, public_observation: PublicObservation) -> &Self;
    fn cfr(&mut self, )
}