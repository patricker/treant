# Phase 0 — Backend rearchitecture: `treant-games` shared grid engine

**Date:** 2026-06-15
**Status:** Approved, ready for implementation
**Scope:** Rust/WASM only. No frontend feature work. Existing demos act as the regression net.

> Spec lives in repo-root `specs/` (not `docs/`) because `docs/` is the
> Docusaurus site; keeping design docs out of it avoids any build/serve confusion.

## Context

This is the first phase of a larger effort to turn the treant docs "playground"
into a mobile-first **family game arcade** on mcts.dev (static GitHub Pages,
WASM AI, local pass-and-play — no backend, no accounts). Phase 0 cleans up the
backend *before* any arcade UI exists, so the arcade later lands on a uniform,
tested engine interface.

### Why backend-first
- **Refactor-under-test:** the existing browser demos + 111 core tests pin
  current behavior. Rearchitecting *behind a stable WASM interface* means the
  demos immediately reveal regressions.
- **Separates risk:** backend consolidation is correctness-critical but
  objectively verifiable; arcade UI is subjective/exploratory. Don't tangle them.
- **Testability:** today the grid games live entirely in `treant-wasm` and are
  only exercised through the browser. Moving the logic into a plain Rust crate
  makes it `cargo test`-able directly.

### Current state (the problem)
- `treant-wasm/src/connectfour.rs` (~428 lines) and `treant-wasm/src/tictactoe.rs`
  (~393 lines) are **independent implementations** that share a near-identical
  m,n,k structure (board of cells, k-in-a-row win scan, draw-on-full, modular
  turn advance, window-based heuristic eval) but duplicate all of it.
- `shift.rs`, `mancala.rs`, `nim.rs`, `game2048.rs`, `dice.rs` are separate and
  out of scope this phase.
- The React demos (`ConnectFourDemo.tsx`, `TicTacToeDemo.tsx`) each call their
  own WASM class and parse the board string. They must keep working untouched.

## Goal

Extract one shared, unit-tested m,n,k grid engine into a new `treant-games`
crate covering **Connect Four (gravity) + Tic-Tac-Toe (free placement)**. The
two `*Wasm` classes become thin facades over it, preserving their exact public
WASM API and string formats so the frontend does not change.

Non-goal: changing any observable behavior, search results, or the WASM surface.

## Architecture

### New crate: `treant-games`
- Workspace member at `treant-games/`, **path-only / unpublished** (no release
  overhead; `publish = false`).
- Depends on `treant` (path) and `rand` (matching the workspace version).
- Added to the root `Cargo.toml` `members` list.
- Future home for the other games and the duplicated `examples/` game code, but
  that migration is **out of scope** for Phase 0.

### Module: `treant_games::grid`
A single `GameState` + `Evaluator` + MCTS-spec set, parameterized by config.

```rust
pub struct GridConfig {
    pub cols: usize,
    pub rows: usize,
    pub k: usize,
    pub num_players: usize,
    pub gravity: bool,          // true=Connect Four, false=Tic-Tac-Toe
    pub center_column_bonus: bool, // true=CF heuristic, false=TTT
}

pub struct GridGame { /* board + config + current player */ }   // impl GameState
pub struct GridEval;                                            // impl Evaluator
pub struct GridMcts { pub solver: bool }                        // impl MCTS
```

- **Board representation:** internal layout is the engine's choice (likely flat
  `Vec<Cell>` row-major), but the engine exposes `cell(row, col) -> Option<u8>`
  (0-indexed player) so each facade can format its *own* exact board string.
- **Move type:** `GridMove(u8)` whose meaning depends on `gravity`:
  - gravity → column index; `make_move` drops to lowest empty row in that column.
  - placement → cell index `row * cols + col`.
- **`available_moves`:** non-full columns (gravity) or empty cells (placement);
  empty once `winner().is_some()` — matching both current games.
- **Win detection:** k-in-a-row scan in 4 directions (H, V, both diagonals).
  Order-independent; reproduces both games' `winner()`.
- **`terminal_value`:** `Some(Loss)` if there's a winner (the winner just moved,
  so the current player lost), `Some(Draw)` if full, else `None`. Matches both.
- **Turn advance:** `current = (current + 1) % num_players`.
- **MCTS spec:** `TreePolicy = UCTPolicy` (built with `UCTPolicy::new(1.4)` in the
  facade, as today); `solver_enabled()` returns `self.solver`.

### Facades (`treant-wasm`)
`ConnectFourWasm` and `TicTacToeWasm` keep **every** current method and signature,
now delegating to `MCTSManager<GridMcts>` over `GridGame`. Each facade retains
only the genuinely game-specific bits:
- its **constructor clamps** (CF: cols/rows/k min 3; TTT: min 2; both max 10,
  players clamp 2..=4),
