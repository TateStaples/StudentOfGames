use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use rand::Rng;
use crate::policies::Prior;
// TODO: make reward positive for player 1 and negative for player 2 - need to adapt all infostates and reward functions to make this work
// Task list
// 1. Figure out how to handle infostates for non-active players (update the way we store counterfactuals and up propagate them)
// 2. Two Spies Game format: Public(control,reveals),State(city, xp, tech), Private(exact city [S = city index]), Action(move_to_city, attack, tech)

// Long-term:
// 1. Update with better Generics
// 2. Implement a better CFR algorithm
// 3. Make it equivalent to MCTS in perfect information scenarios
// 4. Find a way to filter out equivalent histories

pub type ActionId = usize;  // Index on generated ActionIterator
pub type Reward = f32;
pub type Probability = f32;

pub trait HasTurnOrder: Eq + Clone + Copy + std::fmt::Debug {
    fn prev(&self) -> Self;
    fn next(&self) -> Self;
}

pub type PrivateObservation = usize;
pub type PublicObservation = usize;

pub trait AbstractPolicy: Clone {
    fn new() -> Self;
    fn eval(&self, action_id: ActionId) -> Probability;
    fn sample(&self) -> ActionId;
    fn uniform() -> Self;
    fn mix_in(&self, other: &Self, p: Probability) -> Self;
}
pub trait AbstractRange: Clone {
    fn new() -> Self;
    fn eval(&self, state_id: InfoStateId) -> Probability;
    fn mix_in(&self, other: &Self, p: Probability) -> Self;
}
pub trait AbstractCounterfactual: Clone {
    fn new() -> Self;
    fn outcome(value: Reward) -> Self;
    fn eval(&self, state_id: InfoStateId) -> Reward;
}
pub trait Game: Eq + Hash + Clone + std::fmt::Debug + Send {
    type PlayerId: HasTurnOrder + Into<usize>;  // The playerId enum must have .next() and .prev()
    type Action: Eq + Clone + Copy + std::fmt::Debug + Into<usize> + From<usize>;
    type ActionIterator: Iterator<Item = Self::Action>;

    const MAX_TURNS: usize;
    const NUM_PLAYERS: usize;

    fn new() -> Self;  // Default initialization
    fn player(&self) -> Self::PlayerId;  // The active player
    fn is_over(&self) -> bool;
    fn reward(&self, player_id: Self::PlayerId) -> Reward;  // The reward for the player getting to active state.
    fn iter_actions(&self) -> Self::ActionIterator;
    fn step(&mut self, action: &Self::Action) -> (PublicObservation, PrivateObservation);  // Implement action and return whether done
    fn print(&self);  // Output the game state
}
pub trait FixedGame<const A: usize, const S: usize>: Game {
    const MAX_NUM_ACTIONS: usize = A;
    const HIDDEN_STATES: usize = S;
}

// Types: History (Private Node), Private InfoState, Public Belief State (Public Node)
// History: Transition, Game,
// Private InfoState: Policy, Search Statistics (visits, rewards, regrets, reach prob)
// Public Belief State: Range, Counterfactuals
pub type PrivateNodeId = usize;
pub type PublicNodeId = usize;
pub type InfoStateId = usize;
#[derive(PartialEq, Clone, Copy)]
pub enum NodeType {
    Inner,
    Leaf,
    Terminal
}
#[derive(Clone, Copy)]
pub struct FixedStrategy<const A: usize> { strategy: [Probability; A] }
#[derive(Clone, Copy)]
pub struct FixedRange<const S: usize> { range: [Probability; S] }
#[derive(Clone, Copy)]
pub struct FixedCounterfactuals<const S: usize> { cfvs: [Reward; S] }  // FIXME: I think this should be defined along S

