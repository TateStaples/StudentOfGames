/// Binary demo for neural network evaluation in large game trees
use StudentOfGames::neural_demo::{run_neural_demo, DemoConfig, run_performance_comparison};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "perf" {
        // Run performance comparison
        run_performance_comparison();
    } else {
        // Run neural network demo
        let config = if args.len() > 1 && args[1] == "parallel" {
            DemoConfig {
                dice_per_player: 1,
                solve_time_secs: 5.0,
                use_parallel: true,
                num_threads: 4,
            }
        } else {
            DemoConfig {
                dice_per_player: 1,
                solve_time_secs: 5.0,
                use_parallel: false,
                num_threads: 1,
            }
        };
        
        run_neural_demo(config);
    }
}
