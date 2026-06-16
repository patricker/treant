//! Mu Tōrere — a traditional Māori game on an 8-point star with a centre.
//! Slide a piece into the empty point; a piece may only move to the centre if
//! it sits next to an opponent's piece (the kewai rule, which keeps it from
//! being a trivial first-move win). The player who cannot move loses. Tiny, so
//! the exact solver plays it perfectly.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MtMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for MtMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

#[derive(Clone)]
struct MuTorere {
    // index 0 = centre; 1..=8 = the outer ring (clockwise). -1 empty.
    cells: [i8; 9],
    current: u8,
}
impl MuTorere {
    fn new() -> Self {
        // index 0 = centre (empty); ring 1..=4 = player 0, ring 5..=8 = player 1.
        let cells = [-1, 0, 0, 0, 0, 1, 1, 1, 1];
        Self { cells, current: 0 }
    }
    fn ring_prev(i: usize) -> usize {
        if i == 1 { 8 } else { i - 1 }
    }
    fn ring_next(i: usize) -> usize {
        if i == 8 { 1 } else { i + 1 }
    }
    fn gen(&self) -> Vec<MtMove> {
        let mut v = Vec::new();
        let empty = self.cells.iter().position(|&c| c < 0);
        let empty = match empty {
            Some(e) => e,
            None => return v,
        };
        for from in 0..9usize {
            if self.cells[from] != self.current as i8 {
                continue;
            }
            if from == 0 {
                // centre piece may slide to any empty outer point
                if empty != 0 {
                    v.push(MtMove { from: 0, to: empty as u16 });
                }
            } else {
                // ring neighbours
                for to in [Self::ring_prev(from), Self::ring_next(from)] {
                    if to == empty {
                        v.push(MtMove { from: from as u16, to: to as u16 });
                    }
                }
                // to the centre — only if adjacent to an enemy (kewai rule)
                if empty == 0 {
                    let enemy = 1 - self.current as i8;
                    if self.cells[Self::ring_prev(from)] == enemy || self.cells[Self::ring_next(from)] == enemy {
                        v.push(MtMove { from: from as u16, to: 0 });
                    }
                }
            }
        }
        v
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.gen().is_empty() {
            Some(ProvenValue::Loss) // current can't move -> loses
        } else {
            None
        }
    }
}
impl GameState for MuTorere {
    type Move = MtMove;
    type Player = u8;
    type MoveList = Vec<MtMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<MtMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &MtMove) {
        self.cells[m.to as usize] = self.current as i8;
        self.cells[m.from as usize] = -1;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct MtEval;
impl Evaluator<MtCfg> for MtEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &MuTorere, m: &Vec<MtMove>, _: Option<SearchHandle<MtCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &MuTorere, _: &i64, _: SearchHandle<MtCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct MtCfg;
impl MCTS for MtCfg {
    type State = MuTorere;
    type Eval = MtEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct MuTorereWasm {
    manager: MCTSManager<MtCfg>,
}
#[wasm_bindgen]
impl MuTorereWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { manager: MCTSManager::new(MuTorere::new(), MtCfg, MtEval, UCTPolicy::new(1.4), ()) }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap()
    }
    /// 9 chars: index 0 = centre, then ring 1..8. ' '=empty, 'X'=p0, 'O'=p1.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .cells
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
        let parts: Vec<&str> = mov.split('-').collect();
        if parts.len() != 2 {
            return false;
        }
        let (from, to): (u16, u16) = match (parts[0].parse(), parts[1].parse()) {
            (Ok(a), Ok(b)) => (a, b),
            _ => return false,
        };
        let m = MtMove { from, to };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, MtCfg, MtEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(MuTorere::new(), MtCfg, MtEval, UCTPolicy::new(1.4), ());
    }
}

impl Default for MuTorereWasm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_moves_only_into_centre_when_next_to_enemy() {
        let g = MuTorere::new();
        // empty is centre (0). P0 pieces 1..4; piece 4 is next to enemy (5) -> can
        // enter centre. piece 1 is next to enemy (8) -> can enter. pieces 2,3 cannot.
        let moves = g.gen();
        let to_centre: Vec<u16> = moves.iter().filter(|m| m.to == 0).map(|m| m.from).collect();
        assert!(to_centre.contains(&1) && to_centre.contains(&4));
        assert!(!to_centre.contains(&2) && !to_centre.contains(&3));
    }

    #[test]
    fn sliding_updates_cells_and_turn() {
        let mut g = MuTorere::new();
        g.make_move(&MtMove { from: 4, to: 0 }); // P0 piece 4 -> centre
        assert_eq!(g.cells[0], 0);
        assert_eq!(g.cells[4], -1);
        assert_eq!(g.current, 1);
    }

    #[test]
    fn ai_plays() {
        let mut g = MuTorereWasm::new();
        g.playout_n(500);
        assert!(g.best_move().is_some());
    }
}
