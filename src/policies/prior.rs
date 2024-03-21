use crate::game::Game;
use tch::{nn::VarStore, Tensor};
use crate::game_tree::Outcome;
use crate::search_statistics::Range;

pub trait Policy<G: Game, const A: usize, const S: usize> {  // CVPN: Counterfactual Value Policy Network
    fn eval(&mut self, public_info: G::PublicInformation, ranges: [Range; 2]) -> ([[Outcome; A]; S], [[f32; A]; S]);

    // TODO: setup for training as well, required for alpha zero and SoG training
}

pub trait NNPolicy<G: Game, const N: usize> {  // TODO: connect this more directly to tch
    fn new(vs: &VarStore) -> Self;
    fn forward(&self, xs: &Tensor) -> (Tensor, Tensor);
}
