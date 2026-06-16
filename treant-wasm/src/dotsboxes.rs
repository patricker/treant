//! Dots and Boxes. Claim an unclaimed edge; completing the 4th side of a box
//! claims it and grants another turn. Most boxes when all edges are claimed
//! wins. Bonus-turn structure (like Mancala) drives the chain strategy.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone)]
struct DotsBoxes {
    cols: usize, // boxes wide
    rows: usize, // boxes tall
    edges: Vec<bool>,
    owner: Vec<i8>, // box owner, -1 none
    current: u8,
    hoff: usize, // index where vertical edges begin
}
impl DotsBoxes {
    fn new(cols: usize, rows: usize) -> Self {
        let hoff = (rows + 1) * cols; // # of horizontal edges
        let n_edges = hoff + rows * (cols + 1);
        Self { cols, rows, edges: vec![false; n_edges], owner: vec![-1; cols * rows], current: 0, hoff }
    }
    fn h(&self, r: usize, c: usize) -> usize {
        r * self.cols + c
    }
    fn v(&self, r: usize, c: usize) -> usize {
        self.hoff + r * (self.cols + 1) + c
    }
    fn box_complete(&self, r: usize, c: usize) -> bool {
        self.edges[self.h(r, c)]
            && self.edges[self.h(r + 1, c)]
            && self.edges[self.v(r, c)]
            && self.edges[self.v(r, c + 1)]
    }
    /// boxes adjacent to an edge
    fn adjacent_boxes(&self, edge: usize) -> Vec<(usize, usize)> {
        let mut v = Vec::new();
        if edge < self.hoff {
            let (r, c) = (edge / self.cols, edge % self.cols);
            if r > 0 {
                v.push((r - 1, c));
            }
            if r < self.rows {
                v.push((r, c));
            }
        } else {
            let e = edge - self.hoff;
            let (r, c) = (e / (self.cols + 1), e % (self.cols + 1));
            if c > 0 {
                v.push((r, c - 1));
            }
            if c < self.cols {
                v.push((r, c));
            }
        }
        v
    }
    fn score(&self, p: i8) -> i64 {
        self.owner.iter().filter(|&&o| o == p).count() as i64
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.edges.iter().any(|&e| !e) {
            return None;
        }
        let (me, you) = (self.score(self.current as i8), self.score(1 - self.current as i8));
        Some(match me.cmp(&you) {
            std::cmp::Ordering::Greater => ProvenValue::Win,
            std::cmp::Ordering::Less => ProvenValue::Loss,
            std::cmp::Ordering::Equal => ProvenValue::Draw,
        })
    }
}
impl GameState for DotsBoxes {
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
        (0..self.edges.len()).filter(|&i| !self.edges[i]).map(|i| i as u16).collect()
    }
    fn make_move(&mut self, m: &u16) {
        let edge = *m as usize;
        self.edges[edge] = true;
        let mut completed = false;
        for (r, c) in self.adjacent_boxes(edge) {
            if self.owner[r * self.cols + c] < 0 && self.box_complete(r, c) {
                self.owner[r * self.cols + c] = self.current as i8;
                completed = true;
            }
        }
        if !completed {
            self.current = 1 - self.current; // completing a box -> go again
        }
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct DbEval;
impl Evaluator<DbCfg> for DbEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &DotsBoxes, m: &Vec<u16>, _: Option<SearchHandle<DbCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], s.score(0) - s.score(1))
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &DotsBoxes, _: &i64, _: SearchHandle<DbCfg>) -> i64 {
        s.score(0) - s.score(1)
    }
}
#[derive(Default)]
struct DbCfg;
impl MCTS for DbCfg {
    type State = DotsBoxes;
    type Eval = DbEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
}

#[wasm_bindgen]
pub struct DotsBoxesWasm {
    manager: MCTSManager<DbCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl DotsBoxesWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32) -> Self {
        let cols = (cols as usize).clamp(2, 5);
        let rows = (rows as usize).clamp(2, 5);
        Self { manager: MCTSManager::new(DotsBoxes::new(cols, rows), DbCfg, DbEval, UCTPolicy::new(1.4), ()), cols, rows }
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
    /// "edgesBits|boxOwners|score0,score1"  (edges: '0'/'1', boxes: '.'/'0'/'1')
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        let edges: String = s.edges.iter().map(|&e| if e { '1' } else { '0' }).collect();
        let boxes: String = s
            .owner
            .iter()
            .map(|&o| match o {
                0 => '0',
                1 => '1',
                _ => '.',
            })
            .collect();
        format!("{}|{}|{},{}", edges, boxes, s.score(0), s.score(1))
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
            Some(ProvenValue::Loss) => format!("{}", (1 - s.current) + 1),
            Some(ProvenValue::Draw) => "Draw".into(),
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

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m: u16 = match mov.parse() {
            Ok(v) if (v as usize) < self.manager.tree().root_state().edges.len() => v,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, DbCfg, DbEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(DotsBoxes::new(self.cols, self.rows), DbCfg, DbEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completing_a_box_claims_it_and_grants_another_turn() {
        let mut g = DotsBoxes::new(2, 2);
        // box (0,0): top H(0,0), bottom H(1,0), left V(0,0), right V(0,1)
        let (t, b, l, r) = (g.h(0, 0), g.h(1, 0), g.v(0, 0), g.v(0, 1));
        g.make_move(&(t as u16)); // p0
        assert_eq!(g.current, 1);
        g.current = 0; // pretend p0 again for the test (place 3 sides for p0)
        g.make_move(&(b as u16));
        g.current = 0;
        g.make_move(&(l as u16));
        g.current = 0;
        g.make_move(&(r as u16)); // completes box (0,0)
        assert_eq!(g.owner[0], 0, "box claimed by p0");
        assert_eq!(g.current, 0, "completing a box -> p0 goes again");
    }

    #[test]
    fn full_board_scores_decide() {
        let mut g = DotsBoxesWasm::new(2, 2);
        // claim every edge
        while !g.is_terminal() {
            let m = g.legal_moves();
            let first = m.split(',').next().unwrap().to_string();
            assert!(g.apply_move(&first));
        }
        assert!(g.is_terminal());
        // 4 boxes total -> someone has >=2 or a 2-2 draw
        assert!(!g.result().is_empty());
    }

    #[test]
    fn ai_plays() {
        let mut g = DotsBoxesWasm::new(3, 3);
        g.playout_n(300);
        assert!(g.best_move().is_some());
    }
}
