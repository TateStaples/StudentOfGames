use StudentOfGames::games::AKQ::Akq;
use StudentOfGames::training::{Trainer, TrainingConfig};
use StudentOfGames::self_play::interactive;
use StudentOfGames::utils::Player;

fn main() {
    println!("🃏 AKQ Poker Training Demo");
    println!("=========================\n");
    
    // Train the solver
    let config = TrainingConfig {
        iterations: 10,
        greedy_depth: 5,
        replay_buffer_size: 20,
        checkpoint_frequency: 5,
    };
    
    println!("Training solver for {} iterations...", config.iterations);
    let mut trainer = Trainer::<Akq>::new(config);
    trainer.train();
    
    println!("\n🎯 Training Statistics:");
    println!("  - Total iterations: {}", 10);
    println!("  - Solver has been trained on accumulated self-play experience");
    
    // Ask user if they want to play
    println!("\n Would you like to play against the trained bot? (y/n)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read line");
    
    if input.trim().eq_ignore_ascii_case("y") {
        println!("\n🎮 Starting interactive game!");
        println!("You are Player 1. Enter the number of the action you want to take.");
        let result = interactive::<Akq>(Player::P1);
        println!("\n Final result: {}", result);
    } else {
        println!("Thanks for training! Goodbye 👋");
    }
}
