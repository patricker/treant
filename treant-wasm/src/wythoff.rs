//! Wythoff's Nim, visualised as a queen on a board: each turn slide the queen
//! any distance up, left, or diagonally up-left (toward the corner). Whoever
//! lands the queen on the corner (0,0) wins. P-positions are the golden-ratio
//! Beatty pairs — the exact solver finds them.
use crate::gridlib::idx;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct QMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for QMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

#[derive(Clone)]
struct Wythoff {
    n: usize,
    pos: usize, // queen cell
    current: u8,
}
impl Wythoff {
    fn new(n: usize) -> Self {
        Self { n, pos: idx(n - 1, n - 1, n), current: 0 }
    }
    fn gen(&self) -> Vec<QMove> {
        let (r, c) = (self.pos / self.n, self.pos % self.n);
        let mut v = Vec::new();
        for k in 1..=r {
            v.push(QMove { from: self.pos as u16, to: idx(r - k, c, self.n) as u16 }); // up
        }
        for k in 1..=c {
            v.push(QMove { from: self.pos as u16, to: idx(r, c - k, self.n) as u16 }); // left
        }
        for k in 1..=r.min(c) {
            v.push(QMove { from: self.pos as u16, to: idx(r - k, c - k, self.n) as u16 }); // diagonal
        }
        v
    }
    fn term(&self) -> Option<ProvenValue> {
        // queen on the corner -> the current player cannot move and has lost
        if self.pos == 0 {
            Some(ProvenValue::Loss)
        } else {
            None
        }
    }
}
impl GameState for Wythoff {
    type Move = QMove;
    type Player = u8;
    type MoveList = Vec<QMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<QMove> {
        if self.term().is_some() {
            return vec![];
        }
        self.gen()
    }
    fn make_move(&mut self, m: &QMove) {
        self.pos = m.to as usize;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct WyEval;
impl Evaluator<WyCfg> for WyEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Wythoff, m: &Vec<QMove>, _: Option<SearchHandle<WyCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Wythoff, _: &i64, _: SearchHandle<WyCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct WyCfg;
impl MCTS for WyCfg {
    type State = Wythoff;
    type Eval = WyEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct WythoffWasm {
    manager: MCTSManager<WyCfg>,
    n: usize,
}
#[wasm_bindgen]
impl WythoffWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(n: u32) -> Self {
        let n = (n as usize).clamp(4, 12);
        Self { manager: MCTSManager::new(Wythoff::new(n), WyCfg, WyEval, UCTPolicy::new(1.4), ()), n }
    }
    pub fn cols(&self) -> u32 {
        self.n as u32
    }
    pub fn rows(&self) -> u32 {
        self.n as u32
    }
    pub fn playout_n(&mut self, k: u32) {
        self.manager.playout_n(k as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap()
    }
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        (0..s.n * s.n).map(|i| if i == s.pos { 'X' } else { ' ' }).collect()
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
            .map(|m| format!("{m}"))
            .collect::<Vec<_>>()
            .join(",")
    }
    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
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
        let m = QMove { from, to };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, WyCfg, WyEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Wythoff::new(self.n), WyCfg, WyEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queen_moves_toward_corner() {
        let g = WythoffWasm::new(6);
        let moves: Vec<&str> = {
            let s = g.legal_moves();
            // start at (5,5): 5 up + 5 left + 5 diagonal = 15 moves
            assert_eq!(s.split(',').count(), 15);
            vec![]
        };
        let _ = moves;
    }

    #[test]
    fn landing_on_corner_wins() {
        let mut g = Wythoff::new(6);
        // move the queen straight to the corner via the diagonal
        g.make_move(&QMove { from: idx(5, 5, 6) as u16, to: 0 });
        assert!(g.term().is_some());
        assert_eq!(g.term(), Some(ProvenValue::Loss)); // current (didn't move) is stuck on corner
    }

    #[test]
    fn ai_plays() {
        let mut g = WythoffWasm::new(8);
        g.playout_n(400);
        assert!(g.best_move().is_some());
    }
}
