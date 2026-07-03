//! Y — the connection game on a triangular board of hexagonal cells. Two players
//! alternately place a stone on any empty cell; you win by connecting all THREE
//! sides of the triangle with a single connected group. Like Hex, Y can never
//! draw: when the board fills, exactly one player has joined the three sides.
//!
//! The board of side `n` holds `n(n+1)/2` cells `(r, c)` with `0 <= c <= r < n`,
//! stored in a flat triangular enumeration: `idx = r*(r+1)/2 + c`. Hex-adjacency
//! on this lattice makes each cell `(r, c)` neighbour
//! `(r, c-1), (r, c+1), (r-1, c-1), (r-1, c), (r+1, c), (r+1, c+1)`.
//! Win detection is union-find over own stones plus three virtual side nodes
//! (left edge `c==0`, right edge `c==r`, bottom edge `r==n-1`); a player has won
//! when all three virtual nodes share a root.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

/// Flat triangular index of cell `(r, c)`.
#[inline]
fn tri(r: usize, c: usize) -> usize {
    r * (r + 1) / 2 + c
}

#[derive(Clone)]
struct Y {
    board: Vec<i8>, // -1 empty, 0 = X, 1 = O; indexed by tri(r, c)
    n: usize,
    current: u8,
}
impl Y {
    fn new(n: usize) -> Self {
        Self { board: vec![-1; n * (n + 1) / 2], n, current: 0 }
    }
    /// Recover `(row, col)` from a flat triangular index.
    fn rc(&self, i: usize) -> (usize, usize) {
        let mut r = 0usize;
        while (r + 1) * (r + 2) / 2 <= i {
            r += 1;
        }
        (r, i - r * (r + 1) / 2)
    }
    fn neighbors(&self, i: usize) -> Vec<usize> {
        let (r, c) = self.rc(i);
        let n = self.n as i32;
        [(0, -1), (0, 1), (-1, -1), (-1, 0), (1, 0), (1, 1)]
            .iter()
            .filter_map(|(dr, dc)| {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if nr >= 0 && nr < n && nc >= 0 && nc <= nr {
                    Some(tri(nr as usize, nc as usize))
                } else {
                    None
                }
            })
            .collect()
    }
    /// Bitmask of the sides cell `(r, c)` touches: 1 = left, 2 = right, 4 = bottom.
    fn side_mask(&self, r: usize, c: usize) -> u8 {
        let mut m = 0u8;
        if c == 0 {
            m |= 1;
        }
        if c == r {
            m |= 2;
        }
        if r == self.n - 1 {
            m |= 4;
        }
        m
    }
    /// Does `pl` have a *single* connected group touching all three sides?
    ///
    /// Union-find over `pl`'s stones only, accumulating a side bitmask per
    /// component; a win is any one component whose cells collectively touch left,
    /// right and bottom. We deliberately do NOT union stones through shared
    /// virtual side nodes: with three sides that would let two disjoint groups —
    /// say one on {left, bottom} and another on {right, bottom} — bridge through
    /// the common bottom node and report a phantom connection. Requiring a single
    /// component's own mask to reach all three sides is the Y win condition, and
    /// it yields exactly one winner on a full board (verified by fuzzing).
    fn connected(&self, pl: u8) -> bool {
        fn find(parent: &mut [usize], x: usize) -> usize {
            let mut x = x;
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        fn union(parent: &mut [usize], a: usize, b: usize) {
            let ra = find(parent, a);
            let rb = find(parent, b);
            if ra != rb {
                parent[ra] = rb;
            }
        }
        let ncells = self.board.len();
        let mut parent: Vec<usize> = (0..ncells).collect();
        for i in 0..ncells {
            if self.board[i] != pl as i8 {
                continue;
            }
            for nb in self.neighbors(i) {
                if self.board[nb] == pl as i8 {
                    union(&mut parent, i, nb);
                }
            }
        }
        let mut mask = vec![0u8; ncells];
        for i in 0..ncells {
            if self.board[i] != pl as i8 {
                continue;
            }
            let (r, c) = self.rc(i);
            let root = find(&mut parent, i);
            mask[root] |= self.side_mask(r, c);
        }
        (0..ncells).any(|i| self.board[i] == pl as i8 && mask[find(&mut parent, i)] == 0b111)
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.connected(0) {
            Some(if self.current == 0 { ProvenValue::Win } else { ProvenValue::Loss })
        } else if self.connected(1) {
            Some(if self.current == 1 { ProvenValue::Win } else { ProvenValue::Loss })
        } else {
            None
        }
    }
    /// Cells of the winning group: the single connected component of the winner's
    /// stones whose accumulated side-mask touches all three sides. Empty when
    /// nobody has connected. `connected(pl)` guarantees such a component exists;
    /// we recover it by flooding each of the winner's components and returning the
    /// one whose mask reaches `0b111`.
    fn winning_chain(&self) -> Vec<usize> {
        let pl: i8 = if self.connected(0) {
            0
        } else if self.connected(1) {
            1
        } else {
            return Vec::new();
        };
        let ncells = self.board.len();
        let mut seen = vec![false; ncells];
        for start in 0..ncells {
            if self.board[start] != pl || seen[start] {
                continue;
            }
            let mut comp = Vec::new();
            let mut mask = 0u8;
            let mut stack = vec![start];
            seen[start] = true;
            while let Some(i) = stack.pop() {
                comp.push(i);
                let (r, c) = self.rc(i);
                mask |= self.side_mask(r, c);
                for nb in self.neighbors(i) {
                    if self.board[nb] == pl && !seen[nb] {
                        seen[nb] = true;
                        stack.push(nb);
                    }
                }
            }
            if mask == 0b111 {
                comp.sort_unstable();
                return comp;
            }
        }
        Vec::new()
    }
}
impl GameState for Y {
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
struct YEval;
impl Evaluator<YCfg> for YEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Y, m: &Vec<u16>, _: Option<SearchHandle<YCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Y, _: &i64, _: SearchHandle<YCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct YCfg;
impl MCTS for YCfg {
    type State = Y;
    type Eval = YEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct YGameWasm {
    manager: MCTSManager<YCfg>,
    n: usize,
}
#[wasm_bindgen]
impl YGameWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(n: u32) -> Self {
        let n = (n as usize).clamp(5, 12);
        Self { manager: MCTSManager::new(Y::new(n), YCfg, YEval, UCTPolicy::new(1.4), ()), n }
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
    /// `n(n+1)/2` chars in triangular enumeration order (idx = r*(r+1)/2 + c):
    /// ' '=empty, 'X'=player 0, 'O'=player 1.
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

