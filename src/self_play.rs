//! # Self-Play Game Generation
//!
//! Generates training data by running games between solver instances or trained agents.
//! Used to collect experience for neural network training via self-play.

use rand::prelude::IndexedRandom;
use crate::obscuro::Obscuro;
use crate::utils::{Game, Player, Probability, ReplayBuffer, Reward};

pub fn is_verbose() -> bool {
    std::env::var("VERBOSE_SELFPLAY").is_ok()
}


pub fn student_of_games<G: Game>(iterations: i32, greedy_depth: i32) -> Obscuro<G> {
    let mut solver: Obscuro<G> = Obscuro::default();
    for iter in 0..iterations {
        println!("=== Iteration {} ===", iter);
        let replay_buffer = self_play_with_solver::<G>(greedy_depth, &mut solver);
        solver.learn_from(replay_buffer);
        // solver.debug();
    }
    solver
}
/// Exploration weight for mixing exploit and uniform random policies during self-play
const EXPLORATION_WEIGHT: f64 = 0.5;

/// Do self-learning by playing a game against yourself & updating your learning policies
/// This version reuses an existing solver for iterative improvement
pub fn self_play_with_solver<G: Game>(GREEDY_DEPTH: i32, solver: &mut Obscuro<G>) -> ReplayBuffer<G> {
    // Setup
    let mut game = G::new();
    {
        let observation = game.trace(Player::P1);
        let actions = game.available_actions();
        solver.seed_infoset(observation.clone(), Player::P2, &actions);
        solver.study_position(observation, Player::P2);
    }
    let mut depth = 0;
    // Collect per-move training records without value; assign terminal return at the end
    // Store (trace, policy, player_who_moved) to assign correct value sign
    let mut pending_records: Vec<(G::Trace, Vec<Probability>, Player)> = Vec::new();
    
    // Main loop
    while !game.is_over() {
        let player = game.active_player();
        if player == Player::Chance {
            let action = game.available_actions().choose(&mut rand::rng()).unwrap().clone();  // TODO: support non-uniform chance actions
            println!("Randomly plays: {:?}", action);
            game = game.play(&action);
            continue;
        } 
        let observation = game.trace(player);
        let actions = game.available_actions();
        solver.seed_infoset(observation.clone(), player, &actions);
        solver.study_position(observation.clone(), player);
        
        // Capture the current decision policy to supervise the policy head
        // Prefer instantaneous regret-matching distribution as teacher
        // Also track which player made this decision for correct value target sign
        let policy = solver.inst_policy(observation.clone());
        let inst_pi: Vec<Probability> = policy.inst_policy();
        pending_records.push((observation.clone(), inst_pi.clone(), player));
        
        // Verbose output: show bot's state and policy
        if is_verbose() {
            let action_count = policy.actions.len();
            let top_actions: Vec<_> = inst_pi.iter()
                .enumerate()
                .map(|(i, &p)| (i, p))
                .filter(|(_, p)| *p > 0.01)  // Only show actions with >1% probability
                .collect();
            
            eprintln!("  Bot({:?}): {} actions available", player, action_count);
            eprintln!("    Top policies:");
            for (idx, prob) in top_actions.iter().take(5) {
                if let Some(action) = policy.actions.get(*idx) {
                    eprintln!("      [{:3.1}%] {:?}", prob * 100.0, action);
                }
            }
        }
        
        // Load the action
        let action = if depth > GREEDY_DEPTH {
            policy.purified()
        } else {
            let exploring_policy: Vec<Probability> = policy.avg_strategy.iter().map(|x| EXPLORATION_WEIGHT * x + 1.0/(policy.actions.len() as Probability)).collect();
            let exploring_action = policy.sample_from(&exploring_policy);
            exploring_action
        };
        println!("Bot({:?}) plays: {:?}", player, action);
        game = game.play(&action);
        depth += 1;
    }
    // Game ended: compute terminal return and assign correct sign per player
    // evaluate() returns value from P1 perspective: +1 if P1 wins, -1 if P2 wins
    let final_reward_p1: Reward = game.evaluate();
    let replay_buffer: ReplayBuffer<G> = pending_records
        .into_iter()
        .map(|(trace, pi, player_who_moved)| {
            // If P1 moved, use P1 perspective; if P2 moved, flip sign
            let value = if player_who_moved == Player::P1 {
                final_reward_p1
            } else {
                -final_reward_p1
            };
            (trace, pi, value)
        })
        .collect();
    replay_buffer
}

/// Simple setup to let human player take either side in a game against the bot
pub fn interactive<G: Game>(human: Player) -> Reward {
    let computer = human.other();
    let mut game = G::new();
    let mut solver: Obscuro<G> = Obscuro::default();
    solver.study_position(game.trace(computer), computer);
    // solver.debug();
    while !game.is_over() {
        let actions = game.available_actions();
        match game.active_player() {
            Player::Chance => {
                // Randomly sample TODO: support non-uniform randomness
                let action = actions.choose(&mut rand::rng()).unwrap().clone();
                // println!("Randomly plays: {:?}", action);
                game = game.play(&action);
            }
            p if p == human => {
                // Ask for input by user typing in the index (usize) of the actions
                println!("Trace: {:?}, Available actions: {:?}", game.trace(p), actions);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).expect("Failed to read line");
                if let Ok(action_idx) = input.trim().parse::<usize>() {
                    let action = &actions[action_idx];
                    println!("Human plays: {:?}", action);
                    game = game.play(action);
                }
            }
            _ => {
                // Computer Plays
                let computer_trace = game.trace(computer);
                let action = solver.make_move(computer_trace, computer);
                println!("Computer plays: {:?}", action);
                game = game.play(&action);
                // I think the way the computer is sampling it's policy is wrong. It's also not stepping down correctly
            }
        }
    }
    println!("Evaluation: {}", game.evaluate());
    println!("{:?}", game);
    game.evaluate()
}
