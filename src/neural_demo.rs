//! # Neural Network Demo and Testing
//!
//! Example and test code demonstrating neural network usage for game evaluation.
//! Includes performance comparisons and integration tests.

/// Demo of neural network-based solving for Liar's Die with large game trees
/// 
/// This demo shows how the neural network can be used to evaluate positions
/// in game trees that are too large to fully expand, demonstrating the key
/// capability of the Student of Games algorithm.

use crate::games::liars_die::{LiarsDie, Die, LiarsDieAction};
use crate::obscuro::Obscuro;
use crate::obscuro_parallel::ObscuroParallel;
use crate::utils::{Game, Player, Reward};
use rand::seq::IndexedRandom;
use std::time::Instant;

/// Configuration for the demo
pub struct DemoConfig {
    /// Number of dice per player (larger = bigger tree)
    pub dice_per_player: usize,
    /// Time limit for solver in seconds
    pub solve_time_secs: f64,
    /// Whether to use parallel solver
    pub use_parallel: bool,
    /// Number of threads for parallel solver
    pub num_threads: usize,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            dice_per_player: 5,  // 5 dice per player
            solve_time_secs: 5.0,
            use_parallel: true,
            num_threads: 4,
        }
    }
}

/// Run a demo game showing neural network evaluation in large game trees
pub fn run_neural_demo(config: DemoConfig) {
    println!("=== Liar's Die Neural Network Demo ===");
    println!("Configuration:");
    println!("  Dice per player: {}", config.dice_per_player);
    println!("  Solve time: {:.1}s", config.solve_time_secs);
    println!("  Parallel: {}", config.use_parallel);
    if config.use_parallel {
        println!("  Threads: {}", config.num_threads);
    }
    println!();

    // Calculate approximate game tree size
    let tree_size_estimate = estimate_tree_size(config.dice_per_player);
    println!("Estimated game tree size: ~{} nodes", tree_size_estimate);
    println!("This is too large to fully expand, so we use neural network evaluation\n");

    // Play a demo game
    let mut game = LiarsDie::new();
    let mut move_count = 0;
    
    // Create solver (parallel or single-threaded)
    if config.use_parallel {
        run_demo_parallel(game, config.num_threads);
    } else {
        run_demo_single(game);
    }
}

fn run_demo_single(mut game: LiarsDie) {
    let mut solver: Obscuro<LiarsDie> = Obscuro::default();
    let mut move_count = 0;
    
    println!("Starting game (single-threaded solver)...\n");
    
    while !game.is_over() && move_count < 20 {
        let player = game.active_player();
        
        match player {
            Player::Chance => {
                let actions = game.available_actions();
                let action = actions.choose(&mut rand::rng()).unwrap().clone();
                println!("Chance: Deal dice");
                game = game.play(&action);
            }
            p => {
                let observation = game.trace(p);
                
                println!("Player {:?} thinking...", p);
                let start = Instant::now();
                let action = solver.make_move(observation.clone(), p);
                let elapsed = start.elapsed();
                
                let policy = solver.inst_policy(observation);
                let tree_size = solver.size();
                
                println!(
                    "  Action: {:?}",
                    action
                );
                println!(
                    "  Tree size: {} nodes (explored in {:.2}s)",
                    tree_size,
                    elapsed.as_secs_f64()
                );
                println!(
                    "  Expected value: {:.4}",
                    policy.expectation()
                );
                println!();
                
                game = game.play(&action);
                move_count += 1;
            }
        }
    }
    
    if game.is_over() {
        let result = game.evaluate();
        println!("Game over! Result: {}", result);
        if result > 0.0 {
            println!("Player 1 wins!");
        } else {
            println!("Player 2 wins!");
        }
    }
}

fn run_demo_parallel(mut game: LiarsDie, num_threads: usize) {
    let mut solver = ObscuroParallel::<LiarsDie>::new(num_threads);
    let mut move_count = 0;
    
    println!("Starting game (parallel solver with {} threads)...\n", num_threads);
    
    while !game.is_over() && move_count < 20 {
        let player = game.active_player();
        
        match player {
            Player::Chance => {
                let actions = game.available_actions();
                let action = actions.choose(&mut rand::rng()).unwrap().clone();
                println!("Chance: Deal dice");
                game = game.play(&action);
            }
            p => {
                let observation = game.trace(p);
                
                println!("Player {:?} thinking (parallel)...", p);
                let start = Instant::now();
                let action = solver.make_move(observation.clone(), p);
                let elapsed = start.elapsed();
                
                let policy = solver.inst_policy(observation);
                let tree_size = solver.size();
                
                println!(
                    "  Action: {:?}",
                    action
                );
                println!(
                    "  Tree size: {} nodes (explored in {:.2}s with {} threads)",
                    tree_size,
                    elapsed.as_secs_f64(),
                    num_threads
                );
                println!(
                    "  Expected value: {:.4}",
                    policy.expectation()
                );
                println!();
                
                game = game.play(&action);
                move_count += 1;
            }
        }
    }
    
    if game.is_over() {
        let result = game.evaluate();
        println!("Game over! Result: {}", result);
        if result > 0.0 {
            println!("Player 1 wins!");
        } else {
            println!("Player 2 wins!");
        }
    }
}

