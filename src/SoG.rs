use std::sync::{Arc, Mutex};
use std::thread;
use crate::config::Exploration;
use crate::game_tree::{ImperfectNode, Node, NodeTransition, NodeId, ActionId};
use crate::prelude::{Game, NNPolicy, Policy};

type StateId = usize;
type Outcome = f32;
type Counterfactuals = [Vec<Outcome>; 2];  // is 2p0s this can be stored as a single array
type Probability = f32;
type Range = Vec<(StateId, Probability)>;  // TODO: redefine this type
type Belief<G: Game, const A: usize, const S: usize> = (G::PublicInformation, [Range; 2]);
type ActionPolicy<const A: usize> = [Probability; A];  // TODO: redefine this type
type ReplayBuffer<G: Game, const A: usize, const S: usize> = Arc<Mutex<Vec<(Belief<G,A,S>, Outcome, ActionPolicy<A>)>>>;  // distribution of information states for each player

pub struct GtCfr<'a, G: Game, P: Policy<G, A, S>, const A: usize, const S: usize> {  // TODO: maybe the nodes should have a different lifetime
    root: NodeId,
    starting_game: G,
    prior: &'a P,
    exploration_rate: f32,  // PUCT parameter
}
impl<'a , G: Game, P: Policy<G, A, S>, N: ImperfectNode<'a, G, C>, const A: usize, const S: usize, const C: usize> GtCfr<'a, G, P, A, S> {
    // ---------- Usage ---------- //
    // Create the game tree, reserving space for 'capacity' nodes without reallocation
    pub fn with_capacity(game: G, belief: Belief<G, A, S>, capacity: usize, prior: &'a P) -> Self {
        let public_information = belief.0;
        let root = N::empty(public_information);
        let mut sog = Self {
            root,
            starting_game: game,
            prior,
            exploration_rate: 0.5,  // FIXME: figure out a decent value for this
        };
        sog
    }
    pub fn exploit(&mut self, game: G) -> G::Action {
        let (_, policy) = self.search(game);
        let action = sample_policy(policy);
        action.into()
    }

    // Note: I think SoG mixes old range with gadget range
    // move the root to the new game state (https://github.dev/lifrordi/DeepStack-Leduc/tree/master/Source - cfrd_gadget.lua, resolving.lua, continual_resolving.lua [compute_action])
    fn continual_resolving(&mut self, root_ranges: [Range; 2], new_state: G) {  // Find the information states at the start of GT-CFR
        // update invariant from node, state (make range and values consistent with actions taken
            // Chance Node: adjust CFVs based off EV, renormalize ranges based off observations
        // resolve based off node, player range, opponent cfvs bound
            // Create lookahead tree (Deep-stack up to chance node)
            // Iterate _compute: opp_range, curr_strat, ranges, update_avg, terminal equities, cfvs, regrets, avg cfvs
            // normalize average strategies and cfvs

        // get the cfr_root (lowest part of history matching) and the grow_root (latest observation)
    }

    fn depth_limit_tree(&self, depth: u8) {
        // Expand the tree to the depth limit
    }
    // ---------- Major Algorithms ---------- //
    fn search(&mut self, world_state: G, ranges: [Range; 2], opponent_counterfactuals: Option<Counterfactuals>) {
        self.continual_resolving(ranges, world_state);
    }

    // Search: Growing Tree Counter Factual Regret Minimization (Search)
    fn gt_cfr(&mut self, node_id: NodeId, ranges: [Range; 2], expansions: usize, update_per: usize) -> (Counterfactuals, ActionPolicy<A>) {
        let node = self.node(node_id);
        // Reroot the tree then connect to the new state
        // Clear SearchStatistics (besides root CFV) -> does it make sense to set previous policy as starting point
        let mut value = [vec![0.0]; 2];
        for _ in 0..(expansions/update_per) {
            // Recalculate gadget range and mix in -> CFV should be saved in the SearchStatistics
            value = self.cfr(node_id, ranges.clone());  // from CFR root

            for _ in 0..update_per {  // CFR slow, so do it in batches
                let (state_id, world_state) = Game::sample_state(node.public_state());  // Should grow off of root_grow
                self.grow(node_id, state_id, world_state);
            }
        }
        return (value, node.cfr_policy());
    }

    // CFR+ (populate SearchStatistics): belief down and counterfactual values (for given policy) up
    fn cfr(&mut self, node_id: NodeId, ranges: [Range; 2]) -> Counterfactuals {
        // DeepStack order: opp_range √, strategy (reach probabilities), ranges, update avg_strat, terminal values, values, regrets, avg_values
        let node = self.mut_node(node_id);
        // TODO: clear the search statistics (maybe leave a base value to improve convergence)
        let evaluation = if node.leaf() { Some(self.prior((node.public_state(), ranges))) } else { None };
        // r(s,a) = v(s,a) - EV(policy)
        // Q(s,a) += r(s,a) [min value of 0]
        // π(s,a) = percentage of Q
        // Note: DeepStack stores the average CFVs for later storage
        // propagate the belief down
        for (result, new_ranges, cases) in node.iter_results(&ranges) {
            // propagate search_stats back up
            match result {
                NodeTransition::Edge(id) => {
                    let counterfactuals = self.cfr(id, new_ranges);
                    for (state, next_state, action, probability) in cases {
                        let value = counterfactuals.get(next_state).expect("Transfer to unknown state"); // TODO: figure out the type
                        node.update_action_quality(state, action, value, probability)
                    }
                },
                NodeTransition::Terminal(v) => {
                    for (state, action, probability) in cases {
                        node.update_action_quality(state, action, v, probability)
                    }
                }
                NodeTransition::Undefined => {
                    let (value, _) = evaluation.unwrap();
                    for (state, next_state, action, probability) in cases {
                        let value = value.get(next_state).expect("Transfer to unknown state");
                        node.update_action_quality(state, action, value, probability)
                    }
                }
            };
        }
        // TODO: normalize the regret to be min of 0
    }

    // Use policy select leaf for expansion
    fn grow(&mut self, mut node_id: NodeId, state_id: StateId, mut world_state: G) {
        let mut action_id = 0;
        loop {
            let node = self.mut_node(node_id);
            if node.is_visited(state_id) {
                action_id = self.grow_step(node_id, state_id, &world_state);
                node.update_action_counts(state_id, action_id);
                let action = action_id.into();
                world_state.step(action);
                let state = world_state.public_state();
                node_id = self.match_child(node_id, action_id, state);
            }
            else if node.solved() {
                return;
            }
            else {
                node.visit(node_id, action_id, world_state);
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
        let parent = self.node(parent);
        parent.action_probability(state_id, action_id)
    }
    // exploration step value. Normalizes frequently visited nodes
    fn explore_value(&self, parent: NodeId, state_id: StateId, action_id: ActionId) -> Outcome {  // TODO: is this constrained 0-1
        // TODO: figure out virtual losses - https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=&cad=rja&uact=8&ved=2ahUKEwjkhdvOr_eEAxVANVkFHTsmB9oQFnoECBIQAQ&url=https%3A%2F%2Fmedium.com%2Foracledevs%2Flessons-from-alpha-zero-part-5-performance-optimization-664b38dc509e&usg=AOvVaw0FolKsdBOuGLML3WIqTTu4&opi=89978449
        let parent = self.node(parent);  // Search Statistics
        let action_visits = parent.action_counts(action_id);
        let quality = parent.action_quality(state_id, action_id);
        let node_visits = parent.visits();
        let puct = quality / action_visits + self.exploration_rate * parent.action_probability(state_id, action_id) * node_visits.sqrt() / (1.0 + action_visits);
        puct
    }
}

struct SoGConfigs {
    explores: usize,  // number of nodes to expand
    exploration: Exploration,  // exploration strategy
    exploration_chance: Probability,  // chance to explore
    update_per: usize,  // number of updates per expansion
    AUTO_EXTEND: bool, // extend visits
    MAX_ACTIONS: u8,  // maximum number of actions
    move_greedy: u8,  // after this number of actions, be greedy for training
    update_prob: Probability,  // probability of updating the network
}
struct StudentOfGames<'a, G: Game, const A: usize, const S: usize> {
    starting_game: G,
    starting_belief: Belief<G, A, S>,
    resign_threshold: Option<Outcome>,
    longest_self_play: u8,
    greedy_depth: u8,
    self_play_explores: usize,
    self_play_updates_per: usize,
    self_play_explore_chance: Probability,
}
impl<'a , G: Game, P: Policy<G, A, S>, N: Node<'a, G, C>, const A: usize, const S: usize, const C: usize> StudentOfGames<'a, G, A, S> {
    // Learn from self-play. Important
    pub fn learn(&self, capacity: usize, play_threads: u8, prior: P, configs: SoGConfigs) {
        let replay_buffer = Arc::new(Mutex::new(Vec::new()));
        let training = false;
        let mut games = 0;
        // self-play
        for _ in 0..play_threads {
            let handle = thread::spawn(move || {
                let mut sog = Self::with_capacity(self.starting_game.clone(), self.starting_belief.clone(), capacity, &configs, &prior);
                while training {
                    self.self_play(sog, Arc::clone(&replay_buffer));
                    sog.clear();
                    games += 1;
                }
            });
        }

        // train network
        self.save();  // TODO: add a termination condition
    }
    fn self_play(&self, mut tree: GtCfr<G, P, A, S>, replay_buffer: ReplayBuffer<G, A, S>) {
        let mut actions = 0;
        let mut action: ActionId; // play self
        let mut game = self.starting_game.clone();

        while !game.is_over() && actions < self.longest_self_play {
            tree.continual_resolving(game.clone());  // update the tree to be rooted at the new location
            // SoG agent
            let (value, policy) = tree.gt_cfr(tree.root, self.starting_belief.1.clone(), self.self_play_explores, self.self_play_updates_per);  // do your gtcfr search of the tree
            replay_buffer.lock().unwrap().push((tree.node(tree.root).belief(), value, policy));  // add to the replay buffer
            if value < self.resign_threshold.unwrap_or(f32::NEG_INFINITY) {  // not worth compute for self-play
                return
            }
            let self_play_policy = (1 - self.self_play_explore_chance) * policy + self.self_play_explore_chance * [1/A; A];
            if actions < self.greedy_depth {  // explore shallowly then be greedy for better approximation
                action = sample_policy(self_play_policy);
            } else {  // greedy at depth
                action = policy.arg_max();  // take "best" action - greedy
            }

            game.step(action);  // should return information to each player
            actions += 1;
        }
    }
}