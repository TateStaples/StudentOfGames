// Growing Tree Counter Factual Regret Minimization
// Based on the DeepStack implementation of CFRd (https://github.dev/lifrordi/DeepStack-Leduc/tree/master/Source - cfrd_gadget.lua, resolving.lua, continual_resolving.lua [compute_action])

use std::marker::PhantomData;
use crate::cfr::cfr;

use crate::game::{Game, ImperfectGame};
use crate::game_tree::{NodeType, PrivateNode, PrivateNodeId, PublicNode, PublicNodeId};
use crate::policies::Prior;
use crate::types::{AbstractCounterfactual, AbstractPolicy, AbstractRange, ActionId, Belief, Reward, PublicObservation, StateId};

// What needs mutable node access: growth, search statistics
// DeepStack: (https://github.dev/lifrordi/DeepStack-Leduc/tree/master/Source - cfrd_gadget.lua, resolving.lua, continual_resolving.lua [compute_action])
pub struct GtCfr<'a, 'b, G: ImperfectGame + 'a, P: Prior<G, Counterfactuals, Range, Policy>, N: PrivateNode<'a, G>, I: PublicNode<'a, N, G, Range>, Counterfactuals: AbstractCounterfactual, Range: AbstractRange, Policy: AbstractPolicy> {
    tree: Vec<N>,
    imperfect_tree: Vec<I>,
    root_ranges: [Range; 2],
    prior: &'b P,
    exploration_rate: f32,                                      // PUCT parameter
    search_explorations: usize,                                 // number of nodes to expand
    updates_per: usize,                                         // number of updates per expansion
    _phantom: PhantomData<&'a (G, Counterfactuals, Policy)>,
}

