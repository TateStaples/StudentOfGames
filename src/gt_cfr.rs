use rand::{Rng, thread_rng};
use crate::game_tree::{GameTree, NodeTransition, NodeId, ActionId, StateId, Outcome, Counterfactuals};
use crate::search_statistics::{FixedStatistics, Range, ActionPolicy, ImperfectNode};
use crate::helpers::prelude::{Game, NNPolicy, Policy};
use crate::cfr::cfr;
// What needs mutable node access: growth, search statistics

type Belief<G: Game, const A: usize, const S: usize> = (G::PublicInformation, [Range; 2]);

fn mix_range(range1: Range, range2: Range, weight: f32) -> Range {
    range1.iter().zip(range2.iter()).map(|((s1, p1), (s2, p2))| {
        (s1, p1 * weight + p2 * (1.0 - weight))
    }).collect()
}
// DeepStack: (https://github.dev/lifrordi/DeepStack-Leduc/tree/master/Source - cfrd_gadget.lua, resolving.lua, continual_resolving.lua [compute_action])
pub struct GtCfr<'a, G: Game, P: Policy<G, A, S>, N: ImperfectNode<'a, G>, const A: usize, const S: usize> {
    tree: GameTree<'a, G, N>,
    root_ranges: [Range; 2],
    prior: &'a P,
    exploration_rate: f32,  // PUCT parameter
    search_explorations: usize,  // number of nodes to expand
    updates_per: usize,  // number of updates per expansion
}
impl<'a , G: Game, P: Policy<G, A, S>, N: ImperfectNode<'a, G>, const A: usize, const S: usize> GtCfr<'a, G, P, N, A, S> {
    // ---------- Usage ---------- //
    // Create the game tree, reserving space for 'capacity' nodes without reallocation
    pub fn with_capacity(belief: Belief<G, A, S>, capacity: usize, prior: &'a P, search_explorations: usize, updates_per: usize) -> Self {
        let (public_information, ranges) = belief;
        let mut root = N::empty(public_information);
        root.initialize(ranges.clone());
        let tree = GameTree::with_capacity(capacity, root);
        let mut sog = Self {
            tree,
            root_ranges: ranges,
            prior,
            exploration_rate: 0.5,
            search_explorations,
            updates_per,
        };
        sog
    }
    pub fn exploit(&mut self, game: G) -> G::Action {
        let policy = self.search(game);
        sample_policy(policy).into()
    }
    fn search(&mut self, world_state: G) -> ActionPolicy {
        let grow_id = self.safe_resolving(world_state);
        self.gt_cfr(self.tree.root_id(), grow_id, self.search_explorations, self.updates_per).1
    }
    // Search: Growing Tree Counter Factual Regret Minimization (Search)
    fn gt_cfr(&mut self, cfr_id: NodeId, grow_id: NodeId, expansions: usize, update_per: usize) -> (Counterfactuals, ActionPolicy) {
        let grow_root = self.tree.node(grow_id);
        let cfr_root = self.tree.node(cfr_id);
        let mut value = Counterfactuals::new();
        let player = grow_root.player();

        for _ in 0..(expansions/update_per) {
            // Recalculate gadget range and mix in -> CFV should be saved in the SearchStatistics
            let current_range = cfr_root.range(player);
            let other_range = cfr_root.range(player.other());
            let exploit_range =
                if let Some(bound) = cfr_root.average_value().clone() { self.cfrd_gadget(cfr_id, bound) }
                else { current_range.clone() };  // from CFR root
            let ranges = [current_range, mix_range(other_range, exploit_range, 0.5)];

            // Execute the search
            value = cfr(&self.tree, cfr_id, ranges, &self.prior);  // from CFR root
            self.explore_n(grow_id, update_per);
        }
    }
    // --------- Helper Functions --------- //
    // Grow the tree from node (based on strategy)
    fn explore_n(&mut self, node_id: NodeId, n: usize) {
        let node = self.tree.node(node_id);
        for _ in 0..n {
            let (state_id, world_state) = Game::sample_state(node.public_state());
            self.grow(node_id, state_id, world_state);
        }
    }
    // Use policy select leaf for expansion
    fn grow(&mut self, mut node_id: NodeId, state_id: StateId, mut world_state: G) {
        let mut action_id = 0;
        loop {
            let node = self.tree.mut_node(node_id);
            if node.is_visited(state_id) {
                action_id = self.grow_step(node_id, state_id, &world_state);
                node.update_action_counts(state_id, action_id);
                let action = action_id.into();
                world_state.step(action);
                let state = world_state.public_state();
                node_id = self.match_child(node_id, state);
            }
            else if node.solved() {
                return;
            }
            else {
                node.visit((), (), action_id, world_state, &mut self.tree);
                return;
            }
        }
    }
    // select the next step down the tree
    fn grow_step(&self, parent: NodeId, state_id: StateId, world_state: &G) -> ActionId {
        (0..A).iter().max_by_key(|action: ActionId| {
            let cfr = self.exploit_value(parent, state_id, action) * 0.5;
            let puct = self.explore_value(parent, state_id, action) * 0.5;
            cfr + puct
        }).unwrap()
    }
    // greedy step value
    fn exploit_value(&self, parent: NodeId, state_id: StateId, action_id: ActionId) -> Outcome {
        let parent = self.tree.node(parent);
        parent.action_probability(state_id, action_id)
    }
    // exploration step value. Normalizes frequently visited nodes
    fn explore_value(&self, parent: NodeId, state_id: StateId, action_id: ActionId) -> Outcome {  // TODO: is this constrained 0-1
        // figure out virtual losses - https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=&cad=rja&uact=8&ved=2ahUKEwjkhdvOr_eEAxVANVkFHTsmB9oQFnoECBIQAQ&url=https%3A%2F%2Fmedium.com%2Foracledevs%2Flessons-from-alpha-zero-part-5-performance-optimization-664b38dc509e&usg=AOvVaw0FolKsdBOuGLML3WIqTTu4&opi=89978449
        let parent = self.tree.node(parent);  // Search Statistics
        let action_visits = parent.action_counts(action_id);
        let quality = parent.action_quality(state_id, action_id);
        let node_visits = parent.visits();
        let puct = quality / action_visits + self.exploration_rate * parent.action_probability(state_id, action_id) * node_visits.sqrt() / (1.0 + action_visits);
        puct
    }
    // --------- Updating Functions --------- //
    // move the root to the new game state. Returns the new state (grow_root)
    fn safe_resolving(&mut self, new_state: G) -> NodeId {
        let mut root = self.tree.root();
        let mut root_id = self.tree.root_id();
        let mut grow_root = root_id;
        let root_public = root.public_state();
        let mut moving = false;
        let mut expanding = false;
        for (state_id, action_id, public_info) in new_state.history() {
            if moving {
                let node_id = self.match_child(root, public_info);
                if let Some(id) = node_id {  // TODO: update the ranges here
                    root = self.tree.node(id);
                    root_id = id;
                    grow_root = id;
                }
                else {  // child not found under parent. Novel branching
                    expanding = true;
                    moving = false;
                }
            }
            if expanding {
                let new_node = N::empty(public_info.clone());
                let new_node_id = self.tree.push(new_node);
                root.add_transition(state_id, action_id, NodeTransition::Edge(new_node_id));
                grow_root = new_node_id;
            }
            moving = moving || root_public == public_info;   // Below the root
        }
        assert!(moving || expanding, "New state is not a child of the root");
        self.tree.reroot(root_id);
        // during traversal, update ranges and values
        return grow_root;
    }
    // Find the NodeID below the parent that matches the public state
    fn match_child(&self, parent: NodeId, child_state: G::PublicInformation) -> Option<NodeId> {
        let parent = self.tree.node(parent);
        for transition in parent.children() {
            if let NodeTransition::Edge(id) = transition {
                let child = self.tree.node(id);
                if child.public_state() == child_state {
                    return Some(id);
                }
            }
        }
        None
    }
    // reroot the action with your move - should this store ranges for true value. Can I run two of these on the same game tree
    fn update_with_action(&mut self, action: G::Action, result: G) {
        todo!("Update the game state with the action")
        // Need the new state and nodeID
    }
}

pub(crate) fn sample_policy<const A:usize>(policy: ActionPolicy) -> ActionId {
    let mut rng = thread_rng();
    let mut sum = 0.0;
    let random_number: f32 = rng.gen_range(0.0..1.0);
    for (p, a) in policy.iter() {
        sum += p;
        if sum > random_number {
            return *a;
        }
    }
    policy.last().expect("Empty action policy sampled").1
}
