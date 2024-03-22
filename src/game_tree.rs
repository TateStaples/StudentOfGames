use std::collections::{HashMap, HashSet};
use crate::game_tree::NodeTransition::Undefined;
use crate::helpers::prelude::Game;

pub type NodeId = usize;
pub type ActionId = usize;
pub type StateId = usize;  // TODO: idk how to index in generic format
pub type Outcome = f32;
pub type Counterfactuals = Vec<Outcome>;  // is 2p0s this can be stored as a single array
pub type PublicInformation<G: Game> = G::PublicInformation;  // public and private information knowledge of player

pub(crate) struct GameTree<'a, G: Game, N: Node<'a, G>> {
    nodes: Vec<N>,
    root: NodeId,
}

impl <'a, G: Game, N: Node<'a, G>> GameTree<'a, G, N> {
    pub(crate) fn with_capacity(capacity: usize, root: N) -> Self {
        let mut new = Self {
            nodes: Vec::with_capacity(capacity),
            root: 0,
        };
        new.nodes.push(root);
        new
    }
    pub(crate) fn push(&mut self, node: N) -> NodeId {
        let node_id = self.nodes.len();
        self.nodes.push(node);
        node_id
    }
    pub(crate) fn node(&self, node_id: NodeId) -> &N { self.nodes.get(node_id).expect("Node not found") }
    pub(crate) fn mut_node(&mut self, node_id: NodeId) -> &mut N { self.nodes.get_mut(node_id).expect("Node not found") }
    pub(crate) fn root(&self) -> &N { self.node(self.root) }
    pub(crate) fn root_id(&mut self) -> NodeId { self.root }
    pub(crate) fn reroot(&mut self, new_root: NodeId) { self.root = new_root } // maybe we can clear space -> array of options
}

pub enum NodeTransition<'a, G: Game> {
    Edge(NodeId),  // TODO: should this be the node and the stateID
    Terminal(Outcome),
    Undefined
    // Chance(Vec<Probability, NodeId>)
}

pub trait Node<'a, G: Game> {
    fn new(public_information: G::PublicInformation, transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<'a, G>>) -> Self;
    fn empty(public_information: G::PublicInformation) -> Box<Self> { Self::new(public_information, HashMap::new()) }
    fn leaf(&self) -> bool;
    fn player(&self) -> G::PlayerId { self.public_state().player() }
    fn public_state(&self) -> &G::PublicInformation;
    fn transition(&self, state_1: StateId, state_2: StateId, action_id: ActionId) -> (StateId, &NodeTransition<'a, G>);
    fn children(&self) -> HashSet<NodeTransition<G>>;
    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G, next_node_id: NodeId) -> Option<Self>;
}

pub struct TransitionMatrix<'a, G: Game, const A: usize, const S: usize> {  // Decision and chance nodes
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

    fn children(&self) -> HashSet<NodeTransition<G>> {
        self.transition_map.iter().flatten().map(|res| *res).collect()
    }

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
                Some(*new_node)
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
        self.children().iter().any(|x| x == Undefined)
    }

    fn public_state(&self) -> &G::PublicInformation { &self.public_state }

    fn transition(&self, state_1: StateId, state_2: StateId, action_id: ActionId) -> (StateId, &NodeTransition<'a, G>) {
        return (state_1, self.transition_map.get(&(state_1, state_2, action_id)).unwrap_or(&Undefined))
    }

    fn children(&self) -> HashSet<NodeTransition<G>> {
        todo!()
    }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G, next_node_id: NodeId) -> Option<Self> {
        todo!()
    }
}
