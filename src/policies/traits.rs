use crate::game::Game;
use tch::{nn::VarStore, Tensor};

pub trait Policy<G: Game<N>, const N: usize> {  // CVPN: Counterfactual Value Policy Network
    fn eval(&mut self, belief: Belief) -> (f32, [f32; N]);  // TODO: change to input type Belief

    // TODO: setup for training as well, required for alpha zero and SoG training
}

pub trait NNPolicy<G: Game<N>, const N: usize> {  // TODO: connect this more directly to tch
    fn new(vs: &VarStore) -> Self;
    fn forward(&self, xs: &Tensor) -> (Tensor, Tensor);
}
