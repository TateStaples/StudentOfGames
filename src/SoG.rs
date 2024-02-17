use rand::prelude::*;
use rand::seq::index::sample;
use crate::config::{ActionSelection, MCTSConfig};
use crate::prelude::{Game, Policy};

// Terminology:
// World States (w): decision node, terminal node, chance node
// actions (a): choice by player
// history (h): sequence of actions
// Observations (O): public / private update given on each action
// Information State (s): The set of public / private indistinguishable histories from observations
// Policy (P/π): Information State -> Distribution of action probabilities
// Transition Function (T): world state update from active players actions
// Belief State (B): A range for the private information states conditioned on observation history
// Value (v): how good an action is given information state
// Regret (r): oppurtunity cost, r(s,a) = v(s,a) - EV(P(s, a)).
// Counter Factual Regret (CFR): P'(s, a) = averaged regret over time
// CFR+: update quality (Q) of an action instead of exploring whole tree. P(s, a) = weighted average of quality
// Proper Game: subgame with corresponding public information and belief of information state
// Counterfactual Value Network (CVN): approximates value as a function of beliefs
// Continual-Resolving: repeated safe resolving (from prev sol and opp CFR values)
type NodeId = usize;
type ActionId = u8;
type ChildrenCount = u8;
type Outcome = f32;

struct Node<G: Game<N>, const N:usize> {
    parent: NodeId,
    first_child: NodeId,
    num_children: ChildrenCount,
    game: G,
    solved: bool,          // Optionally solved
    action: ActionId,
    expected_value: f32,
    visits: f32                     // float for easy division
}
impl<G: Game<N>, const N: usize> Node<G, N> {
    fn quality(&self) -> f32 { self.expected_value / self.visits}
    fn action(&self) -> G::Action {(self.action as usize).into()}
    #[inline]
    fn is_unvisited(&self) -> bool {
        self.num_children == 0 && !self.solved
    }
    #[inline]
    fn is_visited(&self) -> bool {
        self.num_children != 0
    }
    #[inline]
    fn last_child(&self) -> NodeId {
        self.first_child + self.num_children as u32
    }  // how is this reserved
    #[inline]
    fn mark_visited(&mut self, first_child: NodeId, num_children: u8) {
        assert!(self.is_unvisited());
        self.first_child = first_child;
        self.num_children = num_children;
    }
    #[inline]
    fn mark_solved(&mut self, outcome: f32) {
        self.solved = true;
        self.expected_value = outcome;
    }
    #[inline]
    fn solution(&self) -> Option<f32> {if self.solved { Some(self.expected_value) } else { None }}
}
struct SoGConfigs {

}
pub struct StudentOfGames<'a, G: Game<N>, P: Policy<G, N>, const N: usize> {
    // Growing Tree Counter Factual Regret Minimization
    root: NodeId,
    offset: NodeId,
    nodes: Vec<Node<G, N>>,
    policy: &'a mut P,  //
    cfg: SoGConfigs,
}

