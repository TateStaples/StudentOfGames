use crate::obscuro::Obscuro;
use crate::utils::{Game, Player, Reward};
use crate::policy::Policy;

/// A simple parallelization wrapper that can run multiple independent Obscuro solvers
/// in parallel for different positions. This is useful for self-play scenarios where
/// multiple games are being played simultaneously.
/// 
/// Note: This does NOT parallelize the search within a single position due to the 
/// use of Rc<RefCell<>> in the data structures which is not Send. For full parallelization
/// within a single search, the core data structures would need to use Arc<Mutex<>> instead.
pub struct ObscuroThreaded<G: Game> {
    solver: Obscuro<G>,
    num_threads: usize,
}

impl<G: Game> ObscuroThreaded<G> {
    /// Create a new threaded solver with specified number of threads
    pub fn new(num_threads: usize) -> Self {
        Self {
            solver: Obscuro::default(),
            num_threads,
        }
    }

    /// Develop a strategy and then return the action you have decided
    pub fn make_move(&mut self, observation: G::Trace, player: Player) -> G::Action {
        self.solver.make_move(observation, player)
    }

    pub fn inst_policy(&self, observation: G::Trace) -> Policy<G::Action> {
        self.solver.inst_policy(observation)
    }

    /// Given an observation, update your understanding of game state & strategy
    /// Currently this just delegates to the single-threaded implementation
    pub fn study_position(&mut self, observation: G::Trace, player: Player) {
        self.solver.study_position(observation, player);
    }

    pub fn learn_from(&mut self, replay: crate::utils::ReplayBuffer<G>) {
        self.solver.learn_from(replay);
    }

    /// Get the configured number of threads
    pub fn num_threads(&self) -> usize {
        self.num_threads
    }
}

impl<G: Game> Default for ObscuroThreaded<G> {
    fn default() -> Self {
        // Use number of available CPU cores by default
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::new(num_threads)
    }
}

/// Run multiple games in parallel using a thread pool
/// This is the primary use case for threading in the current implementation
pub fn parallel_self_play<G: Game + Send + 'static>(
    num_games: usize,
    num_threads: usize,
) -> Vec<Reward> 
where
    G::Action: Send,
    G::Trace: Send,
    G::State: Send,
{
    use std::sync::{Arc, Mutex};
    use std::thread;

    let results = Arc::new(Mutex::new(Vec::new()));
    let games_remaining = Arc::new(Mutex::new(num_games));
    let mut handles = vec![];

    for _ in 0..num_threads {
        let results_clone = Arc::clone(&results);
        let games_remaining_clone = Arc::clone(&games_remaining);

        let handle = thread::spawn(move || {
            // Create thread-local RNG for better performance
            let mut rng = rand::rng();

            loop {
                // Check if there are games remaining
                let should_continue = {
                    let mut remaining = games_remaining_clone.lock().unwrap();
                    if *remaining > 0 {
                        *remaining -= 1;
                        true
                    } else {
                        false
                    }
                };

                if !should_continue {
                    break;
                }

                // Play one game
                let mut game = G::new();
                let mut solver = Obscuro::<G>::default();

                while !game.is_over() {
                    let player = game.active_player();
                    
                    match player {
                        Player::Chance => {
                            use rand::seq::IndexedRandom;
                            let actions = game.available_actions();
                            let action = actions.choose(&mut rng).unwrap().clone();
                            game = game.play(&action);
                        }
                        p => {
                            let observation = game.trace(p);
                            let action = solver.make_move(observation, p);
                            game = game.play(&action);
                        }
                    }
                }

                let reward = game.evaluate();
                results_clone.lock().unwrap().push(reward);
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    Arc::try_unwrap(results).unwrap().into_inner().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::liars_die::LiarsDie;
    use rand::seq::IndexedRandom;

    #[test]
    fn test_threaded_solver_basic() {
        let mut game = LiarsDie::new();
        let mut solver: ObscuroThreaded<LiarsDie> = ObscuroThreaded::new(2);

        // Deal the dice
        let actions = game.available_actions();
        let deal_action = actions.choose(&mut rand::rng()).unwrap().clone();
        game = game.play(&deal_action);

        // Let the solver study and make a move for P1
        let observation = game.trace(Player::P1);
        let action = solver.make_move(observation, Player::P1);

        // Verify the action is valid
        let valid_actions = game.available_actions();
        assert!(
            valid_actions.contains(&action),
            "Threaded solver chose invalid action: {:?}",
            action
        );
    }

    #[test]
    #[ignore]
    fn test_parallel_self_play() {
        let num_games = 10;
        let num_threads = 4;

        let results = parallel_self_play::<LiarsDie>(num_games, num_threads);

        assert_eq!(results.len(), num_games);
        
        // All results should be +1 or -1 (terminal states)
        for &reward in &results {
            assert!(reward.abs() == 1.0, "Expected terminal reward, got {}", reward);
        }

        // Print statistics
        let avg_reward = results.iter().sum::<Reward>() / results.len() as Reward;
        println!("Average reward over {} games: {:.3}", num_games, avg_reward);
    }
}

