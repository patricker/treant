//! Pinch-Five (public-domain Ninuki-renju / the game sold as Pente). Place one
//! stone per turn on an empty point. Win by making five-in-a-row **or** by
//! capturing a target number of enemy PAIRS. A capture happens when your placed
//! stone brackets EXACTLY two adjacent enemy stones against another of your
//! stones (`X O O X`) along any of the 8 lines — the pair is removed and counts
//! as one captured pair. Placing INTO such a bracket is safe: captures only ever
//! trigger for the player who just moved.
//!
//! Because captures re-open cells, a random rollout could in principle run for a
//! very long time, so the engine caps total plies (`cols*rows*4`) and adjudicates
//! any game reaching the cap as a Draw — this guarantees every playout terminates.
use crate::gridlib::{self, idx};
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

/// Fixed win length (five-in-a-row). A knob would complicate capture balance, so
/// only board size, pairs-to-win and player count are tunable.
const WIN_LEN: usize = 5;
/// This engine supports 2–4 seats; symbols map seat 0..=3 → 'X','O','A','B'.
const MAX_PLAYERS: u8 = 4;

/// The 8 line directions used for both capture detection and runs.
const DIRS8: [(i32, i32); 8] =
    [(0, 1), (0, -1), (1, 0), (-1, 0), (1, 1), (1, -1), (-1, 1), (-1, -1)];

fn symbol(v: i8) -> char {
    match v {
        0 => 'X',
        1 => 'O',
        2 => 'A',
        3 => 'B',
        _ => ' ',
    }
}

#[derive(Clone)]
struct PinchFive {
    board: Vec<i8>, // -1 empty, else seat index
    cols: usize,
    rows: usize,
    current: u8,
    num_players: u8,
    captures: Vec<u16>, // captured PAIRS per seat
    pairs_to_win: u16,
    winner: Option<u8>,
    plies: u32,
    ply_cap: u32,
}

impl PinchFive {
    fn new(cols: usize, rows: usize, pairs_to_win: u16, num_players: u8) -> Self {
        Self {
            board: vec![-1; cols * rows],
            cols,
            rows,
            current: 0,
            num_players,
            captures: vec![0; num_players as usize],
            pairs_to_win,
            winner: None,
            plies: 0,
            ply_cap: (cols * rows * 4) as u32,
        }
    }

    /// Cell `k` steps from `(r, c)` along `(dr, dc)`, if it stays on the board.
    fn at(&self, r: usize, c: usize, dr: i32, dc: i32, k: i32) -> Option<usize> {
        let nr = r as i32 + dr * k;
        let nc = c as i32 + dc * k;
        if nr >= 0 && nr < self.rows as i32 && nc >= 0 && nc < self.cols as i32 {
            Some(idx(nr as usize, nc as usize, self.cols))
        } else {
            None
        }
    }

    fn term(&self) -> Option<ProvenValue> {
        if let Some(w) = self.winner {
            // The winner completed the win on the previous move, so the player to
            // move now is a non-winner and sees a Loss (Win kept for generality).
            return Some(if w == self.current { ProvenValue::Win } else { ProvenValue::Loss });
        }
        if self.plies >= self.ply_cap || gridlib::is_full(&self.board) {
            return Some(ProvenValue::Draw);
        }
        None
    }

    // Per-seat evaluation snapshot: captured pairs + line-progress toward five.
    fn eval_state(&self) -> PfStateEval {
        let mut cap = Vec::with_capacity(self.num_players as usize);
        let mut lines = Vec::with_capacity(self.num_players as usize);
        for p in 0..self.num_players {
            cap.push(self.captures[p as usize] as i64);
            lines.push(gridlib::window_eval(&self.board, self.cols, self.rows, p as i8, WIN_LEN));
        }
        PfStateEval { cap, lines }
    }
}

