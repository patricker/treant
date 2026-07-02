use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

// --- Oware (traditional Mancala variant) ---
//
// Two players, `pits` houses each, `seeds` seeds per house at the start.
// There are NO stores on the board: captured seeds go straight into a per-player
// score pile. Sowing is counter-clockwise (always +1 mod ring_len).
//
// Ring layout (no store cells): ring index `i` is owned by `i / pits`, and its
// local index within that row is `i % pits`.
//   P0 houses: ring 0 .. pits-1
//   P1 houses: ring pits .. 2*pits-1
//
// Captures happen only in the OPPONENT's houses: if the last sown seed lands in
// an opponent house making it exactly 2 or 3, capture it, then keep capturing
// BACKWARD (reverse sowing direction) through consecutive opponent houses that
// also hold 2 or 3, stopping at the first that doesn't. Grand-slam rule: a move
// that would capture ALL the opponent's seeds captures nothing (the seeds stay).

pub const MAX_PITS: usize = 8;
pub const MAX_RING: usize = 2 * MAX_PITS; // 16
pub const NUM_PLAYERS: usize = 2;

/// Ply cap: Oware endgames (a few seeds circling) can cycle indefinitely, which
/// would let a random MCTS playout loop forever. At the cap we adjudicate by
/// sweeping each side's own row into its own pile and comparing.
pub const PLY_CAP: u32 = 300;

#[derive(Clone, Debug)]
pub struct Oware {
    pub board: [u16; MAX_RING],
    pub score: [u16; NUM_PLAYERS],
    pub pits: usize,
    pub seeds_per: u16,
    pub current: u8,
    ring_len: usize,
    ply: u32,
    finished: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwareMove(pub u8); // local house index (0..pits)

impl std::fmt::Display for OwareMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Oware {
    pub fn new(pits: usize, seeds_per: u16) -> Self {
        assert!((1..=MAX_PITS).contains(&pits));
        let ring_len = 2 * pits;
        let mut board = [0u16; MAX_RING];
        for cell in board.iter_mut().take(ring_len) {
            *cell = seeds_per;
        }
        Self {
            board,
            score: [0; NUM_PLAYERS],
            pits,
            seeds_per,
            current: 0,
            ring_len,
            ply: 0,
            finished: false,
        }
    }

    #[inline]
    pub fn ring_len(&self) -> usize {
        self.ring_len
    }

    #[inline]
    fn owner(&self, ring_idx: usize) -> usize {
        ring_idx / self.pits
    }

    #[inline]
    fn row_start(&self, player: usize) -> usize {
        player * self.pits
    }

    #[inline]
    fn total_seeds(&self) -> u16 {
        (self.ring_len as u16) * self.seeds_per
    }

    /// Sum of seeds currently on `player`'s row.
    fn row_sum(&self, player: usize) -> u16 {
        let start = self.row_start(player);
        self.board[start..start + self.pits].iter().sum()
    }

    #[inline]
    fn step(&self, pos: usize) -> usize {
        let n = pos + 1;
        if n >= self.ring_len {
            0
        } else {
            n
        }
    }

    /// Sow the seeds from local house `local` of the current player. Returns the
    /// ring index of the last seed. Skips the ORIGIN house on a full-lap wrap.
    fn sow(&mut self, local: usize) -> usize {
        let cur = self.current as usize;
        let start = self.row_start(cur) + local;
        let seeds = self.board[start];
        self.board[start] = 0;

        let mut pos = start;
        for _ in 0..seeds {
            pos = self.step(pos);
            if pos == start {
                pos = self.step(pos); // never drop a seed back into the origin
            }
            self.board[pos] += 1;
        }
        pos
    }

    /// The houses that a capture starting at `last` would take, walking backward
    /// through consecutive opponent houses holding exactly 2 or 3.
    fn capture_chain(&self, last: usize, cur: usize) -> Vec<usize> {
        let opp = 1 - cur;
        let mut caps = Vec::new();
        if self.owner(last) != opp {
            return caps;
        }
        let mut pos = last;
        loop {
            if self.owner(pos) != opp {
                break;
            }
            let v = self.board[pos];
            if v == 2 || v == 3 {
                caps.push(pos);
            } else {
                break;
            }
            // Step backward (reverse sowing direction) with wrap.
            pos = if pos == 0 { self.ring_len - 1 } else { pos - 1 };
        }
        caps
    }

