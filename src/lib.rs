mod alpha_zero;
mod helpers{
    mod config;
    mod data;
    mod evaluator;
    pub mod prelude;
    pub mod utils;
}

mod examples {
    mod chess {}
    mod connect4 {
        mod connect4;
        // mod main;
        mod policies;
    }
    mod go {}
    mod poker {}
    mod rps {}
    mod tictactoe {
        mod tictactoe;
    }
}

pub mod game;
mod mcts;
pub mod policies;
mod gt_cfr;
mod game_tree;
mod search_statistics;
mod student_of_games;
mod cfr;