    /// Comma-joined 0-based cell indices (triangular enumeration `r*(r+1)/2 + c`)
    /// of the winning group, or "" when nobody has connected. The UI glows these.
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
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m: u16 = match mov.parse() {
            Ok(v) if (v as usize) < self.n * (self.n + 1) / 2 => v,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, YCfg, YEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Y::new(self.n), YCfg, YEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangular_size_and_index() {
        let g = Y::new(5);
        assert_eq!(g.board.len(), 15); // 5*6/2
        assert_eq!(tri(0, 0), 0);
        assert_eq!(tri(4, 0), 10); // 4*5/2
        assert_eq!(tri(4, 4), 14);
        for i in 0..g.board.len() {
            let (r, c) = g.rc(i);
            assert_eq!(tri(r, c), i); // rc is the inverse of tri
            assert!(c <= r && r < 5);
        }
    }

    #[test]
    fn adjacency_matches_spec() {
        // Interior cell (2,1) on a side-4 board has all six neighbours.
        let g = Y::new(4);
        let mut got = g.neighbors(tri(2, 1));
        got.sort_unstable();
        let mut want: Vec<usize> = [(2, 0), (2, 2), (1, 0), (1, 1), (3, 1), (3, 2)]
            .iter()
            .map(|&(r, c)| tri(r, c))
            .collect();
        want.sort_unstable();
        assert_eq!(got, want);
        // The apex (0,0) only reaches down: (1,0) and (1,1).
        let mut apex = g.neighbors(tri(0, 0));
        apex.sort_unstable();
        assert_eq!(apex, vec![tri(1, 0), tri(1, 1)]);
    }

    #[test]
    fn corners_belong_to_two_sides_each() {
        let g = Y::new(6);
        assert_eq!(g.side_mask(0, 0), 1 | 2); // apex: left + right
        assert_eq!(g.side_mask(5, 0), 1 | 4); // bottom-left: left + bottom
        assert_eq!(g.side_mask(5, 5), 2 | 4); // bottom-right: right + bottom
        // A pure edge cell (not a corner) touches exactly one side.
        assert_eq!(g.side_mask(3, 0), 1); // left edge only
        assert_eq!(g.side_mask(3, 3), 2); // right edge only
        assert_eq!(g.side_mask(5, 2), 4); // bottom edge only
        // An interior cell touches no side.
        assert_eq!(g.side_mask(3, 1), 0);
    }

    #[test]
    fn partial_edge_touching_two_sides_does_not_win() {
        // Left column but skipping the apex (0,0): cells (1..5, 0). This touches
        // the left side and — via the bottom-left corner (4,0) — the bottom side,
        // but never the right side, so it is not a connection.
        let mut g = Y::new(5);
        for r in 1..5 {
            g.board[tri(r, 0)] = 0;
        }
        assert!(!g.connected(0));
        assert!(g.term().is_none());
    }

    #[test]
    fn a_full_edge_wins_via_its_corners() {
        // A complete edge is a Y connection: its two endpoints are corners that
        // each lie on one of the other two sides. The full left edge runs from the
        // apex (on the right side) to the bottom-left corner (on the bottom side).
        let mut g = Y::new(5);
        for r in 0..5 {
            g.board[tri(r, 0)] = 0;
        }
        assert!(g.connected(0));
    }

    #[test]
    fn single_group_touching_all_three_sides_wins() {
        // The whole left edge + the bottom edge form one connected group that
        // touches all three sides (left, bottom, and the bottom-right corner is
        // on the right side). This is a genuine Y connection.
        let mut g = Y::new(4);
        for r in 0..4 {
            g.board[tri(r, 0)] = 0; // left edge
        }
        for c in 0..4 {
            g.board[tri(3, c)] = 0; // bottom edge (shares (3,0) with the left edge)
        }
        assert!(g.connected(0));
        // Player 0 just connected; it's player 1 (current) to move, so they lost.
        g.current = 1;
        assert_eq!(g.term(), Some(ProvenValue::Loss));
        // And from player 0's own perspective it reads as a win.
        g.current = 0;
        assert_eq!(g.term(), Some(ProvenValue::Win));
    }

    #[test]
    fn two_sides_without_the_third_does_not_win() {
        // The right edge from the apex down, stopping short of the bottom-right
        // corner: touches left (apex) and right, but never the bottom side.
        let mut g = Y::new(5);
        for r in 0..4 {
            g.board[tri(r, r)] = 1; // (0,0),(1,1),(2,2),(3,3) — no (4,4)
        }
        assert!(!g.connected(1));
    }

    #[test]
    fn two_groups_sharing_a_side_do_not_falsely_connect() {
        // Regression: one group on {left, bottom}, a separate group on
        // {right, bottom}. A naive union-find that routed both through a shared
        // "bottom" virtual node would report a phantom win; a real Y connection
        // needs a single group to reach all three sides.
        let mut g = Y::new(3);
        // group A: (1,0) left, (2,0) left+bottom
        g.board[tri(1, 0)] = 0;
        g.board[tri(2, 0)] = 0;
        // group B (not adjacent to A): (2,2) right+bottom
        g.board[tri(2, 2)] = 0;
        assert!(!g.connected(0));
    }

    #[test]
    fn no_draw_when_full() {
        // Randomly fill several side-5 boards to completion; a full Y board must
        // always have exactly one connected winner (Y can never draw).
        let mut state = 0x9e3779b9u32;
        let mut rng = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        for _ in 0..50 {
            let mut g = Y::new(5);
            let mut cells: Vec<usize> = (0..g.board.len()).collect();
            // Fisher-Yates shuffle, then alternate colours.
            for i in (1..cells.len()).rev() {
                let j = (rng() as usize) % (i + 1);
                cells.swap(i, j);
            }
            for (turn, &cell) in cells.iter().enumerate() {
                g.board[cell] = (turn % 2) as i8;
            }
            // A full board: exactly one player connected all three sides.
            assert!(g.connected(0) ^ g.connected(1), "a full Y board must have exactly one winner");
        }
    }

    #[test]
    fn ai_plays() {
        let mut g = YGameWasm::new(5);
        g.playout_n(400);
        assert!(g.best_move().is_some());
    }

    #[test]
    fn winning_chain_is_the_group_touching_all_three_sides() {
        // The left edge + the bottom edge form one group touching all three sides
        // (the bottom-right corner (5,5) is on the right side).
        let mut g = Y::new(6);
        let mut want: Vec<usize> = Vec::new();
        for r in 0..6 {
            g.board[tri(r, 0)] = 0; // left edge
            want.push(tri(r, 0));
        }
        for c in 0..6 {
            g.board[tri(5, c)] = 0; // bottom edge
            want.push(tri(5, c));
        }
        // A stray player-0 stone in the interior, not adjacent to the winning
        // group — it must NOT be reported. (3,2)'s neighbours are all off both
        // the left column and the bottom row.
        g.board[tri(3, 2)] = 0;
        assert!(g.connected(0));
        want.sort_unstable();
        want.dedup();
        let got = g.winning_chain();
        assert_eq!(got, want);
        // Every returned cell is the winner's stone…
        assert!(got.iter().all(|&i| g.board[i] == 0));
        // …and the group collectively touches all three sides.
        let mask = got.iter().fold(0u8, |m, &i| {
            let (r, c) = g.rc(i);
            m | g.side_mask(r, c)
        });
        assert_eq!(mask, 0b111);
    }

    #[test]
    fn winning_cells_empty_mid_game_and_for_non_winner() {
        let mut g = YGameWasm::new(5);
        assert_eq!(g.winning_cells(), "", "empty board has no group");
        assert!(g.apply_move("0")); // a lone stone connects nothing
        assert_eq!(g.winning_cells(), "");
    }
}
