use burn::backend::NdArray;
use StudentOfGames::games::liars_die::LiarsDie;
use rand::seq::IndexedRandom;
use StudentOfGames::obscuro::Obscuro;
use StudentOfGames::utils::{Game, Player, Reward};

fn game_loop<G: Game>(human: Player) -> Reward {
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

fn main() {
    // games::resources::test::main();
    type T = LiarsDie;
    game_loop::<T>(Player::P2);
    
    let mut reward = 0.0;
    let iters = 50;
    for _ in 0..iters {
        reward += game_loop::<T>(Player::P2);
    }
    println!("Average reward: {}", reward / iters as Reward);
    // let game = T::new();
    // let mut obscuro: Obscuro<T> = Obscuro::default();
    // obscuro.make_move(game.trace(Player::P1), Player::P1);
}

