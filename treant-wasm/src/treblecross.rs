//! Treblecross — a one-dimensional, impartial tic-tac-toe. The board is a
//! single strip of cells and BOTH players place the same mark (an X). Whoever
//! completes three X's in a row wins. The strip only fills, so the game always
//! terminates; it is tiny, so treant's solver plays it perfectly.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone)]
struct Treblecross {
    cells: Vec<i8>, // -1 empty, 1 marked (shared symbol)
    current: u8,
}
impl Treblecross {
    fn new(len: usize) -> Self {
        Self { cells: vec![-1; len], current: 0 }
    }
    fn has_three(&self) -> bool {
        self.cells.windows(3).any(|w| w[0] == 1 && w[1] == 1 && w[2] == 1)
    }
    fn gen(&self) -> Vec<u16> {
        (0..self.cells.len()).filter(|&i| self.cells[i] == -1).map(|i| i as u16).collect()
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.has_three() {
            // The previous player just completed a triple and won; from the
            // current player's perspective that is a loss.
            Some(ProvenValue::Loss)
        } else if self.gen().is_empty() {
            Some(ProvenValue::Draw)
        } else {
            None
        }
    }
}
impl GameState for Treblecross {
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
        self.cells[*m as usize] = 1;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct TcEval;
impl Evaluator<TcCfg> for TcEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Treblecross, m: &Vec<u16>, _: Option<SearchHandle<TcCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Treblecross, _: &i64, _: SearchHandle<TcCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct TcCfg;
impl MCTS for TcCfg {
    type State = Treblecross;
    type Eval = TcEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct TreblecrossWasm {
    manager: MCTSManager<TcCfg>,
    len: usize,
}
#[wasm_bindgen]
impl TreblecrossWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(len: usize) -> Self {
        Self { manager: MCTSManager::new(Treblecross::new(len), TcCfg, TcEval, UCTPolicy::new(1.4), ()), len }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// One char per cell: ' '=empty, 'X'=marked.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .cells
            .iter()
            .map(|&v| if v == 1 { 'X' } else { ' ' })
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
        if s.has_three() {
            format!("{}", 2 - s.current) // previous player (the completer) won
        } else {
            String::new() // empty == draw / not decided
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

    /// Comma-joined 0-based indices of the three-in-a-row that just won, or "".
    pub fn winning_cells(&self) -> String {
        let s = self.manager.tree().root_state();
        if !s.has_three() {
            return String::new();
        }
        for i in 0..s.cells.len().saturating_sub(2) {
            if s.cells[i] == 1 && s.cells[i + 1] == 1 && s.cells[i + 2] == 1 {
                return format!("{},{},{}", i, i + 1, i + 2);
            }
        }
        String::new()
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
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
        self.manager = MCTSManager::new(s, TcCfg, TcEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Treblecross::new(self.len), TcCfg, TcEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_in_a_row_is_terminal_loss_for_mover() {
        let mut g = Treblecross::new(7);
        g.cells[2] = 1;
        g.cells[3] = 1;
        g.current = 0;
        g.make_move(&4); // completes 2,3,4
        assert!(g.has_three());
        assert_eq!(g.term(), Some(ProvenValue::Loss)); // current (now player 1) lost
    }

    #[test]
    fn gaps_do_not_count() {
        let mut g = Treblecross::new(7);
        g.cells[1] = 1;
        g.cells[3] = 1;
        g.cells[5] = 1;
        assert!(!g.has_three());
        assert_eq!(g.term(), None);
    }

    #[test]
    fn winning_cells_reports_the_triple() {
        let mut g = TreblecrossWasm::new(7);
        assert_eq!(g.winning_cells(), "");
        // Both players place X; complete 2,3,4.
        for m in ["2", "0", "3", "6", "4"] {
            assert!(g.apply_move(m), "move {m}");
        }
        assert!(g.is_terminal());
        assert_eq!(g.winning_cells(), "2,3,4");
    }

    #[test]
    fn ai_plays() {
        let mut g = TreblecrossWasm::new(11);
        g.playout_n(500);
        assert!(g.best_move().is_some());
    }
}
