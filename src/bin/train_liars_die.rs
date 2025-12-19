/// Train the Liar's Die heuristic via self-play (5 dice per player).
///
/// Now includes parallel training mode for better performance!
/// 
/// Usage (standard sequential):
///   cargo run --release --bin train_liars_die [iterations] [greedy_depth]
///
/// Usage (parallel batching mode with --parallel flag):
///   cargo run --release --bin train_liars_die -- --parallel [num_batches] [batch_size] [num_threads]
///
/// Defaults (sequential): iterations=10, greedy_depth=6
/// Defaults (parallel): num_batches=5, batch_size=256, num_threads=4
///
/// Thinking times:
///   - Interactive play (vs human): 5 seconds per move
///   - Self-play training: No explicit delay (fast computation)
use std::env;
use StudentOfGames::games::liars_die::LiarsDie;
use StudentOfGames::self_play::student_of_games;
use StudentOfGames::parallel_training::{ParallelTrainer, ParallelTrainingConfig};

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check if using parallel mode
    if args.len() > 1 && args[1] == "--parallel" {
        run_parallel_training(&args[2..]);
    } else {
        run_sequential_training(&args[1..]);
    }
}

fn run_sequential_training(args: &[String]) {
    let iterations: i32 = args.get(0).and_then(|s| s.parse().ok()).unwrap_or(10);
    let greedy_depth: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(6);

    println!("🎲 Training Liar's Die heuristic via self-play (Sequential Mode)");
    println!("  Dice per player: 5");
    println!("  Iterations: {}", iterations);
    println!("  Greedy depth: {}", greedy_depth);
    println!("  💡 For parallel training, use: --parallel [num_batches] [batch_size] [num_threads]\n");

    let solver = student_of_games::<LiarsDie>(iterations, greedy_depth);

    // Save the trained model if supported
    match solver.save_model("liars_die_trained.burn") {
        Ok(_) => println!("✅ Training run complete. Model saved to liars_die_trained.burn"),
        Err(e) => println!("⚠️ Training complete, but failed to save model: {e}"),
    }
}

fn run_parallel_training(args: &[String]) {
    let num_batches: usize = args.get(0).and_then(|s| s.parse().ok()).unwrap_or(5);
    let batch_size: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(256);
    let num_threads: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);

    println!("🎲 Training Liar's Die heuristic via Parallel Self-Play");
    println!("════════════════════════════════════");
    println!("  Dice per player: 5");
    println!("  Batches: {}", num_batches);
    println!("  Games per batch: {}", batch_size);
    println!("  Parallel threads: {}", num_threads);
    println!("  Solve time per move: 30.0s (deep thinking)");
    println!("════════════════════════════════════\n");

    let config = ParallelTrainingConfig {
        batch_size,
        num_threads,
        greedy_depth: 10,
        num_batches,
        solve_time_secs: 30.0,
    };

    let mut trainer = ParallelTrainer::<LiarsDie>::new(config);
    trainer.train();

    // Save the trained model if supported
    match trainer.save_model("liars_die_parallel_trained.burn") {
        Ok(_) => println!("✅ Training complete. Model saved to liars_die_parallel_trained.burn"),
        Err(e) => println!("⚠️ Training complete, but failed to save model: {e}"),
    }
}

