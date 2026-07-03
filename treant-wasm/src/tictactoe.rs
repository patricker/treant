use treant::tree_policy::UCTPolicy;
use treant::*;
use treant_games::grid::{GridConfig, GridEval, GridGame, GridMcts, GridMove};
use wasm_bindgen::prelude::*;

use crate::types;

// Must cover the largest board any UI offers on this engine (Gomoku goes to
// 15×15). The engine itself is Vec-backed with no dimension limit; the only hard
// cap is the u8 move index, so cols*rows must stay ≤ 256 (15×15 = 225 ✓).
// Keeping this below the UI's max silently clamps the engine to a smaller board
// than React renders, which desyncs coordinates (wrong win detection, dead cells).
const MAX_DIM: usize = 15;
const MAX_PLAYERS: usize = 6;
const PLAYER_SYMBOLS: [char; MAX_PLAYERS] = ['X', 'O', 'A', 'B', 'C', 'D'];

fn config(cols: usize, rows: usize, k: usize, num_players: usize) -> GridConfig {
    let cols = cols.clamp(2, MAX_DIM);
    let rows = rows.clamp(2, MAX_DIM);
    let k = k.clamp(2, cols.max(rows));
    let num_players = num_players.clamp(2, MAX_PLAYERS);
    GridConfig {
        cols,
        rows,
        k,
        num_players,
        gravity: false,
        center_column_bonus: false,
    }
}

fn new_manager(cfg: GridConfig) -> MCTSManager<GridMcts> {
    MCTSManager::new(
        GridGame::new(cfg),
        GridMcts { solver: true },
        GridEval,
        UCTPolicy::new(1.4),
        (),
    )
}

#[wasm_bindgen]
pub struct TicTacToeWasm {
    manager: MCTSManager<GridMcts>,
    cfg: GridConfig,
}

impl Default for TicTacToeWasm {
    fn default() -> Self {
        let cfg = config(3, 3, 3, 2);
        Self {
            manager: new_manager(cfg),
            cfg,
        }
    }
}

