# Perfect-Information Games: Rules and Code Mapping

This document summarizes the exact rules implemented for the perfect-information games and how they map to code, so you can quickly verify correctness.

## Othello (Reversi)
- Board: 8x8; initial discs at D4/E5 (White) and D5/E4 (Black).
- Turn: Alternating; a legal move must flip at least one opponent disc along a straight line (8 directions).
- Pass: Only if no legal placement exists; game ends when both players have no legal placement.
- Scoring: Terminal evaluation is normalized disc differential: (black - white)/64.
- Code:
  - State: [src/games/perfect_info/othello.rs](src/games/perfect_info/othello.rs)
  - Actions: `OthelloAction::{Place(x,y), Pass}`
  - Legality: `legal_dirs_from()` and `flips_for()` implement the bracketing/flip rule.
  - Terminal: `is_over()` checks that neither player has legal placements.
  - Eval: `evaluate()` computes normalized disc difference at terminal, else 0.0.

## Go (9x9, simplified)
- Board: 9x9; P1=Black, P2=White. Actions: `Place(x,y)` or `Pass`.
- Captures: After placement, adjacent opponent groups with no liberties are removed.
- Suicide: Illegal unless the placement captures at least one opponent group.
- Ko: Simple-ko only — reject moves that recreate the position from one ply ago.
- End: Two consecutive passes.
- Scoring: Area-style approximation: stones + surrounded empty regions, with komi 6.5 for White. Eval is (BlackScore - WhiteScore)/(81).
- Code:
  - State: [src/games/perfect_info/go.rs](src/games/perfect_info/go.rs)
  - Actions: `GoAction::{Place, Pass}`
  - Group/liberties: `group_and_liberties()` flood-fills; `remove_group()` applies captures.
  - Suicide check: `would_be_legal_place()` verifies liberties unless capture happens.
  - Ko: `prev_hash` stores previous position hash; new hash must differ.
  - Terminal: `is_over()` checks pass streak.
  - Eval: `area_score()` + komi ⇒ normalized differential.

## Chess (standard)
- Rules: Full FIDE rules via the `chess` crate, including castling, en passant, promotions, check status, and legal move generation.
- Terminal: Stalemate (draw) or Checkmate. Eval: +1 for P1 (White) checkmating P2; -1 when P1 is checkmated; 0 for draws/non-terminal.
- Code:
  - State: [src/games/perfect_info/chess.rs](src/games/perfect_info/chess.rs)
  - Actions: `chess::ChessMove` via `MoveGen::new_legal()`.
  - Terminal: `board.status()`.
  - Trace: FEN string (`ChessTrace`) to uniquely represent the public state.

## Atomic Chess (status)
- Not yet implemented. It requires custom legality (explosion on capture) and filtering moves that would explode own king. This is planned as a separate module with clear tests:
  - Explosion: Capturing piece and all non-pawn pieces in the 3x3 around the capture square are removed.
  - King elimination ends the game.
  - Moves that explode own king will be illegal.
- If you want this prioritized next, I can implement a self-contained rule engine and tests.

## Testing Guidance
- Othello: Verify opening legal moves (four diagonals for Black), mid-game flips in multiple directions, forced passes, and game-end disc counts.
- Go: Test single-stone captures, snapbacks with simple-ko prevention, multi-group captures, suicide prohibition, and two-pass termination.
- Chess: Spot-check a few known FEN positions for legal move counts; verify checkmate and stalemate detection.

If you need additional rule variants (board sizes, komi, superko, alternative scoring), I can parameterize the modules and add tests.
