# Algorithm Execution Trace Examples

This document provides concrete execution traces showing how the Obscuro algorithm processes a FoW chess position, with actual code paths and data flow.

## Example 1: Making a Move (Complete Flow)

Let's trace through making a move from a FoW chess position.

### Initial State
```
Position: After move 1.e4 e5
Observation: P1 can see their pieces and squares they control
Time budget: 5 seconds
```

### Step 1: Entry Point

**Code**: `obscuro.rs:25-31` - `make_move()`
```rust
pub fn make_move(&mut self, observation: G::Trace, player: Player) -> G::Action {
    self.study_position(observation.clone(), player);
    self.info_sets[&observation].borrow().policy.purified()
}
```

**What happens**:
1. Calls `study_position()` to analyze the position
2. Retrieves the policy for the current observation
3. Returns the purified (best) action

### Step 2: Study Position

**Code**: `obscuro.rs:37-51` - `study_position()`
```rust
pub fn study_position(&mut self, observation: G::Trace, player: Player) {
    self.start_time = SystemTime::now();
    self.construct_subgame(observation.clone(), player);
    
    while self.start_time.elapsed().unwrap_or(Duration::from_secs(0)) 
        < Duration::from_millis((SOLVE_TIME_SECS*1000.0) as u64) {
        self.expansion_step();
        for _ in 0..10 {
            self.solve_step();
        }
    }
}
```

**What happens**:
1. Records start time
2. Constructs initial subgame (Step 3.1 from paper)
3. Iteratively expands and solves until time budget exhausted

**Example timing**:
- 5 second budget → 5000ms
- Each expansion + 10 CFR iterations ≈ 50ms
- Total iterations ≈ 100

### Step 3: Construct Subgame (KLUSS)

**Code**: `obscuro.rs:55-76` - `construct_subgame()`

#### Step 3a: Pop Histories from Previous Tree

```rust
let mut positions = self.pop_histories(hist.clone(), player);
```

**What happens**: `obscuro.rs:77-113` - `pop_histories()`
1. Drains resolver gadgets from previous move's tree
2. Flattens to get all histories
3. Applies 3-cover filter to remove irrelevant positions

**Example**:
```
Previous tree: 1000 nodes
After drain: 200 histories from 10 resolvers
After 3-cover: 50 histories (removed 150 that are 3+ levels away in knowledge graph)
```

#### Step 3b: k-Cover Filtering

```rust
let mut covered = Self::k_cover(root_histories, hist.clone(), player, 3);
```

**Code trace**: `obscuro.rs:146-169` - `k_cover()`

**Example execution for k=3**:

Iteration 1 (k=1): "What do I think are possible positions?"
```
Input: 200 histories from previous tree
Target: Current observation trace T
Player: P1 (viewing as P1)
Output: All histories where P1's view matches or is ancestor of T
Result: 80 histories found
```

Iteration 2 (k=2): "What does opponent think I might think?"
```
Input: 80 histories from iteration 1
Target: All opponent views from iteration 1 output
Player: P2 (flipped)
Output: All histories where P2's view matches targets
Result: 120 histories found (more, as we add opponent's perspective)
```

Iteration 3 (k=3): "What do I think opponent thinks I think?"
```
Input: 120 histories from iteration 2
Target: All P1 views from iteration 2 output
Player: P1 (flipped back)
Output: Final filtered set
Result: 100 histories (some pruned as too distant in knowledge graph)
```

#### Step 3c: Populate New Samples

```rust
Self::populate_histories(&mut positions, hist, player);
```

**What happens**: `obscuro.rs:114-141`
1. Counts existing unique opponent infosets
2. Samples new positions from belief set until MIN_INFO_SIZE reached
3. Groups by opponent's observation

**Example**:
```
Existing positions: 100 histories in 8 opponent infosets
MIN_INFO_SIZE: 20
Need: 12 more samples
Sampled: 12 new positions, added to 6 new opponent infosets
Result: 112 histories in 14 opponent infosets
```

#### Step 3d: Create Resolver Structure

```rust
let root = SubgameRoot::new(positions, player);
```

**Code**: `obscuro.rs:464-506` - `SubgameRoot::new()`

**Example structure created**:
```
SubgameRoot {
    player: P1,
    maxmargin: Policy over 14 resolvers,
    children: [
        ResolverGadget {
            resolver: Policy(ENTER vs SKIP),
            alt: -0.2 (Stockfish eval),
            children: [hist1, hist2, hist3],  // 3 histories for this opp infoset
            info: Policy over these 3 histories (for sampling),
            prior_probability: 0.15  // P(opponent in this infoset)
        },
        ResolverGadget {
            resolver: Policy(ENTER vs SKIP),
            alt: 0.1,
            children: [hist4, hist5, hist6, hist7, hist8],  // 5 histories
            info: Policy over these 5,
            prior_probability: 0.08
        },
        // ... 12 more resolver gadgets
    ]
}
```

