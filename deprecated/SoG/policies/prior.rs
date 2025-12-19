use tch::{nn::VarStore, Tensor};
use crate::fresh::{FixedGame, BeliefState, FixedCounterfactuals, FixedStrategy};

pub trait Prior<G: FixedGame<A, S>, const A: usize, const S: usize> {  // CVPN: Counterfactual Value Policy Network (intuition)
    fn eval(&self, belief: &BeliefState<G, A, S>) -> ([FixedCounterfactuals<S>; 2], [FixedStrategy<A>; S]);  // To generalize use generics and support reward each player separately
    fn learn(&mut self); // todo: determine inputs

    // TODO: setup for training as well, required for alpha zero and SoG training
}

pub trait NNPolicy<G: FixedGame<A, S>, const A: usize, const S: usize> {  // TODO: connect this more directly to tch
    fn new(vs: &VarStore) -> Self;
    fn forward(&self, xs: &Tensor) -> (Tensor, Tensor);
}
