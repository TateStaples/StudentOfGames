# Core Solver (Obscuro) — Condensed

This summarizes the single-threaded Obscuro implementation and how it aligns with the paper.

## What It Implements
- KLUSS: k-cover reasoning over infosets
- PCFR+ (CFR+ with linear weighting)
- PUCT-based expansion
- Resolver gadget with ENTER/SKIP and alternate values
- Move selection via average strategy + optional purification

## Key Fixes (Applied)
- Resolver policy usage: use `p_exploit(ENTER)` instead of hardcoded 1.0
- Alternate value: subtract gift value ĝ(J) to match paper
- Priors α(J): 0.5·(uniform + belief) blending

## Minor Differences (Accepted)
- Initial expansion uses uniform reach; corrected on first CFR update
- Single-threaded execution (performance only)
- No pruning for zero-prob opponent branches (minor efficiency)

## Where In Code
- Main: `src/obscuro.rs`
- Support: `src/history.rs`, `src/policy.rs`, `src/info.rs`, `src/utils.rs`

## Verification
- Pseudocode alignment: resolves ENTER/SKIP policy and priors
- Proof sketches: CFR convergence, safe subgame solving, PUCT exploration

## Learn More
- Detailed mapping and proofs were consolidated into this summary. For concrete traces and deviation checks, see CHANGELOG.md (Key Changes) and the source files noted above.