    /// Apply the capture rule for a move whose last seed landed at `last`.
    fn apply_capture(&mut self, last: usize, cur: usize) {
        let caps = self.capture_chain(last, cur);
        if caps.is_empty() {
            return;
        }
        let cap_total: u16 = caps.iter().map(|&p| self.board[p]).sum();
        // Grand slam: a move that would take EVERY seed on the opponent's row
        // captures nothing (the seeds stay in place).
        if cap_total >= self.row_sum(1 - cur) {
            return;
        }
        for &p in &caps {
            self.score[cur] += self.board[p];
            self.board[p] = 0;
        }
    }

    /// Does sowing from local house `local` drop at least one seed on the
    /// opponent's row? (Used for the starvation feeding obligation.)
    fn move_feeds_opponent(&self, local: usize) -> bool {
        let cur = self.current as usize;
        let start = self.row_start(cur) + local;
        let seeds = self.board[start];
        let mut pos = start;
        for _ in 0..seeds {
            pos = self.step(pos);
            if pos == start {
                pos = self.step(pos);
            }
            if self.owner(pos) != cur {
                return true;
            }
        }
        false
    }

    /// Legal local-house indices for the current player, honouring the
    /// starvation rule (if the opponent is empty you must feed them if you can).
    fn legal_local(&self) -> Vec<u8> {
        if self.finished {
            return Vec::new();
        }
        let cur = self.current as usize;
        let start = self.row_start(cur);
        let base: Vec<u8> = (0..self.pits)
            .filter(|&j| self.board[start + j] > 0)
            .map(|j| j as u8)
            .collect();
        if base.is_empty() {
            return base;
        }
        if self.row_sum(1 - cur) == 0 {
            let feeding: Vec<u8> = base
                .iter()
                .copied()
                .filter(|&j| self.move_feeds_opponent(j as usize))
                .collect();
            return feeding; // may be empty → current cannot feed → game ends
        }
        base
    }

    /// Sweep every seed into its owner's pile (end-of-game adjudication).
    fn sweep_own_rows(&mut self) {
        for p in 0..NUM_PLAYERS {
            let start = self.row_start(p);
            let mut sum = 0u16;
            for i in start..start + self.pits {
                sum += self.board[i];
                self.board[i] = 0;
            }
            self.score[p] += sum;
        }
    }

    /// After a move (player already switched), decide whether the game has ended
    /// and finalize scores if so.
    fn check_end(&mut self) {
        let total = self.total_seeds();
        // Win by majority: strictly more than half the seeds are locked away.
        if self.score[0] * 2 > total || self.score[1] * 2 > total {
            self.finished = true;
            return;
        }
        if self.ply >= PLY_CAP {
            self.sweep_own_rows();
            self.finished = true;
            return;
        }
        // The player to move has no legal move (own row empty, or a starved
        // opponent they cannot feed): sweep and end.
        if self.legal_local().is_empty() {
            self.sweep_own_rows();
            self.finished = true;
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.finished
    }

    /// Winning seat (0-indexed), or None for a draw / unfinished game.
    pub fn winner(&self) -> Option<u8> {
        if !self.finished {
            return None;
        }
        match self.score[0].cmp(&self.score[1]) {
            std::cmp::Ordering::Greater => Some(0),
            std::cmp::Ordering::Less => Some(1),
            std::cmp::Ordering::Equal => None,
        }
    }

    pub fn score_of(&self, player: u8) -> u16 {
        self.score[player as usize]
    }
}

impl GameState for Oware {
    type Move = OwareMove;
    type Player = u8;
    type MoveList = Vec<OwareMove>;

    fn current_player(&self) -> u8 {
        self.current
    }

    fn available_moves(&self) -> Vec<OwareMove> {
        self.legal_local().into_iter().map(OwareMove).collect()
    }

    fn make_move(&mut self, mov: &OwareMove) {
        let cur = self.current as usize;
        let last = self.sow(mov.0 as usize);
        self.apply_capture(last, cur);
        self.ply += 1;
        self.current = 1 - self.current;
        self.check_end();
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if !self.finished {
            return None;
        }
        // Score comparison from the perspective of the player to move.
        let cur = self.current as usize;
        let me = self.score[cur];
        let them = self.score[1 - cur];
        Some(match me.cmp(&them) {
            std::cmp::Ordering::Greater => ProvenValue::Win,
            std::cmp::Ordering::Less => ProvenValue::Loss,
            std::cmp::Ordering::Equal => ProvenValue::Draw,
        })
    }
}

// --- Evaluator ---

#[derive(Clone, Debug)]
pub struct OwareStateEval {
    pub scores: [i32; NUM_PLAYERS],
}

pub struct OwareEval;

impl OwareEval {
    fn compute(state: &Oware) -> OwareStateEval {
        // Heuristic: captured seeds (×2 weight, locked in) + seeds on own row.
        let mut scores = [0i32; NUM_PLAYERS];
        for (p, slot) in scores.iter_mut().enumerate() {
            *slot = state.score[p] as i32 * 2 + state.row_sum(p) as i32;
        }
        OwareStateEval { scores }
    }
}

impl Evaluator<OwareConfig> for OwareEval {
    type StateEvaluation = OwareStateEval;

