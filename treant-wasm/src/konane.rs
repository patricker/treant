//! Kōnane — a traditional Hawaiian jump-capture game. The board starts as a
//! full checkerboard of black/white stones with two adjacent central stones
//! removed. On your turn you jump one of your stones orthogonally over an
//! adjacent enemy into the empty cell beyond, capturing the jumped stone. The
//! player who cannot jump loses (last to move wins). No draws, so treant's
//! exact solver plays small boards perfectly.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct KMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for KMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

#[derive(Clone)]
struct Konane {
    cols: usize,
    rows: usize,
    grid: Vec<i8>, // -1 empty, else owner (0 or 1)
    current: u8,
}
impl Konane {
    fn new(cols: usize, rows: usize) -> Self {
        // Checkerboard: cell (r,c) holds player (r+c)%2.
        let mut grid = vec![0i8; cols * rows];
        for r in 0..rows {
            for c in 0..cols {
                grid[r * cols + c] = ((r + c) % 2) as i8;
            }
        }
        // Remove two horizontally-adjacent central stones (one of each colour).
        let cr = rows / 2;
        let cc = cols / 2;
        let a = cr * cols + cc;
        grid[a] = -1;
        grid[a - 1] = -1;
        Self { cols, rows, grid, current: 0 }
    }
    fn gen(&self) -> Vec<KMove> {
        let mut v = Vec::new();
        let me = self.current as i8;
        let enemy = 1 - me;
        for r in 0..self.rows {
            for c in 0..self.cols {
                let i = r * self.cols + c;
                if self.grid[i] != me {
                    continue;
                }
                // four orthogonal jump directions, expressed as (dr, dc)
                let dirs: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
                for (dr, dc) in dirs {
                    let mr = r as isize + dr;
                    let mc = c as isize + dc;
                    let lr = r as isize + 2 * dr;
                    let lc = c as isize + 2 * dc;
                    if lr < 0 || lr >= self.rows as isize || lc < 0 || lc >= self.cols as isize {
                        continue;
                    }
                    let mid = mr as usize * self.cols + mc as usize;
                    let land = lr as usize * self.cols + lc as usize;
                    if self.grid[mid] == enemy && self.grid[land] == -1 {
                        v.push(KMove { from: i as u16, to: land as u16 });
                    }
                }
            }
        }
        v
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.gen().is_empty() {
            Some(ProvenValue::Loss) // current can't jump -> loses
        } else {
            None
        }
    }
}
impl GameState for Konane {
    type Move = KMove;
    type Player = u8;
    type MoveList = Vec<KMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<KMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &KMove) {
        let from = m.from as usize;
        let to = m.to as usize;
        let mid = (from + to) / 2; // the jumped cell sits exactly between
        self.grid[to] = self.current as i8;
        self.grid[from] = -1;
        self.grid[mid] = -1;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct KEval;
impl Evaluator<KCfg> for KEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Konane, m: &Vec<KMove>, _: Option<SearchHandle<KCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Konane, _: &i64, _: SearchHandle<KCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct KCfg;
impl MCTS for KCfg {
    type State = Konane;
    type Eval = KEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct KonaneWasm {
    manager: MCTSManager<KCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl KonaneWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            manager: MCTSManager::new(Konane::new(cols, rows), KCfg, KEval, UCTPolicy::new(1.4), ()),
            cols,
            rows,
        }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap()
    }
    /// One char per cell, row-major: ' '=empty, 'X'=player 0, 'O'=player 1.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .grid
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
        let m = KMove { from, to };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, KCfg, KEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Konane::new(self.cols, self.rows), KCfg, KEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_board_has_two_central_holes() {
        let g = Konane::new(6, 6);
        let empties = g.grid.iter().filter(|&&v| v == -1).count();
        assert_eq!(empties, 2);
        // player 0 moves first and has at least one legal jump into a hole
        assert!(!g.gen().is_empty());
    }

    #[test]
    fn jump_captures_the_middle_stone() {
        let mut g = Konane::new(6, 6);
        let m = g.gen()[0];
        let mid = (m.from as usize + m.to as usize) / 2;
        g.make_move(&m);
        assert_eq!(g.grid[m.to as usize], 0); // mover landed
        assert_eq!(g.grid[m.from as usize], -1); // vacated
        assert_eq!(g.grid[mid], -1); // captured enemy removed
        assert_eq!(g.current, 1);
    }

    #[test]
    fn ai_plays() {
        let mut g = KonaneWasm::new(6, 6);
        g.playout_n(800);
        assert!(g.best_move().is_some());
    }
}
