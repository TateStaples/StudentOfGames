# Changelog — Key Changes & Fixes

This summarizes the important implementation changes and their intent.

## Core Solver (Obscuro)
- Resolver policy fix: use `p_exploit(ENTER)`
- Alternate value: implemented gift value ĝ(J); use `u(x,y|J) - ĝ(J)`
- Prior α(J): `0.5*(1/m + y(J)/Σ y)` blending
- Minor: initial uniform reach in expansion (corrected by first CFR update)

## Parallel Training
- Added `ParallelTrainer` and `train_parallel` binary
- Batched consolidation pattern for safe, fast learning
- Defaults tuned for 30s think time with thread-based speedup

## Neural Stack
- Value/Policy net + `NeuralSolver` scaffolding
- Demo binary for evaluation and perf comparisons
- Thread-safety caveat in burn; recommend lazy init or sequential

## File Map (New Structure)
- README.md — Index and orientation
- USAGE.md — Build, run, tune, troubleshoot
- CORE.md — Core solver summary
- PARALLEL.md — Parallel training overview
- NEURAL.md — Neural stack overview
- CHANGELOG.md — This file

Older long-form docs were consolidated into these six for clarity and maintenance.
