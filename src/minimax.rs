use std::cmp::Ordering;
use crate::game::Game;
use crate::game_tree::{GameTree, PrivateNode, NodeTransition};
use crate::types::Reward;

fn minimax<'a, G: Game, N: PrivateNode<'a, G>>(tree: &GameTree<'a, G, N>) -> (Reward, G::Action) {
    let root = tree.root();
    let game = tree.game();
    let depth = 0;
    let (outcome, action) = minimax_recursive(game, root, depth);
    (outcome, action)
}

fn minimax_recursive<'a, G: Game, N: PrivateNode<'a, G>>(game: G, root: &N, depth: usize) -> Reward {
    if depth > 10 || game.is_over() {
        return game.reward(game.player())
    }
    let maximizing = depth % 2 == 0;
    
    if maximizing {
        game.iter_actions().map(|a| {
            let (_, transition) = root.transition(1, 1, a.into());
            let mut new_state = game.clone();
            new_state.step(a);
            match transition {
                NodeTransition::Edge(next) => { minimax_recursive(new_state, next, depth + 1)},
                NodeTransition::Terminal(res) => *res,
                _ => {panic!("Minimaxing on incomplete tree")}
            }.min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
        }).unwrap_or(Reward::MIN)
    } else {
        game.iter_actions().map(|a| {
            let (_, transition) = root.transition(1, 1, a.into());
            let mut new_state = game.clone();
            new_state.step(a);
            match transition {
                NodeTransition::Edge(next) => { minimax_recursive(new_state, next, depth + 1)},
                NodeTransition::Terminal(res) => *res,
                _ => {panic!("Minimaxing on incomplete tree")}
            }.max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
        }).unwrap_or(Reward::MAX)
    }
    // todo!()
}