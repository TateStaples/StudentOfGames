use StudentOfGames::games::rps::Rps;
use StudentOfGames::training::{Trainer, TrainingConfig};

fn main() {
    println!("🎮 Starting Rock-Paper-Scissors Training Example");
    
    let config = TrainingConfig {
        iterations: 20,
        greedy_depth: 5,
        replay_buffer_size: 50,
        checkpoint_frequency: 5,
    };
    
    let mut trainer = Trainer::<Rps>::new(config);
    trainer.train();
    
    println!("\n✅ Training complete!");
    println!("Trained solver is ready for use.");
}
