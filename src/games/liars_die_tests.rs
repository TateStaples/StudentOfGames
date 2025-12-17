/// Tests for Liar's Die game comparing against snyd repository ground truth
/// 
/// The snyd repository (https://github.com/thomasahle/snyd) computes Nash equilibrium
/// values for Liar's Dice endgames using linear programming.
/// 
/// This test module verifies that our CFR-based solver converges to similar values.

#[cfg(test)]
mod tests {
    use crate::games::liars_die::{Die, LiarsDie, LiarsDieAction};
    use crate::obscuro::Obscuro;
    use crate::utils::{Game, Player, Reward};
    use rand::seq::IndexedRandom;
    use std::collections::HashMap;

    /// Ground truth values from snyd repository for 1v1 games with 6-sided dice
    /// Format: (mode, expected_value_for_p1)
    /// Values are fractions represented as floats
    fn get_ground_truth_values() -> HashMap<&'static str, Reward> {
        let mut values = HashMap::new();
        
        // 1v1 6-sided dice values from snyd
        // Value is from P1's perspective (P1 goes first)
        values.insert("11_normal", -1.0/9.0);  // -0.111...
        values.insert("11_joker", -7.0/327.0);   // -0.0214...
        values.insert("11_stairs", 0.0);         // Perfectly balanced
        
        // Additional configurations could be added here
        // values.insert("21_normal", 1.0/9.0);
        // values.insert("12_normal", -1.0/27.0);
        
