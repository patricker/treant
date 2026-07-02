//! Hex — connect your two opposite edges with an unbroken chain. Player 0 (X)
//! links top↔bottom, player 1 (O) links left↔right. Famously can never draw.
//! Win detection + evaluation use a 0-1 BFS "shortest completion" through own
//! (cost 0) and empty (cost 1) cells, with enemy stones as walls.
use crate::gridlib::idx;
use std::collections::VecDeque;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone)]
struct Hex {
    board: Vec<i8>, // -1 empty, 0 = X (top-bottom), 1 = O (left-right)
    n: usize,
    current: u8,
}
impl Hex {
    fn new(n: usize) -> Self {
        Self { board: vec![-1; n * n], n, current: 0 }
    }
    fn neighbors(&self, r: usize, c: usize) -> Vec<(usize, usize)> {
        let n = self.n as i32;
        [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, 1), (1, -1)]
            .iter()
            .filter_map(|(dr, dc)| {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if nr >= 0 && nr < n && nc >= 0 && nc < n {
                    Some((nr as usize, nc as usize))
                } else {
                    None
                }
            })
            .collect()
    }
    /// Min number of empty cells to add to connect `player`'s two edges (0 = the
    /// chain already connects them).
    fn dist(&self, player: u8) -> i32 {
        let n = self.n;
        let pass = |r: usize, c: usize| self.board[idx(r, c, n)] != (1 - player) as i8; // not enemy
        let cost = |r: usize, c: usize| if self.board[idx(r, c, n)] == player as i8 { 0 } else { 1 };
        let mut dist = vec![i32::MAX; n * n];
        let mut dq: VecDeque<(usize, usize)> = VecDeque::new();
        // seed: cells on the player's start edge that are passable
        let starts: Vec<(usize, usize)> = if player == 0 {
            (0..n).map(|c| (0, c)).collect() // top row
        } else {
            (0..n).map(|r| (r, 0)).collect() // left col
        };
        for (r, c) in starts {
            if pass(r, c) {
                let d = cost(r, c);
                if d < dist[idx(r, c, n)] {
                    dist[idx(r, c, n)] = d;
                    if d == 0 {
                        dq.push_front((r, c));
                    } else {
                        dq.push_back((r, c));
                    }
                }
            }
        }
        while let Some((r, c)) = dq.pop_front() {
            let d = dist[idx(r, c, n)];
            for (nr, nc) in self.neighbors(r, c) {
                if !pass(nr, nc) {
                    continue;
                }
                let nd = d + cost(nr, nc);
                if nd < dist[idx(nr, nc, n)] {
                    dist[idx(nr, nc, n)] = nd;
                    if cost(nr, nc) == 0 {
                        dq.push_front((nr, nc));
                    } else {
                        dq.push_back((nr, nc));
                    }
                }
            }
        }
        // answer: min over the player's end edge
        let ends: Vec<(usize, usize)> = if player == 0 {
            (0..n).map(|c| (n - 1, c)).collect()
        } else {
            (0..n).map(|r| (r, n - 1)).collect()
        };
        ends.iter().map(|&(r, c)| dist[idx(r, c, n)]).min().unwrap_or(i32::MAX)
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.dist(0) == 0 {
            Some(if self.current == 0 { ProvenValue::Win } else { ProvenValue::Loss })
        } else if self.dist(1) == 0 {
            Some(if self.current == 1 { ProvenValue::Win } else { ProvenValue::Loss })
        } else {
            None
        }
    }
}
impl GameState for Hex {
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
        (0..self.board.len()).filter(|&i| self.board[i] < 0).map(|i| i as u16).collect()
    }
    fn make_move(&mut self, m: &u16) {
        self.board[*m as usize] = self.current as i8;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct HexEval;
impl Evaluator<HexCfg> for HexEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &Hex, m: &Vec<u16>, _: Option<SearchHandle<HexCfg>>) -> (Vec<()>, i64) {
        // player 0 favored when its completion distance is smaller than player 1's
        (vec![(); m.len()], (s.dist(1) - s.dist(0)) as i64)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &Hex, _: &i64, _: SearchHandle<HexCfg>) -> i64 {
        (s.dist(1) - s.dist(0)) as i64
    }
}
#[derive(Default)]
struct HexCfg;
impl MCTS for HexCfg {
    type State = Hex;
    type Eval = HexEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct HexWasm {
    manager: MCTSManager<HexCfg>,
    n: usize,
}
#[wasm_bindgen]
impl HexWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(n: u32) -> Self {
        let n = (n as usize).clamp(4, 11);
        Self { manager: MCTSManager::new(Hex::new(n), HexCfg, HexEval, UCTPolicy::new(1.4), ()), n }
    }
    pub fn size(&self) -> u32 {
        self.n as u32
    }
    pub fn playout_n(&mut self, k: u32) {
        self.manager.playout_n(k as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .board
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
            Some(ProvenValue::Win) => format!("{}", s.current + 1),
            Some(ProvenValue::Loss) => format!("{}", (1 - s.current) + 1),
            _ => String::new(),
        }
    }
    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m: u16 = match mov.parse() {
            Ok(v) if (v as usize) < self.n * self.n => v,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, HexCfg, HexEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Hex::new(self.n), HexCfg, HexEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_chain_connects_player_0() {
        let mut g = Hex::new(5);
        for r in 0..5 {
            g.board[idx(r, 2, 5)] = 0; // a straight column connects top-bottom for X
        }
        assert_eq!(g.dist(0), 0);
        // simulate it being O's turn to move (X just connected)
        g.current = 1;
        assert_eq!(g.term(), Some(ProvenValue::Loss)); // current O lost
    }

    #[test]
    fn empty_board_not_terminal() {
        let g = Hex::new(5);
        assert!(g.term().is_none());
        assert!(g.dist(0) > 0 && g.dist(1) > 0);
    }

    #[test]
    fn ai_plays_and_no_draw_pressure() {
        let mut g = HexWasm::new(5);
        g.playout_n(300);
        assert!(g.best_move().is_some());
    }
}
