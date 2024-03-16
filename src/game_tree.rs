use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use rand::prelude::*;
use crate::game_tree::NodeTransition::Undefined;
use crate::prelude::Game;

pub type NodeId = usize;
pub type ActionId = usize;
pub type StateId = usize;  // TODO: IDK if this makes sense
pub type Outcome = f32;
pub type CounterFactuals = Vec<Outcome>;  // is 2p0s this can be stored as a single array
pub type Probability = f32;
pub type Range = Vec<(StateId, Probability)>;
pub type Belief<G: Game, const A: usize, const S: usize> = (G::PublicInformation, [Range; 2]);
pub type ActionPolicy<const A: usize> = [Probability; A];
pub type ReplayBuffer<G: Game, const A: usize, const S: usize> = Arc<Mutex<Vec<(Belief<G,A,S>, Outcome, ActionPolicy<A>)>>>;  // distribution of information states for each player
pub type PublicInformation<G: Game> = G::PublicInformation;  // public and private information knowledge of player

fn sample_policy<const A:usize>(policy: ActionPolicy<A>) -> ActionId {
    let mut rng = thread_rng();
    let mut sum = 0.0;
    let mut action = 0;
    let random_number: f32 = rng.gen_range(0.0..1.0);
    for (i, p) in policy.iter().enumerate() {
        sum += p;
        if sum > random_number {
            action = *i;
            break;
        }
    }
    action
}
pub enum NodeTransition<'a, G: Game> {
    Edge(NodeId),  // TODO: should this be the node and the stateID
    Terminal(Outcome),
    Undefined
    // Chance(Vec<Probability, NodeId>)
}


pub trait Node<'a, G: Game> {  // TODO: figure out how to handle chance nodes
    // Can you tie this to a lifetime and use a reference instead of NodeId?
    fn new(public_information: G::PublicInformation, transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<'a, G>>) -> Self;
    fn empty(public_information: G::PublicInformation) -> Self { Self::new(public_information, HashMap::new()) }
    fn leaf(&self) -> bool;
    fn player(&self) -> G::PlayerId { self.public_state().player() }
    fn public_state(&self) -> &G::PublicInformation;
    fn transition(&self, state_1: StateId, state_2: StateId, action_id: ActionId) -> (StateId, &NodeTransition<'a, G>);
    // open a new node.
    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G, next_node_id: NodeId) -> Option<Self>;
}

struct TransitionMatrix<'a, G: Game, const A: usize, const S: usize> {  // Decision and chance nodes
    public_state: PublicInformation<G>,      // public information state // player to move (can be chance player)
    transition_map: [[[NodeTransition<'a, G>; A]; S]; S],   // payoff matrix
}

struct TransitionMap<'a, G: Game> {
    public_state: PublicInformation<G>,
    transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<'a, G>>
}

impl<'a, G: Game, const A: usize, const S: usize> Node<'a, G> for TransitionMatrix<'a, G, A, S> {
    fn new(public_information: G::PublicInformation, transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<'a, G>>) -> Self {
        let mut map = [[[Undefined; A]; S]; S];
        for ((s1, s2, a), t) in transition_map {
            map[s1][s2][a] = t;
        }

        Self {
            public_state: public_information,
            transition_map: map,
        }

    }
    #[inline]
    fn leaf(&self) -> bool { self.transition_map.iter().flatten().any(|res| res == Undefined) }
    #[inline]
    fn public_state(&self) -> &G::PublicInformation { &self.public_state }
    #[inline]
    fn transition(&self, active_state: StateId, other_state: StateId, action_id: ActionId) -> (StateId, &NodeTransition<G>) { (active_state, &self.transition_map[active_state][other_state][action_id]) }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G, next_node_id: NodeId) -> Option<Self> {
        let (new_id, current_result) = self.transition(active_state, other_state, action_id);
        let public = new_state.public_information();
        for s1 in 0..S {
            for s2 in 0..S {
                for a in 0..A {
                    // TODO: check if the transition is already defined
                }
            }
        }
        match current_result {
            Undefined => {
                let new_node = Self::empty(new_state.public_information());
                let result = NodeTransition::Edge(next_node_id);
                self.transition_map[active_state][other_state][action_id] = result;
                Some(new_node)
            }
            _ => None
        }
    }
}

impl<'a, G: Game> Node<'a, G> for TransitionMap<'a, G> {
    fn new(public_information: G::PublicInformation, transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<'a, G>>) -> Self {
        Self {
            public_state: public_information,
            transition_map
        }
    }

    fn leaf(&self) -> bool {
        todo!()
    }

    fn public_state(&self) -> &G::PublicInformation { &self.public_state }

    fn transition(&self, state_1: StateId, state_2: StateId, action_id: ActionId) -> (StateId, &NodeTransition<'a, G>) {
        return (state_1, self.transition_map.get(&(state_1, state_2, action_id)).unwrap_or(&Undefined))
    }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G, next_node_id: NodeId) -> Option<Self> {

    }
}