impl<'a, 'b, G: ImperfectGame, P: Prior<G, Counterfactuals, Range, Policy>, N: PrivateNode<'a, G>, I: PublicNode<'a, N, G, Range>, Counterfactuals: AbstractCounterfactual, Range: AbstractRange, Policy: AbstractPolicy>
    GtCfr<'a, 'b, G, P, N, I, Counterfactuals, Range, Policy> {
    // ---------- Usage ---------- //
    // Create the game tree, reserving space for 'capacity' nodes without reallocation
    pub fn with_capacity(belief: Belief<G, Range>, capacity: usize, prior: &'a P, search_explorations: usize, updates_per: usize) -> Self {
        // let (public_information, ranges) = belief;
        // let mut root = *N::empty(public_information);
        // root.initialize(ranges.clone());
        // let tree = GameTree::with_capacity(capacity, root);
        // let mut sog = Self {
        //     tree,
        //     root_ranges: ranges,
        //     root_state: todo!(),
        //     prior,
        //     exploration_rate: 0.5,
        //     search_explorations,
        //     updates_per,
        //     _phantom: PhantomData,
        // };
        // sog
        todo!()
    }

    pub(crate) fn search(&mut self, update_public: PublicObservation) -> (Counterfactuals, Policy) {
        let grow_id = self.safe_resolving(update_public);
        let root = self.tree.mut_node(self.tree.root());
        // TODO: fix with cfr root and grow root
        // let (v, p) = self.gt_cfr(root, root, self.search_explorations, self.updates_per);
        todo!()
    }
    // Search: Growing Tree Counter Factual Regret Minimization (Search). Iteratively search then selectively expand tree. (mut GameTree, Belief) -> (Counterfactuals, Policy)
    fn gt_cfr(&mut self, cfr_root: &mut I, grow_root: &mut I, state: StateId, expansions: usize, update_per: usize) -> (Counterfactuals, Policy) {
        let mut value = Counterfactuals::new();
        let player = grow_root.player();
        // let reconstruction_cfvs = if let Some(bound) = cfr_root.value().clone() { Some(bound) } else { None };
        for _ in 0..(expansions/update_per) {
            // Recalculate gadget range and mix in -> CFV should be saved in the SearchStatistics
            let current_range = cfr_root.range(player);
            let other_range = cfr_root.range(player.next());
            // let exploit_range =
            //     if let Some(bound) = reconstruction_cfvs.clone() { self.reconstruct_range(cfr_root, bound) }
            //     else { current_range.clone() };  // from CFR root
            let ranges = [current_range, other_range];//.mix_in(&exploit_range, 0.5)];
        
            // Execute the search
            value = cfr(&mut self.tree, cfr_root, ranges, self.prior);  // populate SearchStatistics on Imperfect Information Game Tree
            self.explore_n(grow_root, update_per);
        }
        (value, grow_root.cfr_policy(state))
    }
    // --------- GETTERS --------- //
    fn next_node_id(&self) -> PrivateNodeId {
        self.tree.len() as PrivateNodeId
    }

    fn node(&self, node_id: PrivateNodeId) -> &N {   // get_node: nodeId -> Node
        &self.tree[node_id as usize]
    }

    fn mut_node(&mut self, node_id: PrivateNodeId) -> &mut N {  // nodeID -> &mut Node
        &mut self.tree[node_id as usize]
    }
    // --------- Helper Functions --------- //
    // Grow the tree from node (based on strategy)
    fn explore_n(&mut self, grow_root: PublicNodeId, n: usize) {
        let node = self.tree.mut_node(grow_root);
        for _ in 0..n {
            let history = node.sample();
            self.grow(node, history);
        }
    }
    
    // Use policy select leaf for further exploration (replace prior with child nodes). (mut Tree, Belief) -> mutated Tree
    fn grow(&mut self, mut node: &mut PublicNode<N, G, Range>, mut world_state: &mut N) {
        loop {
            // Fixme: need to track PBS and history
            // Get action, update node, transition, end => expand
            // Node
            action_id = self.grow_step(node, state_id, &world_state);
            node.update_action_counts(state_id, action_id);
            // NodeTransition
            let action = action_id.into();
            let next_id = world_state.transition(action);
            world_state = 
            node = node.transition(public_info);
            world_state.transition(private_info);
            match node.location() { 
                NodeType::Inner => {
                    // Continue the loop
                },
                NodeType::Leaf => {
                    node.expand();
                    return; // Done
                },
                NodeType::Terminal => {
                    return;  // Nothing to see here
                }
            }
        }
    }
    
    // select the next step down the tree
    fn grow_step(&self, parent: &N) -> ActionId {
        parent.iter_actions().map(|a| a.into())
            .max_by_key(|action: ActionId| {
            let cfr = self.exploit_value(parent, action) * 0.5;
            let puct = self.explore_value(parent, action) * 0.5;
            (cfr + puct) as i64  // fixme: I hate rust so I did this
        }).unwrap()
    }
    
    // greedy step value
    fn exploit_value(&self, parent: &I, state_id: StateId, action_id: ActionId) -> Reward {
        parent.action_probability(state_id, action_id)
    }
    
    // exploration step value. Normalizes frequently visited nodes
    fn explore_value(&self, parent: &N, action_id: ActionId) -> Reward {  // TODO: is this constrained 0-1
        // figure out virtual losses - https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=&cad=rja&uact=8&ved=2ahUKEwjkhdvOr_eEAxVANVkFHTsmB9oQFnoECBIQAQ&url=https%3A%2F%2Fmedium.com%2Foracledevs%2Flessons-from-alpha-zero-part-5-performance-optimization-664b38dc509e&usg=AOvVaw0FolKsdBOuGLML3WIqTTu4&opi=89978449
        let action_visits = parent.action_counts(action_id);
        let quality = parent.action_quality(action_id);
        let node_visits = parent.visits();
        let puct = quality / action_visits + self.exploration_rate * parent.action_probability(action_id) * node_visits.sqrt() / (1.0 + action_visits);
        puct
        // todo!()
    }
}