type Belief = todo!();
impl<'a, G: Game<N>, P: Policy<G, N>, const N: usize> StudentOfGames<'a, G, P, N> {
    // ---------- Usage ---------- //
    // Use the policy to play as well as possible
    pub fn exploit(explores: usize, cfg: SoGConfigs, policy: &'a mut P, game: G) -> G::Action {
        let mut mcts = Self::with_capacity(explores + 1, cfg, policy, game);
        mcts.explore_n(explores);  // creates 'explores' nodes
        mcts.best_action()  // creates the +1 node
    }
    // Create the analysis
    pub fn with_capacity(capacity: usize, cfg: &SoGConfigs, policy: &'a mut P, game: G) -> Self {
        let mut nodes = Vec::with_capacity(capacity);
        nodes.push(Node::unvisited(0, game, None, 0, 0.0));
        let mut mcts = Self {
            root: 0,
            offset: 0,
            nodes,
            policy,
            cfg,
        };
        let (node_id, outcome_probs, any_solved) = mcts.visit(mcts.root);
        mcts.backprop(node_id, outcome_probs, any_solved);
        mcts.add_root_noise();
        mcts
    }
    // Learn from self-play. Important
    pub fn self_play(&self, game: G, config: &SoGConfigs) -> (Value, Policy<G>) {
        let mut actions = 0;
        let mut action: Option<Action> = None;
        let mut history = game.history();  // list of actions to get to this point
        let belief = game.belief();  // range of private information to reach this public state
        // play self
        while !game.is_over() && actions < MAX_ACTIONS {
            if game.stochasitc() {  // chance node -> choose a random action
                action = random_action()  // FIXME: should this be uniform or proportional?
            }
            else {  // SoG agent
                let (value, policy) = self.gt_cfr(game);  // do your gtcfr search of the tree
                if value < config.RESIGN_THRESHOLD {  // not worth compute for self-play
                    return
                }
                let selfplay_policy = (1-config.exploration_chance)  * policy + config.exploration_chance * random();  // mix with uniform to encourage exploration
                if actions < move_greedy {  // explore shallowly then be greedy for better approximation
                    action  = sample(selfplay_policy);
                }
                else {  // greedy at depth
                    action = policy.arg_max();  // take "best" action - greedy
                }
            }
            game.step(action);
            actions +=1;
        }

        for belief in history {
            if random() < config.update_prob {  // TODO: why random use? - ask GPT
                value.update(trajectory.outcome());  // the value of the terminal state
                policy.update(belief.policy)
                replay_buffer.add((belief, v, p));  // save for network training
            }
        }
    }
    // ---------- Getters ---------- //
    #[inline]
    fn next_node_id(&self) -> NodeId { self.nodes.len() as NodeId + self.offset }
    #[inline]
    fn node(&self, node_id: NodeId) -> &Node<G, N> { &self.nodes[(node_id - self.offset) as usize] }
    #[inline]
    fn mut_node(&mut self, node_id: NodeId) -> &mut Node<G, N> {  // nodeID -> &mut Node
        &mut self.nodes[(node_id - self.offset) as usize]
    }
    #[inline]
    fn children_of(&self, node: &Node<G, N>) -> &[Node<G, N>] { &self.nodes[(node.first_child - self.offset) as usize..(node.last_child() - self.offset) as usize] }
    #[inline]
    fn mut_nodes(&mut self, first_child: NodeId, last_child: NodeId) -> &mut [Node<G, N>] { &mut self.nodes[(first_child - self.offset) as usize..(last_child - self.offset) as usize] }
    // ---------- Major Algs ---------- //
    // Search: Growing Tree Counter Factual Regret Minimization
    fn gt_cfr(mut tree: StudentOfGames<G, P, N>, belief: Belief, expansion: usize, sims_per: f32) -> P {
        for i in 0..(expansion/sims_per as usize) {
            crate::SoG::CFR(&tree, (1/sims_per).ceil());
            crate::SoG::grow(&mut tree);
        }
        return value_policy, action_policy  // TODO: ensure that by here the nn_queries are updated
    }
    // CFT+: update the action policy
    fn CFR(tree: &StudentOfGames, count: usize) {
        let regret =
    }
    // Use policy to target tree expansion
    fn grow(tree: &mut StudentOfGames<G, P, N>, belief: Belief, count: usize) {
        for i in 0..count {
            let history = follow_belief(tree, belief);
            add_top_children(tree, history, k);
            update_visit_counts(tree, history);
        }
        return tree
    }

    fn visit(&mut self, node: NodeId) {
        let node =
    }

    fn control(game) {
        let belief = game.belief;
        let action_tree = trace_belief(game, belief);
        let (v, p) = crate::SoG::training(action_tree);
        return v(game), p(game)
    }

    fn training(game: Game) -> (Value<G>, Policy<G>){
        let v, p, nn_queries = crate::SoG::gt_cfr(tree);
        let queries = nn_queries.sample(config.q_average);
        to_solve.extend(queries);
        return v, p
    }

    fn solver() {  // offline trainer
        for belief in to_solve {
            let v, p, nn_queries = crate::SoG::gt_cfr(belief);  // complete search
            replay_buffer.extend((belief, v, p));  // send example to trainer
            let queries = nn_queries.sample(config.q_recursive);
            to_solve.extend(queries)
        }
    }
}