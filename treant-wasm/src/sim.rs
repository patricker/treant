//! Sim — the game of Sim, played on the complete graph K6 (six vertices, all
//! fifteen connecting edges). Players take turns colouring an uncoloured edge
//! with their own colour. You LOSE if you ever complete a triangle whose three
//! edges are all your colour (a misère game). By Ramsey's theorem R(3,3)=6 any
//! full 2-colouring of K6 contains a monochromatic triangle, so the game can
//! never end in a draw — and it is tiny, so treant's solver plays it perfectly.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

/// 15 edges of K6, enumerated as (i,j) with i<j in row-major order.
fn edge_index(a: usize, b: usize) -> usize {
    let (i, j) = if a < b { (a, b) } else { (b, a) };
    let mut idx = 0;
    for x in 0..i {
        idx += 5 - x;
    }
    idx + (j - i - 1)
}

/// The 20 triangles of K6, each as its three edge indices.
fn triangles() -> Vec<[usize; 3]> {
    let mut v = Vec::with_capacity(20);
    for a in 0..6 {
        for b in (a + 1)..6 {
            for c in (b + 1)..6 {
                v.push([edge_index(a, b), edge_index(a, c), edge_index(b, c)]);
            }
        }
    }
    v
}

#[derive(Clone)]
struct Sim {
    edges: [i8; 15], // -1 uncoloured, else owner (0 or 1)
    current: u8,
}
impl Sim {
    fn new() -> Self {
        Self { edges: [-1; 15], current: 0 }
    }
    /// If a player has completed a monochromatic triangle, return that player's
    /// colour (they have lost).
    fn losing_color(&self) -> Option<i8> {
        for t in triangles() {
            let c = self.edges[t[0]];
            if c >= 0 && self.edges[t[1]] == c && self.edges[t[2]] == c {
                return Some(c);
            }
        }
        None
    }
    fn gen(&self) -> Vec<u16> {
        (0..15).filter(|&e| self.edges[e] == -1).map(|e| e as u16).collect()
    }
    fn term(&self) -> Option<ProvenValue> {
        match self.losing_color() {
            // The loser is whoever just completed their triangle. Reported from
            // the current player's perspective: if it's my colour I lost.
            Some(c) if c == self.current as i8 => Some(ProvenValue::Loss),
            Some(_) => Some(ProvenValue::Win),
            None => {
                if self.gen().is_empty() {
                    Some(ProvenValue::Draw) // unreachable by Ramsey, but be safe
                } else {
                    None
                }
            }
        }
    }
}
impl GameState for Sim {
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
        self.edges[*m as usize] = self.current as i8;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct SimEval;
impl Evaluator<SimCfg> for SimEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Sim, m: &Vec<u16>, _: Option<SearchHandle<SimCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Sim, _: &i64, _: SearchHandle<SimCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct SimCfg;
impl MCTS for SimCfg {
    type State = Sim;
    type Eval = SimEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct SimWasm {
    manager: MCTSManager<SimCfg>,
}
#[wasm_bindgen]
impl SimWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { manager: MCTSManager::new(Sim::new(), SimCfg, SimEval, UCTPolicy::new(1.4), ()) }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// 15 chars, one per edge in (i<j) row-major order: ' '=uncoloured,
    /// 'X'=player 0, 'O'=player 1.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .edges
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
        match s.losing_color() {
            Some(c) => format!("{}", 2 - c), // winner is the other player
            None => String::new(),
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
        let e: u16 = match mov.parse() {
            Ok(a) => a,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&e) {
            return false;
        }
        s.make_move(&e);
        self.manager = MCTSManager::new(s, SimCfg, SimEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Sim::new(), SimCfg, SimEval, UCTPolicy::new(1.4), ());
    }
}

impl Default for SimWasm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifteen_edges_and_twenty_triangles() {
        let g = Sim::new();
        assert_eq!(g.gen().len(), 15);
        assert_eq!(triangles().len(), 20);
    }

    #[test]
    fn completing_a_monochrome_triangle_loses() {
        let mut g = Sim::new();
        // Player 0 colours the three edges of triangle (0,1,2): (0,1),(0,2),(1,2).
        // Interleave so player 0 plays them on its turns.
        g.edges[edge_index(0, 1)] = 0;
        g.edges[edge_index(0, 2)] = 0;
        g.current = 0; // player 0 to move, about to complete its own triangle
        g.make_move(&(edge_index(1, 2) as u16));
        // now player 1 is to move and player 0 has lost
        assert_eq!(g.losing_color(), Some(0));
        assert_eq!(g.term(), Some(ProvenValue::Win)); // from player 1's view: a win
    }

    #[test]
    fn mixed_triangle_is_not_a_loss() {
        let mut g = Sim::new();
        g.edges[edge_index(0, 1)] = 0;
        g.edges[edge_index(0, 2)] = 0;
        g.edges[edge_index(1, 2)] = 1; // third edge is the other colour
        assert_eq!(g.losing_color(), None);
    }

    #[test]
    fn ai_plays() {
        let mut g = SimWasm::new();
        g.playout_n(500);
        assert!(g.best_move().is_some());
    }
}
