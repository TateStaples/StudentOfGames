// Growing Tree Counter Factual Regret Minimization
// Based on the DeepStack implementation of CFRd (https://github.dev/lifrordi/DeepStack-Leduc/tree/master/Source - cfrd_gadget.lua, resolving.lua, continual_resolving.lua [compute_action])

use std::marker::PhantomData;
use crate::cfr::cfr;

use crate::game::{Game, ImperfectGame};
use crate::game_tree::{PrivateNode, PublicNode};
use crate::policies::Prior;
use crate::types::{AbstractCounterfactual, AbstractPolicy, AbstractRange, ActionId, Belief, Reward, PublicObservation, StateId};

// What needs mutable node access: growth, search statistics
// DeepStack: (https://github.dev/lifrordi/DeepStack-Leduc/tree/master/Source - cfrd_gadget.lua, resolving.lua, continual_resolving.lua [compute_action])
pub struct GtCfr<'a, 'b, G: ImperfectGame + 'a, P: Prior<G, Counterfactuals, Range, Policy>, N: PrivateNode<'a, G>, Counterfactuals: AbstractCounterfactual, Range: AbstractRange, Policy: AbstractPolicy> {
    tree: Vec<N>,
    imperfect_tree: Vec<N>,
    root_ranges: [Range; 2],
    prior: &'b P,
    exploration_rate: f32,                                      // PUCT parameter
    search_explorations: usize,                                 // number of nodes to expand
    updates_per: usize,                                         // number of updates per expansion
    _phantom: PhantomData<&'a (G, Counterfactuals, Policy)>,
}

