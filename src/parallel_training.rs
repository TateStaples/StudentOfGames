//! # Parallel Training Pipeline
//!
//! Multi-process training framework for efficiently training game-playing agents.
//! Distributes self-play and neural network training across multiple CPU cores.

use crate::obscuro::Obscuro;
use crate::self_play::self_play_with_solver;
use crate::utils::{Game, Player, ReplayBuffer};
use std::sync::mpsc;
use std::thread;

/// Configuration for parallel batch training
#[derive(Debug, Clone)]
pub struct ParallelTrainingConfig {
    /// Number of self-play games per batch
    pub batch_size: usize,
    /// Number of threads for parallel self-play
    pub num_threads: usize,
    /// Greedy depth for self-play decisions
    pub greedy_depth: i32,
    /// Total number of batches to train
    pub num_batches: usize,
    /// Time per move in seconds (higher = deeper thinking)
    pub solve_time_secs: f64,
}

impl Default for ParallelTrainingConfig {
    fn default() -> Self {
        Self {
            batch_size: 256,
            num_threads: 4,
            greedy_depth: 10,
            num_batches: 10,
            solve_time_secs: 1.0, // 1 second per move
        }
    }
}

/// Parallel trainer that collects batches of games and trains between batches
pub struct ParallelTrainer<G: Game> {
    config: ParallelTrainingConfig,
    solver: Obscuro<G>,
    batch_number: usize,
}

impl<G: Game + Send + 'static> ParallelTrainer<G>
where
    G::Trace: Send + Clone,
    G::Action: Send + Clone,
    G::Solver: Send,
{
    pub fn new(config: ParallelTrainingConfig) -> Self {
        Self {
            config,
            solver: Obscuro::default(),
            batch_number: 0,
        }
    }

    pub fn train(&mut self) {
        println!(
            "🚀 Starting parallel training with {} batches of {} games",
            self.config.num_batches, self.config.batch_size
        );
        println!(
            "   Threads: {}, Solve time: {:.1}s per move",
            self.config.num_threads, self.config.solve_time_secs
        );

        for batch_idx in 0..self.config.num_batches {
            self.batch_number = batch_idx;
            println!("\n📊 ===== BATCH {} =====" , batch_idx + 1);

            // Phase 1: Collect games in parallel with independent solvers per thread
            println!("🎮 Starting parallel self-play collection ({} threads)...", self.config.num_threads);
            let batch_data = self.collect_parallel_games();
            let total_experiences: usize = batch_data.iter().map(|b| b.len()).sum();
            println!("✅ Collected {} total experience tuples from {} games", 
                total_experiences, batch_data.len());

            // Phase 2: Combine all experiences and train shared solver
            println!("🧠 Consolidating {} experience buffers and training shared model...", batch_data.len());
            let combined_replay: ReplayBuffer<G> = batch_data
                .into_iter()
                .flatten()
                .collect();

            self.solver.learn_from(combined_replay);
            println!("✅ Batch {} training complete - solver updated with shared knowledge", batch_idx + 1);
        }

        println!("\n🎉 All batches complete! Model has learned from {} total games", 
            self.config.num_batches * self.config.batch_size);
    }

    /// Collect games in parallel using independent solvers per thread
    /// Each thread maintains its own Obscuro solver to avoid lock contention
    /// After collection, all experience is combined and used to train the shared solver
    fn collect_parallel_games(&self) -> Vec<ReplayBuffer<G>> {
        let games_per_thread = self.config.batch_size / self.config.num_threads;
        let (tx, rx) = mpsc::channel();
        let mut handles = vec![];

        // Pre-initialize a dummy solver to warm up neural network backend
        // This prevents race conditions when multiple threads initialize simultaneously
        {
            // CRITICAL: Initialize neural network in main thread BEFORE spawning workers
            // burn library spawns internal threads during init that aren't thread-safe
            println!("  Initializing neural network in main thread...");
            let mut warmup_solver: Obscuro<G> = Obscuro::default();
            // Force initialization by seeding an infoset (triggers neural network scoring)
            // Use a dummy game to get a valid trace
            let dummy_game = G::new();
            let dummy_trace = dummy_game.trace(Player::P1);
            warmup_solver.seed_infoset(dummy_trace, Player::P1, &[]);
            drop(warmup_solver);
            println!("  ✓ Neural network initialized and ready");
        }

        for thread_id in 0..self.config.num_threads {
            let tx = tx.clone();
            let greedy_depth = self.config.greedy_depth;
            let games_count = games_per_thread;

            let handle = thread::spawn(move || {
                println!("  Thread {} starting {} games (independent solver)", thread_id, games_count);
                
                // Each thread gets its own independent solver
                // The neural network will be lazily initialized with global lock protection
                let mut thread_solver: Obscuro<G> = Obscuro::default();
                let mut thread_batch = vec![];

                for game_num in 0..games_count {
                    // Play self-play game with this thread's independent solver
                    let replay = self_play_with_solver::<G>(greedy_depth, &mut thread_solver);
                    thread_batch.push(replay);

                    if (game_num + 1) % 10 == 0 {
                        println!("  Thread {} progress: {}/{} games", 
                            thread_id, game_num + 1, games_count);
                    }
                }

                println!("  Thread {} finished - sending {} experiences to trainer", 
                    thread_id, thread_batch.iter().map(|b| b.len()).sum::<usize>());
                let _ = tx.send(thread_batch);
            });

            handles.push(handle);
        }


        drop(tx); // Drop original sender so channel closes when all threads finish

        // Collect all results from threads
        let mut all_games = vec![];
        let mut thread_count = 0;
        for thread_batch in rx.iter() {
            thread_count += 1;
            println!("  ✓ Thread {} batch received ({} experiences)", thread_count - 1, 
                thread_batch.iter().map(|b| b.len()).sum::<usize>());
            all_games.extend(thread_batch);
        }

        // Wait for all threads to finish
        for handle in handles {
            let _ = handle.join();
        }

        all_games
    }

    pub fn get_solver(&self) -> &Obscuro<G> {
        &self.solver
    }

    pub fn get_solver_mut(&mut self) -> &mut Obscuro<G> {
        &mut self.solver
    }

    pub fn save_model<P: Into<std::path::PathBuf>>(
        &self,
        path: P,
    ) -> Result<(), burn::record::RecorderError>
    where
        G::Solver: crate::utils::SaveModel,
    {
        self.solver.save_model(path)
    }
}
