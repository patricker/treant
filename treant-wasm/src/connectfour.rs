use treant::tree_policy::UCTPolicy;
use treant::*;
use treant_games::grid::{GridConfig, GridEval, GridGame, GridMcts, GridMove, POP_OFFSET};
use wasm_bindgen::prelude::*;

use crate::types;

const MAX_DIM: usize = 12;
const MAX_PLAYERS: usize = 6;

/// `variant`: 0 = classic, 1 = Pop Out (pop your own bottom disc), 2 = Cylinder
/// (the board wraps into a tube). Anything else is treated as classic.
fn config(cols: usize, rows: usize, k: usize, num_players: usize, variant: u32) -> GridConfig {
    let cols = cols.clamp(3, MAX_DIM);
    let rows = rows.clamp(3, MAX_DIM);
    let k = k.clamp(2, cols.max(rows));
    let num_players = num_players.clamp(2, MAX_PLAYERS);
    GridConfig {
        cols,
        rows,
        k,
        num_players,
        gravity: true,
        center_column_bonus: true,
        wrap_cols: variant == 2,
        allow_pop: variant == 1,
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
        let cfg = config(7, 6, 4, 2, 0);
        Self {
            manager: new_manager(cfg),
            cfg,
        }
    }
}

#[wasm_bindgen]
impl ConnectFourWasm {
    /// `variant`: 0 = classic, 1 = Pop Out, 2 = Cylinder.
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32, k: u32, num_players: u32, variant: u32) -> Self {
        let cfg = config(cols as usize, rows as usize, k as usize, num_players as usize, variant);
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

    /// Re-encode a raw `GridMove` string into the UI move vocabulary. Drops stay
    /// bare column indices; a pop (value ≥ `POP_OFFSET`) becomes `"pop<col>"`.
    /// (Connect Four columns are ≤ 12, so there is no ambiguity here — unlike
    /// TicTacToe's placement indices, which is why `GridMove`'s own `Display`
    /// stays pop-agnostic.)
    fn encode_move(s: String) -> String {
        match s.parse::<u16>() {
            Ok(v) if v >= POP_OFFSET as u16 => format!("pop{}", v - POP_OFFSET as u16),
            _ => s,
        }
    }

    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| Self::encode_move(format!("{m}")))
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
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed).map(Self::encode_move)
    }

    /// The current player's legal moves, comma-joined. Drops are the bare column
    /// index (`"3"`); pops (Pop Out variant only) are `"pop<col>"`. The UI reads
    /// this to enable per-column drop arrows and the pop-out affordance.
    pub fn legal_moves(&self) -> String {
        let state = self.manager.tree().root_state();
        state
            .available_moves()
            .iter()
            .map(|m| Self::encode_move(format!("{m}")))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Apply a drop (`"3"`) or a pop (`"pop3"`, Pop Out only). Validates against
    /// the live legal moves and rebuilds the manager from the new root state
    /// (a fresh tree), mirroring TicTacToe/gridpack — `advance` only succeeds for
    /// an already-expanded child, which pops and low columns often are not.
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m = if let Some(rest) = mov.strip_prefix("pop") {
            match rest.parse::<u8>() {
                Ok(c) if (c as usize) < self.cfg.cols => GridMove(c + POP_OFFSET),
                _ => return false,
            }
        } else {
            match mov.parse::<u8>() {
                Ok(c) if (c as usize) < self.cfg.cols => GridMove(c),
                _ => return false,
            }
        };
        let mut state = self.manager.tree().root_state().clone();
        if !state.available_moves().contains(&m) {
            return false;
        }
        state.make_move(&m);
        self.manager = MCTSManager::new(state, GridMcts { solver: false }, GridEval, UCTPolicy::new(1.4), ());
        true
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
        let mut g = ConnectFourWasm::new(7, 6, 4, 2, 0);
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
        let mut g = ConnectFourWasm::new(7, 6, 4, 2, 0);
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

    // ---- Task 2.7: the Connect Four family ----

    #[test]
    fn k2_is_accepted() {
        // k=2: two of P0's discs in a line win immediately. Drive real drop
        // physics — P0 stacks column 0 twice (P1 answers harmlessly in column 1),
        // giving P0 a vertical 2-in-a-row.
        let mut g = ConnectFourWasm::new(7, 6, 2, 2, 0);
        assert_eq!(g.win_length(), 2, "k=2 must not be clamped up to 3");
        assert!(g.apply_move("0")); // P0 → (row5,col0)
        assert!(g.apply_move("1")); // P1 → (row5,col1)
        assert!(!g.is_terminal(), "one disc each, no line yet");
        assert!(g.apply_move("0")); // P0 → (row4,col0): vertical two
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1");
    }

    #[test]
    fn cylinder_wraps_the_win_check() {
        // Variant 2, 12-wide tube, k=4. P0 lays a bottom-row line in cols
        // 10,11,0,1 — contiguous only because the seam wraps. P1 stacks column 5
        // harmlessly (never reaches 4). P0's fourth disc wins across the seam.
        let mut g = ConnectFourWasm::new(12, 6, 4, 2, 2);
        assert!(g.apply_move("10")); // P0
        assert!(g.apply_move("5")); // P1
        assert!(g.apply_move("11")); // P0
        assert!(g.apply_move("5")); // P1
        assert!(g.apply_move("0")); // P0
        assert!(g.apply_move("5")); // P1 (3 stacked, k=4 → no win)
        assert!(!g.is_terminal(), "P0 has only 3 in the wrapping run so far");
        assert!(g.apply_move("1")); // P0 completes 10-11-0-1 across the seam
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1");
        // The wrapped winning cells are highlighted (bottom row = indices 60..71).
        let cells = g.winning_cells();
        for c in [10, 11, 0, 1] {
            assert!(cells.split(',').any(|s| s == (60 + c).to_string()), "col {c} highlighted");
        }
    }

    #[test]
    fn pop_that_completes_opponents_line_loses() {
        // Variant 1 (Pop Out), 7×6, k=4. We build a position where popping P0's
        // own bottom disc in column 3 drops nothing of P0's own but lets the
        // former row-4 P1 disc fall to the bottom row, completing P1's horizontal
        // four (cols 2,3,4,5). Since the pop hands P1 the win, result is "2".
        //
        // Pre-pop bottom row (row5): col0=P0, col2=P1, col3=P0, col4=P1, col5=P1;
        // col3 row4 = P1. P0 also stacks column 0 to keep the turn count even so
        // it is P0 to move (4 discs each). Drops alternate P0/P1:
        let mut g = ConnectFourWasm::new(7, 6, 4, 2, 1);
        assert!(g.apply_move("3")); // P0 → col3 r5
        assert!(g.apply_move("2")); // P1 → col2 r5
        assert!(g.apply_move("0")); // P0 → col0 r5
        assert!(g.apply_move("4")); // P1 → col4 r5
        assert!(g.apply_move("0")); // P0 → col0 r4
        assert!(g.apply_move("5")); // P1 → col5 r5
        assert!(g.apply_move("0")); // P0 → col0 r3 (vertical 3, k=4 → no win)
        assert!(g.apply_move("3")); // P1 → col3 r4
        assert!(!g.is_terminal(), "no line before the pop");
        assert_eq!(g.current_player(), 0, "P0 to move — its disc is col3's bottom");
        assert!(g.apply_move("pop3")); // P0 pops col3; P1's row-4 disc falls to row5
        assert!(g.is_terminal());
        assert_eq!(g.result(), "2", "the pop completed P1's line, so P1 wins");
    }

    #[test]
    fn ai_encodes_a_pop_move_as_a_pop_token() {
        // Regression for the raw-sentinel leak: `best_move`/`weak_move` must route
        // the AI's chosen move through `encode_move`, so a pop surfaces as
        // "pop<col>" — never the bare wire value ("131") the search speaks.
        //
        // Robust shape: fill a 4×3 Pop Out board (k=3) completely so the ONLY
        // legal moves are pops (no column has room to drop), with P0 to move and
        // P0 owning ≥1 bottom disc. The column order [0,0,0,1,1,1,3,2,2,3,3,2] is
        // a witnessed 12-drop alternating fill with no intermediate win (found by
        // DFS); P0 ends owning the bottom of cols 0 and 3, so best_move can only
        // return a pop.
        let mut g = ConnectFourWasm::new(4, 3, 3, 2, 1);
        for c in ["0", "0", "0", "1", "1", "1", "3", "2", "2", "3", "3", "2"] {
            assert!(g.apply_move(c), "drop {c} must be legal");
        }
        assert_eq!(g.current_player(), 0, "P0 to move after 12 drops");
        // Only pops remain: every column is full, so no bare-column drop is legal.
        let legal = g.legal_moves();
        assert!(
            legal.split(',').all(|m| m.starts_with("pop")),
            "only pops should be legal, got {legal:?}"
        );

        // AI-encode side must round-trip: best_move yields a "pop…" token that
        // apply_move accepts on this same position (the bug returned "131").
        let mut g2 = ConnectFourWasm::new(4, 3, 3, 2, 1);
        for c in ["0", "0", "0", "1", "1", "1", "3", "2", "2", "3", "3", "2"] {
            assert!(g2.apply_move(c));
        }
        g2.playout_n(300);
        let best = g2.best_move().expect("a move on a pops-only position");
        assert!(best.starts_with("pop"), "best_move must encode a pop, got {best:?}");
        assert!(g2.apply_move(&best), "the encoded best_move must be apply-able: {best:?}");

        // weak_move takes the same encode path; its token must also apply.
        let mut g3 = ConnectFourWasm::new(4, 3, 3, 2, 1);
        for c in ["0", "0", "0", "1", "1", "1", "3", "2", "2", "3", "3", "2"] {
            assert!(g3.apply_move(c));
        }
        let weak = g3.weak_move(50, 3, 1.0, 42).expect("a weak move on a pops-only position");
        assert!(weak.starts_with("pop"), "weak_move must encode a pop, got {weak:?}");
        assert!(g3.apply_move(&weak), "the encoded weak_move must be apply-able: {weak:?}");
    }

    #[test]
    fn legal_moves_lists_pops_only_in_pop_out() {
        // Classic: never a pop token.
        let mut classic = ConnectFourWasm::new(7, 6, 4, 2, 0);
        assert!(classic.apply_move("0"));
        assert!(!classic.legal_moves().contains("pop"));
        // Pop Out: after P0 owns a bottom disc it becomes poppable on P0's turn.
        let mut popg = ConnectFourWasm::new(7, 6, 4, 2, 1);
        assert!(popg.apply_move("0")); // P0's disc at col0 bottom
        assert!(!popg.legal_moves().contains("pop"), "P1 to move — col0 is not P1's");
        assert!(popg.apply_move("1")); // P1 elsewhere → back to P0
        assert!(popg.legal_moves().split(',').any(|m| m == "pop0"));
    }
}