### Step 4: Expansion Step

**Code**: `obscuro.rs:225-233` - `expansion_step()`

**Iteration 1**: P1 explores
```rust
let hist1 = Self::sample_history(subgame_root);  // Sample resolver → sample history
Obscuro::expansion_step_inner(Player::P1, hist1, info_sets);
```

**Sample path**:
1. Maxmargin policy samples resolver #3 (probability 0.12)
2. Resolver #3 decides ENTER (probability 0.95)
3. Resolver #3's info policy samples history #2 from its children
4. Now have pointer to a History node

**Traversal**: `obscuro.rs:234-253` - `expansion_step_inner()`
```
Start: History::Expanded at root of selected history tree
Player P1 explores → policy.explore() → PUCT selects action "Nf3"
Move to child "Nf3"

Current: History::Expanded at "Nf3" node
Player P2 plays (not exploring) → policy.exploit() → best action "Nc6"
Move to child "Nc6"

Current: History::Expanded at "Nf3 Nc6"
Player P1 explores → policy.explore() → PUCT selects "Bb5"
Move to child "Bb5"

Current: History::Visited (leaf node)
Call history.expand()
```

**Expansion**: `history.rs:30-68` - `expand()`
```
Position: After Nf3 Nc6 Bb5
Actions: [a6, a5, Nf6, d6, Bc5, ...]
Generate children for all legal moves
Create/get infoset with trace corresponding to this position
Initialize policy with Stockfish evaluations:
  a6: -0.1, a5: -0.15, Nf6: 0.2, d6: 0.1, Bc5: 0.15, ...
Convert to Policy(actions, init_regrets from evals)
Update History from Visited → Expanded
```

**Iteration 2**: P2 explores (different path)

### Step 5: Solve Step (CFR Iteration)

**Code**: `obscuro.rs:255-272` - `solve_step()`

Called 10 times for every 1 expansion.

#### CFR Iteration for P1

**Code**: `obscuro.rs:274-304` - `cfr_iterations(Player::P1)`

**Example trace**:

Resolver #1:
```
- r_prob: 0.15 (maxmargin policy probability)
- p_enter: 0.92 (resolver policy for ENTER)
- Sample history according to info policy: hist_a (prob 0.4)
- Traverse hist_a tree, computing utilities...
```

**Tree traversal**: `obscuro.rs:307-341` - `make_utilities()`

```
At root (P1 to play): History::Expanded
  actions: [Nf3: 0.6, Nc3: 0.3, e5: 0.1]
  reach_probs: {Chance: 0.4, P2: 0.15*0.92}
  
  Branch Nf3 (prob 0.6):
    reach_probs: {Chance: 0.4, P2: 0.15*0.92, P1: 0.6}
    P2 node: actions [Nc6: 0.5, e5: 0.5]
    
    Branch Nf3-Nc6 (prob 0.5):
      reach_probs: {Chance: 0.4, P2: 0.15*0.92*0.5, P1: 0.6}
      P1 node: actions [Bb5: 0.7, Bc4: 0.3]
      
      Branch Nf3-Nc6-Bb5:
        Terminal or Visited: return payoff 0.15
        Counterfactual reach for P1: 0.4 * 0.15 * 0.92 * 0.5 = 0.0276
        Add to policy: add_counterfactual(Bb5, 0.15, 0.0276)
      
      Branch Nf3-Nc6-Bc4:
        Terminal: return payoff 0.1
        Add to policy: add_counterfactual(Bc4, 0.1, 0.0276)
      
      Value of Nf3-Nc6 node: 0.7*0.15 + 0.3*0.1 = 0.135
      Counterfactual reach for P2: 0.4 * 0.6 = 0.24
      Add to P2 policy: add_counterfactual(Nc6, 0.135, 0.24)
    
    Branch Nf3-e5:
      ... similar recursion ...
      Value: 0.08
      Add to P2 policy: add_counterfactual(e5, 0.08, 0.24)
    
    Value of Nf3 node: 0.5*0.135 + 0.5*0.08 = 0.1075
    Counterfactual reach for P1: 0.4 * 0.15 * 0.92 = 0.0552
    Add to P1 policy: add_counterfactual(Nf3, 0.1075, 0.0552)
  
  Branch Nc3:
    ... similar ...
    Value: 0.05
    Add to P1 policy: add_counterfactual(Nc3, 0.05, 0.0552)
  
  Branch e5:
    ... similar ...
    Value: -0.1
    Add to P1 policy: add_counterfactual(e5, -0.1, 0.0552)
  
  Overall value: 0.6*0.1075 + 0.3*0.05 + 0.1*(-0.1) = 0.0595
```

