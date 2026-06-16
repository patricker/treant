//! Trails — light-cycles. Each turn move your token to an adjacent empty cell;
//! the cell you left becomes a permanent wall. The player who cannot move loses.
//! Evaluation is reachable-area (more open space = better).
use crate::gridlib::idx;
use std::collections::VecDeque;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

const WALL: i8 = -2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for TrMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

#[derive(Clone)]
struct Trails {
    board: Vec<i8>, // -1 empty, -2 wall, 0/1 = a player's current token cell
    pos: [usize; 2],
    cols: usize,
    rows: usize,
    current: u8,
}
impl Trails {
    fn new(cols: usize, rows: usize) -> Self {
        let mut board = vec![-1i8; cols * rows];
        let p0 = idx(0, 0, cols);
        let p1 = idx(rows - 1, cols - 1, cols);
        board[p0] = 0;
        board[p1] = 1;
        Self { board, pos: [p0, p1], cols, rows, current: 0 }
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
    fn gen(&self) -> Vec<TrMove> {
        let from = self.pos[self.current as usize];
        self.orth(from)
            .into_iter()
            .filter(|&to| self.board[to] == -1)
            .map(|to| TrMove { from: from as u16, to: to as u16 })
            .collect()
    }
    fn reach(&self, player: u8) -> i64 {
        // count empty cells reachable from the player's token through empties
        let mut seen = vec![false; self.board.len()];
        let mut dq = VecDeque::new();
        dq.push_back(self.pos[player as usize]);
        seen[self.pos[player as usize]] = true;
        let mut n = 0i64;
        while let Some(cell) = dq.pop_front() {
            for nb in self.orth(cell) {
                if !seen[nb] && self.board[nb] == -1 {
                    seen[nb] = true;
                    n += 1;
                    dq.push_back(nb);
                }
            }
        }
        n
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.gen().is_empty() {
            Some(ProvenValue::Loss) // current can't move -> loses
        } else {
            None
        }
    }
}
impl GameState for Trails {
    type Move = TrMove;
    type Player = u8;
    type MoveList = Vec<TrMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<TrMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &TrMove) {
        self.board[m.from as usize] = WALL;
        self.board[m.to as usize] = self.current as i8;
        self.pos[self.current as usize] = m.to as usize;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct TrEval;
impl Evaluator<TrCfg> for TrEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &Trails, m: &Vec<TrMove>, _: Option<SearchHandle<TrCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], s.reach(0) - s.reach(1))
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &Trails, _: &i64, _: SearchHandle<TrCfg>) -> i64 {
        s.reach(0) - s.reach(1)
    }
}
#[derive(Default)]
struct TrCfg;
impl MCTS for TrCfg {
    type State = Trails;
    type Eval = TrEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct TrailsWasm {
    manager: MCTSManager<TrCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl TrailsWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32) -> Self {
        let cols = (cols as usize).clamp(4, 9);
        let rows = (rows as usize).clamp(4, 9);
        Self { manager: MCTSManager::new(Trails::new(cols, rows), TrCfg, TrEval, UCTPolicy::new(1.4), ()), cols, rows }
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
                WALL => '#',
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
        let m = TrMove { from, to };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, TrCfg, TrEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Trails::new(self.cols, self.rows), TrCfg, TrEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_leaves_a_wall() {
        let mut g = Trails::new(5, 5);
        let from = g.pos[0];
        let mv = g.gen()[0];
        g.make_move(&mv);
        assert_eq!(g.board[from], WALL);
        assert_eq!(g.board[mv.to as usize], 0);
        assert_eq!(g.pos[0], mv.to as usize);
        assert_eq!(g.current, 1);
    }

    #[test]
    fn boxed_in_player_loses() {
        let mut g = Trails::new(5, 5);
        // wall off player 0 (corner 0,0): its two neighbors become walls
        g.board[idx(0, 1, 5)] = WALL;
        g.board[idx(1, 0, 5)] = WALL;
        g.current = 0;
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn ai_plays() {
        let mut g = TrailsWasm::new(6, 6);
        g.playout_n(300);
        assert!(g.best_move().is_some());
    }
}
