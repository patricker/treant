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

/// Move sentinel for the pie-rule swap. Out of range of every real cell index
/// (0..n*n, max 120 for the 11×11 board), so it never collides with a placement.
const SWAP: u16 = u16::MAX;

/// Render an engine move as the string the UI / apply_move speak: real cells are
/// their numeric index, the swap sentinel is the literal `"swap"`.
fn fmt_move(m: u16) -> String {
    if m == SWAP {
        "swap".to_string()
    } else {
        format!("{m}")
    }
}

#[derive(Clone)]
struct Hex {
    board: Vec<i8>, // -1 empty, 0 = X (top-bottom), 1 = O (left-right)
    n: usize,
    current: u8,
    /// Pie (swap) rule enabled: the second player may answer the opening stone
    /// with a single `swap` instead of placing.
    pie: bool,
    /// Number of half-moves played so far (used to offer `swap` only at ply 1).
    ply: u32,
}
impl Hex {
    fn new(n: usize) -> Self {
        Self { board: vec![-1; n * n], n, current: 0, pie: false, ply: 0 }
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
    /// Cells of the winning chain: the single connected component of the winner's
    /// stones that touches both of their goal edges (player 0: top↔bottom, player
    /// 1: left↔right). Empty when nobody has connected yet. `dist(pl) == 0`
    /// guarantees such a component exists; we recover it by flooding each of the
    /// winner's components and returning the one that reaches both edges.
    fn winning_chain(&self) -> Vec<usize> {
        let n = self.n;
        let pl: i8 = if self.dist(0) == 0 {
            0
        } else if self.dist(1) == 0 {
            1
        } else {
            return Vec::new();
        };
        let mut seen = vec![false; n * n];
        for start in 0..n * n {
            if self.board[start] != pl || seen[start] {
                continue;
            }
            let mut comp = Vec::new();
            let mut touch_lo = false; // player 0: top row; player 1: left col
            let mut touch_hi = false; // player 0: bottom row; player 1: right col
            let mut stack = vec![start];
            seen[start] = true;
            while let Some(i) = stack.pop() {
                comp.push(i);
                let (r, c) = (i / n, i % n);
                let (lo, hi) = if pl == 0 { (r == 0, r == n - 1) } else { (c == 0, c == n - 1) };
                touch_lo |= lo;
                touch_hi |= hi;
                for (nr, nc) in self.neighbors(r, c) {
                    let j = idx(nr, nc, n);
                    if self.board[j] == pl && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
            if touch_lo && touch_hi {
                comp.sort_unstable();
                return comp;
            }
        }
        Vec::new()
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
        let mut moves: Vec<u16> =
            (0..self.board.len()).filter(|&i| self.board[i] < 0).map(|i| i as u16).collect();
        // Pie rule: the second player's reply may be a swap instead of a placement.
        if self.pie && self.ply == 1 {
            moves.push(SWAP);
        }
        moves
    }
    fn make_move(&mut self, m: &u16) {
        if *m == SWAP {
            // Reflect the single existing stone across the long diagonal (r,c)→(c,r)
            // and recolour it to the swapping player. The post-swap position is a
            // normal one-stone position — solver / win detection need no special case.
            if let Some(i) = self.board.iter().position(|&v| v >= 0) {
                let (r, c) = (i / self.n, i % self.n);
                self.board[i] = -1;
                self.board[idx(c, r, self.n)] = self.current as i8;
            }
        } else {
            self.board[*m as usize] = self.current as i8;
        }
        self.current = 1 - self.current;
        self.ply += 1;
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
    pie: bool,
}
#[wasm_bindgen]
impl HexWasm {
    /// `pie != 0` enables the pie (swap) rule: after the opening stone the
    /// second player's legal moves include the literal move `"swap"`.
    #[wasm_bindgen(constructor)]
    pub fn new(n: u32, pie: u32) -> Self {
        let n = (n as usize).clamp(4, 11);
        let pie = pie != 0;
        let mut st = Hex::new(n);
        st.pie = pie;
        Self { manager: MCTSManager::new(st, HexCfg, HexEval, UCTPolicy::new(1.4), ()), n, pie }
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
        self.manager.best_move().map(fmt_move)
    }

    /// Comma-joined legal move strings for the current player. Cell indices are
    /// row-major (`r*n + c`); the pie-rule swap surfaces as the literal `"swap"`.
    pub fn legal_moves(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .available_moves()
            .iter()
            .map(|&m| fmt_move(m))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Comma-joined 0-based cell indices (row-major `r*n + c`) of the winning
    /// chain, or "" when nobody has connected. The UI glows these cells.
    pub fn winning_cells(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .winning_chain()
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        // pick_weak formats moves via Display, so the swap sentinel comes back as
        // its raw number — remap it to the "swap" token the UI/apply_move expect.
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
            .map(|s| if s == format!("{SWAP}") { "swap".to_string() } else { s })
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m: u16 = if mov == "swap" {
            SWAP
        } else {
            match mov.parse::<u16>() {
                Ok(v) if v == SWAP => SWAP,
                Ok(v) if (v as usize) < self.n * self.n => v,
                _ => return false,
            }
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
        let mut st = Hex::new(self.n);
        st.pie = self.pie;
        self.manager = MCTSManager::new(st, HexCfg, HexEval, UCTPolicy::new(1.4), ());
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
        let mut g = HexWasm::new(5, 0);
        g.playout_n(300);
        assert!(g.best_move().is_some());
    }

    #[test]
    fn winning_chain_is_the_component_touching_both_edges() {
        // Player 0 (top↔bottom) runs a straight column down col 2 on a 5×5 board.
        let mut g = Hex::new(5);
        let chain: Vec<usize> = (0..5).map(|r| idx(r, 2, 5)).collect();
        for &i in &chain {
            g.board[i] = 0;
        }
        // Add a stray, disconnected player-0 stone that must NOT be reported.
        g.board[idx(0, 0, 5)] = 0;
        assert_eq!(g.dist(0), 0);
        let mut got = g.winning_chain();
        got.sort_unstable();
        let mut want = chain.clone();
        want.sort_unstable();
        assert_eq!(got, want);
        // Every returned cell is the winner's stone…
        assert!(got.iter().all(|&i| g.board[i] == 0));
        // …and the set touches both the top row and the bottom row.
        assert!(got.iter().any(|&i| i / 5 == 0));
        assert!(got.iter().any(|&i| i / 5 == 4));
    }

    #[test]
    fn winning_chain_for_player_1_spans_left_to_right() {
        // Player 1 (left↔right) runs a straight row across row 2.
        let mut g = Hex::new(5);
        let chain: Vec<usize> = (0..5).map(|c| idx(2, c, 5)).collect();
        for &i in &chain {
            g.board[i] = 1;
        }
        assert_eq!(g.dist(1), 0);
        let mut got = g.winning_chain();
        got.sort_unstable();
        let mut want = chain.clone();
        want.sort_unstable();
        assert_eq!(got, want);
        assert!(got.iter().all(|&i| g.board[i] == 1));
        assert!(got.iter().any(|&i| i % 5 == 0)); // left col
        assert!(got.iter().any(|&i| i % 5 == 4)); // right col
    }

    #[test]
    fn pie_swap_mirrors_and_recolors_the_first_stone() {
        let mut g = HexWasm::new(7, 1);
        assert!(g.apply_move("9")); // P0: (r1,c2)
        assert!(g.legal_moves().split(',').any(|m| m == "swap"));
        assert!(g.apply_move("swap"));
        let b = g.get_board();
        assert_eq!(b.chars().nth(9), Some(' ')); // original cell cleared
        assert_eq!(b.chars().nth(15), Some('O')); // mirrored (r2,c1) is now P1's
        assert_eq!(g.current_player(), 0);
    }

    #[test]
    fn decline_swap_plays_normally() {
        let mut g = HexWasm::new(5, 1);
        assert!(g.apply_move("12")); // P0 opens
        assert!(g.legal_moves().split(',').any(|m| m == "swap"));
        assert!(g.apply_move("6")); // P1 declines: places a normal stone
        // Ply 2 now: swap is no longer offered and the game is a normal position.
        assert!(!g.legal_moves().split(',').any(|m| m == "swap"));
        assert_eq!(g.current_player(), 0);
        assert!(!g.is_terminal());
    }

    #[test]
    fn no_swap_when_pie_disabled() {
        let mut g = HexWasm::new(5, 0);
        assert!(g.apply_move("12"));
        assert!(!g.legal_moves().split(',').any(|m| m == "swap"));
        assert!(!g.apply_move("swap"));
    }

    #[test]
    fn swapped_stone_can_complete_a_win() {
        // pie board: P0 opens at (1,2); P1 swaps -> O appears at the mirror (2,1).
        let mut g = Hex::new(5);
        g.pie = true;
        g.make_move(&7); // P0 (X) at (1,2)
        assert_eq!(g.board[7], 0);
        g.make_move(&SWAP); // P1 swap -> clears (1,2), sets (2,1) to O
        assert_eq!(g.board[7], -1);
        assert_eq!(g.board[idx(2, 1, 5)], 1);
        assert_eq!(g.current, 0);
        // The swapped stone completes a left-right chain along row 2 for O.
        for c in [0usize, 2, 3, 4] {
            g.board[idx(2, c, 5)] = 1;
        }
        assert_eq!(g.dist(1), 0);
        let cells = g.winning_chain();
        assert!(cells.contains(&idx(2, 1, 5))); // swapped stone is in the winning chain
        assert_eq!(cells.len(), 5);
    }

    #[test]
    fn full_game_after_swap_terminates() {
        let mut g = HexWasm::new(5, 1);
        assert!(g.apply_move("7"));
        assert!(g.apply_move("swap"));
        for _ in 0..80 {
            if g.is_terminal() {
                break;
            }
            let m = g.weak_move(40, 3, 0.5, 7).or_else(|| g.best_move());
            match m {
                Some(mv) => assert!(g.apply_move(&mv), "engine emitted an unplayable move: {mv}"),
                None => break,
            }
        }
        assert!(g.is_terminal());
        assert!(!g.result().is_empty());
    }

    #[test]
    fn winning_cells_empty_mid_game() {
        let mut g = HexWasm::new(5, 0);
        assert_eq!(g.winning_cells(), "", "empty board has no chain");
        assert!(g.apply_move("12")); // one stone, no connection
        assert_eq!(g.winning_cells(), "");
    }
}
