# Algorithm-to-Code Mapping Quick Reference

This document provides a quick reference for mapping paper sections to code locations.

## Main Algorithm Flow

| Paper Section | Description | Code Location | Key Functions |
|--------------|-------------|---------------|---------------|
| Section 3 | Main Algorithm Loop | `src/obscuro.rs:37-51` | `study_position()` |
| Section 3.1 | KLUSS (Subgame Construction) | `src/obscuro.rs:55-141` | `construct_subgame()`, `k_cover()`, `k_cover_rec()` |
| Section 3.2 | PCFR+ Equilibrium | `src/obscuro.rs:255-304` | `solve_step()`, `cfr_iterations()` |
| Section 3.3 | Tree Expansion | `src/obscuro.rs:225-253` | `expansion_step()`, `expansion_step_inner()` |
| Section 3.4 | Iteration Loop | `src/obscuro.rs:43-48` | (in `study_position()`) |
| Section 3.5 | Move Selection | `src/obscuro.rs:25-31` | `make_move()` |

## Data Structures

| Paper Concept | Implementation | File | Lines |
|--------------|----------------|------|-------|
| Game Tree Γ | `History<G>` enum | `src/history.rs` | 12-17 |
| Information Set | `Info<A, T>` struct | `src/info.rs` | - |
| Policy/Strategy | `Policy<A>` struct | `src/policy.rs` | 13-23 |
| Resolver Gadget | `ResolverGadget<G>` | `src/obscuro.rs` | 516-523 |
| Subgame Root | `SubgameRoot<G>` | `src/obscuro.rs` | 458-460 |

## Key Algorithm Components

### KLUSS (Knowledge-Limited Unfrozen Subgame Solving)

```
Paper Algorithm 1 → Implementation:
1. Pop histories from old tree    → pop_histories() (line 77-113)
2. Apply k-cover filtering        → k_cover() (line 146-169)
3. Add new sampled positions      → populate_histories() (line 114-141)
4. Create resolver structure      → SubgameRoot::new() (line 464-506)
```

### CFR+ (Counterfactual Regret Minimization)

```
Paper Algorithm 2 → Implementation:
1. Traverse tree with reach probs → make_utilities() (line 307-341)
2. Accumulate counterfactual regrets → Policy::add_counterfactual() (policy.rs:85-89)
3. Update regrets with CFR+       → Policy::update() (policy.rs:48-81)
4. Compute new strategy           → Policy::inst_policy() (policy.rs:102-111)
```

### Tree Expansion (PUCT-based)

```
Paper Algorithm 3 → Implementation:
1. Sample exploring player         → expansion_step() (line 225-233)
2. Traverse with explore/exploit   → expansion_step_inner() (line 234-253)
3. Expand leaf node                → History::expand() (history.rs:30-68)
4. Initialize with heuristic       → Policy::from_rewards() (policy.rs:28-45)
```

## Policy Calculation Methods

| Method | Purpose | Location | Formula |
|--------|---------|----------|---------|
| `inst_policy()` | Current strategy (last iterate) | `policy.rs:102-111` | Regret matching: `p_i = max(r_i, 0) / Σ max(r_j, 0)` |
| `avg_strategy` | Average strategy over time | `policy.rs:19` | `avg_i += inst_policy_i` at each iteration |
| `explore()` | PUCT exploration | `policy.rs:113-130` | `policy_i + c * sqrt(ln(N_total)/(1+N_i))` |
| `exploit()` | Best response | `policy.rs:132-141` | `argmax_i policy_i` |
| `purified()` | Deterministic move | `policy.rs:164-171` | `argmax_i avg_strategy_i` |

## Correctness Checklist

| Component | Paper Reference | Implementation Status | Verified |
|-----------|----------------|----------------------|----------|
| k-cover with k=3 | Section 3.1, Figure 2 | `k_cover()` with k=3 | ✅ |
| CFR+ positive projection | Section 3.2, Appendix | `max(r, 0)` in update | ✅ |
| Linear CFR momentum | Section 3.2 | `t/(t+1)` coefficient | ✅ |
| PUCT exploration | Section 3.3 | UCB formula with expansions | ✅ |
| Resolver ENTER/SKIP | Appendix B | `ResolveActions` enum | ✅ |
| Alternating explorer | Section 3.3 | P1 then P2 in `expansion_step()` | ✅ |
| Last-iterate strategy | Section 3.2 | Uses `inst_policy()` in traversal | ✅ |
| Purification | Section 3.5 | `purified()` using avg_strategy | ✅ |

## Fixed Issues

| Issue | Description | Old Code | New Code | File:Line |
|-------|-------------|----------|----------|-----------|
| #1 | Resolver policy unused | `p_enter = 1.0` | `p_enter = resolver.p_exploit(&ENTER)` | `obscuro.rs:281` |

## Known Deviations

| Deviation | Paper | Implementation | Impact | Priority |
|-----------|-------|----------------|--------|----------|
| Threading | 1 CFR + 2 expansion threads | Single-threaded sequential | Performance only | Low |
| Reach probs | Track actual policy probs | Assumes uniform during expand | Minor convergence rate | Low |
| Purification | "Cheap hints" with stability | Uses avg_strategy only | Actually more stable | N/A |

## Constants and Configuration

| Paper Parameter | Code Constant | Location | Default Value |
|----------------|---------------|----------|---------------|
| Time budget | `SOLVE_TIME_SECS` | `utils.rs` | (varies by time control) |
| k-order knowledge | k parameter | `obscuro.rs:84` | 3 |
| Min samples | `MIN_INFO_SIZE` | `utils.rs` | (game-specific) |
| Exploration constant | `EXPLORE_CONSTANT` | `policy.rs` | (PUCT c value) |
| CFR+ baseline | baseline calculation | `policy.rs:61` | `expectation()` |

## Testing and Verification

To verify correctness of the implementation:

1. **k-cover correctness**: Check that `k_cover()` produces same results as manual graph traversal
2. **CFR+ convergence**: Verify regrets converge in simple test games
3. **Exploration coverage**: Ensure all actions get explored
4. **Resolver structure**: Verify ENTER/SKIP decisions make sense
5. **Move quality**: Compare against paper's reported performance

## Related Files

- Main algorithm: `src/obscuro.rs` (538 lines)
- Game tree: `src/history.rs` (208 lines)
- Policy/CFR+: `src/policy.rs` (226 lines)
- Information sets: `src/info.rs` (57 lines)
- Game interface: `src/utils.rs` (94 lines)
- Self-play: `src/self_play.rs` (98 lines)
- Game implementations: `src/games/` directory

## Quick Navigation Commands

```bash
# View main algorithm
view src/obscuro.rs 37-51

# View KLUSS implementation
view src/obscuro.rs 146-223

# View CFR+ implementation
view src/obscuro.rs 255-304

# View policy calculations
view src/policy.rs 48-81

# View tree expansion
view src/history.rs 30-68
```