impl<'a, 'b, G: ImperfectGame, P: Prior<G, Counterfactuals, Range, Policy>, N: PrivateNode<'a, G>, Counterfactuals: AbstractCounterfactual, Range: AbstractRange, Policy: AbstractPolicy>
    GtCfr<'a, 'b, G, P, N, Counterfactuals, Range, Policy> {
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
    fn gt_cfr(&mut self, cfr_root: &mut PublicNode, grow_root: &mut PublicNode, state: StateId, expansions: usize, update_per: usize) -> (Counterfactuals, Policy) {
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
    // --------- Helper Functions --------- //
    
    // Grow the tree from node (based on strategy)
    fn explore_n(&mut self, node: &mut PublicNode, n: usize) {
        for _ in 0..n {
            let history = node.sample();
            self.grow(node, history);
        }
        let x = self.grow;
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
            let (public_info, priv_info) = world_state.step(action);
            node = node.transition(priv_info);
            // Visit
            if node.leaf() {
                action_id = self.grow_step(node, state_id, &world_state);
                node.update_action_counts(state_id, action_id);
                let action = action_id.into();
                world_state.step(action);
                let state = world_state.public_state();
                node = self.match_child(node, state);  // todo: don't match child, just allow action step  
            }
            else if node.solved() {
                return;
            }
            else {
                node.visit((), (), action_id, world_state);
                return;
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
    fn exploit_value(&self, parent: &N, action_id: ActionId) -> Reward {
        parent.action_probability(action_id)
    }
    
    // exploration step value. Normalizes frequently visited nodes
    fn explore_value(&self, parent: &N, action_id: ActionId) -> Reward {  // TODO: is this constrained 0-1
        // figure out virtual losses - https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=&cad=rja&uact=8&ved=2ahUKEwjkhdvOr_eEAxVANVkFHTsmB9oQFnoECBIQAQ&url=https%3A%2F%2Fmedium.com%2Foracledevs%2Flessons-from-alpha-zero-part-5-performance-optimization-664b38dc509e&usg=AOvVaw0FolKsdBOuGLML3WIqTTu4&opi=89978449
        let action_visits = parent.action_counts(action_id);
        let quality = parent.action_quality(action_id);
        let node_visits = parent.visits();
        let puct = quality / action_visits + self.exploration_rate * parent.action_probability(state_id, action_id) * node_visits.sqrt() / (1.0 + action_visits);
        puct
        // todo!()
    }
    
    // --------- Updating Functions --------- //
    // move the root to the new game state. Returns the new state (grow_root)
    fn safe_resolving(&mut self, update: PublicObservation) -> &mut N {
        let mut root = self.tree.mut_node(self.tree.root());
        let mut grow_root = root;
        let root_public = root.public_state();
        let mut moving = false;
        let mut expanding = false;
        for (state_id, action_id, public_info) in new_state.history() {
            if moving {
                let node_id = self.match_child(root, public_info);
                if let Some(id) = node_id {  // TODO: update the ranges here
                    root = id;
                    grow_root = id;
                }
                else {  // child not found under parent. Novel branching
                    expanding = true;
                    moving = false;
                }
            }
            if expanding {
                let mut new_node = N::empty(public_info.clone()).into();
                self.tree.push(new_node);
                root.add_transition(state_id, action_id, NodeTransition::Edge(&new_node));
                grow_root = new_node.as_mut();
            }
            moving = moving || root_public == public_info;   // Below the root
        }
        assert!(moving || expanding, "New state is not a child of the root");
        self.tree.reroot(root);
        // during traversal, update ranges and values
        return grow_root;
    }
    
    // CFR-D gadget: reconstruct the range for the current player using value bound
    fn reconstruct_range(&self, root: &N, bound: Counterfactuals) -> Range {
        let play_values = bound;
        let terminate_values = root.value().unwrap();
        let play_current_strategy = root.cfr_policy(root.public_state().state_id());

        // Compute current regrets
        let mut total_values = play_values.clone();
        total_values.element_mul(&play_current_strategy);
        let mut terminate_current_strategy = Range::ones(play_current_strategy.len()) - &play_current_strategy;
        let mut total_values_terminate = terminate_values.clone();
        total_values_terminate.element_mul(&terminate_current_strategy);
        total_values += &total_values_terminate;

        let mut play_current_regret = play_values - &total_values;
        let mut terminate_current_regret = terminate_values - &total_values;

        // Cumulate regrets
        self.play_regrets += &play_current_regret;
        self.terminate_regrets += &terminate_current_regret;

        // Apply CFR+ (regret clamping)
        let regret_epsilon = 1e-8;
        self.play_regrets.clamp(regret_epsilon, f64::MAX);
        self.terminate_regrets.clamp(regret_epsilon, f64::MAX);

        // Regret matching
        let mut regret_sum = self.play_regrets.clone();
        regret_sum += &self.terminate_regrets;

        let mut new_play_strategy = self.play_regrets.clone();
        new_play_strategy /= &regret_sum;

        let mut new_terminate_strategy = self.terminate_regrets.clone();
        new_terminate_strategy /= &regret_sum;

        // Apply range mask to ensure only valid hands are considered
        let range_mask = root.public_state().range_mask();
        new_play_strategy.element_mul(&range_mask);
        new_terminate_strategy.element_mul(&range_mask);

        // Normalize the strategies
        let total_strategy = &new_play_strategy + &new_terminate_strategy;
        new_play_strategy /= &total_strategy;

        new_play_strategy
        /*
            // --1.0 compute current regrets
        torch.cmul(self.total_values, play_values, self.play_current_strategy)
        self.total_values_p2 = self.total_values_p2 or self.total_values:clone():zero()
        torch.cmul(self.total_values_p2, terminate_values, self.terminate_current_strategy)
        self.total_values:add(self.total_values_p2)

        self.play_current_regret = self.play_current_regret or play_values:clone():zero()
        self.terminate_current_regret = self.terminate_current_regret or self.play_current_regret:clone():zero()

        self.play_current_regret:copy(play_values)
        self.play_current_regret:csub(self.total_values)

        self.terminate_current_regret:copy(terminate_values)
        self.terminate_current_regret:csub(self.total_values)

        // --1.1 cumulate regrets
        self.play_regrets:add(self.play_current_regret)
        self.terminate_regrets:add(self.terminate_current_regret)

        // --2.0 we use cfr+ in reconstruction
        self.terminate_regrets:clamp(self.regret_epsilon, tools:max_number())
        self.play_regrets:clamp(self.regret_epsilon, tools:max_number())

        self.play_possitive_regrets = self.play_regrets
        self.terminate_possitive_regrets = self.terminate_regrets

        // --3.0 regret matching
        self.regret_sum = self.regret_sum or self.play_possitive_regrets:clone():zero()
        self.regret_sum:copy(self.play_possitive_regrets)
        self.regret_sum:add(self.terminate_possitive_regrets)

        self.play_current_strategy:copy(self.play_possitive_regrets)
        self.terminate_current_strategy:copy(self.terminate_possitive_regrets)

        self.play_current_strategy:cdiv(self.regret_sum)
        self.terminate_current_strategy:cdiv(self.regret_sum)

        // --4.0 for poker, the range size is larger than the allowed hands
        // --we need to make sure reconstruction does not choose a range
        // --that is not allowed
        self.play_current_strategy:cmul(self.range_mask)
        self.terminate_current_strategy:cmul(self.range_mask)

        self.input_opponent_range = self.input_opponent_range or self.play_current_strategy:clone():zero()
        self.input_opponent_range:copy(self.play_current_strategy)

        return self.input_opponent_range
         */
    }
    // fn update(observation) -> ()
    // 
}