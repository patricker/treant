//! Clobber. The board starts as a full checkerboard. Move one of your stones
//! onto an orthogonally-adjacent enemy stone, removing it. The player who cannot
//! move loses. Finite and never drawn — a clean exact-solver target.
use crate::gridlib::idx;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CbMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for CbMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

#[derive(Clone)]
struct Clobber {
    board: Vec<i8>,
    cols: usize,
    rows: usize,
    current: u8,
}
impl Clobber {
    fn new(cols: usize, rows: usize) -> Self {
        let mut board = vec![-1i8; cols * rows];
        for r in 0..rows {
            for c in 0..cols {
                board[idx(r, c, cols)] = ((r + c) % 2) as i8;
            }
        }
        Self { board, cols, rows, current: 0 }
    }
    fn orth(&self, r: usize, c: usize) -> Vec<usize> {
        let mut v = Vec::new();
        if r > 0 {
            v.push(idx(r - 1, c, self.cols));
        }
        if r + 1 < self.rows {
            v.push(idx(r + 1, c, self.cols));
        }
        if c > 0 {
            v.push(idx(r, c - 1, self.cols));
        }
        if c + 1 < self.cols {
            v.push(idx(r, c + 1, self.cols));
        }
        v
    }
    fn gen(&self) -> Vec<CbMove> {
        let mut v = Vec::new();
        let enemy = 1 - self.current as i8;
        for r in 0..self.rows {
            for c in 0..self.cols {
                let from = idx(r, c, self.cols);
                if self.board[from] != self.current as i8 {
                    continue;
                }
                for to in self.orth(r, c) {
                    if self.board[to] == enemy {
                        v.push(CbMove { from: from as u16, to: to as u16 });
                    }
                }
            }
        }
        v
    }
    fn moves_for(&self, p: i8) -> i64 {
        let mut m = 0i64;
        let enemy = 1 - p;
        for r in 0..self.rows {
            for c in 0..self.cols {
                if self.board[idx(r, c, self.cols)] != p {
                    continue;
                }
                for to in self.orth(r, c) {
                    if self.board[to] == enemy {
                        m += 1;
                    }
                }
            }
        }
        m
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.gen().is_empty() {
            Some(ProvenValue::Loss) // current can't move -> loses
        } else {
            None
        }
    }
}
impl GameState for Clobber {
    type Move = CbMove;
    type Player = u8;
    type MoveList = Vec<CbMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<CbMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &CbMove) {
        self.board[m.to as usize] = self.current as i8;
        self.board[m.from as usize] = -1;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct CbEval;
impl Evaluator<CbCfg> for CbEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &Clobber, m: &Vec<CbMove>, _: Option<SearchHandle<CbCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], s.moves_for(0) - s.moves_for(1))
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &Clobber, _: &i64, _: SearchHandle<CbCfg>) -> i64 {
        s.moves_for(0) - s.moves_for(1)
    }
}
#[derive(Default)]
struct CbCfg;
impl MCTS for CbCfg {
    type State = Clobber;
    type Eval = CbEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct ClobberWasm {
    manager: MCTSManager<CbCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl ClobberWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32) -> Self {
        let cols = (cols as usize).clamp(3, 8);
        let rows = (rows as usize).clamp(3, 8);
        Self { manager: MCTSManager::new(Clobber::new(cols, rows), CbCfg, CbEval, UCTPolicy::new(1.4), ()), cols, rows }
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
            Some(ProvenValue::Loss) => format!("{}", (1 - s.current) + 1), // current lost
            _ => String::new(),
        }
    }
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
        let m = CbMove { from, to };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, CbCfg, CbEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Clobber::new(self.cols, self.rows), CbCfg, CbEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkerboard_start_has_moves() {
        let g = ClobberWasm::new(5, 5);
        assert!(g.legal_moves().split(',').count() > 5);
        assert!(!g.is_terminal());
    }

    #[test]
    fn no_moves_loses() {
        // a board with one X and one O not adjacent: current (X) may have a move;
        // contrive an X fully isolated -> no move -> loss.
        let mut g = Clobber::new(3, 3);
        g.board = vec![-1; 9];
        g.board[idx(0, 0, 3)] = 0; // lone X, no adjacent O
        g.current = 0;
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn capture_removes_enemy_and_vacates() {
        let mut g = Clobber::new(3, 3);
        // X at (0,0), O at (0,1)
        g.board = vec![-1; 9];
        g.board[idx(0, 0, 3)] = 0;
        g.board[idx(0, 1, 3)] = 1;
        g.current = 0;
        g.make_move(&CbMove { from: idx(0, 0, 3) as u16, to: idx(0, 1, 3) as u16 });
        assert_eq!(g.board[idx(0, 0, 3)], -1);
        assert_eq!(g.board[idx(0, 1, 3)], 0);
        assert_eq!(g.current, 1);
    }
}