    fn evaluate_new_state(
        &self,
        state: &Oware,
        moves: &Vec<OwareMove>,
        _: Option<SearchHandle<OwareConfig>>,
    ) -> (Vec<()>, OwareStateEval) {
        (vec![(); moves.len()], Self::compute(state))
    }

    fn interpret_evaluation_for_player(&self, evaln: &OwareStateEval, player: &u8) -> i64 {
        let p = *player as usize;
        (evaln.scores[p] - evaln.scores[1 - p]) as i64
    }

    fn evaluate_existing_state(
        &self,
        state: &Oware,
        _evaln: &OwareStateEval,
        _: SearchHandle<OwareConfig>,
    ) -> OwareStateEval {
        Self::compute(state)
    }
}

// --- MCTS Config ---

#[derive(Default)]
pub struct OwareConfig;

impl MCTS for OwareConfig {
    type State = Oware;
    type Eval = OwareEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();

    fn solver_enabled(&self) -> bool {
        true
    }
}

// --- WASM API ---

#[wasm_bindgen]
pub struct OwareWasm {
    manager: MCTSManager<OwareConfig>,
    pits: usize,
    seeds_per: u16,
}

impl OwareWasm {
    fn create(pits: usize, seeds_per: u16) -> Self {
        let pits = pits.clamp(4, MAX_PITS);
        let seeds_per = seeds_per.clamp(3, 6);
        Self {
            manager: MCTSManager::new(
                Oware::new(pits, seeds_per),
                OwareConfig,
                OwareEval,
                UCTPolicy::new(1.4),
                (),
            ),
            pits,
            seeds_per,
        }
    }
}

#[wasm_bindgen]
impl OwareWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(pits: u32, seeds: u32) -> Self {
        Self::create(pits as usize, seeds as u16)
    }

    pub fn pits(&self) -> u32 {
        self.pits as u32
    }
    pub fn seeds(&self) -> u32 {
        self.seeds_per as u32
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        let stats = types::build_stats(&self.manager, |_| None);
        serde_wasm_bindgen::to_value(&stats).unwrap_or(JsValue::NULL)
    }

    pub fn get_tree(&self, max_depth: u32) -> JsValue {
        let tree =
            types::export_tree::<OwareConfig>(self.manager.tree().root_node(), max_depth, &|_| None);
        serde_wasm_bindgen::to_value(&tree).unwrap_or(JsValue::NULL)
    }

