//! Domineering — a partisan tile-placing game. The two players have *different*
//! moves: player 0 (Vertical) places dominoes covering two vertically-adjacent
//! empty cells, player 1 (Horizontal) places horizontally-adjacent ones. The
//! player who cannot place a domino loses (normal play). No draws, so treant's
//! exact solver plays it perfectly on small boards.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone)]
struct Domineering {
    cols: usize,
    rows: usize,
    grid: Vec<i8>, // -1 empty, else owner (0 or 1)
    current: u8,
}
impl Domineering {
    fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows, grid: vec![-1; cols * rows], current: 0 }
    }
    /// A move is the anchor cell index: the top cell of a vertical domino
    /// (player 0) or the left cell of a horizontal one (player 1).
    fn gen(&self) -> Vec<u16> {
        let mut v = Vec::new();
        for r in 0..self.rows {
            for c in 0..self.cols {
                let i = r * self.cols + c;
                if self.grid[i] != -1 {
                    continue;
                }
                if self.current == 0 {
                    // vertical: also need the cell directly below to be empty
                    if r + 1 < self.rows && self.grid[i + self.cols] == -1 {
                        v.push(i as u16);
                    }
                } else if c + 1 < self.cols && self.grid[i + 1] == -1 {
                    // horizontal: also need the cell directly to the right
                    v.push(i as u16);
                }
            }
        }
        v
    }
    fn partner(&self, anchor: usize) -> usize {
        if self.current == 0 { anchor + self.cols } else { anchor + 1 }
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.gen().is_empty() {
            Some(ProvenValue::Loss) // current can't move -> loses
        } else {
            None
        }
    }
}
impl GameState for Domineering {
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
        let anchor = *m as usize;
        let partner = self.partner(anchor);
        self.grid[anchor] = self.current as i8;
        self.grid[partner] = self.current as i8;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct DomEval;
impl Evaluator<DomCfg> for DomEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Domineering, m: &Vec<u16>, _: Option<SearchHandle<DomCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Domineering, _: &i64, _: SearchHandle<DomCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct DomCfg;
impl MCTS for DomCfg {
    type State = Domineering;
    type Eval = DomEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct DomineeringWasm {
    manager: MCTSManager<DomCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl DomineeringWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            manager: MCTSManager::new(Domineering::new(cols, rows), DomCfg, DomEval, UCTPolicy::new(1.4), ()),
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

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let anchor: u16 = match mov.parse() {
            Ok(a) => a,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&anchor) {
            return false;
        }
        s.make_move(&anchor);
        self.manager = MCTSManager::new(s, DomCfg, DomEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager =
            MCTSManager::new(Domineering::new(self.cols, self.rows), DomCfg, DomEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_player_only_makes_vertical_dominoes() {
        let g = Domineering::new(2, 2); // 2x2, player 0 to move
        // anchors 0 and 1 are valid (each has a cell below): covers {0,2} and {1,3}.
        let mut moves = g.gen();
        moves.sort();
        assert_eq!(moves, vec![0, 1]);
    }

    #[test]
    fn placing_fills_both_cells_and_swaps_player() {
        let mut g = Domineering::new(3, 3);
        g.make_move(&0); // vertical domino covers cells 0 and 3
        assert_eq!(g.grid[0], 0);
        assert_eq!(g.grid[3], 0);
        assert_eq!(g.current, 1);
        // horizontal player now: anchor 1 covers {1,2}; anchor 0 is taken.
        assert!(g.gen().contains(&1));
        assert!(!g.gen().contains(&0));
    }

    #[test]
    fn full_blocked_board_is_a_loss_for_mover() {
        // 1x2 board: player 0 (vertical) can never move -> immediate loss.
        let g = Domineering::new(2, 1);
        assert!(g.gen().is_empty());
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn ai_plays() {
        let mut g = DomineeringWasm::new(4, 4);
        g.playout_n(800);
        assert!(g.best_move().is_some());
    }
}
