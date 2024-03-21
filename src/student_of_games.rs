use std::sync::{Arc, Mutex};
use std::thread;
use crate::helpers::prelude::*;
use crate::game::Game;
use crate::game_tree::{ActionId, Outcome};
use crate::gt_cfr::{GtCfr, sample_policy};
use crate::policies::Policy;
use crate::search_statistics::{ActionPolicy, Belief, ImperfectNode, Probability};


pub type ReplayBuffer<G: Game, const A: usize, const S: usize> = Arc<Mutex<Vec<(Belief<G,A,S>, Outcome, ActionPolicy)>>>;  // distribution of information states for each player

struct TrainingConfigs {
    explores: usize,  // number of nodes to expand
    exploration: Exploration,  // exploration strategy
    exploration_chance: Probability,  // chance to explore
    update_per: usize,  // number of updates per expansion
    AUTO_EXTEND: bool, // extend visits
    MAX_ACTIONS: u8,  // maximum number of actions
    move_greedy: u8,  // after this number of actions, be greedy for training
    update_prob: Probability,  // probability of updating the network
}
struct Trainer<'a, G: Game, const A: usize, const S: usize> {
    starting_game: G,
    starting_belief: Belief<G, A, S>,
    resign_threshold: Option<Outcome>,
    longest_self_play: u8,
    greedy_depth: u8,
    self_play_explores: usize,
    self_play_updates_per: usize,
    self_play_explore_chance: Probability,
}
impl<'a , G: Game, P: Policy<G, A, S>, N: ImperfectNode<'a, G>, const A: usize, const S: usize, const C: usize> Trainer<'a, G, A, S> {
    // Learn from self-play. Important
    pub fn learn(&self, capacity: usize, play_threads: u8, prior: P, configs: TrainingConfigs) {
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
    fn self_play(&self, mut tree: GtCfr<G, P, N, A, S>, replay_buffer: ReplayBuffer<G, A, S>) {
        let mut actions = 0;
        let mut action: ActionId; // play self
        let mut game = self.starting_game.clone();

        while !game.is_over() && actions < self.longest_self_play {
            // TODO: handle chance nodes
            tree.safe_resolving(game.clone());  // update the tree to be rooted at the new location
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