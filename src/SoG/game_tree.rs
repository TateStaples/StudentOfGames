use crate::game::Game;
use crate::types::{Reward, PrivateObservation, PublicObservation, AbstractRange, ActionId};

// Types: History (Private Node), Private Infostate, Public Belief State (Public Node)
// History: Transition, Game, 
// Private InfoState: Policy, Search Statistics (visits, rewards, regrets, reach prob)
// Public Belief State: Range, Counterfactuals
pub type PrivateNodeId = usize;
pub type PublicNodeId = usize;
pub type InfoStateId = usize;
pub enum NodeType {
    Inner,
    Leaf,
    Terminal
}
pub trait PublicNode<'a, Node: PrivateNode<'a, G>, G: Game, Range: AbstractRange> {
    fn sample(&self) -> Node;
    fn location(&self) -> NodeType;
    fn transition(&self, public_observation: PublicObservation) -> Option<PrivateNodeId>;  // TODO: swap this with public to public transition
    fn range(&self, player: G::PlayerId) -> Range;
    fn iter_results(&self, ranges: &[Range; 2]) -> impl Iterator<Item=(Option<&Self>, [Range; 2])>;
    fn histories(&self) -> Vec<Node>;
    fn expand(&mut self) -> Vec<Self>;
}
// Node Store: priv / pub update (for reconstruction), transition: priv -> self, reward
pub trait PrivateNode<'a, G: Game> : Sized {  // A node in a game tree
    // fn new(public_information: G::PublicInformation, transition_map: HashMap<(StateId, StateId, ActionId), NodeTransition<'a, G, Self>>) -> Self;
    fn empty() -> Self;  // todo: does this make sense? could be useful for prior eval
    fn reward(&self) -> Reward;
    fn location(&self) -> NodeType;
    fn player(&self) -> G::PlayerId {   // The acting player for the given state
        // self.public_state().player() 
        todo!()
    }
    fn actions(&self) -> G::ActionIterator;  // The possible actions for the given state
    fn game(&self) -> G;  // The game state
    fn transition(&self, update: PrivateObservation) -> Option<PrivateNodeId>;  // get the result fo 
    fn step(&self, action: ActionId) -> Option<PrivateNodeId> {
        self.children().get(action).map(|x| *x)
    }
    fn children(&self) -> [PrivateNodeId];     // indexed on ActionId
    fn public_state(&self) -> PublicNodeId;
    fn state_id(&self) -> InfoStateId;
    fn expand(&mut self) -> Vec<(Self, PublicObservation)>;
}

/*
pub struct TransitionMatrix<'a, G: Game, const A: usize, const S: usize> {  // Decision and chance nodes
    transition_map: [[[Option<NodeTransition<'a, G, Self>>; A]; S]; S],   // payoff matrix
}

impl<'a, G: Game, const A: usize, const S: usize> 
    Node<'a, G> for TransitionMatrix<'a, G, A, S> {

    #[inline]
    fn leaf(&self) -> bool { 
        // self.transition_map.iter().flatten().any(|res| res == Undefined) 
        todo!()
    }

    #[inline]
    fn transition(&self, active_state: StateId, other_state: StateId, action_id: ActionId) -> (StateId, &Option<NodeTransition<'a, G, Self>>) {
        // (active_state, &self.transition_map[active_state][other_state][action_id])
        todo!()
    }

    fn children(&self) -> HashSet<&NodeTransition<'a, G, Self>> {
        // self.transition_map.iter().flatten().map(|res| *res).collect()
        todo!()
    }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G) -> Option<Self> {
        // let (new_id, current_result) = self.transition(active_state, other_state, action_id);
        // let public = new_state.public_information();
        // for transition in self.children() {
        //     if let NodeTransition::Edge(&next) = transition {
        //         self.transition_map[active_state][other_state][action_id] = NodeTransition::Edge(&next);
        //         return None;
        //     }
        // }
        // let new_node = *Self::empty(new_state.public_information());
        // let result = NodeTransition::Edge(&new_node);
        // self.transition_map[active_state][other_state][action_id] = result;
        // Some(new_node)
        todo!()
    }
}


// struct TransitionMap<'a, G: Game> {
//     transition_map: HashMap<ActionId, NodeTransition<'a, G, Self>>
// }

impl<'a, G: Game> Node<'a, G> for 
    TransitionMap<'a, G> {

    fn leaf(&self) -> bool {
        // self.children().iter().any(|x| x == Undefined)
        todo!()
    }
    
    fn transition(&self, state_1: StateId, state_2: StateId, action_id: ActionId) -> (StateId, &NodeTransition<'a, G, Self>) {
        return (state_1, self.transition_map.get(&(state_1, state_2, action_id)).unwrap_or(&Undefined))
    }

    fn children(&self) -> HashSet<&NodeTransition<'a, G, Self>> {
        todo!()
    }

    fn visit(&mut self, active_state: StateId, other_state: StateId, action_id: ActionId, new_state: G) -> Option<Self> {
        todo!()
    }
}
*/