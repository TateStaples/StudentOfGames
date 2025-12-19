/// Test parallel training with AKQ game (non-neural solver)
use StudentOfGames::games::AKQ::Akq;
use StudentOfGames::parallel_training::{ParallelTrainer, ParallelTrainingConfig};

fn main() {
    let config = ParallelTrainingConfig {
        batch_size: 16,
        num_threads: 8,
        greedy_depth: 3,
        num_batches: 2,
        solve_time_secs: 5.0,  // Faster for testing
    };

    let mut trainer = ParallelTrainer::<Akq>::new(config);
    
    println!("🃏 Testing parallel training with AKQ (simple poker variant)");
    println!("   This uses DummySolver (no neural networks) - should work perfectly!\n");
    
    trainer.train();
    
    println!("\n✅ Parallel training test completed successfully!");
}
