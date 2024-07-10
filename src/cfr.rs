use once_cell::sync::Lazy;

use crate::game::{Game, ImperfectGame};
use crate::policies::Prior;
use crate::types::{AbstractCounterfactual, AbstractPolicy, AbstractRange, Belief};

// CFR+ (populate SearchStatistics): belief down and counterfactual values (for given policy) up
// DeepStack: opp_range, reach prob, range, avg reach, values, regrets, avg_regrets [repeat]
pub(crate) fn cfr<'a, G: ImperfectGame + 'a, N: ImperfectNode<'a, G, Counterfactuals, Range, Policy>, P: Prior<G, Counterfactuals, Range, Policy>, Counterfactuals: AbstractCounterfactual, Range: AbstractRange, Policy: AbstractPolicy>
    (tree: &mut GameTree<'a, G, N>, node: &mut N, ranges: [Range; 2], prior: &P) -> Counterfactuals {
    // DeepStack order: opp_range √, strategy (reach probabilities), ranges, update avg_strat, terminal values, values, regrets, avg_values
    let evaluation = Lazy::new(|| {
        let belief: Belief<G, Range> = (node.public_state(), ranges.clone());
        let val = prior.eval(belief);
        val
    });
    // node.reset();
    
    // Note: DeepStack stores the average CFVs for later storage
    for (result, new_ranges) in node.iter_results(&ranges) { // FIXME: this doesn't allow for distinguishing terminal states
        // propagate search_stats back up
        match result {
            Some(NodeTransition::Edge(next)) => {
                let mut_next = tree.mut_node(next);
                let counterfactuals = cfr(tree, mut_next, new_ranges, prior);  // propagate down
                node.update_children(result, counterfactuals);  // TODO: update player details
            },
            Some(NodeTransition::Terminal(v)) => {
                node.update_children(result, Counterfactuals::outcome(*v));
            },
            None => {
                let (value, _) = evaluation.unwrap();
                node.update_children(None, value);
            }
            _ => {assert!(false, "Phantom data in transition map")}
        };
    }
    node.update_value();  // combine all values for calculation
    return node.value();
}