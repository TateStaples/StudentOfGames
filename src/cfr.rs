use crate::game::Game;
use crate::game_tree::{Counterfactuals, GameTree, Node, NodeId, NodeTransition};
use crate::policies::Policy;
use crate::search_statistics::Range;

// CFR+ (populate SearchStatistics): belief down and counterfactual values (for given policy) up
pub(crate) fn cfr<G: Game, N:Node<G>, P: Policy<G, A, S>, const A: usize, const S: usize>(tree: &mut GameTree<G, N>, node_id: NodeId, ranges: [Range; 2], prior: &P) -> Counterfactuals {
    // DeepStack order: opp_range √, strategy (reach probabilities), ranges, update avg_strat, terminal values, values, regrets, avg_values
    let node = tree.mut_node(node_id);
    // TODO: clear the search statistics (maybe leave a base value to improve convergence)
    let evaluation = if node.leaf() { Some(prior((node.public_state(), ranges))) } else { None };
    // r(s,a) = v(s,a) - EV(policy)
    // Q(s,a) += r(s,a) [min value of 0]
    // π(s,a) = percentage of Q
    // Note: DeepStack stores the average CFVs for later storage
    // propagate the belief down
    for (result, new_ranges, cases) in node.iter_results(&ranges) {
        // propagate search_stats back up
        match result {
            NodeTransition::Edge(id) => {
                let counterfactuals = cfr(tree, id, new_ranges, &prior);
                for (state, next_state, action, probability) in cases {
                    let value = counterfactuals.get(next_state).expect("Transfer to unknown state"); // TODO: figure out the type
                    node.update_action_quality(state, action, value, probability)
                }
            },
            NodeTransition::Terminal(v) => {
                for (state, action, probability) in cases {
                    node.update_action_quality(state, action, v, probability)
                }
            }
            NodeTransition::Undefined => {
                let (value, _) = evaluation.unwrap();
                for (state, next_state, action, probability) in cases {
                    let value = value.get(next_state).expect("Transfer to unknown state");
                    node.update_action_quality(state, action, value, probability)
                }
            }
        };
    }
    // TODO: normalize the regret to be min of 0
}