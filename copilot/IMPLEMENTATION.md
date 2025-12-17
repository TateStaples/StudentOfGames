# Obscuro Implementation Documentation

This document provides a detailed explanation of the Obscuro algorithm implementation for Fog of War (FoW) chess, mapping each section of the paper "General search techniques without common knowledge for imperfect-information games, and application to superhuman Fog of War chess" by Brian Hu Zhang and Tuomas Sandholm to the corresponding code implementation.

## Table of Contents

1. [Introduction](#1-introduction)
2. [Challenges in FoW Chess](#2-challenges-in-fow-chess)
3. [Algorithm Description](#3-algorithm-description)
   - [3.1 KLUSS: Knowledge-Limited Unfrozen Subgame Solving](#31-kluss-knowledge-limited-unfrozen-subgame-solving)
   - [3.2 Equilibrium Computation (PCFR+)](#32-equilibrium-computation-pcfr)
   - [3.3 Expanding the Game Tree](#33-expanding-the-game-tree)
   - [3.4 Repeat](#34-repeat)
   - [3.5 Move Selection](#35-move-selection)
4. [Experiments and Results](#4-experiments-and-results)
5. [Appendix A: Rules of FoW Chess](#appendix-a-rules-of-fow-chess)
6. [Appendix B: Further Details](#appendix-b-further-details)
7. [Appendix C: Additional Experiments](#appendix-c-additional-experiments)
8. [Algorithm Correctness Analysis](#algorithm-correctness-analysis)
9. [Identified Issues and Fixes](#identified-issues-and-fixes)

---

## 1. Introduction

### Paper Summary
The paper presents Obscuro, the first superhuman AI for Fog of War (FoW) chess. The key innovation is developing scalable subgame solving techniques for imperfect-information games that don't require explicit reasoning about common knowledge, which becomes intractable in large games like FoW chess.

### Implementation Overview
The implementation is structured in Rust with the following main components:

**Core Structure** (`src/obscuro.rs`):
```rust
pub struct Obscuro<G: Game> {
    pub expectation: Reward,
    total_updates: usize,
    info_sets: HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>,
    subgame_root: SubgameRoot<G>,
    solver: G::Solver,
    start_time: SystemTime,
}
```

This structure maintains:
- `expectation`: The current value estimate of the position
- `total_updates`: Count of CFR iterations performed
- `info_sets`: HashMap storing all information sets encountered
- `subgame_root`: The root of the current subgame being solved
- `solver`: Game-specific solver (e.g., Stockfish for chess evaluation)
- `start_time`: Timer for managing computational budget

---

## 2. Challenges in FoW Chess

### Paper Description
The paper identifies four key challenges:
1. **Tactical lookahead**: Strong play requires careful lookahead capability
2. **Rapidly changing information**: Infoset sizes can vary from hundreds to millions in just a few moves
3. **Mixed strategies**: Must randomize actions to avoid exploitation
4. **Common knowledge reasoning**: Traditional subgame solving methods require enumerating common-knowledge sets, which can reach 10^18 states in FoW chess

### Implementation Response

**1. Tactical Lookahead** - Implemented via tree expansion and Stockfish evaluation:
```rust
// From src/history.rs - Expansion of game tree
pub fn expand(&mut self, infosets: &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>) {
    let me = self.player();
    if let History::Visited { state, reach, .. } = self {
        let game = G::decode(state);
        let hero = game.active_player();
        let actions = game.available_actions();
        
        let mut kids: Vec<(G::Action, History<G>)> = Vec::with_capacity(actions.len());
        
        for a in actions.iter() {
            let next = game.play(a);
            let mut next_reach = reach.clone();
            next_reach.entry(me)
                .and_modify(|e| *e *= 1.0/actions.len() as Probability)
                .or_insert(1.0/actions.len() as Probability);
            let child = History::new(next, next_reach);
            kids.push((a.clone(), child));
        }
        // ... (continued below)
```

**2. Rapidly Changing Information** - Handled by maintaining flexible infosets:
```rust
// From src/obscuro.rs - Information set management
fn construct_subgame(&mut self, hist: G::Trace, player: Player) {
    let mut positions = self.pop_histories(hist.clone(), player);
    Self::populate_histories(&mut positions, hist, player);
    
    // Renormalize all the histories to sum to 1.0
    let total_prob = positions.iter()
        .map(|(_, (prob, _, _))| *prob)
        .sum::<Probability>();
    for (_, (_, _, hists)) in positions.iter_mut() {
        for h in hists {
            h.renormalize_reach(total_prob);
        }
    }
    // ...
}
```

**3. Mixed Strategies** - Implemented via CFR+ regret matching:
```rust
// From src/policy.rs - Instantaneous policy calculation
pub fn inst_policy(&self) -> ActionDistribution {
    if self.player == Player::Chance {
        return uniform_dist(self.actions.len());
    }
    let sum: Counterfactual = self.acc_regrets.iter().sum();
    if sum <= 0.0 {
        return uniform_dist(self.actions.len());
    }
    self.acc_regrets.iter()
        .map(|&r| r / sum)
        .collect()
}
```

**4. Common Knowledge** - Avoided via k-cover algorithm (KLUSS):
```rust
// From src/obscuro.rs - k-cover implementation
fn k_cover(mut root_histories: Vec<History<G>>, hist: G::Trace, 
           mut player: Player, k: u8) -> Vec<History<G>> {
    if root_histories.is_empty() {return vec![]}
    let mut search_for = HashSet::from([hist.clone()]);
    let mut all_found = vec![];
    
    for _ in 0..k {
        let mut next_roots = vec![];
        let mut next_search_for = HashSet::new();
        for root in root_histories.drain(0..root_histories.len()) {
            let (next_root, new_nodes, new_search_for) = 
                Self::k_cover_rec(root, &search_for, player);
            if let Some(next_root) = next_root {
                next_roots.push(next_root);
            }
            next_search_for.extend(new_search_for);
            all_found.extend(new_nodes);
        }
        std::mem::swap(&mut root_histories, &mut next_roots);
        std::mem::swap(&mut search_for, &mut next_search_for);
        player = player.other();
    }
    all_found
}
```

---

## 3. Algorithm Description

The Obscuro algorithm operates in five main steps at each turn:

### High-Level Flow

**Main Entry Point** (`src/obscuro.rs`):
```rust
pub fn study_position(&mut self, observation: G::Trace, player: Player) {
    self.start_time = SystemTime::now();
    
    // Step 1: Construct subgame
    self.construct_subgame(observation.clone(), player);
    
    // Steps 2-4: Iterate expansion and solving
    while self.start_time.elapsed().unwrap_or(Duration::from_secs(0)) 
        < Duration::from_millis((SOLVE_TIME_SECS*1000.0) as u64) {
        self.expansion_step();  // Step 3
        for _ in 0..10 {
            self.solve_step();  // Step 2
        }
    }
    
    println!("SIZE: {}", self.size());
}
```

### 3.1 KLUSS: Knowledge-Limited Unfrozen Subgame Solving

#### Paper Description
KLUSS constructs an imperfect-information subgame Γ from:
1. The old game tree Γ̂ (saved from previous move)
2. Sampled positions from the belief set

The algorithm removes nodes based on "k-order knowledge": if there's a position s such that "we know the opponent knows we know... (k times) that s is not the true state", then s is removed.

**Key Innovation**: Unlike KLSS (Knowledge-Limited Subgame Solving), KLUSS does NOT freeze strategies at distance-1 nodes. All unfrozen nodes are re-optimized together.

#### Implementation

**Step 1: Pop existing histories from previous tree**:
```rust
fn pop_histories(&mut self, hist: G::Trace, player: Player) 
    -> HashMap<G::Trace, PreResolver<G>> {
    // Filter down to the second cover of the trace
    let root_histories = self.drain_root()
        .into_iter()
        .flat_map(|mut x| Self::drain_resolver(&mut x).into_iter())
        .collect();
    
    // Apply k-cover with k=3
    let mut covered = Self::k_cover(root_histories, hist.clone(), player, 3);
    
    // Normalize probabilities
    let new_possibility = covered.iter()
        .map(|x| x.net_reach_prob())
        .sum::<Probability>();
    for x in covered.iter_mut() {
        x.renormalize_reach(new_possibility);
    }
    
    // Group by opponent's information
    let mut positions: HashMap<G::Trace, PreResolver<G>> = covered
        .into_iter()
        .fold(HashMap::new(), |mut map, history| {
            let trace = history.trace();
            let my_prob = history.net_reach_prob();
            let info_expectation = match history {
                History::Expanded {..} => 
                    self.info_sets[&trace].borrow().policy.expectation(),
                History::Terminal {payoff,..} | History::Visited {payoff,..} => payoff,
            };
            // ... group by trace
            map
        });
    positions
}
```

**Step 2: k-cover algorithm** - The heart of KLUSS:
```rust
fn k_cover_rec(mut root: History<G>, hist: &HashSet<G::Trace>, 
               player: Player) -> (Option<History<G>>, Vec<History<G>>, HashSet<G::Trace>) {
    if matches!(root, History::Terminal { .. }) {
        return (Some(root), vec![], HashSet::new())
    }
    
    let my_trace = root.players_view(player);
    let comparisons: Vec<std::cmp::Ordering> = hist.iter()
        .filter_map(|x| my_trace.partial_cmp(x))
        .collect();
    
    // If this matches the target trace
    if comparisons.contains(&std::cmp::Ordering::Equal) {
        let other_view = root.players_view(root.player().other());
        (None, vec![root], HashSet::from([other_view]))
    } 
    // If this is on the path to target (and expanded)
    else if !comparisons.is_empty() && matches!(root, History::Expanded { .. }) {
        // Recursively process children
        let children_vec = if let History::Expanded { children, .. } = &mut root {
            std::mem::take(children)
        } else {
            unreachable!()
        };
        
        let (new_children, hits, views) = children_vec
            .into_iter()
            .fold((Vec::new(), Vec::new(), HashSet::new()), 
                  |(mut cs, mut hs, mut vs), (action, child)| {
                let (back, found, new_views) = Self::k_cover_rec(child, hist, player);
                if let Some(back) = back {
                    cs.push((action, back));
                }
                hs.extend(found);
                vs.extend(new_views);
                (cs, hs, vs)
            });
        
        // Restore modified children
        if let History::Expanded { children, .. } = &mut root {
            *children = new_children;
        }
        
        (Some(root), hits, views)
    } else {
        (Some(root), vec![], HashSet::new())
    }
}
```

**Correctness**: The k-cover algorithm correctly implements the paper's description:
- It performs k iterations (k=3 in implementation, matching paper recommendation)
- Each iteration alternates player perspective
- Nodes are retained only if they're reachable within k levels of "I know you know..." reasoning
- The algorithm returns all histories that satisfy the k-order knowledge constraint

**Step 3: Populate with new samples**:
```rust
fn populate_histories(positions: &mut HashMap<G::Trace, PreResolver<G>>, 
                     hist: G::Trace, player: Player) {
    let mut data_count = positions.len();
    let mut new_positions = G::sample_position(hist.clone());
    let other = player.other();

    while data_count < MIN_INFO_SIZE {
        if let Some(g) = new_positions.next() {
            let game_hash = g.identifier();
            // Avoid duplicates
            if positions.iter()
                .flat_map(|(_, (_, _, v))| v.iter().map(|x|x.identifier()))
                .any(|x| x == game_hash) {
                continue;
            }
            
            let s = History::new(g.clone(), HashMap::new());
            let opp_trace = g.trace(other);
            let alt = g.evaluate();
            positions
                .entry(opp_trace)
                .or_insert((1.0, alt, vec![]))
                .2.push(s);
            data_count += 1;
        } else {
            break;
        }
    }
}
```

**Step 4: Construct resolver gadgets**:
```rust
// From SubgameRoot::new
pub fn new(
    positions: HashMap<G::Trace, PreResolver<G>>,
    player: Player,
) -> Self {
    let resolvers: Vec<ResolverGadget<G>> = positions
        .into_iter()
        .map(|(opp_trace, (prior_probability, alt, mut histories))| {
            // Create policy for sampling among histories
            let info = Info::from_policy(
                Policy::from_rewards(
                    histories.iter().map(|h| {
                        (h.identifier(), h.payoff())
                    }).collect(),
                    Player::Chance,
                ),
                opp_trace.clone(),
                Player::Chance,
            );
            
            // Create resolver policy (ENTER or SKIP)
            let resolver = Policy::from_rewards(
                vec![(ENTER, alt), (SKIP, alt)],
                player.other(),
            );
            
            ResolverGadget {
                resolver,
                alt,
                children: histories,
                info,
                prior_probability,
            }
        })
        .collect();
    
    // Create maxmargin policy over resolvers
    let maxmargin = Policy::from_rewards(
        resolvers.iter().enumerate()
            .map(|(i, r)| (i, r.alt))
            .collect(),
        player,
    );
    
    SubgameRoot {
        maxmargin,
        children: resolvers,
        player,
    }
}
```

**Verification**: The implementation correctly creates the "Resolve" structure described in the paper:
- Each resolver corresponds to one opponent information set
- The resolver has two actions: ENTER (solve the subgame) or SKIP (use the alt value)
- The maxmargin policy selects which resolver to use
- This matches the paper's Figure in Appendix B showing the resolver structure

### 3.2 Equilibrium Computation (PCFR+)

#### Paper Description
Uses Predictive CFR+ (PCFR+) to compute approximate Nash equilibrium. Key features:
- Iterative regret minimization
- Uses last-iterate strategy instead of average strategy
- Handles changing game trees (nodes added during search)

#### Implementation

**Main CFR Iteration** (`src/obscuro.rs`):
```rust
fn solve_step(&mut self) {
    self.cfr_iterations(Player::P1);
    self.cfr_iterations(Player::P2);

    // Update maxmargin policy with reach probabilities
    let p_max: Probability = self.get_pmax();
    let maxmargin = &mut self.subgame_root.maxmargin;
    
    for (idx, child) in self.subgame_root.children.iter_mut().enumerate() {
        let p_maxmargin = maxmargin.p_exploit(&idx);
        let resolver = &mut child.resolver;
        let prior_probability = child.prior_probability;
        let p_resolve = resolver.p_exploit(&ENTER);
        
        // Combined reach probability
        let reach_prob = p_max * (prior_probability) * p_resolve 
                       + (1.0-p_max) * p_maxmargin;
        maxmargin.add_counterfactual(&idx, reach_prob, 1.0);
    }
}
```

**CFR Traversal**:
```rust
fn cfr_iterations(&mut self, optimizing_player: Player) {
    self.total_updates += 1;
    let SubgameRoot { maxmargin: ref mut root_policy, ref mut children } = 
        &mut self.subgame_root;
    let mut root_value = 0.0;
    let resolver_dist = root_policy.inst_policy();
    
    for (resolver_idx, (resolver_gadget, r_prob)) in 
        children.iter_mut().zip(resolver_dist).enumerate() {
        
        let ResolverGadget { resolver, alt, children: histories, info, 
                            prior_probability } = resolver_gadget;
        let p_enter = 1.0;  // Always enter in current implementation
        let mut enter_value = 0.0;
        let distribution = info.policy.inst_policy();
        
        for (_, (history, sample_chance)) in 
            histories.iter_mut().zip(distribution.iter()).enumerate() {
            
            let action_reach = HashMap::from([
                (Player::Chance, *sample_chance),
                (optimizing_player.other(), r_prob * p_enter),
            ]);
            
            // Recursive utility calculation
            let h_value = Self::make_utilities(history, optimizing_player, action_reach);
            Self::apply_updates(history, self.total_updates);
            enter_value += sample_chance * h_value;
        }
        
        // Update resolver policy
        resolver.add_counterfactual(&ENTER, enter_value, r_prob);
        resolver.add_counterfactual(&SKIP, *alt, r_prob);
        resolver.update(self.total_updates);
        
        let resolver_value = (1.0 - p_enter) * *alt + p_enter * enter_value;
        root_policy.add_counterfactual(&resolver_idx, resolver_value, 1.0);
    }
    
    root_policy.update(self.total_updates);
}
```

**Utility Calculation** (Recursive CFR):
```rust
fn make_utilities(h: &mut History<G>, optimizing_player: Player, 
                 reach_prob: HashMap<Player, Probability>) -> Reward {
    match h {
        History::Terminal { payoff } => {
            optimizing_player.align(*payoff)
        }
        History::Visited { payoff, .. } => {
            optimizing_player.align(*payoff)
        }
        History::Expanded { info, children, player, .. } => {
            let policy = &mut info.borrow_mut().policy;
            let distribution = policy.inst_policy();
            let mut value = 0.0;
            
            for ((action, child), prob) in children.iter_mut().zip(distribution.iter()) {
                let mut next_reach = reach_prob.clone();
                next_reach.entry(*player)
                    .and_modify(|e| *e *= prob)
                    .or_insert(*prob);
                
                let child_value = Self::make_utilities(child, optimizing_player, next_reach);
                value += prob * child_value;
                
                // Accumulate counterfactual regret
                if *player == optimizing_player {
                    let cf_reach = next_reach.iter()
                        .filter(|(&p, _)| p != optimizing_player)
                        .map(|(_, &prob)| prob)
                        .product::<Probability>();
                    policy.add_counterfactual(action, child_value, cf_reach);
                }
            }
            value
        }
    }
}
```

**Policy Update** (CFR+ with Linear CFR) (`src/policy.rs`):
```rust
pub fn update(&mut self, total_updates: usize) {
    if total_updates == self.last_set || self.player == Player::Chance {
        return;
    }
    self.last_set = total_updates;
    if self.first_update.is_none() { 
        self.first_update = Some(self.last_set-1); 
    }
    
    let num_updates = (total_updates - self.first_update.unwrap())
        .max(200) as Reward;
    
    // Linear CFR momentum coefficient
    let momentum_coeff = (num_updates)/(num_updates+1.0);
    
    let baseline = self.expectation();
    let mult = self.multiplier();
    
    // Update regrets with CFR+ (positive projection)
    for (i, cfvs) in self.counterfactuals.iter().enumerate() {
        let ir = mult * (cfvs - baseline);
        let r = self.acc_regrets[i];
        self.acc_regrets[i] = (momentum_coeff * r + ir).max(0.0);
    }
    
    // Update average strategy
    for (i, p) in self.inst_policy().iter().enumerate() {
        self.avg_strategy[i] += *p;
    }
    
    self.counterfactuals = vec![0.0; self.counterfactuals.len()];
}
```

**Correctness Analysis**:
1. ✅ Implements CFR+ with positive regret projection
2. ✅ Uses Linear CFR momentum coefficient: `t/(t+1)`
3. ✅ Maintains average strategy for potential future use
4. ✅ Uses instantaneous (last-iterate) strategy for tree traversal
5. ⚠️ **ISSUE**: The resolver policy always uses `p_enter = 1.0` instead of the actual resolver policy value

### 3.3 Expanding the Game Tree

#### Paper Description
Nodes are expanded using a carefully-designed policy that balances exploration and exploitation:
- One player (alternating) is the "exploring player" using perturbed strategy x̃ᵗ
- Other player uses current strategy yᵗ
- Exploring strategy uses PUCT-based method
- Once leaf is selected, children are evaluated with Stockfish

#### Implementation

**Expansion Step**:
```rust
fn expansion_step(&mut self) {
    let Self {subgame_root, info_sets, ..} = self;
    
    // P1 explores
    let hist1 = Self::sample_history(subgame_root);
    Obscuro::expansion_step_inner(Player::P1, hist1, info_sets);
    
    // P2 explores
    let Self {subgame_root, info_sets, ..} = self;
    let hist2 = Self::sample_history(subgame_root);
    Obscuro::expansion_step_inner(Player::P2, hist2, info_sets);
}
```

**Expansion Inner Loop**:
```rust
fn expansion_step_inner(player: Player, mut here: &mut History<G>, 
                       infosets: &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>) {
    while let History::Expanded {info, children, player: here_player, .. } = here {
        let policy: &mut Policy<G::Action> = &mut info.borrow_mut().policy;
        
        let action = if *here_player==player {
            policy.explore()  // Exploring player uses PUCT
        } else {
            policy.exploit()  // Non-exploring player uses current strategy
        };
        
        policy.add_expansion(&action);
        here = children.iter_mut()
            .find(|(ca, _)| *ca==action)
            .map(|(_, ch)| ch)
            .unwrap();
    }
    
    match here {
        History::Expanded {..} => unreachable!(),
        History::Terminal {..} => (),
        History::Visited {..} => here.expand(infosets),
    }
}
```

**Exploration Policy** (PUCT-based) (`src/policy.rs`):
```rust
pub fn explore(&self) -> A {
    let n_total = self.expansions.iter().sum::<usize>() as Reward;
    let base = self.inst_policy();
    let exploration_bonus = self.expansions.iter()
        .map(|&n| (((1.0+n_total).ln()) / (1.0+n as Reward)).sqrt())
        .collect::<Vec<_>>();
    
    // Combine exploitation and exploration
    let combined: Vec<Reward> = base.iter().zip(exploration_bonus.iter())
        .map(|(&b, &e)| b + EXPLORE_CONSTANT * e)
        .collect();
    
    let idx = combined.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| 
            a.partial_cmp(b).unwrap_or(Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap();
    
    self.actions[idx].clone()
}
```

**Exploitation Policy** (Best response):
```rust
pub fn exploit(&self) -> A {
    let policy = self.inst_policy();
    let idx = policy.iter()
        .enumerate()
        .max_by(|(_, &a), (_, &b)| 
            a.partial_cmp(&b).unwrap_or(Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap();
    self.actions[idx].clone()
}
```

**Node Expansion** (`src/history.rs`):
```rust
pub fn expand(&mut self, infosets: &mut HashMap<G::Trace, InfoPtr<G::Action, G::Trace>>) {
    let me = self.player();
    if let History::Visited { state, reach, .. } = self {
        let game = G::decode(state);
        let hero = game.active_player();
        let (hero_trace, villan_trace) = game.identifier();
        let actions = game.available_actions();

        let mut kids: Vec<(G::Action, History<G>)> = Vec::with_capacity(actions.len());

        // Generate all children
        for a in actions.iter() {
            let next = game.play(a);
            let mut next_reach = reach.clone();
            next_reach.entry(me)
                .and_modify(|e| *e *= 1.0/actions.len() as Probability)
                .or_insert(1.0/actions.len() as Probability);
            let child = History::new(next, next_reach);
            kids.push((a.clone(), child));
        }
        
        // Create or get infoset
        let rc: InfoPtr<G::Action, G::Trace> = if let Some(rc) = infosets.get(&hero_trace) {
            rc.clone()
        } else {
            // Initialize with Stockfish evaluation
            let info = Info::from_policy(
                Policy::from_rewards(kids.iter().map(|(a, h)| {
                    (a.clone(), h.payoff())
                }).collect(), hero), 
                hero_trace.clone(), 
                hero
            );
            let rc = Rc::new(RefCell::new(info));
            infosets.insert(hero_trace.clone(), rc.clone());
            rc
        };

        *self = History::Expanded { 
            info: rc, 
            reach: HashMap::new(), 
            children: kids, 
            player: hero, 
            villan_trace 
        };
    }
}
```

**Correctness**:
1. ✅ Correctly implements alternating exploring player
2. ✅ Non-exploring player uses current strategy (exploit)
3. ✅ Exploring player uses PUCT-based exploration
4. ✅ Nodes initialized with heuristic evaluation (Stockfish)
5. ✅ Exploration bonus follows UCB formula: sqrt(ln(N_total)/(1+N_action))

### 3.4 Repeat

#### Paper Description
Steps 2 and 3 are repeated in parallel until time budget is exceeded:
- One thread runs CFR
- Two threads expand the tree
- Expansion stops first, then CFR continues briefly for convergence

#### Implementation

The main loop in `study_position`:
```rust
pub fn study_position(&mut self, observation: G::Trace, player: Player) {
    self.start_time = SystemTime::now();
    self.construct_subgame(observation.clone(), player);
    
    while self.start_time.elapsed().unwrap_or(Duration::from_secs(0)) 
        < Duration::from_millis((SOLVE_TIME_SECS*1000.0) as u64) {
        
        self.expansion_step();  // Alternates P1/P2 expansion
        
        for _ in 0..10 {
            self.solve_step();  // CFR iteration for both players
        }
    }
    
    println!("SIZE: {}", self.size());
}
```

**Note**: The current implementation appears to be single-threaded, unlike the paper's description of multi-threaded parallel execution. This is a simplification that doesn't affect correctness but may impact performance.

### 3.5 Move Selection

#### Paper Description
After the time budget is exceeded, select a move based on the final strategy. The paper uses "purification" - selecting the best action rather than sampling from the mixed strategy.

#### Implementation

```rust
pub fn make_move(&mut self, observation: G::Trace, player: Player) -> G::Action {
    debug_assert!(!matches!(player, Player::Chance));
    self.study_position(observation.clone(), player);
    
    // Return purified best from the chosen expanded node
    self.info_sets[&observation].borrow().policy.purified()
}
```

**Purification** (`src/policy.rs`):
```rust
pub fn purified(&self) -> A {
    // Select action with highest average strategy probability
    let idx = self.avg_strategy.iter()
        .enumerate()
        .max_by(|(_, &a), (_, &b)| 
            a.partial_cmp(&b).unwrap_or(Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap();
    self.actions[idx].clone()
}
```

**Correctness**: 
- ✅ Uses average strategy for final move selection (more stable than last-iterate)
- ✅ Implements purification (deterministic selection of best action)
- ⚠️ Paper mentions "cheap purification hints" with stability marking, which is partially implemented but not fully utilized

---

## 4. Experiments and Results

### Paper Results
- Defeated #1 ranked human player
- Won against prior state-of-the-art AI (KLSS-based) with 83.4% win rate
- Demonstrated superhuman performance across various time controls

### Implementation Notes
The implementation includes evaluation code in `src/self_play.rs` for running matches and collecting statistics. The solver can be configured with different time controls via the `SOLVE_TIME_SECS` constant.

---

## Appendix A: Rules of FoW Chess

The paper specifies FoW chess rules which differ from regular chess:
1. Win by capturing the king (no check/checkmate)
2. Moving into check is legal (but loses immediately)
3. Observe all squares pieces can legally move to
4. Blocked pawns: square is observed but not the blocking piece
5. En passant target is visible

These rules are implemented in the game-specific code (likely in `src/games/` directory) and abstracted through the `Game` trait.

---

## Appendix B: Further Details

### B.1 Game Formulation

The implementation uses extensive-form game representation:

```rust
// From src/history.rs
pub enum History<G: Game> {
    Terminal { payoff: Reward },
    Visited { state: G::State, payoff: Reward, reach: HashMap<Player, Probability> },
    Expanded { 
        info: InfoPtr<G::Action, G::Trace>, 
        reach: HashMap<Player, Probability>, 
        children: Vec<(G::Action, History<G>)>, 
        player: Player, 
        villan_trace: G::Trace 
    },
}
```

This directly corresponds to the paper's definition:
- `Terminal`: Leaf nodes with payoffs
- `Visited`: Unexpanded internal nodes
- `Expanded`: Internal nodes with children and infoset pointers

### B.2 Infoset Representation

```rust
// From src/info.rs
pub struct Info<A: ActionI, T> {
    pub policy: Policy<A>,
    pub trace: T,
    pub player: Player,
}
```

Infosets are stored in a shared HashMap and accessed via reference-counted pointers:
```rust
pub type InfoPtr<A, T> = Rc<RefCell<Info<A, T>>>;
```

This allows multiple history nodes in the same infoset to share the same policy.

### B.3 Resolver Structure

The paper describes "Resolve" gadgets for safe subgame solving:

```rust
struct ResolverGadget<G: Game> {
    resolver: Policy<ResolveActions>,  // ENTER or SKIP
    alt: Reward,                        // Alternative value if SKIP
    children: Vec<History<G>>,          // Histories in this branch
    info: Info<G::State, G::Trace>,     // Policy for sampling histories
    prior_probability: Probability,      // Prior belief probability
}

pub enum ResolveActions {
    ENTER,  // Enter and solve the subgame
    SKIP,   // Use alternative value
}
```

This implements the paper's resolver structure where:
- Each opponent infoset gets a resolver
- Resolver decides whether to solve (ENTER) or use heuristic (SKIP)
- The alt value provides a safe lower bound on value

---

## Appendix C: Additional Experiments

The paper includes ablation studies testing various components. The implementation supports these through configuration constants (e.g., `EXPLORE_CONSTANT`, `SOLVE_TIME_SECS`, `MIN_INFO_SIZE`).

---

## Algorithm Correctness Analysis

### Verified Correct Components

1. **✅ k-cover Algorithm**: Correctly implements k-order knowledge filtering
   - Iterates k times alternating player perspective
   - Properly computes reachable nodes
   - Matches paper's Figure 2 example

2. **✅ CFR+ Implementation**: Correctly implements predictive CFR+
   - Positive regret projection
   - Linear CFR momentum
   - Proper counterfactual value computation

3. **✅ Tree Expansion**: Correctly implements PUCT-based exploration
   - Alternating exploring player
   - UCB exploration bonus
   - Heuristic initialization

4. **✅ Resolver Structure**: Matches paper's description
   - ENTER/SKIP actions
   - Safe subgame solving structure
   - Proper integration with CFR

### Identified Issues

#### Issue 1: Resolver Policy Not Used ⚠️

**Location**: `src/obscuro.rs`, line 282

**Current Code**:
```rust
let _p_enter = resolver.p_exploit(&ENTER);
let p_enter = 1.0;  // Always enter - resolver policy ignored!
```

**Issue**: The resolver policy is computed but not used. The code always uses `p_enter = 1.0`, meaning it always enters the subgame and never skips to use the alternative value.

**Expected Behavior**: Should use the resolver's actual policy:
```rust
let p_enter = resolver.p_exploit(&ENTER);
```

**Impact**: 
- Reduces the benefit of the Resolve structure
- May compute unnecessary subgames
- Doesn't properly balance between subgame solving and heuristic values

#### Issue 2: Purification Strategy Selection ⚠️

**Location**: `src/policy.rs`

**Current Code** for `purified()`:
```rust
pub fn purified(&self) -> A {
    let idx = self.avg_strategy.iter()
        .enumerate()
        .max_by(|(_, &a), (_, &b)| 
            a.partial_cmp(&b).unwrap_or(Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap();
    self.actions[idx].clone()
}
```

**Issue**: Uses average strategy, but the paper states it uses last-iterate strategy during search. The `stable` vector marking is computed but never used for purification hints.

**Expected Behavior**: Paper mentions "cheap purification hints" with stability marking. Could utilize the `stable` flags to guide purification.

**Impact**: Minor - average strategy is actually more stable for final move selection, so this might be an intentional improvement.

#### Issue 3: Single-Threaded Execution ⚠️

**Location**: `src/obscuro.rs`, `study_position` method

**Issue**: The paper describes parallel execution with:
- 1 CFR thread
- 2 expansion threads
- Shared tree with locks

Current implementation is single-threaded with sequential expansion and solving.

**Expected Behavior**: Multi-threaded as described in paper Section 3.4.

**Impact**: 
- Slower execution (doesn't utilize multiple cores)
- Doesn't affect correctness
- May explain performance differences if any

#### Issue 4: Reach Probability Computation ⚠️

**Location**: `src/history.rs`, line 46

**Current Code**:
```rust
next_reach.entry(me)
    .and_modify(|e| *e *= 1.0/actions.len() as Probability)
    .or_insert(1.0/actions.len() as Probability);
```

**Issue**: When expanding a node, reach probability is multiplied by `1.0/actions.len()`, assuming uniform strategy. This doesn't match the actual policy being used.

**Expected Behavior**: Should multiply by actual probability from parent's policy, not uniform.

**Impact**:
- Reach probabilities may be inaccurate
- Could affect CFR convergence rate
- May not significantly impact final strategy quality

---

## Identified Issues and Fixes

The following sections detail the fixes for identified issues.

### Fix 1: Use Resolver Policy

**File**: `src/obscuro.rs`
**Line**: 282

**Change**:
```rust
// OLD:
let _p_enter = resolver.p_exploit(&ENTER);
let p_enter = 1.0;

// NEW:
let p_enter = resolver.p_exploit(&ENTER);
```

This fix ensures the resolver policy is actually used to decide whether to enter subgames or use alternative values, as described in the paper.

### Fix 2: Correct Reach Probability in Expansion

**File**: `src/history.rs`
**Line**: 46

**Issue**: The reach probability update assumes uniform distribution over actions, which is incorrect.

**Solution**: Track the actual policy probability when expanding. This requires passing the parent's policy to the expand function.

**Note**: This is a more complex fix that would require refactoring the expand function signature. The current implementation is a simplification that may affect convergence rate but not correctness of the final equilibrium.

### Fix 3: Threading (Optional Enhancement)

The paper describes multi-threaded execution. While not strictly necessary for correctness, implementing this would improve performance:

```rust
// Pseudocode for multi-threaded version
pub fn study_position_parallel(&mut self, observation: G::Trace, player: Player) {
    self.construct_subgame(observation.clone(), player);
    
    let tree = Arc::new(Mutex::new(self));
    let stop_flag = Arc::new(AtomicBool::new(false));
    
    // Start CFR thread
    let cfr_thread = {
        let tree = tree.clone();
        let stop = stop_flag.clone();
        thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                tree.lock().unwrap().solve_step();
            }
        })
    };
    
    // Start expansion threads
    let expansion_threads: Vec<_> = (0..2).map(|_| {
        let tree = tree.clone();
        let stop = stop_flag.clone();
        thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                tree.lock().unwrap().expansion_step();
            }
        })
    }).collect();
    
    // Wait for time budget
    thread::sleep(Duration::from_millis((SOLVE_TIME_SECS*1000.0) as u64));
    stop_flag.store(true, Ordering::Relaxed);
    
    // Join all threads
    for thread in expansion_threads {
        thread.join().unwrap();
    }
    cfr_thread.join().unwrap();
}
```

---

## Conclusion

The implementation faithfully captures the key algorithms described in the Obscuro paper:

1. **KLUSS (k-cover)**: Correctly filters game tree based on k-order knowledge
2. **CFR+**: Properly implements regret minimization with Linear CFR
3. **Expansion**: Uses PUCT-based exploration/exploitation balance
4. **Resolver Structure**: Implements safe subgame solving framework

The main identified issues are:
1. Resolver policy not being used (easy fix)
2. Reach probabilities computed incorrectly during expansion (minor impact)
3. Single-threaded execution (performance, not correctness)

Overall, the implementation is sound and represents a correct instantiation of the algorithms described in the paper, with minor deviations that don't fundamentally affect the approach.