- its **board-string formatter** (see contract below),
- its **MCTS config flags** (CF `solver=false`, no... see contract; TTT `solver=true`),
- TTT's extra `root_proven_value()` method (CF has none).

## Preservation contract (the must-not-break list)

The refactor is behavior-preserving. These exact behaviors are pinned by tests
and the demos:

1. **Board strings.**
   - Connect Four `get_board()`: **top row first** (rows emitted high→low),
     left→right, `' '`=empty, `'1'..'4'` = player 0..3 (digit = player+1).
   - Tic-Tac-Toe `get_board()`: **row-major top-left first**, `' '`=empty,
     symbols `['X','O','A','B']` for players 0..3.
2. **Move strings.** Both `apply_move(s)` parse a non-negative integer:
   CF a **column** `< cols`; TTT a **cell index** `< cols*rows`. Invalid → `false`.
   The existing fallback (`advance` fails → `playout_n(100)` → retry `advance`) is
   preserved.
3. **`result()`.** Winner → `"{winner+1}"` (1-indexed); full → `"Draw"`;
   otherwise `""`. (Both games already agree on this.)
4. **`current_player()`** returns the 0-indexed current player as `u32`.
5. **Evaluation heuristic.** Window-based scoring is shared, with the **only**
   difference being Connect Four's **center-column bonus** (+3 per own piece in
   the middle column), absent in Tic-Tac-Toe. Gated by `center_column_bonus`.
   The window branches (`mine==k`→+1000, `theirs==k`→-1000, `mine==k-1 && empty==1`
   →+50, `theirs==k-1 && empty==1`→-80, open run →+5) are identical between the two
   games (CF's `empty==k-mine` and TTT's `theirs==0` are equivalent within a
   k-window), so they unify cleanly.
6. **Solver.** Tic-Tac-Toe enables `solver_enabled()`; Connect Four does not.
   (This affects `root_proven_value()` and proven-value propagation.)
7. **Search determinism / ordering.** `available_moves()` ordering is preserved
   (ascending column index for CF; ascending cell index for TTT) so move
   selection and demo behavior are unchanged.
8. **Constructor clamps** preserved per-game as listed above.

## Testing

Phase 0's payoff is that the engine becomes directly testable in Rust.

**`treant-games` unit tests (`cargo test -p treant-games`):**
- Win detection: horizontal, vertical, both diagonals, varied `k`, near-misses.
- Gravity legal moves (non-full columns, drop-to-lowest) vs placement legal
  moves (empty cells); `available_moves` empties on a win.
- Draw detection on a full board.
- Multi-player (`num_players` = 3 and 4) turn order and win attribution.
- `terminal_value` returns Loss-for-mover / Draw / None correctly.
- Heuristic: a scripted position scores identically to the pre-refactor
  `evaluate_for` for both the CF (center-bonus) and TTT (no-bonus) configs —
  encode a couple of golden numbers to lock it.

**Facade parity (`treant-wasm`):**
- Lightweight Rust-level tests (where feasible without a browser) asserting board
  strings and move parsing match the documented formats for a scripted game.
- Manual regression: `cd docs && npm start`, play a Connect Four and a
  Tic-Tac-Toe game (human + AI), confirm board renders, moves work, win/draw and
  AI play look unchanged.

**Project gates:** `cargo test` (whole workspace), `cargo clippy` at **0
warnings**, `cd treant-wasm && wasm-pack build --target web` succeeds.

## Build sequence

1. Scaffold `treant-games` crate; wire into workspace `members`; `cargo build`.
2. Implement `grid` engine (`GridConfig`, `GridGame: GameState`, `GridEval`,
   `GridMcts: MCTS`) **test-first** — engine unit tests before/with the impl.
3. Re-point `ConnectFourWasm` at the engine; keep its exact API + board format +
   clamps; remove its now-dead duplicated logic.
4. Re-point `TicTacToeWasm` at the engine (incl. `solver=true` and
   `root_proven_value`); remove its duplicated logic.
5. Verify: `cargo test` (core + games), `cargo clippy` (0 warnings),
   `wasm-pack build`, and a manual demo regression pass.

## Out of scope (later phases)
- Arcade UI, the `useGameSession` driver, `GameDefinition`, modes/difficulty.
- Folding **Shift** (place-then-move) into the engine.
- Migrating Mancala / Nim / 2048 / Dice or the `examples/` games into
  `treant-games`.
- Publishing `treant-games` to crates.io.

## Risks

- **Hidden behavioral coupling** (e.g. a board-string off-by-one, or the
  center-bonus) silently changing AI moves. Mitigated by the preservation
  contract + golden heuristic numbers + manual demo pass.
- **Over-generalizing the engine** before Shift/other games exist. Mitigated by
  scoping strictly to CF+TTT and letting later phases extend.
