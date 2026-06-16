//! Chomp. Pick a cookie and eat it plus everything to its right and below.
//! Whoever is forced to eat the poisoned top-left cookie loses. Impartial,
//! tiny — a great exact-solver showcase.
use crate::gridlib::idx;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone)]
struct Chomp {
    board: Vec<i8>, // 1 = cookie present, -1 = eaten
    cols: usize,
    rows: usize,
    current: u8,
}
impl Chomp {
    fn new(cols: usize, rows: usize) -> Self {
        Self { board: vec![1; cols * rows], cols, rows, current: 0 }
    }
    fn term(&self) -> Option<ProvenValue> {
        // the poison (0,0) has been eaten -> the player who just ate it lost,
        // so the current player (who did NOT eat it) wins.
        if self.board[idx(0, 0, self.cols)] == -1 {
            Some(ProvenValue::Win)
        } else {
            None
        }
    }
}
impl GameState for Chomp {
    type Move = u16;
    type Player = u8;
    type MoveList = Vec<u16>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<u16> {
        if self.term().is_some() {
            return vec![];
        }
        (0..self.board.len()).filter(|&i| self.board[i] == 1).map(|i| i as u16).collect()
    }
    fn make_move(&mut self, m: &u16) {
        let cell = *m as usize;
        let (r, c) = (cell / self.cols, cell % self.cols);
        for rr in r..self.rows {
            for cc in c..self.cols {
                self.board[idx(rr, cc, self.cols)] = -1;
            }
        }
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct ChEval;
impl Evaluator<ChCfg> for ChEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Chomp, m: &Vec<u16>, _: Option<SearchHandle<ChCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Chomp, _: &i64, _: SearchHandle<ChCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct ChCfg;
impl MCTS for ChCfg {
    type State = Chomp;
    type Eval = ChEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct ChompWasm {
    manager: MCTSManager<ChCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl ChompWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32) -> Self {
        let cols = (cols as usize).clamp(2, 7);
        let rows = (rows as usize).clamp(2, 7);
        Self { manager: MCTSManager::new(Chomp::new(cols, rows), ChCfg, ChEval, UCTPolicy::new(1.4), ()), cols, rows }
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
    /// '#' = cookie, ' ' = eaten (top-left index 0 is the poison).
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .board
            .iter()
            .map(|&v| if v == 1 { '#' } else { ' ' })
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
        self.manager = MCTSManager::new(s, ChCfg, ChEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Chomp::new(self.cols, self.rows), ChCfg, ChEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eating_clears_lower_right_quadrant() {
        let mut g = Chomp::new(4, 4);
        g.make_move(&(idx(1, 1, 4) as u16)); // eat (1,1) and everything r>=1,c>=1
        assert_eq!(g.board[idx(1, 1, 4)], -1);
        assert_eq!(g.board[idx(3, 3, 4)], -1);
        assert_eq!(g.board[idx(0, 0, 4)], 1, "poison untouched");
        assert_eq!(g.board[idx(0, 1, 4)], 1, "top row beyond col 0 untouched");
    }

    #[test]
    fn eating_poison_ends_the_game() {
        let mut g = Chomp::new(3, 3);
        g.make_move(&(idx(0, 0, 3) as u16)); // eat the poison -> game over
        assert!(g.term().is_some());
        assert_eq!(g.result_for_test(), 2); // current (didn't eat) wins -> player 2
    }

    #[test]
    fn ai_avoids_instant_loss() {
        // 2x2: optimal first move is a corner; ensure the AI never opens by eating poison
        let mut g = ChompWasm::new(3, 3);
        g.playout_n(800);
        let m = g.best_move().unwrap();
        assert_ne!(m, "0", "AI must not eat the poison on move 1");
    }
}

#[cfg(test)]
impl Chomp {
    fn result_for_test(&self) -> u8 {
        match self.term() {
            Some(ProvenValue::Win) => self.current + 1,
            _ => 0,
        }
    }
}