impl GameState for PinchFive {
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
        (0..self.board.len()).filter(|&i| self.board[i] == -1).map(|i| i as u16).collect()
    }

    fn make_move(&mut self, m: &u16) {
        let cell = *m as usize;
        let me = self.current as i8;
        self.board[cell] = me;
        let (r, c) = (cell / self.cols, cell % self.cols);

        // Resolve captures: X O O X (placed X, two same-enemy, our stone beyond).
        for (dr, dc) in DIRS8 {
            if let (Some(i1), Some(i2), Some(i3)) =
                (self.at(r, c, dr, dc, 1), self.at(r, c, dr, dc, 2), self.at(r, c, dr, dc, 3))
            {
                let (v1, v2, v3) = (self.board[i1], self.board[i2], self.board[i3]);
                if v1 >= 0 && v1 != me && v1 == v2 && v3 == me {
                    self.board[i1] = -1;
                    self.board[i2] = -1;
                    self.captures[self.current as usize] += 1;
                }
            }
        }

        let capture_win = self.captures[self.current as usize] >= self.pairs_to_win;
        let line_win = gridlib::max_run(&self.board, self.cols, self.rows, r, c, me) >= WIN_LEN;
        if capture_win || line_win {
            self.winner = Some(self.current);
        }

        self.plies += 1;
        self.current = (self.current + 1) % self.num_players;
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}

struct PfEval;
#[derive(Clone)]
struct PfStateEval {
    cap: Vec<i64>,
    lines: Vec<i64>,
}
impl Evaluator<PfCfg> for PfEval {
    type StateEvaluation = PfStateEval;
    fn evaluate_new_state(&self, s: &PinchFive, m: &Vec<u16>, _: Option<SearchHandle<PfCfg>>) -> (Vec<()>, PfStateEval) {
        (vec![(); m.len()], s.eval_state())
    }
    fn interpret_evaluation_for_player(&self, e: &PfStateEval, p: &u8) -> i64 {
        let p = *p as usize;
        let score = |i: usize| e.cap[i] * 100 + e.lines[i];
        let mine = score(p);
        let best_other = (0..e.cap.len()).filter(|&i| i != p).map(score).max().unwrap_or(0);
        mine - best_other
    }
    fn evaluate_existing_state(&self, s: &PinchFive, _: &PfStateEval, _: SearchHandle<PfCfg>) -> PfStateEval {
        s.eval_state()
    }
}

