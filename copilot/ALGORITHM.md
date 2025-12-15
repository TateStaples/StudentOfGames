# Obscuro Algorithm Summary

## Algorithm Overview

Obscuro is a search algorithm for imperfect-information games that achieves strong performance without explicitly enumerating common-knowledge sets. This document provides a concise summary of the key algorithms.

## Main Algorithm Loop

```
procedure OBSCURO_MAKE_MOVE(observation, player):
    1. CONSTRUCT_SUBGAME(observation, player)
    2. while time_budget not exhausted:
        a. EXPANSION_STEP()
        b. for i = 1 to cfr_iterations_per_expansion:
            CFR_ITERATION(player)
    3. SELECT_ACTION(observation)
```

## 1. Subgame Construction (KLUSS)

**Knowledge-Limited Unfrozen Subgame Solving (KLUSS)**

```
procedure CONSTRUCT_SUBGAME(observation, player):
    Input: observation (current player's view), player (acting player)
    Output: Set of positions grouped by opponent information states

    1. old_histories = saved_tree_from_previous_move
    
    2. covered = K_COVER(old_histories, observation, player, k=3)
       // Find all histories relevant up to k-order knowledge
    
    3. Normalize reach probabilities in covered
    
    4. Group covered by opponent's information state
       positions = {}
       for each history in covered:
           opp_trace = history.trace(opponent)
           positions[opp_trace].append(history)
    
    5. if |positions| < min_positions:
           SAMPLE_ADDITIONAL_POSITIONS(positions, observation)
    
    6. Create resolver gadgets for each opponent info state:
       for each (opp_trace, histories) in positions:
           resolver = ResolverGadget(histories, alt_value, prob)
           resolvers.append(resolver)
    
    7. subgame_root = SubgameRoot(resolvers, player)
```

## 2. K-Cover Algorithm

**Core innovation: Reason about knowledge without enumerating common knowledge**

```
procedure K_COVER(histories, target_trace, player, k):
    Input: 
        - histories: Current game tree nodes
        - target_trace: The observation we want to match
        - player: Current player perspective
        - k: Depth of knowledge reasoning
    Output: Histories believed possible up to k-order knowledge

    search_traces = {target_trace}
    found = []
    
    for i = 1 to k:
        next_histories = []
        next_traces = {}
        
        for each history in histories:
            (returned, found_here, new_traces) = 
                K_COVER_RECURSIVE(history, search_traces, player)
            
            if returned is not None:
                next_histories.append(returned)
            found.extend(found_here)
            next_traces.extend(new_traces)
        
        histories = next_histories
        search_traces = next_traces
        player = player.other()
    
    return found

procedure K_COVER_RECURSIVE(history, target_traces, player):
    if history is terminal:
        return (history, [], {})
    
    my_trace = history.trace(player)
    
    // Check if this matches a target trace
    if my_trace == any target in target_traces:
        opponent_trace = history.trace(opponent)
        return (None, [history], {opponent_trace})
    
    // If this could lead to target (partial order check)
    if my_trace is comparable to targets:
        if history is Expanded:
            // Recursively explore children
            results = [K_COVER_RECURSIVE(child, targets, player) 
                      for each child]
            aggregate results
            return (updated_history, found, new_traces)
    
    return (history, [], {})
```

**Key Insight**: Instead of asking "is this in the common-knowledge set?" (which can be exponentially large), we ask "could the opponent believe we might be here?" Limited to k iterations keeps the search tractable.

## 3. Tree Expansion

**Growing-Tree CFR (GT-CFR)**

```
procedure EXPANSION_STEP(player):
    1. (resolver_idx, history_idx) = SAMPLE_HISTORY()
       // Sample from current tree based on reach probabilities
    
    2. history = subgame_root.get_history(resolver_idx, history_idx)
    
    3. EXPAND_TO_LEAF(history, player)

procedure EXPAND_TO_LEAF(history, target_player):
    while history is Expanded:
        infoset = get_or_create_infoset(history.trace)
        
        if history.player == target_player:
            action = infoset.policy.SELECT_EXPLORATION()
                     // UCB-like: balance value and exploration count
        else:
            action = infoset.policy.BEST_ACTION()
                     // Exploit current best
        
        infoset.policy.record_exploration(action)
        history = history.child(action)
    
    if history is Visited:
        EXPAND_NODE(history)
        // Create children for all legal actions

procedure SAMPLE_HISTORY():
    // Sample based on reach probabilities and maxmargin policy
    resolver_idx ~ maxmargin_policy
    history_idx ~ resolver.sampling_policy
    return (resolver_idx, history_idx)
```

