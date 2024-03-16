# Student of Games
A general purpose games solver for imperfect information and stochastic games using a combination of Counterfactural Regret Minimization and AlphaZero

## What's implemented
### Game

### Policy

### AlphaZero

### CFR
Counter factual Regret Minimization

### Student of Games
#### CFR+

#### Modified Continual-Resolving

#### Growing Tree Counterfactual Regret Minimization (GT-CFR)

#### Counterfactual Value-and-Policy Network (CVPN)


## Terminology 
(Factored-Observation Stochastic Game formalism)
- World States (w): decision node, terminal node, chance node
- Actions (a): choice by player
- History (h): sequence of actions
- Observations (O): action updates on Information State
- Information State (s): The set of public / private indistinguishable histories from observations
- Policy (P/π): Information State -> Distribution of action probabilities
- Transition Function (T): world state update from active players actions (can be stochastic)
- Belief State (B): A range for the private information states conditioned on observation history
- Counter Factual Regret (CFR): P'(s, a) = averaged regret over time
- CFR+: update quality (Q) of an action instead of exploring whole tree. P(s, a) = weighted average of quality
- Proper Game: sub-game with corresponding public information and belief of information state
- Counterfactual Value-and-Policy Network (CVPN): belief -> values and action policies (parameters Ø)
  - Value (v): EV for each information state for each player. Trained on queries
  - Policy(P/π): action probabilities for each information state for acting player. Trained on trajectories
- Regret (r): opportunity cost, r(s,a) = v(s,a) - EV(P(s, a)).
- Modified Continual-Resolving: repeated safe resolving (from prev sol and opp CFR values)

## Questions:
1. What are the transition cases: poker vs Fog of War chess
2. Figure out how DeepStack modified continual-resolving works
3. In AlphaZero, does self-play share game tree?

## In Progress
- Implement the modified_resolving
- Implement the Bayesian pass through
- Implement the policy sampling (trait?)
- Add noise
- Check the Licence for the code
- Implementing games for testing and fun 
- Maybe reimplement the QuerySolver

## Goals
- [ ] Expand to multiplayer games
- [ ] Hierarchical Information State
  - Encode game dynamics into State Space Architecture (ie Mamba), then diffusion planning by varying ∆t
- [ ] Support for non-rivalrous games
- [ ] Support for imitation learning starting point

## Games
- Tic Tac Toe
- Connect 4
- Poker
- Chess
- Go
- Rock Paper Scissors