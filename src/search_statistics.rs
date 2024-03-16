use crate::game::Game;
use crate::game_tree::{ActionId, ActionPolicy, CounterFactuals, Node, NodeTransition, Outcome, Probability, Range, StateId};

// TODO: consider making a torch implementation of this
pub trait SearchStatistics<'a, G> {  // Stores the statistics for the search in CFR
    fn from_prior(prior: ActionPolicy) -> Self;  // TODO: update to action policy for action
    fn reset(&mut self);
    fn action_probability(&self, state_id: StateId, action_id: ActionId) -> Probability;
    fn visits(&self) -> f32;
    fn solved(&self) -> bool;
    fn average_value(&self) -> CounterFactuals;
    fn update_nodes_below(&mut self, state_id: StateId, action_id: ActionId);
    fn update_action_quality(&mut self, state_id: StateId, action_id: ActionId, value: Outcome, p: Probability);
    // fn get_strategy(&self, action_id: ActionId) -> [f32; S];  // Returns hand range for this action -> used for bayesian updating
    fn iter_results(&self, ranges: &[Range; 2]) -> impl Iterator<Item=(NodeTransition<G>, [Range; 2], Vec<(StateId, ActionId, Probability)>)>;  // TODO
}
struct FixedStatistics<'a, N: Node<'a, G>, G: Game, const A: usize, const S: usize, const C: usize> {
    node: N,
    nodes_below: [[f32; A]; S],
    action_quality: [[f32; A]; S],
    visits: f32,
    aggregate_value: CounterFactuals,
}

impl<'a, N: Node<'a, G>, G: Game, const A: usize , const S: usize, const C: usize> SearchStatistics<'a, G> for FixedStatistics<'a, N, G, A, S, C> {
    fn from_prior(prior: ActionPolicy<A>) -> Self {
        todo!()
    }

    fn reset(&mut self) {
        self.nodes_below = [[0.0; A]; S];
        self.action_quality = [[0.0; A]; S];  // TODO: populate quality with average value
        self.aggregate_value = self.average_value();
        self.visits = 1.0;
    }

    fn action_probability(&self, state_id: StateId, action_id: ActionId) -> Probability {
        let action_q = self.action_quality[state_id];
        action_q[action_id] / action_q.iter().sum()
    }

    fn visits(&self) -> f32 {
        self.nodes_below.iter().map(|x| x.iter().sum()).sum()
    }

    fn solved(&self) -> bool {  // Solved only applies to perfect information games -> implement later
        false
    }

    fn average_value(&self) -> CounterFactuals {  //
        self.aggregate_value.iter().map(|x| x/self.visits).collect()
    }

    fn update_nodes_below(&mut self, state_id: StateId, action_id: ActionId) {
        self.nodes_below[state_id][action_id] += 1.0;
    }

    fn update_action_quality(&mut self, state_id: StateId, action_id: ActionId, value: Outcome, p: Probability) {
        // r(s,a) = v(s,a) - EV(policy)
        // Q(s,a) += r(s,a) [min value of 0]
        let state = &mut self.action_quality[state_id];
        for a in 0..A {
            if a == action_id {
                state[a] += value;  // update my quality
            } else {
                state[a] -= value * p;  // decrement other quality
            }
        }
    }

    fn iter_results(&self, ranges: [Range; 2]) -> &dyn Iterator<Item=(NodeTransition<G>, [Range; 2], Vec<(StateId, StateId, ActionId, Probability)>)> {
        todo!()
    }
}

pub trait ImperfectNode<'a, G: Game>: Node<'a, G> + SearchStatistics<'a, G>{}