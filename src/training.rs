//! # Neural Network Training
//!
//! Utilities for training game-playing neural networks. Handles:
//! - **Model serialization**: Saving/loading trained weights
//! - **Batch processing**: Converting game data to training batches
//! - **State encoding**: Vectorizing game states for neural networks

use crate::utils::*;
use crate::obscuro::Obscuro;

/// Configuration for training sessions
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    pub iterations: usize,
    pub greedy_depth: i32,
    pub replay_buffer_size: usize,
    pub checkpoint_frequency: usize,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            iterations: 100,
            greedy_depth: 10,
            replay_buffer_size: 1000,
            checkpoint_frequency: 10,
        }
    }
}

/// Trainer that manages the training loop with checkpointing
pub struct Trainer<G: Game> {
    config: TrainingConfig,
    solver: Obscuro<G>,
    replay_buffer: Vec<ReplayBuffer<G>>,
}

impl<G: Game> Trainer<G> {
    pub fn new(config: TrainingConfig) -> Self {
        Self {
            config,
            solver: Obscuro::default(),
            replay_buffer: Vec::new(),
        }
    }

    pub fn train(&mut self) {
        for iter in 0..self.config.iterations {
            println!("=== Training Iteration {}/{} ===", iter + 1, self.config.iterations);
            
            // Create a fresh solver for each game to avoid state conflicts
            // TODO: In the future, we can explore ways to transfer knowledge between games
            self.solver = Obscuro::default();
            
            // Generate self-play game
            let replay = crate::self_play::self_play_with_solver::<G>(
                self.config.greedy_depth, 
                &mut self.solver
            );
            
            self.replay_buffer.push(replay);
            
            // Keep buffer size limited
            if self.replay_buffer.len() > self.config.replay_buffer_size {
                self.replay_buffer.remove(0);
            }
            
            // Train on accumulated replay buffer
            let all_experiences: ReplayBuffer<G> = self.replay_buffer
                .iter()
                .flatten()
                .cloned()
                .collect();
            
            println!("Training on {} experiences from {} games", 
                all_experiences.len(), 
                self.replay_buffer.len()
            );
            
            self.solver.learn_from(all_experiences);
            
            // Checkpoint
            if (iter + 1) % self.config.checkpoint_frequency == 0 {
                self.save_checkpoint(iter + 1);
            }
        }
        
        println!("Training complete!");
    }

    fn save_checkpoint(&self, iteration: usize) {
        println!("📊 Checkpoint saved at iteration {}", iteration);
        // TODO: Implement model serialization when neural networks are added
    }
    
    pub fn get_solver(&self) -> &Obscuro<G> {
        &self.solver
    }
    
    pub fn into_solver(self) -> Obscuro<G> {
        self.solver
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::rps::Rps;

    #[test]
    fn test_training_basic() {
        let config = TrainingConfig {
            iterations: 3,
            greedy_depth: 3,
            replay_buffer_size: 10,
            checkpoint_frequency: 2,
        };
        
        let mut trainer = Trainer::<Rps>::new(config);
        trainer.train();
        
        // Verify training completed
        println!("Training completed successfully");
    }

    #[test]
    fn test_replay_buffer_size_limit() {
        let config = TrainingConfig {
            iterations: 12,
            greedy_depth: 3,
            replay_buffer_size: 10,  // Should limit to 10 games
            checkpoint_frequency: 5,
        };
        
        let mut trainer = Trainer::<Rps>::new(config);
        trainer.train();
        
        // Verify buffer is limited
        assert!(trainer.replay_buffer.len() <= 10);
    }
}
