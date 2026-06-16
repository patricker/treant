# treant-games Grid Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract a shared, `cargo test`-able m,n,k grid engine (Connect Four + Tic-Tac-Toe) into a new `treant-games` crate, and reduce the two WASM classes to behavior-preserving facades.

**Architecture:** A new path-only workspace crate `treant-games` holds `grid::{GridConfig, GridGame, GridEval, GridMcts, GridMove}`. `GridGame` is one `GameState` parameterized by `gravity` (Connect Four vs Tic-Tac-Toe) and `center_column_bonus` (CF heuristic). `ConnectFourWasm` / `TicTacToeWasm` keep their exact public API + board-string formats, delegating to `MCTSManager<GridMcts>`.

**Tech Stack:** Rust, `treant` core crate, `wasm-bindgen`. Reference spec: `specs/2026-06-15-treant-games-grid-engine-design.md`.

---

## File Structure

- Create: `treant-games/Cargo.toml` — crate manifest (path dep on `treant`, `publish = false`).
- Create: `treant-games/src/lib.rs` — crate root, `pub mod grid;`.
- Create: `treant-games/src/grid.rs` — the engine: config, game, evaluator, MCTS spec, plus unit tests.
- Modify: `Cargo.toml` (root) — add `treant-games` to `[workspace] members`.
- Modify: `treant-wasm/Cargo.toml` — add `treant-games` path dep.
- Modify: `treant-wasm/src/connectfour.rs` — replace engine with a facade over `grid`.
- Modify: `treant-wasm/src/tictactoe.rs` — replace engine with a facade over `grid`.

---

## Task 1: Scaffold the `treant-games` crate

**Files:**
- Create: `treant-games/Cargo.toml`
- Create: `treant-games/src/lib.rs`
- Modify: `Cargo.toml` (root)

- [ ] **Step 1: Create the manifest**

`treant-games/Cargo.toml`:
```toml
[package]
name = "treant-games"
version = "0.1.0"
edition = "2021"
authors = ["Peter Wicks <patricker@gmail.com>"]
description = "Reusable game implementations built on the treant MCTS library (shared m,n,k grid engine)."
repository = "https://github.com/patricker/treant"
homepage = "https://mcts.dev"
license = "MIT"
publish = false

[dependencies]
treant = { version = "0.4", path = ".." }
```

- [ ] **Step 2: Create the crate root**

`treant-games/src/lib.rs`:
```rust
//! Reusable games built on the `treant` MCTS library.
//!
//! Currently provides a shared m,n,k grid engine ([`grid`]) backing both
//! Connect Four (gravity) and Tic-Tac-Toe (free placement).
pub mod grid;
```

- [ ] **Step 3: Register the crate in the workspace**

In root `Cargo.toml`, change the members line to:
```toml
members = [".", "treant-wasm", "treant-dynamic", "treant-gumbel", "treant-games"]
```

- [ ] **Step 4: Verify it builds (grid.rs not yet present → expect failure, then add stub)**

Create a temporary empty `treant-games/src/grid.rs` containing only `// placeholder`.
Run: `cargo build -p treant-games`
Expected: builds (empty module).

- [ ] **Step 5: Commit**

```bash
git add treant-games/Cargo.toml treant-games/src/lib.rs treant-games/src/grid.rs Cargo.toml
git commit -m "feat(games): scaffold treant-games crate"
```

---

## Task 2: Grid board core — `GridConfig`, `GridGame`, win/full detection

**Files:**
- Modify: `treant-games/src/grid.rs`

- [ ] **Step 1: Write the failing tests**