        values
    }

    /// Test that a single game plays out without errors
    #[test]
    fn test_liars_die_game_completion() {
        let mut game = LiarsDie::new();
        let mut moves = 0;
        
        while !game.is_over() && moves < 100 {
            let actions = game.available_actions();
            assert!(!actions.is_empty(), "Game not over but no actions available");
            
            let action = actions.choose(&mut rand::rng()).unwrap();
            game = game.play(action);
            moves += 1;
        }
        
        assert!(game.is_over(), "Game should be over after random play");
        let result = game.evaluate();
        assert!(result.abs() == 1.0, "Terminal result should be +1 or -1, got {}", result);
    }

    /// Test that the game evaluates correctly for known outcomes
    #[test]
    fn test_liars_die_evaluation() {
        use crate::games::liars_die::LiarsDieAction::*;
        
        // Create a game with known dice
        let game = LiarsDie::new();
        let actions = game.available_actions();
        
        // Deal specific dice: P1 gets a Two, P2 gets a Three
        let deal_action = Deal(vec![Die::Two], vec![Die::Three]);
        assert!(actions.contains(&deal_action), "Deal action should be available");
        
        let game = game.play(&deal_action);
        
        // Now P1 makes a bid
        let bet_actions = game.available_actions();
        assert!(!bet_actions.is_empty());
        
        // P1 bids "1 Five" (one die showing 5)
        // This is a lie since neither die is a 5 (P1 has 2, P2 has 3)
        let raise = Raise(Die::Five, 1);
        if bet_actions.contains(&raise) {
            let game = game.play(&raise);
            
            // P2 calls bullshit
            let game = game.play(&BullShit);
            assert!(game.is_over());
            
            // P1's bid was incorrect (no fives on the table)
            // So P1 loses, result should be -1 (from P1's perspective)
            let result = game.evaluate();
            assert_eq!(result, -1.0, "P1 should lose when bid is incorrect");
        }
    }

    /// Test that CFR converges to reasonable values for small games
    /// This is a long-running test that may take several minutes
    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_cfr_convergence_1v1_joker() {
        let tolerance = 0.05; // Allow 5% error
        let ground_truth = get_ground_truth_values();
        let expected = ground_truth["11_joker"];
        
        // Run multiple games to estimate the value
        let num_games = 100;
        let mut total_reward = 0.0;
        
        for _ in 0..num_games {
            let reward = play_game_with_solver(Player::P1);
            total_reward += reward;
        }
        
        let average_reward = total_reward / num_games as Reward;
        
        println!("Expected value (from snyd): {:.6}", expected);
        println!("Computed value (from CFR): {:.6}", average_reward);
        println!("Difference: {:.6}", (average_reward - expected).abs());
        
        // Check if we're within tolerance
        assert!(
            (average_reward - expected).abs() < tolerance,
            "CFR value {:.6} differs from ground truth {:.6} by more than tolerance {}",
            average_reward,
            expected,
            tolerance
        );
    }

    /// Test that the solver can make reasonable moves
    #[test]
    fn test_solver_makes_moves() {
        let mut game = LiarsDie::new();
        let mut solver: Obscuro<LiarsDie> = Obscuro::default();
        
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
            "Solver chose invalid action: {:?}",
            action
        );
    }

    /// Helper function to play a complete game with the solver
    fn play_game_with_solver(human_player: Player) -> Reward {
        let computer = human_player.other();
        let mut game = LiarsDie::new();
        let mut solver: Obscuro<LiarsDie> = Obscuro::default();
        
        // Initial study for computer
        if computer == Player::P1 {
            solver.study_position(game.trace(computer), computer);
        }
        
        while !game.is_over() {
            let player = game.active_player();
            
            match player {
                Player::Chance => {
                    // Random chance actions (dealing dice)
                    let actions = game.available_actions();
                    let action = actions.choose(&mut rand::rng()).unwrap().clone();
                    game = game.play(&action);
                }
                p if p == human_player => {
                    // Human plays randomly for testing
                    let actions = game.available_actions();
                    let action = actions.choose(&mut rand::rng()).unwrap().clone();
                    game = game.play(&action);
                }
                _ => {
                    // Computer plays using solver
                    let observation = game.trace(computer);
                    let action = solver.make_move(observation, computer);
                    game = game.play(&action);
                }
            }
        }
        
        // Return reward from P1's perspective
        game.evaluate()
    }

    /// Test the basic game state transitions
    #[test]
    fn test_game_state_transitions() {
        use crate::games::liars_die::LiarsDieAction::*;
        
        let game = LiarsDie::new();
        
        // Initial state should have chance as active player (pre-deal)
        assert_eq!(game.active_player(), Player::Chance);
        
        // After dealing, P1 should be active
        let deal = Deal(vec![Die::Three], vec![Die::Five]);
        let game = game.play(&deal);
        assert_eq!(game.active_player(), Player::P1);
        
        // After P1 raises, P2 should be active
        let raise = Raise(Die::Two, 1);
        let game = game.play(&raise);
        assert_eq!(game.active_player(), Player::P2);
        
        // Game should not be over yet
        assert!(!game.is_over());
        
        // After P2 calls bullshit, game should be over
        let game = game.play(&BullShit);
        assert!(game.is_over());
    }

    /// Test that traces correctly represent player knowledge
    #[test]
    fn test_player_traces() {
        use crate::games::liars_die::LiarsDieAction::*;
        
        let game = LiarsDie::new();
        let deal = Deal(vec![Die::Three], vec![Die::Five]);
        let game = game.play(&deal);
        
        // P1 should know their own dice but not P2's
        let p1_trace = game.trace(Player::P1);
        assert_eq!(p1_trace.my_dice, vec![Die::Three]);
        
        // P2 should know their own dice but not P1's  
        let p2_trace = game.trace(Player::P2);
        assert_eq!(p2_trace.my_dice, vec![Die::Five]);
        
        // Both should see the same betting history (currently empty)
        assert_eq!(p1_trace.bet_history, p2_trace.bet_history);
        
        // After a raise, both should see the bet in their history
        let raise = Raise(Die::Two, 1);
        let game = game.play(&raise);
        
        let p1_trace = game.trace(Player::P1);
        let p2_trace = game.trace(Player::P2);
        
        assert_eq!(p1_trace.bet_history.len(), 1);
        assert_eq!(p2_trace.bet_history.len(), 1);
        assert_eq!(p1_trace.bet_history, vec![raise.clone()]);
    }

    /// Benchmark test to ensure reasonable performance
    #[test]
    #[ignore]
    fn test_performance() {
        use std::time::Instant;
        
        let start = Instant::now();
        let num_games = 10;
        
        for _ in 0..num_games {
            play_game_with_solver(Player::P2);
        }
        
        let duration = start.elapsed();
        let avg_time = duration.as_secs_f64() / num_games as f64;
        
        println!("Average time per game: {:.2}s", avg_time);
        
        // Should complete reasonably quickly
        assert!(
            avg_time < 30.0,
            "Games taking too long on average: {:.2}s",
            avg_time
        );
    }
}
