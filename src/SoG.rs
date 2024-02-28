use std::sync::{Arc, Mutex};
use std::thread;
use rand::prelude::*;
use crate::config::Exploration;
use crate::prelude::{Game, NNPolicy, Policy};

// TODO: separate the growing tree from the SoG agent
type NodeId = usize;
type ActionId = usize;
type Outcome = f32;
type Range<const S: usize> = [f32; S];
type Belief<G: Game<N>, const N: usize, const S: usize> = (G::PublicInformation, [Range<S>; 2]);
type ActionPolicy<const N: usize> = [f32; N];
fn sample_policy<const N:usize>(policy: ActionPolicy<N>) -> ActionId {
    let mut rng = thread_rng();
    let mut sum = 0.0;
    let mut action = 0;
    let random_number: f32 = rng.gen_range(0.0..1.0);
    for (i, p) in policy.iter().enumerate() {
        sum += p;
        if sum > random_number {
            action = i;
            break;
        }
    }
    action
}
type ReplayBuffer<G: Game<N>, const N: usize, const S: usize> = Arc<Mutex<Vec<(Belief<G,N,S>, Outcome, ActionPolicy<N>)>>>;  // distribution of information states for each player
type QueryBuffer<G: Game<N>, const N: usize, const S: usize> = Arc<Mutex<Vec<Belief<G,N,S>>>>;  // distribution of information states for each player
type PublicInformation<G: Game<N>, const N: usize> = G::PublicInformation;  // public and private information knowledge of player
struct Node<G: Game<N>, const N:usize, const C: usize, const S: usize> {  
    // N: max actions, C: max children per action, S: max number of hidden states
    public_state: PublicInformation<G, N>,      // public information state
    private_ranges: [Range<S>; 2],              // private information ranges
    action_counts: [f32; N],                    // action counts
    action_quality: [Outcome; N],                   // action quality
    action_outcomes: [[Option<NodeId>; C]; N],          // action outcomes
    solved: bool,                               // Optionally solved
}
impl<G: Game<N>, const N: usize, const C: usize, const S: usize> Node<G, N, C, S> {
    fn cfr_policy(&self) -> ActionPolicy<N> {
        let mut policy = self.action_quality;
        for action_id in 0..N {
            policy[action_id] /= self.action_counts[action_id];
        }
        policy
    }
    #[inline]
    fn action_probability(&self, action_id: ActionId) -> f32 {
        self.cfr_policy()[action_id]
    }

    #[inline]
    fn visits(&self) -> f32 {
        self.action_counts.iter().sum()
    }

    fn update_range(&self, action_id: ActionId) -> [Range<S>; 2] {
        let mut new_range = self.private_ranges;
        for i in 0..2 {
            for j in 0..S {
                new_range[i][j] *= self.action_probability(action_id);  // fixme: this is wrong
            }
        }
        new_range
    }

    fn new(game: &G, ranges: [Range<S>; 2], value_prior: [Outcome; N], solved: bool) -> Self {
        Self {
            public_state: game.public_state(),
            private_ranges: ranges,              // private information ranges
            action_counts: [1.0; N],                    // action counts
            action_quality: value_prior,                   // action quality
            action_outcomes: [[None; C]; N],          // action outcomes
            solved
        }
    }
}
struct SoGConfigs {
    explores: usize,  // number of nodes to expand
    exploration: Exploration,  // exploration strategy
    exploration_chance: f32,  // chance to explore
    update_per: usize,  // number of updates per expansion
    AUTO_EXTEND: bool, // extend visits
    RESIGN_THRESHOLD: Option<f32>,  // threshold for resigning
    MAX_ACTIONS: u8,  // maximum number of actions
    move_greedy: u8,  // after this number of actions, be greedy for training
    update_prob: f32,  // probability of updating the network
}
pub struct GtCfr<'a, G: Game<N>, P: Policy<G, N>, const N: usize, const C: usize, const S: usize> {
    root: NodeId,
    starting_game: G,
    nodes: Vec<Node<G, N, C, S>>,
    prior: &'a P,  //
    cfg: &'a SoGConfigs
}
impl<'a , G: Game<N>, P: Policy<G, N>, const N: usize, const C: usize, const S: usize> GtCfr<'a, G, P, N, C, S> {
    // ---------- Usage ---------- //
    // Create the game tree, reserving space for 'capacity' nodes without reallocation
    pub fn with_capacity(game: G, belief: Belief<G, N, S>, capacity: usize, cfg: &'a SoGConfigs, prior: &'a P) -> Self {
        let mut nodes = Vec::with_capacity(capacity);
        nodes.push(Node::new(&game, belief.1, [1.0; N], false));
        let mut sog = Self {
            root: 0,
            starting_game: game,
            nodes,
            prior,
            cfg,
        };
        sog.add_root_noise();
        sog
    }
    fn reset(&mut self, starting_game: G) {
        let root = self.node(0);
        self.nodes.clear();
        todo!()
    }
    pub fn exploit(&mut self, game: G) -> G::Action {
        let (value, policy) = self.search(game);
        let action = sample_policy(policy);
        action.into()
    }

