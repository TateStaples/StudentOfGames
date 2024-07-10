use tch::{nn::VarStore, Tensor};

use crate::game::Game;
use crate::types::{AbstractCounterfactual, AbstractPolicy, AbstractRange};

pub trait Prior<GameState, Counterfactuals: AbstractCounterfactual, Range: AbstractRange, Policy: AbstractPolicy> {  // CVPN: Counterfactual Value Policy Network (intuition)
    fn eval(&self, belief: GameState) -> (Counterfactuals, Policy);
    fn learn(&mut self); // todo: determine inputs

    // TODO: setup for training as well, required for alpha zero and SoG training
}

pub trait NNPolicy<G: Game, const N: usize> {  // TODO: connect this more directly to tch
    fn new(vs: &VarStore) -> Self;
    fn forward(&self, xs: &Tensor) -> (Tensor, Tensor);
}
