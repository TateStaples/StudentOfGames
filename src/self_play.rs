use rand::prelude::IndexedRandom;
use crate::obscuro::Obscuro;
use crate::utils::{Game, Player, Probability, ReplayBuffer, Reward};


pub fn student_of_games<G: Game>(iterations: i32, greedy_depth: i32) -> Obscuro<G> {
    let mut solver: Obscuro<G> = Obscuro::default();
    for iter in 0..iterations {
        println!("=== Iteration {} ===", iter);
        let replay_buffer = self_play::<G>(greedy_depth);
        solver.learn_from(replay_buffer);
        // solver.debug();
    }
    solver
}
/// Exploration weight for mixing exploit and uniform random policies during self-play
const EXPLORATION_WEIGHT: f64 = 0.5;

/// Do self-learning by playing a game against yourself & updating your learning policies
fn self_play<G: Game>(GREEDY_DEPTH: i32) -> ReplayBuffer<G> {
    // Setup
    let mut game = G::new();
    let mut solver: Obscuro<G> = Obscuro::default();
    solver.study_position(game.trace(Player::P1), Player::P2);
    let mut depth = 0;
    let mut replay_buffer: ReplayBuffer<G> = vec![];
    
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
        solver.study_position(observation.clone(), player);
        
        // Update the replay buffer
        let policy = solver.inst_policy(observation.clone()); 
        let avg_strat: &Vec<Probability> = &policy.avg_strategy; 
        let expectation: Reward = policy.expectation();
        replay_buffer.push((observation.clone(), avg_strat.clone(), expectation));  // Maybe only push random subset
        
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