Replace `treant-games/src/grid.rs` placeholder with the imports, the board core, and these tests at the bottom:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn cf(cols: usize, rows: usize, k: usize, players: usize) -> GridConfig {
        GridConfig { cols, rows, k, num_players: players, gravity: true, center_column_bonus: true }
    }
    fn ttt(cols: usize, rows: usize, k: usize, players: usize) -> GridConfig {
        GridConfig { cols, rows, k, num_players: players, gravity: false, center_column_bonus: false }
    }

    // place at (row,col) for player p directly (test helper, bypasses turn order)
    fn put(g: &mut GridGame, row: usize, col: usize, p: u8) {
        let i = g.idx(row, col);
        g.board[i] = Cell::Player(p);
    }

    #[test]
    fn detects_horizontal_win() {
        let mut g = GridGame::new(ttt(5, 5, 3, 2));
        put(&mut g, 2, 0, 0); put(&mut g, 2, 1, 0); put(&mut g, 2, 2, 0);
        assert_eq!(g.winner(), Some(0));
    }

    #[test]
    fn detects_vertical_win() {
        let mut g = GridGame::new(ttt(5, 5, 3, 2));
        put(&mut g, 0, 1, 1); put(&mut g, 1, 1, 1); put(&mut g, 2, 1, 1);
        assert_eq!(g.winner(), Some(1));
    }

    #[test]
    fn detects_both_diagonals() {
        let mut g = GridGame::new(ttt(4, 4, 3, 2));
        put(&mut g, 0, 0, 0); put(&mut g, 1, 1, 0); put(&mut g, 2, 2, 0);
        assert_eq!(g.winner(), Some(0));
        let mut h = GridGame::new(ttt(4, 4, 3, 2));
        put(&mut h, 0, 2, 1); put(&mut h, 1, 1, 1); put(&mut h, 2, 0, 1);
        assert_eq!(h.winner(), Some(1));
    }

    #[test]
    fn no_false_win_below_k() {
        let mut g = GridGame::new(ttt(5, 5, 4, 2));
        put(&mut g, 0, 0, 0); put(&mut g, 0, 1, 0); put(&mut g, 0, 2, 0);
        assert_eq!(g.winner(), None);
    }

    #[test]
    fn full_board_detected() {
        let mut g = GridGame::new(ttt(2, 2, 3, 2));
        for r in 0..2 { for c in 0..2 { put(&mut g, r, c, 0); } }
        assert!(g.is_full());
        assert!(!GridGame::new(ttt(2, 2, 3, 2)).is_full());
    }

    #[test]
    fn cell_accessor_origin_is_top_left() {
        let mut g = GridGame::new(ttt(3, 3, 3, 2));
        put(&mut g, 0, 0, 2);
        assert_eq!(g.cell(0, 0), Some(2));
        assert_eq!(g.cell(1, 1), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p treant-games`
Expected: FAIL — `GridGame`, `GridConfig`, `Cell`, etc. not defined.

- [ ] **Step 3: Implement the board core**

Put this above the `#[cfg(test)]` block in `treant-games/src/grid.rs`:
```rust
//! Shared m,n,k grid engine backing Connect Four (gravity) and Tic-Tac-Toe.
use treant::tree_policy::UCTPolicy;
use treant::*;

/// Maximum board dimension (columns or rows) supported.
pub const MAX_DIM: usize = 10;
/// Maximum number of players supported.
pub const MAX_PLAYERS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Player(u8), // 0-indexed
}

/// Static parameters for a grid game.
#[derive(Clone, Copy, Debug)]
pub struct GridConfig {
    pub cols: usize,
    pub rows: usize,
    pub k: usize,
    pub num_players: usize,
    /// `true` = Connect Four (drop into a column); `false` = free placement.
    pub gravity: bool,
    /// `true` adds Connect Four's center-column heuristic bonus.
    pub center_column_bonus: bool,
}

/// A move. For gravity games it is a **column index**; for placement games it
/// is a **cell index** (`row * cols + col`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridMove(pub u8);

impl std::fmt::Display for GridMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An m,n,k grid game state. Board is stored row-major with **row 0 = top**;
/// gravity drops pieces to the largest empty row index (the bottom).
#[derive(Clone, Debug)]
pub struct GridGame {
    board: Vec<Cell>,
    cfg: GridConfig,
    current: u8,
}

impl GridGame {
    pub fn new(cfg: GridConfig) -> Self {
        Self {
            board: vec![Cell::Empty; cfg.cols * cfg.rows],
            cfg,
            current: 0,
        }
    }

    #[inline]
    fn idx(&self, row: usize, col: usize) -> usize {
        row * self.cfg.cols + col
    }

    /// 0-indexed player at `(row, col)` (row 0 = top), or `None` if empty.
    pub fn cell(&self, row: usize, col: usize) -> Option<u8> {
        match self.board[self.idx(row, col)] {
            Cell::Empty => None,
            Cell::Player(p) => Some(p),
        }
    }

    pub fn config(&self) -> &GridConfig {
        &self.cfg
    }

    pub fn current(&self) -> u8 {
        self.current
    }

    /// Lowest empty row (largest index) in a column, for gravity games.
    fn drop_row(&self, col: usize) -> Option<usize> {
        (0..self.cfg.rows)
            .rev()
            .find(|&row| self.board[self.idx(row, col)] == Cell::Empty)
    }

    /// The player who has a k-in-a-row, if any.
    pub fn winner(&self) -> Option<u8> {
        let (rows, cols, k) = (self.cfg.rows, self.cfg.cols, self.cfg.k);
        let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
        for r in 0..rows {
            for c in 0..cols {
                if let Cell::Player(p) = self.board[self.idx(r, c)] {
                    for &(dr, dc) in &dirs {
                        let mut count = 1usize;
                        for step in 1..k {
                            let nr = r as i32 + dr * step as i32;
                            let nc = c as i32 + dc * step as i32;
                            if nr < 0 || nr >= rows as i32 || nc < 0 || nc >= cols as i32 {
                                break;
                            }
                            if self.board[self.idx(nr as usize, nc as usize)] == Cell::Player(p) {
                                count += 1;
                            } else {
                                break;
                            }
                        }
                        if count >= k {
                            return Some(p);
                        }
                    }
                }
            }
        }
        None
    }

    /// Every cell is occupied.
    pub fn is_full(&self) -> bool {
        self.board.iter().all(|&c| c != Cell::Empty)
    }

    /// Heuristic value of the position from `player`'s perspective.
    fn evaluate_for(&self, player: u8) -> i64 {
        let (rows, cols, k) = (self.cfg.rows, self.cfg.cols, self.cfg.k);
        let my = Cell::Player(player);
        let mut score: i64 = 0;

        if self.cfg.center_column_bonus {
            let center = cols / 2;
            for r in 0..rows {
                if self.board[self.idx(r, center)] == my {
                    score += 3;
                }
            }
        }

        let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
        for r in 0..rows {
            for c in 0..cols {
                for &(dr, dc) in &dirs {
                    let end_r = r as i32 + dr * (k as i32 - 1);
                    let end_c = c as i32 + dc * (k as i32 - 1);
                    if end_r < 0 || end_r >= rows as i32 || end_c < 0 || end_c >= cols as i32 {
                        continue;
                    }
                    let mut mine = 0usize;
                    let mut empty = 0usize;
                    let mut theirs = 0usize;
                    for step in 0..k {
                        let rr = (r as i32 + dr * step as i32) as usize;
                        let cc = (c as i32 + dc * step as i32) as usize;
                        match self.board[self.idx(rr, cc)] {
                            cell if cell == my => mine += 1,
                            Cell::Empty => empty += 1,
                            _ => theirs += 1,
                        }
                    }
                    if mine == k {
                        score += 1000;
                    } else if theirs == k {
                        score -= 1000;
                    } else if mine == k - 1 && empty == 1 {
                        score += 50;
                    } else if theirs == k - 1 && empty == 1 {
                        score -= 80;
                    } else if mine >= 2 && theirs == 0 {
                        score += 5;
                    }
                }
            }
        }
        score
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p treant-games`
Expected: PASS (6 tests). Note: tests reference private `idx`/`board`; they live in the same module so that is allowed.

- [ ] **Step 5: Commit**

```bash
git add treant-games/src/grid.rs
git commit -m "feat(games): grid board core with win/full detection + heuristic"
```

---

## Task 3: `GameState` impl — moves, make_move, terminal_value

**Files:**
- Modify: `treant-games/src/grid.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `treant-games/src/grid.rs`:
```rust
    #[test]
    fn gravity_moves_are_nonfull_columns() {
        let g = GridGame::new(cf(7, 6, 4, 2));
        let moves: Vec<u8> = g.available_moves().iter().map(|m| m.0).collect();
        assert_eq!(moves, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn gravity_drops_to_bottom() {
        let mut g = GridGame::new(cf(7, 6, 4, 2));
        g.make_move(&GridMove(3)); // player 0 into column 3
        // bottom row is rows-1 = 5
        assert_eq!(g.cell(5, 3), Some(0));
        assert_eq!(g.cell(4, 3), None);
        assert_eq!(g.current(), 1);
    }

    #[test]
    fn placement_moves_are_empty_cells_and_place_exactly() {
        let mut g = GridGame::new(ttt(3, 3, 3, 2));
        assert_eq!(g.available_moves().len(), 9);
        g.make_move(&GridMove(4)); // center cell of 3x3
        assert_eq!(g.cell(1, 1), Some(0));
        assert_eq!(g.available_moves().len(), 8);
    }

    #[test]
    fn no_moves_after_win() {
        let mut g = GridGame::new(ttt(5, 5, 3, 2));
        put(&mut g, 2, 0, 0); put(&mut g, 2, 1, 0); put(&mut g, 2, 2, 0);
        assert!(g.available_moves().is_empty());
    }

    #[test]
    fn terminal_value_loss_for_mover_then_draw() {
        let mut win = GridGame::new(ttt(5, 5, 3, 2));
        put(&mut win, 2, 0, 0); put(&mut win, 2, 1, 0); put(&mut win, 2, 2, 0);
        assert_eq!(win.terminal_value(), Some(ProvenValue::Loss));

        let mut full = GridGame::new(ttt(2, 2, 3, 2));
        for r in 0..2 { for c in 0..2 { put(&mut full, r, c, 0); } }
        assert_eq!(full.terminal_value(), Some(ProvenValue::Draw));

        assert_eq!(GridGame::new(ttt(3, 3, 3, 2)).terminal_value(), None);
    }

    #[test]
    fn turn_order_wraps_for_three_players() {
        let mut g = GridGame::new(ttt(5, 5, 4, 3));
        g.make_move(&GridMove(0));
        g.make_move(&GridMove(1));
        g.make_move(&GridMove(2));
        assert_eq!(g.current(), 0); // wrapped 0->1->2->0
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p treant-games`
Expected: FAIL — `available_moves`, `make_move`, `terminal_value` not found (no `GameState` impl yet).

- [ ] **Step 3: Implement `GameState`**

Add below the `impl GridGame` block (before `#[cfg(test)]`):
```rust
impl GameState for GridGame {
    type Move = GridMove;
    type Player = u8;
    type MoveList = Vec<GridMove>;

    fn current_player(&self) -> u8 {
        self.current
    }

    fn available_moves(&self) -> Vec<GridMove> {
        if self.winner().is_some() {
            return vec![];
        }
        if self.cfg.gravity {
            (0..self.cfg.cols as u8)
                .filter(|&col| self.drop_row(col as usize).is_some())
                .map(GridMove)
                .collect()
        } else {
            (0..(self.cfg.cols * self.cfg.rows))
                .filter(|&i| self.board[i] == Cell::Empty)
                .map(|i| GridMove(i as u8))
                .collect()
        }
    }

    fn make_move(&mut self, mov: &GridMove) {
        if self.cfg.gravity {
            let col = mov.0 as usize;
            if let Some(row) = self.drop_row(col) {
                let i = self.idx(row, col);
                self.board[i] = Cell::Player(self.current);
            }
        } else {
            self.board[mov.0 as usize] = Cell::Player(self.current);
        }
        self.current = (self.current + 1) % self.cfg.num_players as u8;
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.winner().is_some() {
            Some(ProvenValue::Loss) // winner just moved; current player lost
        } else if self.is_full() {
            Some(ProvenValue::Draw)
        } else {
            None
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p treant-games`
Expected: PASS (12 tests total).

- [ ] **Step 5: Commit**

```bash
git add treant-games/src/grid.rs
git commit -m "feat(games): GameState impl for grid (moves, make_move, terminal)"
```

---

## Task 4: `GridEval` + `GridMcts` and a search smoke test

**Files:**
- Modify: `treant-games/src/grid.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module:
```rust
    #[test]
    fn center_bonus_only_applies_with_gravity_config() {
        // single piece in the center column, k=4 so no window branch fires
        let mut g = GridGame::new(cf(7, 6, 4, 2));
        put(&mut g, 5, 3, 0); // bottom-center
        assert_eq!(g.evaluate_for(0), 3); // center bonus only
        let mut t = GridGame::new(ttt(7, 6, 4, 2));
        put(&mut t, 5, 3, 0);
        assert_eq!(t.evaluate_for(0), 0); // no center bonus
    }

    #[test]
    fn heuristic_rewards_won_window() {
        let mut g = GridGame::new(ttt(5, 1, 3, 2));
        put(&mut g, 0, 0, 0); put(&mut g, 0, 1, 0); put(&mut g, 0, 2, 0);
        // three-in-a-row window = +1000; player 1 sees -1000
        assert!(g.evaluate_for(0) >= 1000);
        assert!(g.evaluate_for(1) <= -1000);
    }

    #[test]
    fn search_runs_and_picks_a_legal_move() {
        let mut mgr = MCTSManager::new(
            GridGame::new(cf(7, 6, 4, 2)),
            GridMcts { solver: false },
            GridEval,
            UCTPolicy::new(1.4),
            (),
        );
        mgr.playout_n(200);
        let m = mgr.best_move().expect("a move");
        assert!((m.0 as usize) < 7);
    }

    #[test]
    fn solver_proves_trivial_ttt_win() {
        // 3x1 board, player to move can complete 3-in-a-row immediately
        let mut g = GridGame::new(ttt(3, 1, 3, 2));
        // pre-place two of player 0's pieces, leave the third cell for them
        // (turn order: current is 0)
        let idx2 = g.idx(0, 0);
        g.board[idx2] = Cell::Player(0);
        let idx3 = g.idx(0, 1);
        g.board[idx3] = Cell::Player(0);
        let mut mgr = MCTSManager::new(
            g,
            GridMcts { solver: true },
            GridEval,
            UCTPolicy::new(1.4),
            (),
        );
        mgr.playout_n(50);
        assert_eq!(mgr.best_move(), Some(GridMove(2))); // the winning cell
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p treant-games`
Expected: FAIL — `GridEval`, `GridMcts` not defined.

- [ ] **Step 3: Implement the evaluator and MCTS spec**

Add before the `#[cfg(test)]` block:
```rust
/// Evaluator: window-based heuristic, negamax-interpreted per player.
pub struct GridEval;

#[derive(Clone, Debug)]
pub struct GridStateEval {
    score: i64,
    player: u8,
}

impl Evaluator<GridMcts> for GridEval {
    type StateEvaluation = GridStateEval;

    fn evaluate_new_state(
        &self,
        state: &GridGame,
        moves: &Vec<GridMove>,
        _: Option<SearchHandle<GridMcts>>,
    ) -> (Vec<()>, GridStateEval) {
        let player = state.current;
        (
            vec![(); moves.len()],
            GridStateEval { score: state.evaluate_for(player), player },
        )
    }

    fn interpret_evaluation_for_player(&self, evaln: &GridStateEval, player: &u8) -> i64 {
        if *player == evaln.player {
            evaln.score
        } else {
            -evaln.score
        }
    }

    fn evaluate_existing_state(
        &self,
        state: &GridGame,
        _evaln: &GridStateEval,
        _: SearchHandle<GridMcts>,
    ) -> GridStateEval {
        let player = state.current;
        GridStateEval { score: state.evaluate_for(player), player }
    }
}

/// MCTS spec for grid games. `solver` toggles MCTS-Solver (on for Tic-Tac-Toe,
/// off for Connect Four).
pub struct GridMcts {
    pub solver: bool,
}

impl MCTS for GridMcts {
    type State = GridGame;
    type Eval = GridEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();

    fn solver_enabled(&self) -> bool {
        self.solver
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p treant-games`
Expected: PASS (16 tests total).

- [ ] **Step 5: Lint + commit**

Run: `cargo clippy -p treant-games -- -D warnings`
Expected: no warnings.
```bash
git add treant-games/src/grid.rs
git commit -m "feat(games): GridEval + GridMcts spec; search + solver tests"
```

---

## Task 5: Reface `ConnectFourWasm` over the engine

**Files:**
- Modify: `treant-wasm/Cargo.toml`
- Modify: `treant-wasm/src/connectfour.rs`

- [ ] **Step 1: Add the dependency**

In `treant-wasm/Cargo.toml`, under `[dependencies]`, add:
```toml
treant-games = { path = "../treant-games" }
```

- [ ] **Step 2: Replace `connectfour.rs` with the facade**

Replace the **entire** contents of `treant-wasm/src/connectfour.rs` with:
```rust
use treant::tree_policy::UCTPolicy;
use treant::*;
use treant_games::grid::{GridConfig, GridEval, GridGame, GridMcts, GridMove};
use wasm_bindgen::prelude::*;

use crate::types;

const MAX_DIM: usize = 10;
const MAX_PLAYERS: usize = 4;

fn config(cols: usize, rows: usize, k: usize, num_players: usize) -> GridConfig {
    let cols = cols.clamp(3, MAX_DIM);
    let rows = rows.clamp(3, MAX_DIM);
    let k = k.clamp(3, cols.max(rows));
    let num_players = num_players.clamp(2, MAX_PLAYERS);
    GridConfig { cols, rows, k, num_players, gravity: true, center_column_bonus: true }
}

fn new_manager(cfg: GridConfig) -> MCTSManager<GridMcts> {
    MCTSManager::new(GridGame::new(cfg), GridMcts { solver: false }, GridEval, UCTPolicy::new(1.4), ())
}

#[wasm_bindgen]
pub struct ConnectFourWasm {
    manager: MCTSManager<GridMcts>,
    cfg: GridConfig,
}

impl Default for ConnectFourWasm {
    fn default() -> Self {
        let cfg = config(7, 6, 4, 2);
        Self { manager: new_manager(cfg), cfg }
    }
}

#[wasm_bindgen]
impl ConnectFourWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32, k: u32, num_players: u32) -> Self {
        let cfg = config(cols as usize, rows as usize, k as usize, num_players as usize);
        Self { manager: new_manager(cfg), cfg }
    }

    pub fn cols(&self) -> u32 {
        self.cfg.cols as u32
    }
    pub fn rows(&self) -> u32 {
        self.cfg.rows as u32
    }
    pub fn win_length(&self) -> u32 {
        self.cfg.k as u32
    }
    pub fn num_players(&self) -> u32 {
        self.cfg.num_players as u32
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        let stats = types::build_stats(&self.manager, |_| None);
        serde_wasm_bindgen::to_value(&stats).unwrap()
    }

    pub fn get_tree(&self, max_depth: u32) -> JsValue {
        let tree =
            types::export_tree::<GridMcts>(self.manager.tree().root_node(), max_depth, &|_| None);
        serde_wasm_bindgen::to_value(&tree).unwrap()
    }

    /// Board as string, top row first, left to right.
    /// ' '=empty, '1'=player 0, '2'=player 1, '3'=player 2, '4'=player 3.
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        let mut out = String::with_capacity(self.cfg.rows * self.cfg.cols);
        for row in 0..self.cfg.rows {
            for col in 0..self.cfg.cols {
                out.push(match s.cell(row, col) {
                    None => ' ',
                    Some(p) => (b'1' + p) as char,
                });
            }
        }
        out
    }

    /// Current player as 0-indexed number.
    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current() as u32
    }

    pub fn is_terminal(&self) -> bool {
        let s = self.manager.tree().root_state();
        s.winner().is_some() || s.is_full()
    }

    /// Returns winner player number (1-indexed) as string, "Draw", or "" (not over).
    pub fn result(&self) -> String {
        let state = self.manager.tree().root_state();
        if let Some(winner) = state.winner() {
            format!("{}", winner + 1)
        } else if state.is_full() {
            "Draw".into()
        } else {
            String::new()
        }
    }

    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    pub fn apply_move(&mut self, col: &str) -> bool {
        let col_num: u8 = match col.parse() {
            Ok(n) if (n as usize) < self.cfg.cols => n,
            _ => return false,
        };
        let m = GridMove(col_num);
        if self.manager.advance(&m).is_ok() {
            return true;
        }
        self.manager.playout_n(100);
        self.manager.advance(&m).is_ok()
    }

    pub fn reset(&mut self) {
        self.manager = new_manager(self.cfg);
    }
}
```

- [ ] **Step 3: Build the workspace**

Run: `cargo build -p treant-wasm`
Expected: builds. (If `current()`/`cell()`/`winner()`/`is_full()` visibility errors appear, they are `pub` per Task 2 — re-check.)

- [ ] **Step 4: Sanity test the board format (add a wasm-crate unit test)**

Create `treant-wasm/src/connectfour.rs` test module at the end of the file:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_string_shape_and_drop() {
        let mut g = ConnectFourWasm::new(7, 6, 4, 2);
        assert_eq!(g.get_board().len(), 42);
        assert!(g.get_board().chars().all(|c| c == ' '));
        assert!(g.apply_move("3"));
        // bottom row is the last 7 chars; column 3 should now be '1'
        let board = g.get_board();
        let bottom = &board[35..42];
        assert_eq!(bottom.chars().nth(3), Some('1'));
        assert_eq!(g.current_player(), 1);
        assert!(!g.apply_move("9")); // out of range
    }
}
```

Run: `cargo test -p treant-wasm connectfour`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add treant-wasm/Cargo.toml treant-wasm/src/connectfour.rs
git commit -m "refactor(wasm): ConnectFourWasm delegates to treant-games grid engine"
```

---

## Task 6: Reface `TicTacToeWasm` over the engine

**Files:**
- Modify: `treant-wasm/src/tictactoe.rs`

- [ ] **Step 1: Replace `tictactoe.rs` with the facade**

Replace the **entire** contents of `treant-wasm/src/tictactoe.rs` with:
```rust
use treant::tree_policy::UCTPolicy;
use treant::*;
use treant_games::grid::{GridConfig, GridEval, GridGame, GridMcts, GridMove};
use wasm_bindgen::prelude::*;

use crate::types;

const MAX_DIM: usize = 10;
const MAX_PLAYERS: usize = 4;
const PLAYER_SYMBOLS: [char; MAX_PLAYERS] = ['X', 'O', 'A', 'B'];

fn config(cols: usize, rows: usize, k: usize, num_players: usize) -> GridConfig {
    let cols = cols.clamp(2, MAX_DIM);
    let rows = rows.clamp(2, MAX_DIM);
    let k = k.clamp(2, cols.max(rows));
    let num_players = num_players.clamp(2, MAX_PLAYERS);
    GridConfig { cols, rows, k, num_players, gravity: false, center_column_bonus: false }
}

fn new_manager(cfg: GridConfig) -> MCTSManager<GridMcts> {
    MCTSManager::new(GridGame::new(cfg), GridMcts { solver: true }, GridEval, UCTPolicy::new(1.4), ())
}

#[wasm_bindgen]
pub struct TicTacToeWasm {
    manager: MCTSManager<GridMcts>,
    cfg: GridConfig,
}

impl Default for TicTacToeWasm {
    fn default() -> Self {
        let cfg = config(3, 3, 3, 2);
        Self { manager: new_manager(cfg), cfg }
    }
}

#[wasm_bindgen]
impl TicTacToeWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32, k: u32, num_players: u32) -> Self {
        let cfg = config(cols as usize, rows as usize, k as usize, num_players as usize);
        Self { manager: new_manager(cfg), cfg }
    }

    pub fn cols(&self) -> u32 {
        self.cfg.cols as u32
    }
    pub fn rows(&self) -> u32 {
        self.cfg.rows as u32
    }
    pub fn win_length(&self) -> u32 {
        self.cfg.k as u32
    }
    pub fn num_players(&self) -> u32 {
        self.cfg.num_players as u32
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        let stats = types::build_stats(&self.manager, |_| None);
        serde_wasm_bindgen::to_value(&stats).unwrap()
    }

    pub fn get_tree(&self, max_depth: u32) -> JsValue {
        let tree =
            types::export_tree::<GridMcts>(self.manager.tree().root_node(), max_depth, &|_| None);
        serde_wasm_bindgen::to_value(&tree).unwrap()
    }

    /// Board as string: ' '=empty, 'X'=p0, 'O'=p1, 'A'=p2, 'B'=p3. Row-major.
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        let mut out = String::with_capacity(self.cfg.rows * self.cfg.cols);
        for row in 0..self.cfg.rows {
            for col in 0..self.cfg.cols {
                out.push(match s.cell(row, col) {
                    None => ' ',
                    Some(p) => PLAYER_SYMBOLS.get(p as usize).copied().unwrap_or('?'),
                });
            }
        }
        out
    }

    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current() as u32
    }

    pub fn is_terminal(&self) -> bool {
        let s = self.manager.tree().root_state();
        s.winner().is_some() || s.is_full()
    }

    /// Returns winner as "1","2",etc., "Draw", or "" (not over).
    pub fn result(&self) -> String {
        let state = self.manager.tree().root_state();
        if let Some(w) = state.winner() {
            format!("{}", w + 1)
        } else if state.is_full() {
            "Draw".into()
        } else {
            String::new()
        }
    }

    pub fn root_proven_value(&self) -> String {
        format!("{:?}", self.manager.root_proven_value())
    }

    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    pub fn apply_move(&mut self, mov: &str) -> bool {
        let idx: u8 = match mov.parse() {
            Ok(v) if (v as usize) < self.cfg.cols * self.cfg.rows => v,
            _ => return false,
        };
        let m = GridMove(idx);
        if self.manager.advance(&m).is_ok() {
            return true;
        }
        self.manager.playout_n(100);
        self.manager.advance(&m).is_ok()
    }

    pub fn reset(&mut self) {
        self.manager = new_manager(self.cfg);
    }
}
```

- [ ] **Step 2: Add a board-format sanity test**

Append to `treant-wasm/src/tictactoe.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_string_is_row_major_symbols() {
        let mut g = TicTacToeWasm::new(3, 3, 3, 2);
        assert_eq!(g.get_board(), "         "); // 9 spaces
        assert!(g.apply_move("0")); // top-left, player 0 = 'X'
        assert_eq!(g.get_board().chars().next(), Some('X'));
        assert_eq!(g.current_player(), 1);
        assert!(g.apply_move("4")); // center, player 1 = 'O'
        assert_eq!(g.get_board().chars().nth(4), Some('O'));
        assert!(!g.apply_move("9")); // out of range
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p treant-wasm tictactoe`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add treant-wasm/src/tictactoe.rs
git commit -m "refactor(wasm): TicTacToeWasm delegates to treant-games grid engine"
```

---

## Task 7: Full verification

**Files:** none (verification only)

- [ ] **Step 1: Whole-workspace tests**

Run: `cargo test`
Expected: all pass (existing 111 core + treant-dynamic + new treant-games + wasm unit tests).

- [ ] **Step 2: Clippy clean**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: 0 warnings. (Convention: clippy must stay at 0.)

- [ ] **Step 3: WASM build**

Run: `cd treant-wasm && wasm-pack build --target web && cd ..`
Expected: builds successfully.

- [ ] **Step 4: Manual demo regression**

Run: `cd docs && npm start`
Check in browser: Playground → Connect Four and Tic-Tac-Toe. Verify board renders, human moves register, AI responds, win/draw detection and result text are correct, board-size knobs still work. Stop the dev server.

- [ ] **Step 5: Final commit (if any cleanup was needed)**

```bash
git add -A
git commit -m "test(games): workspace green after grid-engine extraction" || echo "nothing to commit"
```

---

## Self-Review Notes

- **Spec coverage:** new crate (Task 1), shared engine incl. gravity/placement/center-bonus/solver (Tasks 2–4), facade preservation of board strings + move parse + result + clamps (Tasks 5–6), testing + clippy + wasm build + manual regression (Task 7). All spec sections mapped.
- **Preservation contract:** board-string formats reproduced explicitly in `get_board` (CF digits/top-first; TTT symbols/row-major); move parse ranges preserved; `result()` 1-indexed; CF clamps min 3, TTT min 2; CF `center_column_bonus: true` + `solver: false`, TTT `center_column_bonus: false` + `solver: true`.
- **Type consistency:** `GridGame`, `GridConfig`, `GridMcts`, `GridEval`, `GridMove`, `GridStateEval` used consistently; facade fields reduced to `manager` + `cfg`; engine accessors `cell`, `current`, `winner`, `is_full`, `config` are `pub`.