    // move the root to the new game state
    fn continual_resolving(&mut self, new_state: G) {  // Find the information states at the start of GT-CFR
        todo!();  // probably need helper function from the game
        // Find nearest parent from the previous search

        // connect nearest state to new state

        // Fill out the auxiliary tree to get range

        // re-root the tree to the new state
        self.starting_game = new_state;
        self.root = 0;
    }
    // ---------- Getters ---------- //
    #[inline]
    fn node(&self, node_id: NodeId) -> &Node<G, N, C, S> { &self.nodes[node_id as usize] }
    #[inline]
    fn mut_node(&mut self, node_id: NodeId) -> &mut Node<G, N, C, S> { &mut self.nodes[node_id as usize] }

    fn match_child(&self, node_id: NodeId, action: ActionId, state: PublicInformation<G, N>) -> Option<NodeId> {
        let node = self.node(node_id);
        let outcomes = node.action_outcomes[action];

    }
    // ---------- Major Algorithms ---------- //
    // Search: Growing Tree Counter Factual Regret Minimization
    fn gt_cfr(&mut self, node_id: NodeId, expansions: usize, update_per: usize, queries: Option<(QueryBuffer<G, N, S>, f32)>) -> (f32, ActionPolicy<N>) {
        let node = self.node(node_id);
        let mut value = 0.0;
        for _ in 0..(expansions/update_per) {
            let query_copy = self.clone_queries(&queries);
            value = self.cfr(node_id, query_copy);
            let world_state = Game::sample_state(node.public_state);  // sample a new game state
            for _ in 0..update_per {  // CFR slow, so do it in batches
                self.grow(node_id, world_state);
            }
        }
        return (value, node.cfr_policy())
    }
    // CFR+: propagate belief down and counterfactual value up
    fn cfr(&mut self, node_id: NodeId, queries: Option<(Arc<Mutex<Vec<Belief<G, N, S>>>>, f32)>) -> f32 {  // returns the value, regret
        let node = self.node(node_id);
        let active_policy = node.cfr_policy();
        return if node.is_visited() {
            let mut node_value = 0.0;
            for action_id in 0..N {
                let action_prob = active_policy[action_id];
                // get the range update
                let new_range = node.update_range(action_id);
                for c in node.action_outcomes[action_id] {
                    if let Some(child) = c {
                        let child_node = self.mut_node(child);
                        child_node.private_ranges = new_range;
                        let query_copy = self.clone_queries(&queries);
                        let value = self.cfr(child, query_copy);
                        for a in 0..N {
                            if a == action_id {
                                child_node.action_quality[a] += value;  // TODO: This math wrong
                            } else {
                                child_node.action_quality[a] -= value * action_prob;
                            }
                            node_value += value * action_prob;
                        }
                    } else {
                        break;
                    }
                }
            }
            node.action_quality.map(|x| x.max(0.0));
            node_value
        } else if node.solved {
            self.terminal_value(node_id)
        } else {
            let random_number: f32 = thread_rng().gen_range(0.0..1.0);
            let belief = (node.public_state, node.private_ranges);
            if let Some((nn_queries, query_rate)) = queries.as_ref() {
                if random_number < query_rate {
                    nn_queries.lock().push(belief);
                }
            }
            let (v, p) = self.prior.eval(belief);
            v
        }
    }

    fn clone_queries(&self, queries: &Option<(Arc<Mutex<Vec<Belief<G, N, S>>>>, f32)>) -> Option<(Arc<Mutex<Vec<Belief<G, N, S>>>>, f32)> {
        if let Some((nn_queries, query_rate)) = queries.as_ref() {
            Some((Arc::clone(&nn_queries), *query_rate))
        } else { None }
    }
    fn terminal_value(&self, node_id: NodeId) -> Outcome {
        todo!()
    }

    // Use policy to target tree expansion
    fn grow(&mut self, mut node_id: NodeId, mut world_state: G) {
        let mut action_id = 0;
        loop {
            let node = self.mut_node(node_id);
            if node.is_visited() {
                action_id = self.grow_step(node);
                node.action_counts[action_id] += 1.0;
                let action = action_id.into();
                world_state.step(action);
                let state = world_state.public_state();
                node_id = self.match_child(node_id, action_id, state);
            }
            else if node.solved {
                return;
            }
            else {
                self.visit(node_id, action_id, world_state);
                return;
            }
        }
    }