#[derive(Clone)]
pub struct BeliefState<G: FixedGame<A, S>, const A: usize, const S: usize> {
    public_observation: PublicObservation,                              // The public observation to get here
    first_child: PublicNodeId,                                          // Tree structure
    num_children: usize,
    infostates: [InfoState<A>; S],                                      // Search statistics for each infostate
    histories: Vec<PrivateNodeId>,                                       // Probably fine for this to be on the heap as they shouldn't be loaded that frequently
    location: NodeType,
    _phantom: PhantomData<G>
} impl<G: FixedGame<A, S>, const A: usize, const S: usize> BeliefState<G, A, S>{
    fn sample(&self, tree: &[History<G>]) -> PrivateNodeId {
        let node_prob: Probability = self.histories.iter().map(|x| tree[*x].reach_prob).sum();
        let random_uniform: Probability = rand::thread_rng().gen_range(0.0..1.0);
        let mut cumulative_prob: Probability = 0.0;
        for history_id in self.histories.iter() {
            let history = &tree[*history_id];
            cumulative_prob += history.reach_prob / node_prob;
            if cumulative_prob >= random_uniform {
                return *history_id;
            }
        }
        unreachable!("Floating point maths a bitch")
    }  // Should sample based off how likely it is
    fn location(&self) -> NodeType {
        self.location
    }
    fn children(&self) -> Box<[PublicNodeId]> { (self.first_child..self.first_child+self.num_children as usize).collect() }
    fn transition(&self, observation: PublicObservation, imperfect_tree: &Vec<BeliefState<G,A,S>>) -> PublicNodeId {
        for child in self.children().into_iter() {
            let child = &imperfect_tree[*child];
            if child.public_observation == observation {
                return child.first_child;
            }
        }
        panic!("Transition not found");
    }
    fn expand(node_id: PublicNodeId, histories: &mut Vec<History<G>>, imperfect_tree: &mut Vec<Self>) {  // Take a leaf PBS
        let mut transition_map: HashMap<PublicObservation, Vec<PrivateNodeId>> = HashMap::new();                               // Map from public observation to histories
        let _self = &imperfect_tree[node_id];                      // Get the public belief state
        for history_id in _self.histories.iter() {                          // All histories in this state
            let my_history = &mut histories[*history_id];                    // Get the history
            let new_children = my_history.expand();          // New Histories
            my_history.first_child = imperfect_tree.len();
            my_history.num_children = new_children.len();

            for (mut child, public_observation) in new_children.into_iter() {
                let child_id = histories.len();
                if transition_map.contains_key(&public_observation) {
                    transition_map.get_mut(&public_observation).unwrap().push(child_id);
                } else {
                    transition_map.insert(public_observation, vec![child_id]);
                }

                // Add the child
                child.parent = *history_id;
                histories.push(child);
            }
        }
        let first_child = imperfect_tree.len();
        let num_children = transition_map.len();
        let _self = &mut imperfect_tree[node_id];           // Get the public belief state
        _self.first_child = first_child;                    // First child in the public belief state
        _self.num_children = num_children;                  // Number of subsequent public states

        for (public_observation, children) in transition_map {
            let mut infostates = [InfoState::blank(); S];  // FIXME: I am in no way confident this is correct
            for (state_id, state) in infostates.iter_mut().enumerate() {
                state.state_id = state_id;      // StateID of infostate in its respective public node
                state.parent = state_id;        // FIXME: this assumes no hidden state changes
            }
            let example_child = &histories[children[0]];
            let node_type = if example_child.game.is_over() { NodeType::Terminal } else { NodeType::Leaf };
            let pbs = Self {
                public_observation,
                first_child: 0,
                num_children: 0,
                infostates,
                histories: children,
                location: node_type,
                _phantom: PhantomData
            };
            imperfect_tree.push(pbs);
        }
    }
    fn clear(&mut self) {
        for state in self.infostates.iter_mut() {
            state.reach_prob = 0.0;
        }
    }
    fn infostate(&self, state_id: InfoStateId) -> &InfoState<A> { &self.infostates[state_id] }  
    fn mut_infostate(&mut self, state_id: InfoStateId) -> &mut InfoState<A> { &mut self.infostates[state_id] }
    fn active_player(&self) -> G::PlayerId { todo!() }
}

