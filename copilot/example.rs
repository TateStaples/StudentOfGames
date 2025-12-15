// example.rs - Example usage of the Obscuro implementation
//
// This file demonstrates how to use the Obscuro algorithm with a simple game.

use std::collections::HashMap;

// Note: In a real implementation, you would import from the obscuro module
// For this example, we show the structure without actual imports

/*
use obscuro::{Game, Player, Obscuro, ObscuroConfig, Reward, Probability};

/// Example: Rock-Paper-Scissors with imperfect information
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum RPSAction {
    Rock,
    Paper,
    Scissors,
}

#[derive(Clone, Debug)]
struct RPSGame {
    p1_action: Option<RPSAction>,
    p2_action: Option<RPSAction>,
    current_player: Player,
}

impl Game for RPSGame {
    type State = (Option<RPSAction>, Option<RPSAction>);
    type Action = RPSAction;
    type Observation = ();
    type Trace = u8;

    fn new() -> Self {
        RPSGame {
            p1_action: None,
            p2_action: None,
            current_player: Player::P1,
        }
    }

    fn active_player(&self) -> Player {
        self.current_player
    }

    fn is_terminal(&self) -> bool {
        self.p1_action.is_some() && self.p2_action.is_some()
    }

    fn payoff(&self) -> Reward {
        match (&self.p1_action, &self.p2_action) {
            (Some(a1), Some(a2)) => {
                if a1 == a2 {
                    0.0 // Tie
                } else if (matches!(a1, RPSAction::Rock) && matches!(a2, RPSAction::Scissors))
                    || (matches!(a1, RPSAction::Paper) && matches!(a2, RPSAction::Rock))
                    || (matches!(a1, RPSAction::Scissors) && matches!(a2, RPSAction::Paper))
                {
                    1.0 // P1 wins
                } else {
                    -1.0 // P2 wins
                }
            }
            _ => 0.0,
        }
    }

    fn legal_actions(&self) -> Vec<Self::Action> {
        if self.is_terminal() {
            vec![]
        } else {
            vec![RPSAction::Rock, RPSAction::Paper, RPSAction::Scissors]
        }
    }

    fn apply_action(&self, action: &Self::Action) -> Self {
        let mut new_game = self.clone();
        match self.current_player {
            Player::P1 => {
                new_game.p1_action = Some(action.clone());
                new_game.current_player = Player::P2;
            }
            Player::P2 => {
                new_game.p2_action = Some(action.clone());
            }
            _ => {}
        }
        new_game
    }

    fn get_observation(&self, player: Player) -> Self::Observation {
        // RPS has no observations (simultaneous play)
        ()
    }

    fn trace(&self, player: Player) -> Self::Trace {
        // Simple trace: 0 at root, 1 after P1 plays, 2 after P2 plays
        if self.is_terminal() {
            2
        } else if self.p1_action.is_some() {
            1
        } else {
            0
        }
    }

    fn identifier(&self) -> u64 {
        // Simple hash based on state
        let p1 = match &self.p1_action {
            None => 0,
            Some(RPSAction::Rock) => 1,
            Some(RPSAction::Paper) => 2,
            Some(RPSAction::Scissors) => 3,
        };
        let p2 = match &self.p2_action {
            None => 0,
            Some(RPSAction::Rock) => 1,
            Some(RPSAction::Paper) => 2,
            Some(RPSAction::Scissors) => 3,
        };
        (p1 * 10 + p2) as u64
    }

    fn evaluate(&self) -> Reward {
        if self.is_terminal() {
            self.payoff()
        } else {
            0.0 // Unknown at non-terminal states
        }
    }

    fn sample_positions(trace: Self::Trace) -> Box<dyn Iterator<Item = Self>> {
        // For RPS, just enumerate all possible positions at this trace
        Box::new(std::iter::empty()) // Simplified
    }

    fn encode(&self) -> Self::State {
        (self.p1_action.clone(), self.p2_action.clone())
    }

    fn decode(state: &Self::State) -> Self {
        RPSGame {
            p1_action: state.0.clone(),
            p2_action: state.1.clone(),
            current_player: if state.0.is_none() {
                Player::P1
            } else {
                Player::P2
            },
        }
    }
}

fn main() {
    println!("=== Obscuro Example: Rock-Paper-Scissors ===\n");

    // Create Obscuro with custom configuration
    let config = ObscuroConfig {
        time_per_move: 1.0,  // 1 second per move
        min_positions: 10,
        k_depth: 2,
        cfr_iterations_per_expansion: 5,
        max_tree_size: 10000,
    };

    let mut obscuro = Obscuro::<RPSGame>::with_config(config);

    // Play a game
    let mut game = RPSGame::new();

    println!("Player 1's turn:");
    let trace_p1 = game.trace(Player::P1);
    let action_p1 = obscuro.make_move(trace_p1, Player::P1);
    println!("  Obscuro chose: {:?}", action_p1);
    game = game.apply_action(&action_p1);

    println!("\nPlayer 2's turn:");
    let trace_p2 = game.trace(Player::P2);
    let action_p2 = obscuro.make_move(trace_p2, Player::P2);
    println!("  Obscuro chose: {:?}", action_p2);
    game = game.apply_action(&action_p2);

    println!("\nGame result:");
    println!("  P1 played: {:?}", game.p1_action);
    println!("  P2 played: {:?}", game.p2_action);
    println!("  Payoff (P1 perspective): {}", game.payoff());

    // Get statistics
    let stats = obscuro.get_stats();
    println!("\n{}", stats);

    println!("\n=== After training, the strategy should converge to uniform (1/3, 1/3, 1/3) ===");
    println!("This is the Nash equilibrium for Rock-Paper-Scissors");
}
*/

fn main() {
    println!("=== Obscuro Example ===");
    println!();
    println!("This example file shows the structure of how to use Obscuro.");
    println!("To use it with a real game:");
    println!("  1. Implement the Game trait for your game");
    println!("  2. Create an Obscuro instance");
    println!("  3. Call make_move() for each decision point");
    println!();
    println!("See the commented code above for a complete Rock-Paper-Scissors example.");
    println!();
    println!("Key components:");
    println!("  - Game trait: Defines game interface (actions, observations, traces)");
    println!("  - Obscuro: Main search algorithm");
    println!("  - ObscuroConfig: Configuration parameters");
    println!("  - study_position(): Builds and solves subgame");
    println!("  - make_move(): Returns action from computed strategy");
}