    // best child for growing the tree
    fn grow_step(&self, parent: NodeId) -> ActionId {
        (0..N).iter().max_by_key(|action: ActionId| {
            let cfr = self.exploit_value(parent, action) * 0.5;
            let puct = self.explore_value(parent, action) * 0.5;
            cfr + puct
        }).unwrap()
    }
    fn exploit_value(&self, parent: NodeId, action_id: ActionId) -> f32 {
        let parent = self.node(parent);
        parent.action_probability(action_id)
    }
    // the value of the node during exploration. Normalizes frequently visited nodes
    fn explore_value(&self, parent: NodeId, action_id: ActionId) -> f32 {  // TODO: recheck the validity
        let parent = self.node(parent);
        let action_visits = parent.action_counts[action_id];
        match self.cfg.exploration {
            Exploration::Uct { c } => {
                let visits = (c * action_visits.ln()).sqrt();
                visits / action_visits.sqrt()
            }
            Exploration::PolynomialUct { c } => {
                let visits = parent.visits().sqrt();
                c * parent.action_probability(action_id) * visits / (1.0 + action_visits)
            }
        }
    }
    // open a new node.
    fn visit(&mut self, parent_id: NodeId, action: ActionId, new_world_state: G) {
        let new_node: Node<G, N, C, S>;
        let parent = self.mut_node(parent_id);
        let range = parent.update_range(action);  // fixme: this is wrong
        let belief: Belief<G, N, S> = (new_world_state.public_state(), range);
        if new_world_state.is_over() {
            let outcome = new_world_state.outcome();
            // TODO: store payoff matrices in a seperate array
            new_node = Node::new(parent_id, new_world_state, true, outcome);
        } else {
            let (v, p) = self.prior.eval(belief);
            new_node = Node::new(parent_id, new_world_state, false, v);
        }
        let new_node_id = self.nodes.len();
        self.nodes.push(new_node);
        for i in 0..C {
            if parent.action_outcomes[action][i].is_none() {
                parent.action_outcomes[action][i] = Some(new_node_id);
                break;
            }
        }
    }
}

struct StudentOfGames<'a, G: Game<N>, P: Policy<G, N>, const N: usize, const C: usize, const S: usize> {
    starting_game: G,
    starting_belief: Belief<G, N, S>,
    resign_threshold: Option<f32>,
    longest_self_play: u8,
    greedy_depth: u8,
    self_play_explores: usize,
    self_play_updates_per: usize,
    self_play_query_rate: f32,
    self_play_explore_chance: f32,
    training_query_rate: f32,
}

impl<'a , G: Game<N>, P: Policy<G, N>, const N: usize, const C: usize, const S: usize> StudentOfGames<G, P, N, C, S> {
    // Learn from self-play. Important
    pub fn learn(&self, capacity: usize, play_threads: u8, prior: P, configs: SoGConfigs) {
        let replay_buffer = Arc::new(Mutex::new(Vec::new()));
        let nn_queries = Arc::new(Mutex::new(vec![]));
        let training = false;
        let mut games = 0;
        // self-play
        for _ in 0..play_threads {
            let handle = thread::spawn(move || {
                let mut sog = Self::with_capacity(self.starting_game.clone(), self.starting_belief, capacity, &configs, &prior);
                while training {
                    sog.self_play(Arc::clone(&nn_queries), Arc::clone(&replay_buffer));
                    sog.clear();
                    games += 1;
                }
            });
        }
        // query solver
        thread::spawn(move || {
            Self::query_solver(&training, Arc::clone(&nn_queries), Arc::clone(&replay_buffer));
        });

        // train network
        self.save();  // TODO: add a termination condition
    }
    fn self_play(&self, mut tree: GtCfr<G, P, N, C, S>, queries: QueryBuffer<G, N, S>, replay_buffer: ReplayBuffer<G, N, S>) {
        let mut actions = 0;
        let mut action: ActionId; // play self
        let mut game = self.starting_game.clone();
        while !game.is_over() && actions < self.longest_self_play {
            tree.continual_resolving(game.clone());  // update the tree to be rooted at the new location
            // SoG agent
            let (value, policy) = tree.gt_cfr(tree.root, self.self_play_explores, self.self_play_updates_per, Some((Arc::clone(&queries), self.training_query_rate)));  // do your gtcfr search of the tree
            if value < self.resign_threshold.unwrap_or(f32::NEG_INFINITY) {  // not worth compute for self-play
                return
            }
            let self_play_policy = (1 - self.self_play_explore_chance) * policy + self.self_play_explore_chance * [1/N; N];
            if actions < self.greedy_depth {  // explore shallowly then be greedy for better approximation
                action = sample_policy(self_play_policy);
            } else {  // greedy at depth
                action = policy.arg_max();  // take "best" action - greedy
            }

            game.step(action);  // should return information to each player
            actions += 1;
        }
        // for belief in history {  // fixme
        //     if random() < self.cfg.update_prob {
        //         value.update(trajectory.outcome());  // the value of the terminal state
        //         policy.update(belief.policy)
        //         replay_buffer.add((belief, v, p));  // save for network training
        //     }
        // }
    }

    fn query_solver(training: &bool, nn_queries: QueryBuffer<G, N, S>, replay_buffer: ReplayBuffer<G, N, S>) {  // offline trainer
        while training {
            let belief = nn_queries.lock().pop();
            // use gt_cfr to get the value and policy, some recurve queries might be added
            // TODO: figure out how to parallelize this
            // add to the replay_buffer
        }
    }
}