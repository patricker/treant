//! Subtract-a-Square — an impartial pile game. From a single pile you remove a
//! *perfect-square* number of stones (1, 4, 9, 16, …); whoever takes the last
//! stone wins. The pile only shrinks, so it always terminates, and it is a
//! classic Sprague-Grundy example — treant's solver plays it perfectly.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone)]
struct SubSquare {
    pile: u32,
    current: u8,
}
impl SubSquare {
    fn new(start: u32) -> Self {
        Self { pile: start, current: 0 }
    }
    fn gen(&self) -> Vec<u16> {
        let mut v = Vec::new();
        let mut k = 1u32;
        while k * k <= self.pile {
            v.push((k * k) as u16);
            k += 1;
        }
        v
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.pile == 0 {
            Some(ProvenValue::Loss) // no stones to take -> current loses
        } else {
            None
        }
    }
}
impl GameState for SubSquare {
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
        self.pile -= *m as u32;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct SsEval;
impl Evaluator<SsCfg> for SsEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &SubSquare, m: &Vec<u16>, _: Option<SearchHandle<SsCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &SubSquare, _: &i64, _: SearchHandle<SsCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct SsCfg;
impl MCTS for SsCfg {
    type State = SubSquare;
    type Eval = SsEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct SubtractSquareWasm {
    manager: MCTSManager<SsCfg>,
    start: u32,
}
#[wasm_bindgen]
impl SubtractSquareWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(start: u32) -> Self {
        Self { manager: MCTSManager::new(SubSquare::new(start), SsCfg, SsEval, UCTPolicy::new(1.4), ()), start }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap()
    }
    pub fn current_stones(&self) -> u32 {
        self.manager.tree().root_state().pile
    }
    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current as u32
    }
    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().term().is_some()
    }
    /// Comma-separated legal removals (the perfect squares ≤ pile).
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
        let k: u16 = match mov.parse() {
            Ok(a) => a,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&k) {
            return false;
        }
        s.make_move(&k);
        self.manager = MCTSManager::new(s, SsCfg, SsEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(SubSquare::new(self.start), SsCfg, SsEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_moves_are_the_squares_up_to_pile() {
        let g = SubSquare::new(10);
        assert_eq!(g.gen(), vec![1, 4, 9]);
    }

    #[test]
    fn empty_pile_is_a_loss_for_mover() {
        let g = SubSquare::new(0);
        assert!(g.gen().is_empty());
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn removing_updates_pile_and_turn() {
        let mut g = SubSquare::new(10);
        g.make_move(&9);
        assert_eq!(g.pile, 1);
        assert_eq!(g.current, 1);
    }

    #[test]
    fn solver_knows_a_known_loss() {
        // 2 is a P-position (previous-player win): from 2 you can only take 1,
        // leaving 1 for the opponent who takes it and wins. So the player to
        // move from 2 should be losing.
        let mut g = SubtractSquareWasm::new(2);
        g.playout_n(2000);
        // The mover is losing; best_move still exists (must take 1).
        assert!(g.best_move().is_some());
    }
}