**After traversal**: Apply updates
```rust
Self::apply_updates(history, self.total_updates);
```

**Code**: `obscuro.rs:342-353` - `apply_updates()`
Recursively calls `policy.update()` on all expanded nodes.

**Policy update**: `policy.rs:48-81` - `update()`

For the root P1 node:
```
Counterfactuals accumulated:
  Nf3: 0.1075 * 0.0552 = 0.00593
  Nc3: 0.05 * 0.0552 = 0.00276
  e5: -0.1 * 0.0552 = -0.00552

Current regrets (from previous iterations):
  Nf3: 2.5
  Nc3: 1.2
  e5: 0.1

Expectation (baseline): 
  (2.5*Nf3 + 1.2*Nc3 + 0.1*e5) / (2.5+1.2+0.1) = weighted avg
  ≈ mix of actions, compute expected value ≈ 0.08

Instant regrets (advantage over baseline):
  Nf3: 0.00593 - 0.08 = -0.07407
  Nc3: 0.00276 - 0.08 = -0.07724
  e5: -0.00552 - 0.08 = -0.08552

Momentum coefficient (Linear CFR):
  t = 100 iterations so far
  momentum = 100/101 = 0.99

Update regrets (CFR+):
  Nf3: max(0, 0.99*2.5 + (-0.07407)) = 2.401
  Nc3: max(0, 0.99*1.2 + (-0.07724)) = 1.111
  e5: max(0, 0.99*0.1 + (-0.08552)) = 0.014

New instantaneous policy:
  sum = 2.401 + 1.111 + 0.014 = 3.526
  Nf3: 2.401/3.526 = 0.681
  Nc3: 1.111/3.526 = 0.315
  e5: 0.014/3.526 = 0.004

Average strategy update:
  avg_strategy[Nf3] += 0.681
  avg_strategy[Nc3] += 0.315
  avg_strategy[e5] += 0.004
```

#### Update Resolver Policies

Back in `cfr_iterations()`:
```rust
resolver.add_counterfactual(&ENTER, enter_value, r_prob);
resolver.add_counterfactual(&SKIP, *alt, r_prob);
resolver.update(self.total_updates);
```

**Example**:
```
Resolver for infoset #1:
  ENTER value: 0.0595 (from tree traversal)
  SKIP value: -0.2 (alt from Stockfish)
  r_prob: 0.15
  
  Counterfactuals:
    ENTER: 0.0595 * 0.15 = 0.00893
    SKIP: -0.2 * 0.15 = -0.03
  
  Current regrets:
    ENTER: 5.2
    SKIP: 0.1
  
  After update:
    ENTER: higher (tree solving is working well)
    SKIP: lower (not attractive)
```

#### Update Maxmargin Policy

```rust
let resolver_value = (1.0 - p_enter) * *alt + p_enter * enter_value;
root_policy.add_counterfactual(&resolver_idx, resolver_value, 1.0);
```

**Example**:
```
Resolver #1 final value: 0.08*(-0.2) + 0.92*0.0595 = 0.0387
Add to maxmargin: add_counterfactual(resolver_1, 0.0387, 1.0)

... repeat for all 14 resolvers ...

Maxmargin update:
  Favors resolvers with high value
  Exploration in maxmargin policy guides which opponent infosets to focus on
```

### Step 6: Final Move Selection

After time budget exhausted (5 seconds, ~100 iterations):

**Code**: `obscuro.rs:30` - `purified()`
```rust
self.info_sets[&observation].borrow().policy.purified()
```

**Code**: `policy.rs:164-171` - `purified()`

**Example**:
```
Average strategy after 1000 updates:
  Nf3: 681.5
  Nc3: 215.3
  e5: 3.2
  d4: 50.0
  ... other moves

Normalize:
  sum = 950
  Nf3: 681.5/950 = 0.717
  Nc3: 215.3/950 = 0.227
  e5: 3.2/950 = 0.003
  d4: 50.0/950 = 0.053

Purify (select best):
  argmax = Nf3

Return: Action::Nf3
```

## Example 2: Exploration vs Exploitation

Let's see how PUCT balances exploration and exploitation.

### Initial State at an Information Set

```rust
Policy {
    actions: [a, b, c, d],
    inst_policy: [0.6, 0.3, 0.08, 0.02],  // Current strategy
    expansions: [10, 5, 1, 0],             // Times each action expanded
}
```

### Exploitation (Non-exploring player)

**Code**: `policy.rs:132-141` - `exploit()`
```rust
pub fn exploit(&self) -> A {
    let policy = self.inst_policy();
    let idx = policy.iter().max_by(...)
    self.actions[idx].clone()
}
```

**Result**: Always selects action `a` (highest probability 0.6)

### Exploration (Exploring player)

