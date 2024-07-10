use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::game::{Game, ImperfectGame};
use crate::gt_cfr::{GtCfr};
use crate::policies::Prior;
use crate::search_statistics::{ImperfectNode};
use crate::types::{AbstractCounterfactual, AbstractPolicy, AbstractRange, ActionId, Belief, Reward, Probability};
use crate::helpers::config::Exploration;


pub type ReplayBuffer<G: Game, Range: AbstractRange, Counterfactuals: AbstractCounterfactual, Policy: AbstractPolicy> = Arc<Mutex<Vec<(Belief<G,Range>, Counterfactuals, Policy)>>>;  // distribution of information states for each player

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

struct Trainer<'a, G: ImperfectGame + 'a, P: Prior<G, Counterfactuals, Range, Policy>, N: ImperfectNode<'a, G, Counterfactuals, Range, Policy>, Counterfactuals: AbstractCounterfactual, Range: AbstractRange, Policy: AbstractPolicy> {
    starting_game: G,
    starting_belief: Belief<G, Range>,
    resign_threshold: Option<Reward>,
    longest_self_play: u8,
    greedy_depth: u8,
    self_play_explores: usize,
    self_play_updates_per: usize,
    self_play_explore_chance: Probability,
    _phantom: PhantomData<(&'a P, N, Counterfactuals, Policy)>,
}
impl<'a , G: ImperfectGame, P: Prior<G, Counterfactuals, Range, Policy>, N: ImperfectNode<'a, G, Counterfactuals, Range, Policy>, Counterfactuals: AbstractCounterfactual, Range: AbstractRange, Policy: AbstractPolicy>
    Trainer<'a, G, P, N, Counterfactuals, Range, Policy, > {
    // Learn from self-play. Important
    pub fn learn(&self, capacity: usize, play_threads: u8, mut prior: P, configs: TrainingConfigs) {
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
        prior.learn();
        self.save();  // TODO: add a termination condition
    }
    fn self_play<'b>(&self, mut strategy: GtCfr<'a, 'b, G, P, N, Counterfactuals, Range, Policy>, replay_buffer: ReplayBuffer<G, Range, Counterfactuals, Policy>) {
        let mut actions = 0;
        //  TODO: why does this start with a tree: shouldn't it create it
        let mut action: ActionId; // play self
        let mut game = self.starting_game.clone();
        let do_not_resign = false;  // coin flip with probability p_no resign

        while !game.is_over() && actions < self.longest_self_play {
            let (value, policy) = strategy.search(game.public_information());
            if value < self.resign_threshold.unwrap_or(f32::NEG_INFINITY) {  // not worth compute for self-play
                return
            }
            let self_play_policy = policy.mix_in(Policy::uniform(), self.self_play_explore_chance);
            if actions < self.greedy_depth {  // explore shallowly then be greedy for better approximation
                action = self_play_policy.sample();  
            } else {  // greedy at depth
                action = policy.arg_max();  // take "best" action - greedy
            }

            game.step(action);  // should return information to each player
            actions += 1;
        }
    }
}

