use treant::tree_policy::UCTPolicy;
use treant::*;
use treant_games::grid::{GridConfig, GridEval, GridGame, GridMcts, GridMove};
use wasm_bindgen::prelude::*;

use crate::types;

const MAX_DIM: usize = 10;
const MAX_PLAYERS: usize = 6;

fn config(cols: usize, rows: usize, k: usize, num_players: usize) -> GridConfig {
    let cols = cols.clamp(3, MAX_DIM);
    let rows = rows.clamp(3, MAX_DIM);
    let k = k.clamp(3, cols.max(rows));
    let num_players = num_players.clamp(2, MAX_PLAYERS);
    GridConfig {
        cols,
        rows,
        k,
        num_players,
        gravity: true,
        center_column_bonus: true,
    }
}

fn new_manager(cfg: GridConfig) -> MCTSManager<GridMcts> {
    MCTSManager::new(
        GridGame::new(cfg),
        GridMcts { solver: false },
        GridEval,
        UCTPolicy::new(1.4),
        (),
    )
}

#[wasm_bindgen]
pub struct ConnectFourWasm {
    manager: MCTSManager<GridMcts>,
    cfg: GridConfig,
}

impl Default for ConnectFourWasm {
    fn default() -> Self {
        let cfg = config(7, 6, 4, 2);
        Self {
            manager: new_manager(cfg),
            cfg,
        }
    }
}

#[wasm_bindgen]
impl ConnectFourWasm {
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

    /// Comma-joined 0-based cell indices of the winning line, or "" when there is
    /// no line (in progress, or a draw). Cell index is `row*cols+col`, top row 0.
    pub fn winning_cells(&self) -> String {
        match self.manager.tree().root_state().winning_line() {
            Some(cells) => cells.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","),
            None => String::new(),
        }
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
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

    #[test]
    fn winning_cells_reports_a_vertical_line() {
        // Player 0 stacks four discs in column 0; player 1 fills column 1.
        let mut g = ConnectFourWasm::new(7, 6, 4, 2);
        assert_eq!(g.winning_cells(), "");
        for (x, o) in [("0", "1"), ("0", "1"), ("0", "1")] {
            assert!(g.apply_move(x));
            assert!(g.apply_move(o));
        }
        assert_eq!(g.winning_cells(), "", "only three stacked so far");
        assert!(g.apply_move("0")); // fourth X, top at row 2 (index 14)
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1");
        // Column 0, rows 2..5 → indices 14, 21, 28, 35 (top-down).
        assert_eq!(g.winning_cells(), "14,21,28,35");
    }
}