**Code**: `policy.rs:113-130` - `explore()`

**Calculation**:
```
n_total = 10 + 5 + 1 + 0 = 16

Exploration bonuses:
  a: sqrt(ln(17)/(1+10)) = sqrt(2.833/11) = 0.507
  b: sqrt(ln(17)/(1+5)) = sqrt(2.833/6) = 0.687
  c: sqrt(ln(17)/(1+1)) = sqrt(2.833/2) = 1.190
  d: sqrt(ln(17)/(1+0)) = sqrt(2.833/1) = 1.683

Combined scores (c = 1.0):
  a: 0.6 + 1.0*0.507 = 1.107
  b: 0.3 + 1.0*0.687 = 0.987
  c: 0.08 + 1.0*1.190 = 1.270  ← HIGHEST
  d: 0.02 + 1.0*1.683 = 1.703  ← Would be highest if EXPLORE_CONSTANT higher

Select: action c (unexplored but not useless)
```

**Impact**: Ensures all actions get tried, especially those with potential but not yet explored.

## Example 3: k-Cover in Action

Concrete example of k-cover filtering.

### Initial Setup

```
Current position: After 1.e4 e5 2.Nf3 Nc6 3.Bb5
My observation: I see my pieces and can infer opponent might have knight on c6
Previous tree: 500 histories

Question: Which of these 500 histories are relevant to current decision?
```

### k-Cover with k=3

**Iteration 1 (k=1)**: "What positions do I think are possible?"

```
Target: My current observation trace T_me
Player: P1 (me)

Check each of 500 histories:
  History A: P1's view = T_me → MATCH ✅
  History B: P1's view ≤ T_me (ancestor) → MATCH ✅
  History C: P1's view incompatible → REMOVE ❌
  ... 

Result: 150 histories match
Collect P2 views from these: {T_opp_1, T_opp_2, ..., T_opp_8}
```

**Iteration 2 (k=2)**: "What might opponent think I think?"

```
Target: {T_opp_1, T_opp_2, ..., T_opp_8}
Player: P2 (flipped)

Check remaining 150 histories:
  History A: P2's view = T_opp_1 → MATCH ✅
  History D: P2's view = T_opp_5 → MATCH ✅
  History E: P2's view not in target set → REMOVE ❌
  ...

Result: 200 histories match (added more as we consider opp perspective)
Collect P1 views from these: {T_me_1, T_me_2, ..., T_me_12}
```

**Iteration 3 (k=3)**: "What do I think opponent thinks I think?"

```
Target: {T_me_1, T_me_2, ..., T_me_12}
Player: P1 (flipped back)

Check remaining 200 histories:
  History A: P1's view = T_me_1 → MATCH ✅
  History D: P1's view = T_me_3 → MATCH ✅
  History F: P1's view not in target set → REMOVE ❌
  ...

Result: 180 histories remain

These 180 histories are all within 3 levels of "I know you know I know"
```

### Interpretation

- Started with 500 histories
- Filtered to 180 relevant ones
- Removed 320 histories that are "too far away" in knowledge graph
- These 320 histories represent positions where we know (to 3rd order) that they're not the true position

### Why k=3?

Paper justification:
- k=1 is too aggressive (removes too much)
- k=∞ would include everything (too expensive)
- k=3 empirically found to be a good balance
- Higher order knowledge ("I know you know I know you know...") becomes less relevant to actual play

## Performance Notes

From a typical 5-second search:
- Initial subgame construction: ~100ms
- Number of expansion steps: ~100
- Number of CFR iterations: ~1000 (10 per expansion)
- Final tree size: ~5000 nodes
- Nodes per second: ~1000
- Memory usage: ~50MB for tree structure
- Information sets: ~500 unique infosets

These numbers vary significantly based on position complexity and time control.

## Debugging Tips

To understand what the algorithm is doing:

1. **Enable debug prints**: Uncomment the `println!` statements in the code
2. **Check tree size**: `self.size()` shows how many nodes expanded
3. **Inspect policies**: Print `inst_policy()` and `avg_strategy` for key infosets
4. **Trace expansion**: Follow `expansion_step_inner()` to see which nodes are expanded
5. **Monitor convergence**: Track how regrets change over iterations
6. **Verify k-cover**: Check that filtered positions make sense given observation

## Summary

This execution trace shows:
1. How KLUSS filters relevant positions using k-cover
2. How CFR+ traverses the tree and updates regrets
3. How PUCT balances exploration and exploitation
4. How resolver structure enables safe subgame solving
5. How the final move is selected via purification

The algorithm successfully combines:
- Safe subgame solving (via resolvers)
- Knowledge-limited reasoning (via k-cover)
- Iterative equilibrium finding (via CFR+)
- Guided tree expansion (via PUCT)

To produce strong play in Fog of War chess.
