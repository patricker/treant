//! First Capture (capture-Go). Place stones like Go; a group with no liberties
//! is captured. The first player to capture ANY enemy group wins. Suicide is
//! illegal. Introduces groups & liberties — a genuinely Go-flavored mechanic.
use crate::gridlib::idx;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::{gridlib, types};

#[derive(Clone)]
struct CaptureGo {
    board: Vec<i8>,
    cols: usize,
    rows: usize,
    current: u8,
    last_capture: Option<u8>,
}
impl CaptureGo {
    fn new(cols: usize, rows: usize) -> Self {
        Self { board: vec![-1; cols * rows], cols, rows, current: 0, last_capture: None }
    }
    fn orth(&self, cell: usize) -> Vec<usize> {
        let (r, c) = (cell / self.cols, cell % self.cols);
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
    // flood the same-colour group containing `start`; report whether it has a liberty
    fn group(&self, board: &[i8], start: usize) -> (Vec<usize>, bool) {
        let color = board[start];
        let mut stack = vec![start];
        let mut seen = vec![false; board.len()];
        seen[start] = true;
        let mut group = Vec::new();
        let mut lib = false;
        while let Some(cell) = stack.pop() {
            group.push(cell);
            for nb in self.orth(cell) {
                if board[nb] == -1 {
                    lib = true;
                } else if board[nb] == color && !seen[nb] {
                    seen[nb] = true;
                    stack.push(nb);
                }
            }
        }
        (group, lib)
    }
    /// Place at `cell` for `current` on a copy; remove captured enemy groups;
    /// returns (resulting board, captured?, suicide?).
    fn simulate(&self, cell: usize) -> (Vec<i8>, bool, bool) {
        let mut b = self.board.clone();
        b[cell] = self.current as i8;
        let enemy = 1 - self.current as i8;
        let mut captured = false;
        for nb in self.orth(cell) {
            if b[nb] == enemy {
                let (grp, lib) = self.group(&b, nb);
                if !lib {
                    for &g in &grp {
                        b[g] = -1;
                    }
                    captured = true;
                }
            }
        }
        let (_, own_lib) = self.group(&b, cell);
        (b, captured, !own_lib && !captured)
    }
    fn term(&self) -> Option<ProvenValue> {
        self.last_capture
            .map(|cap| if cap == self.current { ProvenValue::Win } else { ProvenValue::Loss })
    }
}
impl GameState for CaptureGo {
    type Move = u16;
    type Player = u8;
    type MoveList = Vec<u16>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<u16> {
        if self.last_capture.is_some() {
            return vec![];
        }
        (0..self.board.len())
            .filter(|&i| self.board[i] == -1)
            .filter(|&i| {
                let (_, _, suicide) = self.simulate(i);
                !suicide
            })
            .map(|i| i as u16)
            .collect()
    }
    fn make_move(&mut self, m: &u16) {
        let (b, captured, _) = self.simulate(*m as usize);
        self.board = b;
        if captured {
            self.last_capture = Some(self.current);
        }
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct CgEval;
impl Evaluator<CgCfg> for CgEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &CaptureGo, m: &Vec<u16>, _: Option<SearchHandle<CgCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], s.eval0())
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &CaptureGo, _: &i64, _: SearchHandle<CgCfg>) -> i64 {
        s.eval0()
    }
}
impl CaptureGo {
    // favour having more "almost-captured" enemy groups (groups in atari) and
    // central, well-liberated stones.
    fn eval0(&self) -> i64 {
        let mut s = gridlib::center_eval(&self.board, self.cols, self.rows, 0)
            - gridlib::center_eval(&self.board, self.cols, self.rows, 1);
        // count enemy stones with very low liberties as a threat signal
        for p in 0..2i8 {
            let enemy = 1 - p;
            let mut seen = vec![false; self.board.len()];
            for i in 0..self.board.len() {
                if self.board[i] == enemy && !seen[i] {
                    let (grp, _lib) = self.group(&self.board, i);
                    for &g in &grp {
                        seen[g] = true;
                    }
                    let libs: usize = {
                        let mut l = std::collections::HashSet::new();
                        for &g in &grp {
                            for nb in self.orth(g) {
                                if self.board[nb] == -1 {
                                    l.insert(nb);
                                }
                            }
                        }
                        l.len()
                    };
                    if libs == 1 {
                        // enemy group in atari is good for `p`
                        s += if p == 0 { 6 } else { -6 };
                    }
                }
            }
        }
        s
    }
}
#[derive(Default)]
struct CgCfg;
impl MCTS for CgCfg {
    type State = CaptureGo;
    type Eval = CgEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct CaptureGoWasm {
    manager: MCTSManager<CgCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl CaptureGoWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32) -> Self {
        let cols = (cols as usize).clamp(4, 9);
        let rows = (rows as usize).clamp(4, 9);
        Self { manager: MCTSManager::new(CaptureGo::new(cols, rows), CgCfg, CgEval, UCTPolicy::new(1.4), ()), cols, rows }
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
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap()
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
    pub fn legal_moves(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .available_moves()
            .iter()
            .map(|m| m.to_string())
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
        let m: u16 = match mov.parse() {
            Ok(v) if (v as usize) < self.cols * self.rows => v,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, CgCfg, CgEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(CaptureGo::new(self.cols, self.rows), CgCfg, CgEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surrounding_a_stone_captures_and_wins() {
        // O stone at center of a 5x5; X surrounds it -> X captures -> X wins.
        let mut g = CaptureGo::new(5, 5);
        let mid = idx(2, 2, 5);
        g.board[mid] = 1; // O
        // X plays the last of the 4 liberties; pre-place 3
        g.board[idx(1, 2, 5)] = 0;
        g.board[idx(3, 2, 5)] = 0;
        g.board[idx(2, 1, 5)] = 0;
        g.current = 0;
        g.make_move(&(idx(2, 3, 5) as u16)); // closes the last liberty
        assert_eq!(g.board[mid], -1, "O captured");
        assert!(g.term().is_some());
        assert_eq!(g.term(), Some(ProvenValue::Loss)); // current is now O, who lost
    }

    #[test]
    fn suicide_is_illegal() {
        // surround an empty point with X; O playing into it (no capture) is suicide
        let mut g = CaptureGo::new(5, 5);
        let mid = idx(2, 2, 5);
        g.board[idx(1, 2, 5)] = 0;
        g.board[idx(3, 2, 5)] = 0;
        g.board[idx(2, 1, 5)] = 0;
        g.board[idx(2, 3, 5)] = 0;
        g.current = 1; // O to move
        let legal = g.available_moves();
        assert!(!legal.contains(&(mid as u16)), "playing into a surrounded point is suicide");
    }

    #[test]
    fn ai_plays() {
        let mut g = CaptureGoWasm::new(5, 5);
        g.playout_n(300);
        assert!(g.best_move().is_some());
    }
}
