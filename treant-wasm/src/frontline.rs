//! Frontline (a.k.a. Breakthrough). Pawns move one square forward (straight or
//! diagonal) and capture only diagonally. Reach the far row — or wipe out the
//! enemy — to win. No draws, so the solver can prove many mid-game positions.
use crate::gridlib::idx;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BtMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for BtMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

#[derive(Clone)]
struct Frontline {
    board: Vec<i8>, // -1 empty, 0 = X (moves up), 1 = O (moves down)
    cols: usize,
    rows: usize,
    current: u8,
}
impl Frontline {
    fn new(cols: usize, rows: usize) -> Self {
        let mut board = vec![-1i8; cols * rows];
        for c in 0..cols {
            board[idx(0, c, cols)] = 1;
            board[idx(1, c, cols)] = 1;
            board[idx(rows - 2, c, cols)] = 0;
            board[idx(rows - 1, c, cols)] = 0;
        }
        Self { board, cols, rows, current: 0 }
    }
    fn count(&self, p: i8) -> usize {
        self.board.iter().filter(|&&v| v == p).count()
    }
    fn gen(&self) -> Vec<BtMove> {
        let mut v = Vec::new();
        let fwd: i32 = if self.current == 0 { -1 } else { 1 };
        for r in 0..self.rows {
            for c in 0..self.cols {
                if self.board[idx(r, c, self.cols)] != self.current as i8 {
                    continue;
                }
                let nr = r as i32 + fwd;
                if nr < 0 || nr >= self.rows as i32 {
                    continue;
                }
                let nr = nr as usize;
                // straight forward — only into an empty cell
                if self.board[idx(nr, c, self.cols)] < 0 {
                    v.push(BtMove { from: idx(r, c, self.cols) as u16, to: idx(nr, c, self.cols) as u16 });
                }
                // diagonal forward — empty or capture (not onto own)
                for dc in [-1i32, 1] {
                    let nc = c as i32 + dc;
                    if nc < 0 || nc >= self.cols as i32 {
                        continue;
                    }
                    let nc = nc as usize;
                    if self.board[idx(nr, nc, self.cols)] != self.current as i8 {
                        v.push(BtMove { from: idx(r, c, self.cols) as u16, to: idx(nr, nc, self.cols) as u16 });
                    }
                }
            }
        }
        v
    }
    fn term(&self) -> Option<ProvenValue> {
        let p0_home = (0..self.cols).any(|c| self.board[idx(0, c, self.cols)] == 0);
        let p1_home = (0..self.cols).any(|c| self.board[idx(self.rows - 1, c, self.cols)] == 1);
        if p0_home || self.count(1) == 0 {
            return Some(if self.current == 0 { ProvenValue::Win } else { ProvenValue::Loss });
        }
        if p1_home || self.count(0) == 0 {
            return Some(if self.current == 1 { ProvenValue::Win } else { ProvenValue::Loss });
        }
        if self.gen().is_empty() {
            return Some(ProvenValue::Loss); // stuck = current loses
        }
        None
    }
    /// The single winning cell to glow: the pawn that reached the enemy home row.
    /// `None` for a win by elimination (no pawn broke through) or mid-game — those
    /// have no meaningful cell to highlight.
    fn winning_cell(&self) -> Option<usize> {
        self.term()?; // no cell to glow mid-game
        if let Some(c) = (0..self.cols).find(|&c| self.board[idx(0, c, self.cols)] == 0) {
            return Some(idx(0, c, self.cols)); // player 0 broke through to the top
        }
        if let Some(c) = (0..self.cols).find(|&c| self.board[idx(self.rows - 1, c, self.cols)] == 1) {
            return Some(idx(self.rows - 1, c, self.cols)); // player 1 to the bottom
        }
        None
    }
    fn eval0(&self) -> i64 {
        let mut s = 0i64;
        for r in 0..self.rows {
            for c in 0..self.cols {
                match self.board[idx(r, c, self.cols)] {
                    0 => s += 10 + (self.rows as i64 - 1 - r as i64),
                    1 => s -= 10 + r as i64,
                    _ => {}
                }
            }
        }
        s
    }
}
impl GameState for Frontline {
    type Move = BtMove;
    type Player = u8;
    type MoveList = Vec<BtMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<BtMove> {
        if self.term().is_some() {
            return vec![];
        }
        self.gen()
    }
    fn make_move(&mut self, m: &BtMove) {
        self.board[m.to as usize] = self.current as i8;
        self.board[m.from as usize] = -1;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct FlEval;
impl Evaluator<FlCfg> for FlEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &Frontline, m: &Vec<BtMove>, _: Option<SearchHandle<FlCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], s.eval0())
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &Frontline, _: &i64, _: SearchHandle<FlCfg>) -> i64 {
        s.eval0()
    }
}
#[derive(Default)]
struct FlCfg;
impl MCTS for FlCfg {
    type State = Frontline;
    type Eval = FlEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct FrontlineWasm {
    manager: MCTSManager<FlCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl FrontlineWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32) -> Self {
        let cols = (cols as usize).clamp(4, 10);
        let rows = (rows as usize).clamp(4, 10);
        Self { manager: MCTSManager::new(Frontline::new(cols, rows), FlCfg, FlEval, UCTPolicy::new(1.4), ()), cols, rows }
    }
    pub fn cols(&self) -> u32 {
        self.cols as u32
    }
    pub fn rows(&self) -> u32 {
        self.rows as u32
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .board
            .iter()
            .map(|&v| match v {
                0 => 'X',
                1 => 'O',
                _ => ' ',
            })
            .collect()
    }
    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current as u32
    }
    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().term().is_some()
    }
    pub fn result(&self) -> String {
        let s = self.manager.tree().root_state();
        match s.term() {
            Some(ProvenValue::Win) => format!("{}", s.current + 1),
            Some(ProvenValue::Loss) => format!("{}", (1 - s.current) + 1),
            _ => String::new(),
        }
    }
    /// Comma-separated "from-to" legal moves for the current player.
    pub fn legal_moves(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .available_moves()
            .iter()
            .map(|m| format!("{m}"))
            .collect::<Vec<_>>()
            .join(",")
    }
    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    /// 0-based index of the breakthrough cell to glow, or "" for a win by
    /// elimination / mid-game. The UI glows this cell.
    pub fn winning_cells(&self) -> String {
        match self.manager.tree().root_state().winning_cell() {
            Some(i) => i.to_string(),
            None => String::new(),
        }
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let parts: Vec<&str> = mov.split('-').collect();
        if parts.len() != 2 {
            return false;
        }
        let (from, to): (u16, u16) = match (parts[0].parse(), parts[1].parse()) {
            (Ok(a), Ok(b)) => (a, b),
            _ => return false,
        };
        let m = BtMove { from, to };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, FlCfg, FlEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Frontline::new(self.cols, self.rows), FlCfg, FlEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_has_forward_and_diagonal_moves() {
        let g = FrontlineWasm::new(6, 6);
        let moves = g.legal_moves();
        assert!(!moves.is_empty());
        // 6 pawns on the front rank, each with up to 3 forward moves into empty rank
        assert!(moves.split(',').count() >= 6);
    }

    #[test]
    fn reaching_far_row_wins() {
        // hand-place an X one step from row 0 and let it walk in.
        let mut g = Frontline::new(6, 6);
        g.board = vec![-1; 36];
        g.board[idx(1, 2, 6)] = 0; // X just below the top row
        g.board[idx(5, 0, 6)] = 1; // a lone O so neither side is wiped
        g.current = 0;
        g.make_move(&BtMove { from: idx(1, 2, 6) as u16, to: idx(0, 2, 6) as u16 });
        assert_eq!(g.term(), Some(ProvenValue::Loss)); // current is now O, who lost
        // The breakthrough cell (row 0, col 2) is the one to glow.
        assert_eq!(g.winning_cell(), Some(idx(0, 2, 6)));
    }

    #[test]
    fn winning_cell_none_mid_game_and_on_elimination() {
        let g = Frontline::new(6, 6);
        assert_eq!(g.winning_cell(), None, "opening has no winner");
        // Elimination: player 0 has a lone pawn (not on the far row), player 1 has
        // none — player 0 wins by wipeout, but no pawn broke through, so no glow.
        let mut e = Frontline::new(6, 6);
        e.board = vec![-1; 36];
        e.board[idx(3, 3, 6)] = 0;
        e.current = 1; // player 1 to move but has no pawns -> stuck/eliminated
        assert!(e.term().is_some());
        assert_eq!(e.winning_cell(), None);
    }

    #[test]
    fn ai_plays() {
        let mut g = FrontlineWasm::new(6, 6);
        g.playout_n(300);
        assert!(g.best_move().is_some());
        let m = g.best_move().unwrap();
        assert!(g.apply_move(&m));
    }
}
