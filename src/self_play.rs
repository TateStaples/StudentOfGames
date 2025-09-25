use rand::prelude::IndexedRandom;
use crate::obscuro::Obscuro;
use crate::utils::{Game, Player, Reward};

/// Do self-learning by playing a game against yourself & updating your learning policies
fn self_play<G: Game>() {
    // TODO: alphazero has initial action distribution policy. Should I add something similar
    let mut game = G::new();
    let mut solver: Obscuro<G> = Obscuro::default();
    solver.study_position(game.trace(Player::P1), Player::P2);
    while !game.is_over() {
        // TODO: periodically add root games to replay buffer for study/learning
        let player = game.active_player();
        if player == Player::Chance {
            let action = game.available_actions().choose(&mut rand::rng()).unwrap().clone();
            println!("Randomly plays: {:?}", action);
            game = game.play(&action);
            continue;
        } 
        // TODO: you want to mix in uniform because you are still exploring
        // After a certain depth play greedy
        let action = solver.make_move(game.trace(player), player);
        println!("Bot({:?}) plays: {:?}", player, action);
        game = game.play(&action);
    }
}

/// Simple setup to let human player take either side in a game against the bot
pub fn interactive<G: Game>(human: Player) -> Reward {
    let computer = human.other();
    let mut game = G::new();
    let mut solver: Obscuro<G> = Obscuro::default();
    solver.study_position(game.trace(computer), computer);
    // solver.debug();
    while !game.is_over() {
        let actions = game.available_actions();
        match game.active_player() {
            Player::Chance => {
                // Randomly sample TODO: support non-uniform randomness
                let action = actions.choose(&mut rand::rng()).unwrap().clone();
                // println!("Randomly plays: {:?}", action);
                game = game.play(&action);
            }
            p if p == human => {
                // Ask for input by user typing in the index (usize) of the actions
                println!("Trace: {:?}, Available actions: {:?}", game.trace(p), actions);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).expect("Failed to read line");
                if let Ok(action_idx) = input.trim().parse::<usize>() {
                    let action = &actions[action_idx];
                    println!("Human plays: {:?}", action);
                    game = game.play(action);
                }
            }
            _ => {
                // Computer Plays
                let computer_trace = game.trace(computer);
                let action = solver.make_move(computer_trace, computer);
                println!("Computer plays: {:?}", action);
                game = game.play(&action);
                // I think the way the computer is sampling it's policy is wrong. It's also not stepping down correctly
            }
        }
    }
    println!("Evaluation: {}", game.evaluate());
    println!("{:?}", game);
    game.evaluate()
}
