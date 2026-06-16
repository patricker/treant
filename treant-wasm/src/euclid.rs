//! Euclid's Game — a two-number subtraction game built on the Euclidean
//! algorithm. From a pair of positive integers you replace the larger by the
//! larger minus any positive multiple of the smaller (the result must stay
//! ≥ 0). The player who makes one of the numbers zero wins. The larger number
//! strictly shrinks, so the game always terminates and never draws — and it is
//! small, so treant's solver plays it perfectly.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone)]
struct Euclid {
    a: u32,
    b: u32,
    current: u8,
}
impl Euclid {
    fn new(a: u32, b: u32) -> Self {
        Self { a, b, current: 0 }
    }
    /// Legal moves are the values the larger number can be reduced to:
    /// hi - lo, hi - 2·lo, … down to hi mod lo (which may be 0, a winning move).
    fn gen(&self) -> Vec<u16> {
        if self.a == 0 || self.b == 0 {
            return Vec::new();
        }
        let hi = self.a.max(self.b);
        let lo = self.a.min(self.b);
        let mut v = Vec::new();
        let mut h = hi;
        while h >= lo {
            h -= lo;
            v.push(h as u16);
        }
        v
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.a == 0 || self.b == 0 {
            Some(ProvenValue::Loss) // a zero is already showing; current cannot move
        } else {
            None
        }
    }
}
impl GameState for Euclid {
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
        let target = *m as u32;
        if self.a >= self.b {
            self.a = target;
        } else {
            self.b = target;
        }
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct EuEval;
impl Evaluator<EuCfg> for EuEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Euclid, m: &Vec<u16>, _: Option<SearchHandle<EuCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Euclid, _: &i64, _: SearchHandle<EuCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct EuCfg;
impl MCTS for EuCfg {
    type State = Euclid;
    type Eval = EuEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct EuclidWasm {
    manager: MCTSManager<EuCfg>,
    a0: u32,
    b0: u32,
}
#[wasm_bindgen]
impl EuclidWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(a: u32, b: u32) -> Self {
        Self { manager: MCTSManager::new(Euclid::new(a, b), EuCfg, EuEval, UCTPolicy::new(1.4), ()), a0: a, b0: b }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap()
    }
    /// The two numbers as "a,b".
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        format!("{},{}", s.a, s.b)
    }
    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current as u32
    }
    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().term().is_some()
    }
    pub fn result(&self) -> String {
        let s = self.manager.tree().root_state();
        if s.term().is_some() {
            format!("{}", 2 - s.current) // mover who just made a zero won
        } else {
            String::new()
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
        let target: u16 = match mov.parse() {
            Ok(a) => a,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&target) {
            return false;
        }
        s.make_move(&target);
        self.manager = MCTSManager::new(s, EuCfg, EuEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Euclid::new(self.a0, self.b0), EuCfg, EuEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_reduce_larger_by_multiples_of_smaller() {
        let g = Euclid::new(25, 7);
        // 25 - 7 = 18, -14 = 11, -21 = 4
        assert_eq!(g.gen(), vec![18, 11, 4]);
    }

    #[test]
    fn equal_numbers_have_a_single_winning_move() {
        let g = Euclid::new(9, 9);
        assert_eq!(g.gen(), vec![0]);
    }

    #[test]
    fn a_zero_is_terminal_loss_for_mover() {
        let g = Euclid::new(0, 5);
        assert!(g.gen().is_empty());
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn making_a_zero_is_a_legal_winning_move() {
        let mut g = Euclid::new(14, 7);
        // 14 - 2*7 = 0 should be offered
        assert!(g.gen().contains(&0));
        g.make_move(&0);
        assert!(g.a == 0 || g.b == 0);
    }

    #[test]
    fn ai_plays() {
        let mut g = EuclidWasm::new(25, 16);
        g.playout_n(800);
        assert!(g.best_move().is_some());
    }
}