    /// Compound board string: comma-separated house counts in ring order
    /// (P0 houses, then P1 houses), a `'|'`, then the two capture-pile scores.
    /// e.g. "4,4,4,4,4,4,4,4,4,4,4,4|0,0".
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        let counts = (0..s.ring_len())
            .map(|i| s.board[i].to_string())
            .collect::<Vec<_>>()
            .join(",");
        let scores = (0..NUM_PLAYERS)
            .map(|p| s.score[p].to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!("{counts}|{scores}")
    }

    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current as u32
    }

    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().is_terminal()
    }

    /// Canonical arcade result contract: a bare 1-indexed winner seat digit
    /// ("1" or "2"), "Draw", or "" while the game is in progress.
    pub fn result(&self) -> String {
        let state = self.manager.tree().root_state();
        if !state.is_terminal() {
            return String::new();
        }
        match state.winner() {
            Some(p) => format!("{}", p + 1),
            None => "Draw".into(),
        }
    }

    /// Comma-separated captured-seed scores, one per player.
    pub fn scores(&self) -> String {
        let s = self.manager.tree().root_state();
        (0..NUM_PLAYERS)
            .map(|p| s.score_of(p as u8).to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Comma-separated legal local-house indices for the current player.
    pub fn legal_moves(&self) -> String {
        let s = self.manager.tree().root_state();
        s.available_moves()
            .iter()
            .map(|m| m.0.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }

    pub fn apply_move(&mut self, house_local: &str) -> bool {
        let house: u8 = match house_local.parse() {
            Ok(n) if (n as usize) < self.pits => n,
            _ => return false,
        };
        // Reject a house that isn't legal for the current player up front (empty,
        // or forbidden by the starvation feeding rule) so we don't burn playouts.
        let state = self.manager.tree().root_state();
        if !state.legal_local().contains(&house) {
            return false;
        }
        let m = OwareMove(house);
        if self.manager.advance(&m).is_ok() {
            return true;
        }
        // Legal move that hasn't been expanded yet — give the search a chance to
        // materialize it as a root child, then advance.
        self.manager.playout_n(100);
        self.manager.advance(&m).is_ok()
    }

    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(
            Oware::new(self.pits, self.seeds_per),
            OwareConfig,
            OwareEval,
            UCTPolicy::new(1.4),
            (),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_board() {
        let g = Oware::new(6, 4);
        assert_eq!(g.ring_len(), 12);
        for i in 0..12 {
            assert_eq!(g.board[i], 4);
        }
        assert_eq!(g.score, [0, 0]);
        assert_eq!(g.current, 0);
        assert_eq!(g.total_seeds(), 48);
    }

    #[test]
    fn basic_sow_counterclockwise() {
        let mut g = Oware::new(6, 4);
        // P0 plays house 0 (4 seeds): sow into ring 1,2,3,4.
        g.make_move(&OwareMove(0));
        assert_eq!(g.board[0], 0);
        assert_eq!(g.board[1], 5);
        assert_eq!(g.board[2], 5);
        assert_eq!(g.board[3], 5);
        assert_eq!(g.board[4], 5);
        assert_eq!(g.board[5], 4);
        assert_eq!(g.current, 1); // turn passes to P1
    }

    #[test]
    fn wrap_skips_origin_on_full_lap() {
        let mut g = Oware::new(6, 4);
        g.board = [0; MAX_RING];
        g.board[0] = 12; // a full lap of the 12-house ring
        g.make_move(&OwareMove(0));
        // 11 other houses get 1 seed each; the 12th seed skips the origin and
        // lands back on ring 1, which therefore holds 2.
        assert_eq!(g.board[0], 0); // origin skipped, stays empty
        assert_eq!(g.board[1], 2);
        for i in 2..12 {
            assert_eq!(g.board[i], 1);
        }
    }

    #[test]
    fn capture_single_two() {
        let mut g = Oware::new(6, 4);
        g.board = [0; MAX_RING];
        // P0 house 5 has 1 seed → sows into ring 6 (P1's first house), which had
        // 1, becoming 2 → capture.
        g.board[5] = 1;
        g.board[6] = 1;
        // Keep other P1 houses populated so it's not a grand slam / not terminal.
        g.board[7] = 4;
        g.board[8] = 4;
        g.make_move(&OwareMove(5));
        assert_eq!(g.board[6], 0);
        assert_eq!(g.score[0], 2);
        assert_eq!(g.current, 1);
    }

    #[test]
    fn capture_chain_of_twos_and_threes_stops() {
        let mut g = Oware::new(6, 4);
        g.board = [0; MAX_RING];
        // P0 house 5 has 4 seeds → sows into ring 6,7,8,9 (all P1 houses).
        // Set them so that after sowing: ring9=3, ring8=2, ring7=2 (captured
        // backward), ring6=4 (stops the chain).
        g.board[5] = 4;
        g.board[6] = 3; // becomes 4 after sow → stops chain
        g.board[7] = 1; // becomes 2 → captured
        g.board[8] = 1; // becomes 2 → captured
        g.board[9] = 2; // becomes 3 (last seed) → captured
        // Populate remaining P1 houses so the sweep isn't a grand slam.
        g.board[10] = 4;
        g.board[11] = 4;
        g.make_move(&OwareMove(5));
        assert_eq!(g.board[9], 0); // captured
        assert_eq!(g.board[8], 0); // captured
        assert_eq!(g.board[7], 0); // captured
        assert_eq!(g.board[6], 4); // NOT captured (had 4) — chain stopped here
        assert_eq!(g.score[0], 3 + 2 + 2);
    }

    #[test]
    fn grand_slam_captures_nothing() {
        let mut g = Oware::new(6, 4);
        g.board = [0; MAX_RING];
        // P1's ENTIRE row is exactly what the capture would sweep. P0 house 5
        // sows 1 seed into ring 6 making it 2; ring 6 is P1's only occupied
        // house (all others empty) → capturing it would take all of P1's seeds.
        g.board[5] = 1;
        g.board[6] = 1;
        // P0 keeps seeds so the game isn't otherwise ended.
        g.board[0] = 4;
        g.make_move(&OwareMove(5));
        // Grand slam: no capture; the seed stays.
        assert_eq!(g.board[6], 2);
        assert_eq!(g.score[0], 0);
    }

    #[test]
    fn starvation_must_feed_when_possible() {
        let mut g = Oware::new(6, 4);
        g.board = [0; MAX_RING];
        // P1's row is empty. P0 to move. House 5 (1 seed) feeds P1 (ring 6);
        // house 0 (1 seed) does not reach P1. Only the feeding move is legal.
        g.board[0] = 1;
        g.board[5] = 1;
        assert_eq!(g.current, 0);
        let moves: Vec<u8> = g.available_moves().iter().map(|m| m.0).collect();
        assert_eq!(moves, vec![5]);
    }

    #[test]
    fn starvation_no_feed_ends_game_and_sweeps() {
        // P1 to move with one seed in its first house (ring 6). That seed sows to
        // ring 7 (still P1's row) and does NOT reach P0. After the move it is P0's
        // turn but P0's row is empty → no legal move → game ends, each side sweeps
        // its own row into its own pile.
        let mut g = Oware::new(6, 4);
        g.board = [0; MAX_RING];
        g.current = 1;
        g.board[6] = 1;
        g.make_move(&OwareMove(0)); // P1 plays local house 0 (ring 6)
        assert!(g.is_terminal());
        assert_eq!(g.score[1], 1); // P1's remaining seed swept to P1
        assert_eq!(g.score[0], 0);
    }

    #[test]
    fn no_feeding_move_yields_no_legal_moves() {
        // P1's row empty, P0 has a lone seed in house 0 that only reaches ring 1
        // (still P0) → cannot feed → no legal move.
        let mut g = Oware::new(6, 4);
        g.board = [0; MAX_RING];
        g.board[0] = 1;
        assert!(g.available_moves().is_empty());
    }

    #[test]
    fn terminal_verdict_sign() {
        // Build a finished position where the player to move is behind → Loss.
        let mut g = Oware::new(6, 4);
        g.board = [0; MAX_RING];
        g.score = [20, 5];
        g.finished = true;
        g.current = 1; // P1 to move, P1 is behind
        assert_eq!(g.terminal_value(), Some(ProvenValue::Loss));
        g.current = 0; // P0 to move, P0 is ahead
        assert_eq!(g.terminal_value(), Some(ProvenValue::Win));
        g.score = [12, 12];
        assert_eq!(g.terminal_value(), Some(ProvenValue::Draw));
    }

    #[test]
    fn majority_win_ends_game() {
        let mut g = Oware::new(6, 4); // total 48, need > 24
        g.board = [0; MAX_RING];
        // Arrange a capture that pushes P0 over 24.
        g.score = [23, 0];
        g.board[5] = 1;
        g.board[6] = 1; // capture 2 → P0 to 25 > 24
        g.board[7] = 4; // not a grand slam
        g.make_move(&OwareMove(5));
        assert_eq!(g.score[0], 25);
        assert!(g.is_terminal());
        assert_eq!(g.winner(), Some(0));
    }

    #[test]
    fn ply_cap_adjudicates() {
        let mut g = Oware::new(6, 4);
        g.ply = PLY_CAP - 1;
        // One more move hits the cap → sweep own rows and finish.
        let before: u16 = g.board[0..6].iter().sum();
        g.make_move(&OwareMove(0));
        assert!(g.is_terminal());
        // All 48 seeds are now in the two piles.
        assert_eq!(g.score[0] + g.score[1], 48);
        let _ = before;
    }

    #[test]
    fn ai_plays_a_full_game() {
        let mut mgr = MCTSManager::new(
            Oware::new(6, 4),
            OwareConfig,
            OwareEval,
            UCTPolicy::new(1.4),
            (),
        );
        let mut moves = 0;
        while !mgr.tree().root_state().is_terminal() && moves < 1000 {
            mgr.playout_n(50);
            let m = mgr.best_move().expect("non-terminal state has a best move");
            mgr.advance(&m).expect("best move should be legal");
            moves += 1;
        }
        assert!(mgr.tree().root_state().is_terminal());
    }
}