#[wasm_bindgen]
impl TicTacToeWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32, k: u32, num_players: u32) -> Self {
        let cfg = config(cols as usize, rows as usize, k as usize, num_players as usize);
        Self {
            manager: new_manager(cfg),
            cfg,
        }
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
        serde_wasm_bindgen::to_value(&stats).unwrap_or(JsValue::NULL)
    }

    pub fn get_tree(&self, max_depth: u32) -> JsValue {
        let tree =
            types::export_tree::<GridMcts>(self.manager.tree().root_node(), max_depth, &|_| None);
        serde_wasm_bindgen::to_value(&tree).unwrap_or(JsValue::NULL)
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

    /// Comma-joined 0-based cell indices of the winning k-in-a-row, or "" when
    /// there is no line (in progress, or a draw). The UI glows these cells.
    pub fn winning_cells(&self) -> String {
        match self.manager.tree().root_state().winning_line() {
            Some(cells) => cells.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","),
            None => String::new(),
        }
    }

    /// Difficulty-aware move: see `crate::difficulty::pick_weak`.
    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }

    pub fn apply_move(&mut self, mov: &str) -> bool {
        let idx: u8 = match mov.parse() {
            Ok(v) if (v as usize) < self.cfg.cols * self.cfg.rows => v,
            _ => return false,
        };
        let m = GridMove(idx);
        // Apply against the actual game state and rebuild the manager, rather than
        // relying on MCTSManager::advance. advance only succeeds for a move the
        // search has already expanded as an owned root child; on large boards
        // (Gomoku is up to 15×15 = 225 moves) most legal moves are never expanded
        // within a turn's playouts, so a human tapping an "unsearched" cell — very
        // common in the lower rows — was silently rejected. Cloning the state and
        // rebuilding always works, and matches how the gridpack macro games apply
        // moves.
        let mut state = self.manager.tree().root_state().clone();
        if !state.available_moves().contains(&m) {
            return false;
        }
        state.make_move(&m);
        self.manager = MCTSManager::new(state, GridMcts { solver: true }, GridEval, UCTPolicy::new(1.4), ());
        true
    }

    pub fn reset(&mut self) {
        self.manager = new_manager(self.cfg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_cells(g: &TicTacToeWasm) -> Vec<String> {
        g.get_board()
            .as_bytes()
            .iter()
            .enumerate()
            .filter(|&(_, &c)| c == b' ')
            .map(|(i, _)| i.to_string())
            .collect()
    }

    #[test]
    fn weak_move_temp0_equals_best_and_protects_a_win() {
        // TicTacToe enables the solver (new_manager builds GridMcts { solver: true }),
        // so this is a genuine proven-win-protection test, not just visit
        // concentration: X to move with X at {0,1} and O at {3,4} is a forced win
        // (play 2 to complete the top row). The search proves it, so pick_weak's
        // proven-win guard returns the winning move even at high temp / wide top-K.
        let mut g = TicTacToeWasm::new(3, 3, 3, 2);
        assert!(g.apply_move("0")); // X
        assert!(g.apply_move("3")); // O
        assert!(g.apply_move("1")); // X
        assert!(g.apply_move("4")); // O
        let strong = g.weak_move(2000, 1, 0.0, 1);
        assert_eq!(strong.as_deref(), g.best_move().as_deref());
        let weak = g.weak_move(2000, 9, 3.0, 1);
        assert_eq!(weak.as_deref(), Some("2"));
        // The guard's precondition: the search actually proved the win. Without
        // this (e.g. on a solver-less game) high temp could scatter off the win.
        assert_eq!(g.root_proven_value(), "Win");
    }

    #[test]
    fn weak_move_returns_a_legal_move() {
        let mut g = TicTacToeWasm::new(3, 3, 3, 2);
        let mv = g.weak_move(50, 5, 1.5, 7).unwrap();
        assert!(empty_cells(&g).contains(&mv));
    }

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

    // Regression: Gomoku renders up to 15×15 in the UI. The engine must not clamp
    // below that, or React and the engine disagree on board dimensions — which
    // makes the lower rows unplaceable (apply_move rejects indices past the
    // clamped size) and desyncs win detection (stride mismatch).
    #[test]
    fn large_board_is_not_clamped() {
        let g = TicTacToeWasm::new(15, 15, 5, 2);
        assert_eq!(g.cols(), 15);
        assert_eq!(g.rows(), 15);
        assert_eq!(g.get_board().chars().count(), 225);
    }

    #[test]
    fn large_board_lower_section_is_placeable() {
        let mut g = TicTacToeWasm::new(15, 15, 5, 2);
        // Bottom-right cell (row 14, col 14) — index 224. Under the old MAX_DIM=10
        // clamp the engine was 10×10=100 cells and this move was silently refused.
        assert!(g.apply_move("224"));
        assert_eq!(g.get_board().chars().nth(224), Some('X'));
    }

    #[test]
    fn winning_cells_reports_the_line_and_is_empty_otherwise() {
        let mut g = TicTacToeWasm::new(3, 3, 3, 2);
        assert_eq!(g.winning_cells(), "", "empty board has no line");
        // X: 0,1,2 (top row); O: 3,4 harmlessly. X completes the row.
        assert!(g.apply_move("0")); // X
        assert!(g.apply_move("3")); // O
        assert!(g.apply_move("1")); // X
        assert_eq!(g.winning_cells(), "", "mid-game, still no line");
        assert!(g.apply_move("4")); // O
        assert!(g.apply_move("2")); // X completes 0,1,2
        assert!(g.is_terminal());
        assert_eq!(g.winning_cells(), "0,1,2");
    }

    #[test]
    fn winning_cells_is_empty_on_a_draw() {
        // A full 3×3 with no line: X O X / X O O / O X X.
        let mut g = TicTacToeWasm::new(3, 3, 3, 2);
        for m in ["0", "1", "2", "4", "3", "5", "7", "6", "8"] {
            assert!(g.apply_move(m), "move {m}");
        }
        assert!(g.is_terminal());
        assert_eq!(g.result(), "Draw");
        assert_eq!(g.winning_cells(), "");
    }

    #[test]
    fn five_in_a_row_wins_on_a_gomoku_board() {
        let mut g = TicTacToeWasm::new(15, 15, 5, 2);
        // X builds 0,1,2,3,4 across the top row; O plays harmlessly on row 1.
        for (x, o) in [("0", "15"), ("1", "16"), ("2", "17"), ("3", "18")] {
            assert!(g.apply_move(x));
            assert!(g.apply_move(o));
        }
        assert!(!g.is_terminal()); // only 4 in a row so far
        assert!(g.apply_move("4")); // 5th in a row
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1"); // player 0 (1-indexed) wins
    }
}
