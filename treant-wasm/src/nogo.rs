//! NoGo — the "anti-Go". Players alternate placing stones, but a move is
//! illegal if it would capture an enemy stone OR leave your own stone's group
//! without a liberty. Equivalently: after every move, every group on the board
//! must still have at least one liberty. The player who has no legal move
//! loses. Stones are only ever added, so the game always terminates and there
//! are no draws — treant's exact solver plays small boards perfectly.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone)]
struct NoGo {
    cols: usize,
    rows: usize,
    grid: Vec<i8>, // -1 empty, else owner (0 or 1)
    current: u8,
}
impl NoGo {
    fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows, grid: vec![-1; cols * rows], current: 0 }
    }
    fn neighbors(&self, i: usize) -> Vec<usize> {
        let r = i / self.cols;
        let c = i % self.cols;
        let mut v = Vec::with_capacity(4);
        if r > 0 {
            v.push(i - self.cols);
        }
        if r + 1 < self.rows {
            v.push(i + self.cols);
        }
        if c > 0 {
            v.push(i - 1);
        }
        if c + 1 < self.cols {
            v.push(i + 1);
        }
        v
    }
    /// Does the group containing `start` (same colour as grid[start]) have at
    /// least one liberty (an empty adjacent cell)?
    fn group_has_liberty(&self, grid: &[i8], start: usize) -> bool {
        let color = grid[start];
        let mut stack = vec![start];
        let mut seen = vec![false; grid.len()];
        seen[start] = true;
        while let Some(cur) = stack.pop() {
            for n in self.neighbors(cur) {
                if grid[n] == -1 {
                    return true;
                }
                if grid[n] == color && !seen[n] {
                    seen[n] = true;
                    stack.push(n);
                }
            }
        }
        false
    }
    fn is_legal(&self, i: usize) -> bool {
        if self.grid[i] != -1 {
            return false;
        }
        let me = self.current as i8;
        let enemy = 1 - me;
        let mut g = self.grid.clone();
        g[i] = me;
        // Own group must keep a liberty (no suicide).
        if !self.group_has_liberty(&g, i) {
            return false;
        }
        // No adjacent enemy group may be reduced to zero liberties (no capture).
        for n in self.neighbors(i) {
            if g[n] == enemy && !self.group_has_liberty(&g, n) {
                return false;
            }
        }
        true
    }
    fn gen(&self) -> Vec<u16> {
        (0..self.grid.len()).filter(|&i| self.is_legal(i)).map(|i| i as u16).collect()
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.gen().is_empty() {
            Some(ProvenValue::Loss) // current has no legal move -> loses
        } else {
            None
        }
    }
}
impl GameState for NoGo {
    type Move = u16;
    type Player = u8;
    type MoveList = Vec<u16>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<u16> {
        self.gen()
    }
    fn make_move(&mut self, m: &u16) {
        self.grid[*m as usize] = self.current as i8;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct NoGoEval;
impl Evaluator<NoGoCfg> for NoGoEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &NoGo, m: &Vec<u16>, _: Option<SearchHandle<NoGoCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &NoGo, _: &i64, _: SearchHandle<NoGoCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct NoGoCfg;
impl MCTS for NoGoCfg {
    type State = NoGo;
    type Eval = NoGoEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct NoGoWasm {
    manager: MCTSManager<NoGoCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl NoGoWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            manager: MCTSManager::new(NoGo::new(cols, rows), NoGoCfg, NoGoEval, UCTPolicy::new(1.4), ()),
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
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let i: u16 = match mov.parse() {
            Ok(a) => a,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&i) {
            return false;
        }
        s.make_move(&i);
        self.manager = MCTSManager::new(s, NoGoCfg, NoGoEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(NoGo::new(self.cols, self.rows), NoGoCfg, NoGoEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_board_allows_every_cell() {
        let g = NoGo::new(4, 4);
        // On an empty board no placement captures or suicides, so all 16 are legal.
        assert_eq!(g.gen().len(), 16);
    }

    #[test]
    fn capturing_move_is_illegal() {
        // Surround a single enemy stone so its last liberty is the only empty cell.
        // Playing into that cell would capture -> illegal in NoGo.
        let mut g = NoGo::new(3, 3);
        // grid indices: 0 1 2 / 3 4 5 / 6 7 8. Put enemy (O=1) at centre 4,
        // friendly (X=0) at 1,3,5 leaving 7 as O's last liberty.
        g.grid[4] = 1;
        g.grid[1] = 0;
        g.grid[3] = 0;
        g.grid[5] = 0;
        g.current = 0; // X to move; playing 7 would capture the O at 4
        assert!(!g.is_legal(7));
        // but X may play elsewhere that doesn't capture, e.g. corner 0
        assert!(g.is_legal(0));
    }

    #[test]
    fn suicide_move_is_illegal() {
        // A cell fully surrounded by enemy stones is self-capture -> illegal.
        let mut g = NoGo::new(3, 3);
        g.grid[1] = 1;
        g.grid[3] = 1;
        g.grid[5] = 1;
        g.grid[7] = 1;
        g.current = 0; // X playing centre 4 would have no liberty
        assert!(!g.is_legal(4));
    }

    #[test]
    fn ai_plays() {
        let mut g = NoGoWasm::new(5, 5);
        g.playout_n(600);
        assert!(g.best_move().is_some());
    }
}
