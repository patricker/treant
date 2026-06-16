//! Col — a classic map-colouring combinatorial game. On your turn you colour
//! any uncoloured cell with *your* colour, as long as that cell is not
//! orthogonally adjacent to a cell already in your colour (no two same-colour
//! regions may touch). The player who cannot colour a cell loses. Cells only
//! get filled, so the game always terminates and there are no draws — treant's
//! exact solver plays small boards perfectly.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone)]
struct Col {
    cols: usize,
    rows: usize,
    grid: Vec<i8>, // -1 uncoloured, else owner (0 or 1)
    current: u8,
}
impl Col {
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
    fn is_legal(&self, i: usize) -> bool {
        if self.grid[i] != -1 {
            return false;
        }
        let me = self.current as i8;
        // illegal if any orthogonal neighbour is already my colour
        !self.neighbors(i).iter().any(|&n| self.grid[n] == me)
    }
    fn gen(&self) -> Vec<u16> {
        (0..self.grid.len()).filter(|&i| self.is_legal(i)).map(|i| i as u16).collect()
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.gen().is_empty() {
            Some(ProvenValue::Loss) // current can't colour a cell -> loses
        } else {
            None
        }
    }
}
impl GameState for Col {
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
struct ColEval;
impl Evaluator<ColCfg> for ColEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Col, m: &Vec<u16>, _: Option<SearchHandle<ColCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Col, _: &i64, _: SearchHandle<ColCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct ColCfg;
impl MCTS for ColCfg {
    type State = Col;
    type Eval = ColEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct ColWasm {
    manager: MCTSManager<ColCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl ColWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            manager: MCTSManager::new(Col::new(cols, rows), ColCfg, ColEval, UCTPolicy::new(1.4), ()),
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
    /// One char per cell, row-major: ' '=uncoloured, 'X'=player 0, 'O'=player 1.
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
        self.manager = MCTSManager::new(s, ColCfg, ColEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Col::new(self.cols, self.rows), ColCfg, ColEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_board_allows_every_cell() {
        let g = Col::new(4, 4);
        assert_eq!(g.gen().len(), 16);
    }

    #[test]
    fn cannot_colour_next_to_own_colour() {
        let mut g = Col::new(3, 3);
        g.grid[4] = 0; // X in the centre
        g.current = 0; // X to move
        // the four orthogonal neighbours of centre (1,3,5,7) are now illegal for X
        for n in [1usize, 3, 5, 7] {
            assert!(!g.is_legal(n));
        }
        // but a diagonal (corner 0) and far cells stay legal
        assert!(g.is_legal(0));
    }

    #[test]
    fn opponent_may_colour_adjacent() {
        let mut g = Col::new(3, 3);
        g.grid[4] = 0; // X centre
        g.current = 1; // O to move — O has no O-neighbours, so all empties are legal
        assert!(g.is_legal(1));
        assert!(g.is_legal(3));
    }

    #[test]
    fn ai_plays() {
        let mut g = ColWasm::new(5, 5);
        g.playout_n(600);
        assert!(g.best_move().is_some());
    }
}
