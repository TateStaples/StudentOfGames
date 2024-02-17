use crate::game::Game;
use tch::{nn::VarStore, Tensor};

pub trait Policy<G: Game<MaxAction>, const MaxAction: usize> {
    fn eval(&mut self, game: &G) -> ([f32; MaxAction], [f32; 3]);
}

pub trait NNPolicy<G: Game<N>, const N: usize> {
    fn new(vs: &VarStore) -> Self;
    fn forward(&self, xs: &Tensor) -> (Tensor, Tensor);
}