// Policy, Search Statistics (visits, rewards, regrets, reach prob), and Counterfactuals bound (blind to underlying histories
#[derive(Clone, Copy)]
struct InfoState<const A: usize> {  
    parent: InfoStateId,                            // Parent infostate it its respective public node
    action: ActionId,                               // FIXME: assume only one action can transfer between infostates (obviously not true in the case of poker)
    state_id: InfoStateId,                          // State ID in the public node
    reach_prob: Probability,                        // Reach probability of this infostate from the root
    strategy: FixedStrategy<A>,                     // Strategy for each action (probability of taking each action)
    visits: [f32; A],                               // Number of times each action has been visited (f32 for faster math)
    quality: [f32; A],                              // Aggregate Quality of each action (how good it is) - used for strategy
}
impl<const A: usize> InfoState<A> {
    fn blank() -> Self {
        Self {
            parent: 0,
            action: 0,
            state_id: 0,
            reach_prob: 0.0,
            strategy: FixedStrategy { strategy:[0.0;A]},
            visits: [0.0;A],
            quality: [0.0;A],
        }
    }
    fn action_counts(&self, action_id: ActionId) -> f32 { self.visits[action_id] } // f32 for faster math without casting
    fn action_quality(&self, action_id: ActionId) -> Reward { self.quality[action_id] }  
    fn action_probability(&self, action_id: ActionId) -> Probability { self.cfr_policy().strategy[action_id] }
    #[inline]
    fn cfr_policy(&self) -> FixedStrategy<A> { self.strategy }
}
#[derive(Clone, Copy)]
struct History<G: Game> {
    parent: PrivateNodeId,          // Parent node in the tree
    state_id: [InfoStateId; 2],     // Associated info state for each player
    public_id: PublicNodeId,        // Public belief state
    game: G,                        // Game state (ideally we move this into an optional box)
    first_child: PrivateNodeId,     // First child in the tree
    num_children: usize,            // Child corresponding to each action
    reach_prob: Probability,        // Reach probability of this node from the root
}
impl<G: Game> History<G> {
    fn step(&self, action: ActionId) -> Option<PrivateNodeId> { self.children().get(action).map(|x| *x) }
    fn children(&self) -> Box<[PrivateNodeId]> { (self.first_child..self.first_child+self.num_children).collect() }
    fn game(&self) -> &G { &self.game }
    fn transition(&self, action_id: ActionId) -> Option<PrivateNodeId> { self.children().get(action_id).map(|x| *x) }
    fn mut_game(&mut self) -> &mut G { &mut self.game }
    fn active_player(&self) -> G::PlayerId { self.game.player() }
    fn location(&self) -> NodeType {
        if self.game.is_over() { NodeType::Terminal }
        else if self.num_children == 0 { NodeType::Leaf }
        else { NodeType::Inner }
    }
    fn expand(&mut self) -> Vec<(Self, PublicObservation)> {
        let mut children = Vec::with_capacity(self.game.iter_actions().count());
        for action in self.game.iter_actions() {
            let mut game = self.game.clone();
            let (public_observation, private_observation) = game.step(&action);
            let child = Self {
                parent: 0,
                state_id: self.state_id,   // This is not true in the general case (use private observations)
                public_id: public_observation,
                game,
                first_child: 0,
                num_children: 0,
                reach_prob: 0.0,
            };
            children.push((child, private_observation));
        }
        // LATER: see if you can remove Game from the inner here
        children
    }
    fn active_state(&self) -> InfoStateId { self.state_id[self.active_player().into()] }
}

