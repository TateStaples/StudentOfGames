/// Train Liar's Die with parallel self-play and batch training.
///
/// This training pipeline:
/// 1. Spawns multiple threads for parallel self-play games
/// 2. Collects 256 games per batch
/// 3. Trains the model on each batch
/// 4. Repeats for multiple batches
/// 5. Each player gets 30 seconds per move (deep thinking time)
///
/// Usage:
///   cargo run --release --bin train_parallel -- [num_batches] [batch_size] [num_threads]
///
/// Defaults: num_batches=5, batch_size=256, num_threads=4
use std::env;
use StudentOfGames::games::liars_die::LiarsDie;
use StudentOfGames::parallel_training::{ParallelTrainer, ParallelTrainingConfig};

fn main() {
    let args: Vec<String> = env::args().collect();
    let num_batches: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
    let batch_size: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(256);
    let num_threads: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);

    println!("🎲 Parallel Training for Liar's Die");
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
