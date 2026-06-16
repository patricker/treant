use treant::tree_policy::UCTPolicy;
use treant::*;
use treant_games::grid::{GridConfig, GridEval, GridGame, GridMcts, GridMove};
use wasm_bindgen::prelude::*;

use crate::types;

const MAX_DIM: usize = 10;
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