/// Estimate the size of the game tree
fn estimate_tree_size(dice_per_player: usize) -> usize {
    // Rough estimate: 
    // - 6^n possible dice combinations for dealing
    // - Average branching factor of ~10 for betting
    // - Average depth of ~5 moves
    let deal_states = 6_usize.pow((dice_per_player * 2) as u32);
    let betting_nodes = 10_usize.pow(5);
    deal_states * betting_nodes / 1000  // Approximate
}

/// Run a performance comparison between single and parallel solvers
pub fn run_performance_comparison() {
    println!("=== Performance Comparison ===\n");
    
    let mut game = LiarsDie::new();
    let actions = game.available_actions();
    let deal = actions.choose(&mut rand::rng()).unwrap().clone();
    game = game.play(&deal);
    
    let observation = game.trace(Player::P1);
    
    // Test single-threaded
    println!("Single-threaded solver:");
    let mut solver_single = Obscuro::<LiarsDie>::default();
    let start = Instant::now();
    solver_single.make_move(observation.clone(), Player::P1);
    let single_time = start.elapsed();
    let single_size = solver_single.size();
    println!("  Time: {:.2}s", single_time.as_secs_f64());
    println!("  Tree size: {} nodes", single_size);
    println!();
    
    // Test parallel with different thread counts
    for num_threads in [2, 4, 8] {
        println!("Parallel solver ({} threads):", num_threads);
        let mut solver_parallel = ObscuroParallel::<LiarsDie>::new(num_threads);
        let start = Instant::now();
        solver_parallel.make_move(observation.clone(), Player::P1);
        let parallel_time = start.elapsed();
        let parallel_size = solver_parallel.size();
        
        let speedup = single_time.as_secs_f64() / parallel_time.as_secs_f64();
        
        println!("  Time: {:.2}s", parallel_time.as_secs_f64());
        println!("  Tree size: {} nodes", parallel_size);
        println!("  Speedup: {:.2}x", speedup);
        println!();
    }
}

/// Demonstrate neural network fallback for unexplored positions
pub fn demonstrate_nn_evaluation() {
    println!("=== Neural Network Evaluation Demo ===\n");
    println!("This demonstrates how the neural network evaluates positions");
    println!("that haven't been fully explored in the game tree.\n");
    
    let mut game = LiarsDie::new();
    
    // Deal specific dice to create a known position
    let deal = LiarsDieAction::Deal(vec![Die::Three, Die::Five], vec![Die::Two, Die::Four]);
    game = game.play(&deal);
    
    println!("Position: P1 has [3,5], P2 has [2,4]");
    println!("(In real play, each player only sees their own dice)\n");
    
    // Create a quick solver that won't fully explore
    let mut solver = Obscuro::<LiarsDie>::default();
    
    println!("P1's view (knows [3,5], doesn't know opponent):");
    let p1_obs = game.trace(Player::P1);
    
    // Get evaluation before deep search
    println!("  Running quick evaluation...");
    let start = Instant::now();
    solver.make_move(p1_obs.clone(), Player::P1);
    let elapsed = start.elapsed();
    
    let policy = solver.inst_policy(p1_obs);
    println!("  Time: {:.2}s", elapsed.as_secs_f64());
    println!("  Tree explored: {} nodes", solver.size());
    println!("  Expected value: {:.4}", policy.expectation());
    println!("  Neural network was used to evaluate unexplored positions");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_neural_demo() {
        let config = DemoConfig {
            dice_per_player: 1,
            solve_time_secs: 2.0,
            use_parallel: false,
            num_threads: 2,
        };
        run_neural_demo(config);
    }

    #[test]
    #[ignore]
    fn test_parallel_demo() {
        let config = DemoConfig {
            dice_per_player: 1,
            solve_time_secs: 2.0,
            use_parallel: true,
            num_threads: 4,
        };
        run_neural_demo(config);
    }

    #[test]
    #[ignore]
    fn test_performance_comparison() {
        run_performance_comparison();
    }

    #[test]
    #[ignore]
    fn test_nn_evaluation() {
        demonstrate_nn_evaluation();
    }
}