// What needs mutable node access: growth, search statistics
// DeepStack: (https://github.dev/lifrordi/DeepStack-Leduc/tree/master/Source - cfrd_gadget.lua, resolving.lua, continual_resolving.lua [compute_action])
pub struct GtCfr<'a, 'b, G: FixedGame<A, S> + 'a, CPVN: Prior<G, A, S>, const A: usize, const S: usize> {
    tree: Vec<History<G>>,
    imperfect_tree: Vec<BeliefState<G, A, S>>,
    root: PrivateNodeId,
    prior: &'b CPVN,
    exploration_rate: f32,                                      // PUCT parameter
    search_explorations: usize,                                 // number of nodes to expand
    updates_per: usize,                                         // number of updates per expansion
    _phantom: PhantomData<&'a (G, )>,
}
impl<'a, 'b, G: FixedGame<A, S>, CPVN: Prior<G, A, S>, const A: usize, const S: usize>
GtCfr<'a, 'b, G, CPVN, A, S> {
    // ---------- Usage ---------- //
    // Create the game tree, reserving space for 'capacity' nodes without reallocation
    pub fn with_capacity(initial_game: G, capacity: usize, prior: &'b CPVN, search_explorations: usize, updates_per: usize) -> Self {
        let infostate = InfoState {  // TODO: this is not correct
            parent: 0,
            action: 0,
            state_id: 0,
            reach_prob: 1.0,
            strategy: FixedStrategy { strategy:[0.0;A]},
            visits: [1.0;A],
            quality: [0.0;A],
        };
        let mut states: [InfoState<A>; S] = [infostate; S];
        for (state_id, state) in states.iter_mut().enumerate() {
            state.state_id = state_id;
            state.parent = state_id;
        }
        let histories = vec![0];
        let pbs: BeliefState<G, A, S> = BeliefState {
            public_observation: 0,
            first_child: 0,
            num_children: 0,
            histories,                                                  // History is just the initial state of the game (this might need to be different in fixed games -> setup as chance node?)
            infostates: states,                                              // Initialize the infostates
            location: NodeType::Leaf,                                   // Start as a leaf
            _phantom: PhantomData,
        };
        let start_of_history = History {
            parent: 0,
            state_id: [0, 0],
            public_id: 0,
            game: initial_game,
            first_child: 0,
            num_children: 0,
            reach_prob: 1.0,
        };
        let tree = vec![start_of_history; capacity];
        let imperfect_tree = vec![pbs; capacity];
        GtCfr {
            tree,
            imperfect_tree,
            root: 0,
            prior,
            exploration_rate: 1.0,
            search_explorations,
            updates_per,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn update(&mut self, observation: PublicObservation) -> PublicNodeId {  // TODO: add some way to track individual states (be stateful)
        let pbs = self.pbs(self.root);
        // if pbs is a leaf, expand it (might happen at the start to populate all possible states)
        if pbs.location() == NodeType::Leaf {
            BeliefState::expand(self.root, &mut self.tree, &mut self.imperfect_tree);
        }
        self.root = pbs.transition(observation, &self.imperfect_tree);
        self.root
    }
    // Search: Growing Tree Counter Factual Regret Minimization (Search). Iteratively search then selectively expand tree. (mut GameTree, Belief) -> (Counterfactuals, Policy)
    fn gt_cfr(&mut self, root_id: PublicNodeId, expansions: usize, update_per: usize) {
        // let reconstruction_cfvs = if let Some(bound) = cfr_root.value().clone() { Some(bound) } else { None };
        for _ in 0..(expansions/update_per) {
            // Execute the search
            self.explore_n(root_id, update_per);
            self.cfr(root_id);                       // populate SearchStatistics on Imperfect Information Game Tree
        }
    }

    // see if this can be optimized **later**
    // DeepStack: reach prob, range, avg reach, values, regrets, avg_regrets [repeat]
    fn cfr(&mut self, node: PublicNodeId) {  // Propagate ranges down the tree, update search statistics, and calculate counterfactual values
        self.cfr_clear(node);
        self.cfr_reach(node);
        self.cfr_regret(node);
    }
    fn cfr_clear(&mut self, node_id: PublicNodeId) {
        let node = self.mut_pbs(node_id);
        node.clear();
        
        let children_ids: Vec<PublicNodeId> = node.children().iter().cloned().collect();

        // Now, iterate over the collected child IDs
        for child_id in children_ids {
            self.cfr_clear(child_id);
        }
    }
    fn cfr_reach(&mut self, node_id: PublicNodeId) {  
        // TODO: should normalize the top distribution (maybe depends on full SoG implementation)
        // Read to check if this is correct
        // Immutable borrow to get active player and children IDs
        let node = self.pbs(node_id);
        let children_ids: Vec<PublicNodeId> = node.children().to_vec();

        // Collect necessary data for updates
        let mut updates = Vec::with_capacity(self.tree.len());
        for history_id in node.histories.iter() {
            let history = self.node(*history_id);
            let state = self.active_info(*history_id);
            for action_id in 0..A {
                if let Some(step) = history.step(action_id) {
                    let action_chance = state.action_probability(action_id);
                    let reach_prob_update = history.reach_prob * action_chance;
                    updates.push((step, reach_prob_update));
                }
            }
        }

        // Apply collected updates
        for (step, reach_prob_update) in updates {
            let child = &mut self.tree[step as usize];
            child.reach_prob = reach_prob_update;
            let info_state = self.mut_active_info(step);
            info_state.reach_prob += reach_prob_update;
            // TODO: make sure you don't need to update inactive player (i think this should implicitly update at higher levels of recursion tree)
            
        }

        // Recursive calls after mutable operations
        for child_id in children_ids {
            self.cfr_reach(child_id);
        }
    }
    fn cfr_regret(&mut self, node_id: PublicNodeId) -> [[(InfoStateId, Probability, Reward); S]; 2] {  // CPVN define Counterfactuals and the infostate level
        let node = self.pbs(node_id);
        match node.location() {  // What if we don't store the counterfactuals in the infostates and just qualities and strategies?
            NodeType::Inner => {  // weighting over children
                let children_ids: Vec<PublicNodeId> = node.children().iter().cloned().collect();
                
                let mut v: [[Reward; A]; S] = [[0.0; A]; S];
                for child_belief in children_ids {
                    let counterfactuals = self.cfr_regret(child_belief)[0];  // TODO: index by player instead
                    
                    for (info, prob, reward) in counterfactuals {
                        // let reach_prob = info.reach_prob;  // FIXME this won't work - need to change the data structure
                        let state = info.parent;
                        let action = info.action;
                        v[state][action] += prob * reward;  // NOTE this will decay really fast (need to divide by current_info.reach_prob)
                    }
                }
                let node = self.mut_pbs(node_id);
                // Discount average value and do CFR+ normalization
                for (state_id, values) in v.iter().enumerate() {
                    let state_value: Reward = values.iter().sum::<f32>() / (A as f32);
                    let parent_info = node.mut_infostate(state_id);  // TODO: figure out what to do with the other player
                    let mut net_quality = 0.0;
                    for action in 0..A {
                        // CFR+ update
                        let regret = values[action] - state_value;  // How much better the action is than the average (how much more you want to do it)
                        parent_info.quality[action] += regret;
                        parent_info.quality[action] = parent_info.quality[state_id].max(0.0);
                        net_quality += parent_info.quality[action];
                    }
                    
                    // Strategy Update
                    for action in 0..A {
                        parent_info.strategy.strategy[action] = parent_info.quality[action] / net_quality;
                    }
                }

            },
            NodeType::Leaf => {  // prior eval
                let (v, _) = self.eval(node_id);
                v.map(|c| {
                    c.cfvs.iter().enumerate().map(|(state_id, reward)| v.eval(0))
                })
            },
            NodeType::Terminal => {  // game terminal eval
                let result = self.node(node.histories[0]).game.reward(self.node(node.histories[0]).active_player());
                let histories = node.histories.clone();
                for history in histories {
                    let history = self.node(history);
                    let reward: Reward = history.game.reward(history.active_player());
                    let state_id = history.active_state();
                    let state = self.mut_active_info(state_id);
                    for action in 0..A {
                        state.cfvs[action] = reward;
                    }
                }
                
            }
        }
        todo!()
    }
    // --------- GETTERS --------- //
    fn next_node_id(&self) -> PrivateNodeId { self.tree.len() as PrivateNodeId }
    fn next_pbs_id(&self) -> PublicNodeId { self.imperfect_tree.len() as PublicNodeId }
    fn node(&self, node_id: PrivateNodeId) -> &History<G> { &self.tree[node_id as usize] }
    fn pbs(&self, node_id: PublicNodeId) -> &BeliefState<G, A, S> { &self.imperfect_tree[node_id as usize] }
    fn mut_node(&mut self, node_id: PrivateNodeId) -> &mut History<G> { &mut self.tree[node_id as usize] }
    fn mut_pbs(&mut self, node_id: PublicNodeId) -> &mut BeliefState<G, A, S> { &mut self.imperfect_tree[node_id as usize] }
    fn active_info(&self, node_id: PrivateNodeId) -> &InfoState<A> {
        let node = self.node(node_id);
        let player = node.active_player();
        let pbs = self.pbs(node.public_id);
        &pbs.infostates[node.state_id[player.into()]]
    }
    fn mut_active_info(&mut self, node_id: PrivateNodeId) -> &mut InfoState<A> {
        let node = &self.tree[node_id];
        let player = node.active_player();
        let pbs = &mut self.imperfect_tree[node.public_id];
        &mut pbs.infostates[node.state_id[player.into()]]
    }
    fn inactive_info(&self, node_id: PrivateNodeId) -> &InfoState<A> {  // Trace up the tree until you find the last time the other player made a decision
        let mut node = self.node(node_id);
        let target_player = node.active_player().next();
        while node.game.player() != target_player {
            node = self.node(node.parent);
        }
        let player = node.active_player();
        let pbs = self.pbs(node.public_id);
        &pbs.infostates[node.state_id[player.into()]]
    }
    fn mut_inactive_info(&mut self, node_id: PrivateNodeId) -> &mut InfoState<A> {
        let mut node = &self.tree[node_id];
        let target_player = node.active_player().next();
        while node.game.player() != target_player {
            node = &self.tree[node.parent];
        }
        let player = node.active_player();
        let pbs = &mut self.imperfect_tree[node.public_id];
        &mut pbs.infostates[node.state_id[player.into()]]
    }
    fn eval(&self, node_id: PublicNodeId) -> ([FixedCounterfactuals<S>; 2], [FixedStrategy<A>; S]) {
        let model = self.prior;
        model.eval(self.pbs(node_id))
    }
    // --------- Helper Functions --------- //
    // Grow the tree from node (based on strategy)
    fn explore_n(&mut self, grow_root: PublicNodeId, n: usize) {
        for _ in 0..n {
            // Scope to limit the duration of the mutable borrow
            let history = {
                let node = self.pbs(grow_root);
                node.sample(&self.tree)
            };
            self.grow(grow_root, history);
        }
    }
    // Use policy select leaf for further exploration (replace prior with child nodes). (mut Tree, Belief) -> mutated Tree
    fn grow(&mut self, node_id: PublicNodeId, history: PrivateNodeId) {
        // Get to leaf PBS and then expand
        let node = &self.imperfect_tree[node_id as usize];
        match node.location() {
            NodeType::Inner => {  // Continue down the tree
                let world_state = self.node(history);
                let action_id = self.grow_step(node.infostate(world_state.active_state()));
                let next_history = world_state.transition(action_id).unwrap();
                let next_world_state = &mut self.tree[next_history as usize];
                let next_node = next_world_state.public_id;
                self.grow(next_node, next_history);
            },
            NodeType::Leaf => {  // Expand the leaf
                BeliefState::expand(node_id, &mut self.tree, &mut self.imperfect_tree);
            },
            NodeType::Terminal => {  // Dead end
                // do nothing
            }
        }
    }
    // select the next step down the tree
    fn grow_step(&self, parent: &InfoState<A>) -> ActionId {
        (0..A).map(|a| a.into())
            .max_by_key(|&action: &ActionId| {
                let cfr = self.exploit_value(parent, action) * 0.5;
                let puct = self.explore_value(parent, action) * 0.5;
                ((cfr + puct)*1e9) as i64  
            }).unwrap()
    }
    // greedy step value
    fn exploit_value(&self, parent: &InfoState<A>, action_id: ActionId) -> Reward {
        parent.strategy.strategy[action_id]
    }
    // exploration step value. Normalizes frequently visited nodes
    fn explore_value(&self, parent: &InfoState<A>, action_id: ActionId) -> Reward {
        // figure out virtual losses - https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=&cad=rja&uact=8&ved=2ahUKEwjkhdvOr_eEAxVANVkFHTsmB9oQFnoECBIQAQ&url=https%3A%2F%2Fmedium.com%2Foracledevs%2Flessons-from-alpha-zero-part-5-performance-optimization-664b38dc509e&usg=AOvVaw0FolKsdBOuGLML3WIqTTu4&opi=89978449
        let action_visits = parent.action_counts(action_id);
        let quality = parent.action_quality(action_id);
        let node_visits: f32 = parent.visits.iter().sum();
        let puct = quality / action_visits + self.exploration_rate * parent.action_probability(action_id) * node_visits.sqrt() / (1.0 + action_visits);
        puct
    }
}

struct GtCfrContext {
    public_state: PublicNodeId,
    private_state_p1: PrivateNodeId,
    private_state_p2: PrivateNodeId,
}