#[derive(Default)]
struct PfCfg;
impl MCTS for PfCfg {
    type State = PinchFive;
    type Eval = PfEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

fn new_manager(cols: usize, rows: usize, pairs: u16, players: u8) -> MCTSManager<PfCfg> {
    MCTSManager::new(PinchFive::new(cols, rows, pairs, players), PfCfg, PfEval, UCTPolicy::new(1.4), ())
}

#[wasm_bindgen]
pub struct PinchFiveWasm {
    manager: MCTSManager<PfCfg>,
    cols: usize,
    rows: usize,
    pairs_to_win: u16,
    num_players: u8,
}

#[wasm_bindgen]
impl PinchFiveWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32, pairs_to_win: u32, num_players: u32) -> Self {
        let cols = (cols as usize).clamp(9, 19);
        let rows = (rows as usize).clamp(9, 19);
        let pairs_to_win = (pairs_to_win as u16).clamp(3, 8);
        let num_players = (num_players as u8).clamp(2, MAX_PLAYERS);
        Self {
            manager: new_manager(cols, rows, pairs_to_win, num_players),
            cols,
            rows,
            pairs_to_win,
            num_players,
        }
    }

    pub fn cols(&self) -> u32 {
        self.cols as u32
    }
    pub fn rows(&self) -> u32 {
        self.rows as u32
    }
    pub fn num_players(&self) -> u32 {
        self.num_players as u32
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }

    /// "<cells>|<pairs>": one char per cell (' ' empty, 'X'/'O'/'A'/'B'), then a
    /// comma-joined per-seat captured-pair count. The UI Board parses both halves.
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        let cells: String = s.board.iter().map(|&v| symbol(v)).collect();
        format!("{cells}|{}", self.capture_counts())
    }

    /// Comma-joined captured-pair count per seat, e.g. "2,0".
    pub fn capture_counts(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .captures
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current as u32
    }

    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().term().is_some()
    }

    pub fn result(&self) -> String {
        let s = self.manager.tree().root_state();
        if let Some(w) = s.winner {
            format!("{}", w + 1)
        } else if s.term() == Some(ProvenValue::Draw) {
            "Draw".into()
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
            Ok(v) if (v as usize) < self.cols * self.rows => v,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, PfCfg, PfEval, UCTPolicy::new(1.4), ());
        true
    }

    pub fn reset(&mut self) {
        self.manager = new_manager(self.cols, self.rows, self.pairs_to_win, self.num_players);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_capture_scores_a_pair() {
        // X O O _ ; X plays the far end -> the O pair is captured and scored.
        let mut g = PinchFive::new(9, 9, 5, 2);
        g.board[idx(2, 0, 9)] = 0; // X
        g.board[idx(2, 1, 9)] = 1; // O
        g.board[idx(2, 2, 9)] = 1; // O
        g.current = 0; // X to move
        g.make_move(&(idx(2, 3, 9) as u16));
        assert_eq!(g.board[idx(2, 1, 9)], -1, "O pair removed");
        assert_eq!(g.board[idx(2, 2, 9)], -1);
        assert_eq!(g.captures[0], 1, "X scored one pair");
    }

    #[test]
    fn placing_into_a_bracket_is_safe() {
        // X _ _ X with one O already inside; O completing the pair is NOT captured
        // (captures only trigger for the placer).
        let mut g = PinchFive::new(9, 9, 5, 2);
        g.board[idx(2, 0, 9)] = 0; // X
        g.board[idx(2, 3, 9)] = 0; // X
        g.board[idx(2, 1, 9)] = 1; // O
        g.current = 1; // O to move
        g.make_move(&(idx(2, 2, 9) as u16));
        assert_eq!(g.board[idx(2, 1, 9)], 1, "self-suicide bracket is safe");
        assert_eq!(g.board[idx(2, 2, 9)], 1);
        assert_eq!(g.captures[0], 0);
    }

    #[test]
    fn exactly_three_enemy_stones_are_not_captured() {
        // X O O O X: bracketing three enemy stones does NOT capture (needs exactly two).
        let mut g = PinchFive::new(9, 9, 5, 2);
        g.board[idx(4, 0, 9)] = 0;
        g.board[idx(4, 1, 9)] = 1;
        g.board[idx(4, 2, 9)] = 1;
        g.board[idx(4, 3, 9)] = 1;
        g.current = 0;
        g.make_move(&(idx(4, 4, 9) as u16));
        assert_eq!(g.captures[0], 0, "three-in-a-line is immune");
        assert_eq!(g.board[idx(4, 1, 9)], 1);
    }

    #[test]
    fn five_in_a_row_is_a_loss_for_the_mover() {
        let mut g = PinchFive::new(9, 9, 5, 2);
        for c in 0..4 {
            g.board[idx(0, c, 9)] = 0; // four X in a row
        }
        g.current = 0;
        g.make_move(&(idx(0, 4, 9) as u16)); // completes five
        assert_eq!(g.winner, Some(0));
        // current is now O (the player to move), who sees a Loss.
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn reaching_pair_target_wins() {
        let mut g = PinchFive::new(9, 9, 3, 2); // 3 pairs to win
        g.captures[0] = 2;
        g.board[idx(6, 0, 9)] = 0;
        g.board[idx(6, 1, 9)] = 1;
        g.board[idx(6, 2, 9)] = 1;
        g.current = 0;
        g.make_move(&(idx(6, 3, 9) as u16)); // third pair
        assert_eq!(g.captures[0], 3);
        assert_eq!(g.winner, Some(0));
    }

    #[test]
    fn ply_cap_forces_draw() {
        let mut g = PinchFive::new(9, 9, 5, 2);
        g.plies = g.ply_cap;
        assert_eq!(g.term(), Some(ProvenValue::Draw));
    }

    #[test]
    fn ai_plays() {
        let mut g = PinchFiveWasm::new(9, 9, 5, 2);
        g.playout_n(200);
        assert!(g.best_move().is_some());
        // board string carries the pair counts after a '|'.
        assert!(g.get_board().contains('|'));
        assert_eq!(g.capture_counts(), "0,0");
    }
}
