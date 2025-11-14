# Training API Integration

This guide shows how to use the new training system integrated into StudentOfGames.

## Basic Usage

```rust
use StudentOfGames::training::{Trainer, TrainingConfig};
use StudentOfGames::games::rps::Rps;

fn main() {
    // Configure training parameters
    let config = TrainingConfig {
        iterations: 100,           // Number of self-play games
        greedy_depth: 10,          // Depth before switching to greedy play
        replay_buffer_size: 1000,  // Max games to keep in memory
        checkpoint_frequency: 10,  // Save every N iterations
    };
    
    // Create and run trainer
    let mut trainer = Trainer::<Rps>::new(config);
    trainer.train();
    
    // Extract the trained solver
    let solver = trainer.into_solver();
}
```

## Architecture

### Components

1. **TrainingConfig**: Configurable hyperparameters for training
   - `iterations`: Number of self-play games to generate
   - `greedy_depth`: Exploration vs exploitation tradeoff
   - `replay_buffer_size`: Memory management for experience replay
   - `checkpoint_frequency`: How often to save progress

2. **Trainer**: Manages the training loop
   - Generates self-play games
   - Accumulates experience in replay buffer
   - Calls `learn_from()` on the solver
   - Handles checkpointing

3. **GameSolver trait**: Interface for learning algorithms
   - `score_position()`: Evaluate a position
   - `guess_strategy()`: Suggest a policy
   - `learn_from()`: Update from replay buffer

### Current Implementation

The current implementation uses **pure CFR** (Counterfactual Regret Minimization):
- Each iteration creates a fresh solver to avoid state conflicts
- Replay buffer accumulates experience for future neural network training
- `learn_from()` is currently a no-op (placeholder for future NN training)

### Future Enhancements

When neural networks are integrated:

1. **Value/Policy Networks**: 
   ```rust
   impl GameSolver for NeuralSolver {
       fn learn_from(&mut self, replay: ReplayBuffer<G>) {
           // Convert replay to training batches
           // Train value network on position evaluations
           // Train policy network on average strategies
       }
   }
   ```

2. **Persistent Learning**: Transfer knowledge between games instead of resetting

3. **Hybrid Approach**: Combine CFR with neural network guidance

## Examples

See `examples/train_example.rs` and `examples/akq_training.rs` for working examples.

## Benefits

✅ Clean separation of training logic from game implementation  
✅ Easy to configure and experiment with hyperparameters  
✅ Ready for neural network integration  
✅ Replay buffer management built-in  
✅ Checkpointing support for long training runs
