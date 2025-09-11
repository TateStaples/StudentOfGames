use crate::game::ImperfectGame;

pub type ActionId = usize;
pub type StateId = usize;  // TODO: idk how to index in generic format
pub type Reward = f32;
pub type Probability = f32;
pub type Belief<G: ImperfectGame, Range> = (G, [Range; 2]);

pub trait HasTurnOrder: Eq + Clone + Copy + std::fmt::Debug {
    fn prev(&self) -> Self;
    fn next(&self) -> Self;
}

pub type PrivateObservation = usize;
pub type PublicObservation = usize;

pub trait AbstractPolicy: Clone {
    fn new() -> Self;
    fn eval(&self, action_id: ActionId) -> Probability;
    fn sample(&self) -> ActionId;
    fn uniform() -> Self;
    fn mix_in(&self, other: &Self, p: Probability) -> Self;
}
pub trait AbstractRange: Clone {
    fn new() -> Self;
    fn eval(&self, state_id: StateId) -> Probability;
    fn mix_in(&self, other: &Self, p: Probability) -> Self;
}

pub trait AbstractCounterfactual: Clone {
    fn new() -> Self;
    fn outcome(value: Reward) -> Self;
    fn eval(&self, state_id: StateId) -> Reward;
}