//! Fox & Hounds — an asymmetric chase on the dark squares of a checkerboard.
//! The lone Fox (player 0, moves first) steps one square diagonally in any
//! direction and wins by reaching the Hounds' home row. The Hounds (player 1)
//! each step one square diagonally *forward only* and win by trapping the Fox
//! with no legal move. Because Hounds never retreat, their total advancement
//! strictly increases, so every game — and every random playout — terminates.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FhMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for FhMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

#[derive(Clone)]
struct FoxHounds {
    cols: usize,
    rows: usize,
    grid: Vec<i8>, // -1 empty, 0 fox, 1 hound
    current: u8,
}
impl FoxHounds {
    fn new(cols: usize, rows: usize) -> Self {
        let mut grid = vec![-1i8; cols * rows];
        // Hounds occupy the dark squares of the top row.
        for (c, cell) in grid.iter_mut().take(cols).enumerate() {
            if c % 2 == 1 {
                *cell = 1;
            }
        }
        // Fox starts on the dark square of the bottom row nearest the centre.
        let last = rows - 1;
        let mut fox_c = 0;
        let mut best = isize::MAX;
        for c in 0..cols {
            if (last + c) % 2 == 1 {
                let d = (c as isize - cols as isize / 2).abs();
                if d < best {
                    best = d;
                    fox_c = c;
                }
            }
        }
        grid[last * cols + fox_c] = 0;
        Self { cols, rows, grid, current: 0 }
    }
    fn fox_row(&self) -> usize {
        let f = self.grid.iter().position(|&v| v == 0).unwrap();
        f / self.cols
    }
    fn gen(&self) -> Vec<FhMove> {
        let mut v = Vec::new();
        for from in 0..self.grid.len() {
            if self.grid[from] != self.current as i8 {
                continue;
            }
            let r = from / self.cols;
            let c = from % self.cols;
            // Fox: all four diagonals. Hounds: forward (increasing row) only.
            let drs: &[isize] = if self.current == 0 { &[-1, 1] } else { &[1] };
            for &dr in drs {
                for dc in [-1isize, 1] {
                    let nr = r as isize + dr;
                    let nc = c as isize + dc;
                    if nr < 0 || nr >= self.rows as isize || nc < 0 || nc >= self.cols as isize {
                        continue;
                    }
                    let to = nr as usize * self.cols + nc as usize;
                    if self.grid[to] == -1 {
                        v.push(FhMove { from: from as u16, to: to as u16 });
                    }
                }
            }
        }
        v
    }
    fn term(&self) -> Option<ProvenValue> {
        // Fox reaching the top row (Hounds' home) is an immediate Fox win.
        if self.fox_row() == 0 {
            return Some(if self.current == 0 { ProvenValue::Win } else { ProvenValue::Loss });
        }
        if self.gen().is_empty() {
            Some(ProvenValue::Loss) // current cannot move and loses
        } else {
            None
        }
    }
}
impl GameState for FoxHounds {
    type Move = FhMove;
    type Player = u8;
    type MoveList = Vec<FhMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<FhMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &FhMove) {
        self.grid[m.to as usize] = self.grid[m.from as usize];
        self.grid[m.from as usize] = -1;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct FhEval;
impl Evaluator<FhCfg> for FhEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &FoxHounds, m: &Vec<FhMove>, _: Option<SearchHandle<FhCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &FoxHounds, _: &i64, _: SearchHandle<FhCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct FhCfg;
impl MCTS for FhCfg {
    type State = FoxHounds;
    type Eval = FhEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct FoxHoundsWasm {
    manager: MCTSManager<FhCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl FoxHoundsWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            manager: MCTSManager::new(FoxHounds::new(cols, rows), FhCfg, FhEval, UCTPolicy::new(1.4), ()),
            cols,
            rows,
        }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// One char per cell, row-major: ' '=empty, 'X'=fox, 'O'=hound.
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
        if s.term().is_none() {
            return String::new();
        }
        if s.fox_row() == 0 {
            return "1".into(); // fox (player 0) reached home
        }
        // otherwise the side to move is stuck and loses
        format!("{}", 2 - s.current)
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
        let m = FhMove { from, to };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, FhCfg, FhEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager =
            MCTSManager::new(FoxHounds::new(self.cols, self.rows), FhCfg, FhEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_has_one_fox_and_some_hounds() {
        let g = FoxHounds::new(8, 8);
        assert_eq!(g.grid.iter().filter(|&&v| v == 0).count(), 1);
        assert_eq!(g.grid.iter().filter(|&&v| v == 1).count(), 4);
        assert_eq!(g.fox_row(), 7);
    }

    #[test]
    fn hounds_only_move_forward() {
        let g = FoxHounds::new(8, 8);
        // It's the fox's turn first; switch to hounds and confirm every hound
        // move increases the row index (moves toward the fox).
        let mut h = g.clone();
        h.current = 1;
        for m in h.gen() {
            let fr = m.from as usize / 8;
            let tr = m.to as usize / 8;
            assert_eq!(tr, fr + 1);
        }
    }

    #[test]
    fn fox_reaching_top_row_wins() {
        let mut g = FoxHounds::new(8, 8);
        // Drop the fox onto the top row directly and check the terminal verdict.
        let f = g.grid.iter().position(|&v| v == 0).unwrap();
        g.grid[f] = -1;
        g.grid[1] = 0; // fox now on row 0
        g.current = 1; // hounds to move, but fox already home
        assert_eq!(g.fox_row(), 0);
        assert_eq!(g.term(), Some(ProvenValue::Loss)); // from hounds' view, a loss
    }

    #[test]
    fn ai_plays() {
        let mut g = FoxHoundsWasm::new(8, 8);
        g.playout_n(400);
        assert!(g.best_move().is_some());
    }
}