## 4. CFR+ Iteration

**Counterfactual Regret Minimization Plus**

```
procedure CFR_ITERATION(optimizing_player):
    total_value = 0
    
    for each resolver_gadget:
        enter_value = 0
        
        for each history in resolver:
            // Build reach probability map
            reach_probs = {
                Chance: sampling_probability,
                opponent: resolver_prob * p_enter
            }
            
            // Recursive counterfactual value computation
            value = COMPUTE_CFR_VALUES(
                history, 
                optimizing_player, 
                reach_probs
            )
            
            enter_value += sampling_prob * value
        
        skip_value = resolver.alt_value
        
        // Update resolver policy
        resolver.policy.add_counterfactual(ENTER, enter_value)
        resolver.policy.add_counterfactual(SKIP, skip_value)
        resolver.policy.update(iteration)
        
        resolver_value = p_enter * enter_value + (1-p_enter) * skip_value
        
        maxmargin.add_counterfactual(resolver_idx, resolver_value)
        total_value += resolver_value * resolver.prior_prob
    
    maxmargin.update(iteration)
    return total_value

procedure COMPUTE_CFR_VALUES(history, opt_player, reach_probs):
    if history is Terminal:
        return history.payoff
    
    if history is Visited:
        return history.evaluation
    
    if history is Expanded:
        infoset = history.infoset
        strategy = infoset.policy.current_strategy()
        
        // Net reach = product of all except current player
        net_reach = product of reach_probs excluding history.player
        
        expected_value = 0
        
        for each (action, child) in history.children:
            action_prob = strategy[action]
            
            // Update reach for this action
            child_reach = reach_probs.copy()
            child_reach[history.player] *= action_prob
            
            // Recursive call
            value = COMPUTE_CFR_VALUES(child, opt_player, child_reach)
            
            expected_value += action_prob * value
            
            // Add regret if this is optimizing player's node
            if history.player == opt_player:
                infoset.policy.add_counterfactual(action, value, net_reach)
        
        infoset.policy.update(iteration)
        return expected_value
```

**Regret Matching+**: Policy is computed from cumulative regrets

```
procedure CURRENT_STRATEGY():
    positive_regrets = {action: max(0, cumulative_regret[action]) 
                       for each action}
    total = sum(positive_regrets.values())
    
    if total > 0:
        return {action: regret/total for each action}
    else:
        return uniform distribution
```

## 5. Safe Resolving

**Resolver Gadgets ensure exploitability guarantees**

```
At each opponent information state:
    
    Resolver Node (opponent's choice):
        - ENTER: Play in the subgame (use computed strategy)
        - SKIP: Take alternative value (from previous solution/eval)
    
    The resolver policy learns whether opponent would enter:
        p(ENTER) vs p(SKIP)
    
    Maxmargin policy at root:
        Accounts for opponent's choice
        Weighted by p_max = max resolver's p(ENTER)
```

This ensures that even if our subgame solving is approximate, the opponent can't exploit us by refusing to enter the subgame.

## 6. Action Selection

```
procedure SELECT_ACTION(observation):
    infoset = info_sets[observation]
    average_strategy = infoset.policy.average_strategy()
    
    // Can be deterministic (best) or stochastic (sample)
    return best_action(average_strategy)
```

## Key Properties

1. **No Common Knowledge Enumeration**: k-cover keeps only "nearby" states in knowledge hierarchy

2. **Anytime Algorithm**: Can be stopped at any time and return a valid strategy

3. **Safe Resolving**: Exploitability guarantees even with approximate solving

4. **Scalable**: Works on games much larger than poker (like FoW chess)

5. **Sound**: Converges to Nash equilibrium with sufficient computation

## Complexity

- **Space**: O(|I| + |Γ|) where I = sampled positions, Γ = built game tree
- **Time per iteration**: O(|Γ|) for CFR, O(1) for expansion
- **K-cover**: O(k × |old_tree|) but prunes aggressively

## Comparison to Prior Work

| Feature | Poker Solving | KLSS | KLUSS (Obscuro) |
|---------|--------------|------|------------------|
| Common knowledge | Enumerates | Prunes | Prunes |
| Strategy freezing | No | Yes (distance ≥ 1) | No (unfrozen) |
| Game size | Small-medium | Medium | Large |
| Example | Poker | - | FoW Chess |

## References

See `README.md` for full references and `../resources/obscuro.pdf` for complete details.
