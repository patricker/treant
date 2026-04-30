use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

// --- Generalized Mancala (Kalah): configurable pits, stones, N players ∈ {2, 4} ---
//
// Ring layout: each player owns `pits` consecutive pits followed by a store.
// Player p's pits occupy ring indices p*(pits+1) .. p*(pits+1)+pits-1.
// Player p's store is at ring index p*(pits+1) + pits.
// Sowing direction is counterclockwise (always +1 mod ring_len).

pub const MAX_PITS: usize = 8;
pub const MAX_PLAYERS: usize = 4;
pub const MAX_RING: usize = MAX_PLAYERS * (MAX_PITS + 1); // 4 * 9 = 36

#[derive(Clone, Debug)]
pub struct Mancala {
    pub board: [u8; MAX_RING],
    pub pits: usize,
    pub num_players: usize,
    pub current: u8,
    ring_len: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MancalaMove(pub u8); // local pit index (0..pits)

impl std::fmt::Display for MancalaMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Mancala {
    pub fn new(pits: usize, stones: u8, num_players: usize) -> Self {
        assert!((1..=MAX_PITS).contains(&pits));
        assert!(num_players == 2 || num_players == 4);
        let ring_len = num_players * (pits + 1);
        let mut board = [0u8; MAX_RING];
        for player in 0..num_players {
            for j in 0..pits {
                board[player * (pits + 1) + j] = stones;
            }
        }
        Self {
            board,
            pits,
            num_players,
            current: 0,
            ring_len,
        }
    }

    #[inline]
    pub fn ring_len(&self) -> usize {
        self.ring_len
    }

    #[inline]
    pub fn store_index(&self, player: usize) -> usize {
        player * (self.pits + 1) + self.pits
    }

    #[inline]
    pub fn player_pit_start(&self, player: usize) -> usize {
        player * (self.pits + 1)
    }

    #[inline]
    fn pit_owner(&self, ring_idx: usize) -> usize {
        ring_idx / (self.pits + 1)
    }

    #[inline]
    fn local_pit_index(&self, ring_idx: usize) -> usize {
        ring_idx % (self.pits + 1)
    }

    #[inline]
    fn is_store(&self, ring_idx: usize) -> bool {
        self.local_pit_index(ring_idx) == self.pits
    }

    /// Geometrically opposite pit, used for the capture rule.
    /// 2-player: P0[j] ↔ P1[P-1-j] (mirrored across the horizontal layout).
    /// 4-player: P0[j] ↔ P2[j], P1[j] ↔ P3[j] (square layout, across-the-center).
    fn opposite_pit(&self, ring_idx: usize) -> Option<usize> {
        if self.is_store(ring_idx) {
            return None;
        }
        let owner = self.pit_owner(ring_idx);
        let local = self.local_pit_index(ring_idx);
        match self.num_players {
            2 => {
                let opp_owner = 1 - owner;
                let opp_local = self.pits - 1 - local;
                Some(opp_owner * (self.pits + 1) + opp_local)
            }
            4 => {
                let opp_owner = (owner + 2) % 4;
                Some(opp_owner * (self.pits + 1) + local)
            }
            _ => None,
        }
    }

    /// Sow stones from local pit `pit_local` for the current player.
    /// Returns the ring index where the last stone landed.
    fn sow(&mut self, pit_local: usize) -> usize {
        let cur = self.current as usize;
        let span = self.pits + 1;
        let start_ring = cur * span + pit_local;
        let stones = self.board[start_ring];
        self.board[start_ring] = 0;
        let ring_len = self.ring_len;

        // Pre-compute the (≤ 3) opponent-store ring indices to skip while sowing.
        // Avoids per-iteration modulo just to detect "is this an opponent store".
        let mut skips = [usize::MAX; MAX_PLAYERS - 1];
        let mut nskip = 0;
        for p in 0..self.num_players {
            if p != cur {
                skips[nskip] = p * span + self.pits;
                nskip += 1;
            }
        }

        let mut pos = start_ring;
        for _ in 0..stones {
            pos += 1;
            if pos >= ring_len {
                pos -= ring_len;
            }
            for &skip in &skips[..nskip] {
                if pos == skip {
                    pos += 1;
                    if pos >= ring_len {
                        pos -= ring_len;
                    }
                    break;
                }
            }
            self.board[pos] += 1;
        }
        pos
    }

    /// Standard Kalah capture: last stone landed in own previously-empty pit
    /// → take that stone plus the opposite pit's contents into own store.
    fn maybe_capture(&mut self, last_pos: usize) {
        let cur = self.current as usize;
        if self.is_store(last_pos) {
            return;
        }
        if self.pit_owner(last_pos) != cur {
            return;
        }
        if self.board[last_pos] != 1 {
            return;
        }
        if let Some(opp) = self.opposite_pit(last_pos) {
            if self.board[opp] > 0 {
                let store = self.store_index(cur);
                self.board[store] += self.board[opp] + 1;
                self.board[opp] = 0;
                self.board[last_pos] = 0;
            }
        }
    }

    fn any_row_empty(&self) -> bool {
        for player in 0..self.num_players {
            let start = self.player_pit_start(player);
            let end = start + self.pits;
            if self.board[start..end].iter().all(|&s| s == 0) {
                return true;
            }
        }
        false
    }

    /// If any player's row is empty, sweep every player's remaining stones into
    /// their own store. Returns true if the sweep happened (i.e. game ended).
    fn collect_remaining(&mut self) -> bool {
        if !self.any_row_empty() {
            return false;
        }
        for player in 0..self.num_players {
            let start = self.player_pit_start(player);
            let end = start + self.pits;
            let mut sum = 0u8;
            for i in start..end {
                sum += self.board[i];
                self.board[i] = 0;
            }
            self.board[self.store_index(player)] += sum;
        }
        true
    }

    pub fn is_terminal(&self) -> bool {
        // Equivalent to any_row_empty(): collect_remaining ensures all rows are
        // either all 0 or all non-empty, so checking the current player's row
        // suffices on real play paths.
        let cur = self.current as usize;
        let start = self.player_pit_start(cur);
        self.board[start..start + self.pits].iter().all(|&s| s == 0)
    }

    pub fn winner(&self) -> Option<u8> {
        if !self.is_terminal() {
            return None;
        }
        let max_score = (0..self.num_players)
            .map(|p| self.board[self.store_index(p)])
            .max()
            .unwrap_or(0);
        let winners: Vec<u8> = (0..self.num_players as u8)
            .filter(|&p| self.board[self.store_index(p as usize)] == max_score)
            .collect();
        if winners.len() == 1 {
            Some(winners[0])
        } else {
            None
        }
    }

    pub fn score(&self, player: u8) -> u8 {
        self.board[self.store_index(player as usize)]
    }
}

impl GameState for Mancala {
    type Move = MancalaMove;
    type Player = u8;
    type MoveList = Vec<MancalaMove>;

    fn current_player(&self) -> u8 {
        self.current
    }

    fn available_moves(&self) -> Vec<MancalaMove> {
        // After every move, `collect_remaining` sweeps all rows to stores when
        // any row is empty — so on the hot search path, "current row empty"
        // implies the game has already been finalized. No need for the more
        // expensive any_row_empty() check here.
        let cur = self.current as usize;
        let start = self.player_pit_start(cur);
        (0..self.pits)
            .filter(|&j| self.board[start + j] > 0)
            .map(|j| MancalaMove(j as u8))
            .collect()
    }

    fn make_move(&mut self, mov: &MancalaMove) {
        let last_pos = self.sow(mov.0 as usize);
        let own_store = self.store_index(self.current as usize);
        let bonus_turn = last_pos == own_store;

        if !bonus_turn {
            self.maybe_capture(last_pos);
        }
        self.collect_remaining();

        if !bonus_turn {
            self.current = ((self.current as usize + 1) % self.num_players) as u8;
        }
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if !self.is_terminal() {
            return None;
        }
        let cur = self.current as usize;
        let cur_score = self.board[self.store_index(cur)];
        let max_other = (0..self.num_players)
            .filter(|&p| p != cur)
            .map(|p| self.board[self.store_index(p)])
            .max()
            .unwrap_or(0);
        if cur_score > max_other {
            Some(ProvenValue::Win)
        } else if cur_score < max_other {
            Some(ProvenValue::Loss)
        } else {
            Some(ProvenValue::Draw)
        }
    }
}

// --- Evaluator ---

#[derive(Clone, Debug)]
pub struct MancalaStateEval {
    pub scores: [i32; MAX_PLAYERS],
    pub num_players: u8,
}

pub struct MancalaEval;

impl MancalaEval {
    fn compute(state: &Mancala) -> MancalaStateEval {
        // Heuristic: store stones (×2 weight) + own-row stones.
        // Locked-in store stones count more than stones still on the row.
        let mut scores = [0i32; MAX_PLAYERS];
        for (p, slot) in scores.iter_mut().enumerate().take(state.num_players) {
            let store_val = state.board[state.store_index(p)] as i32;
            let row_start = state.player_pit_start(p);
            let row_total: i32 = state.board[row_start..row_start + state.pits]
                .iter()
                .map(|&s| s as i32)
                .sum();
            *slot = store_val * 2 + row_total;
        }
        MancalaStateEval {
            scores,
            num_players: state.num_players as u8,
        }
    }
}

impl Evaluator<MancalaConfig> for MancalaEval {
    type StateEvaluation = MancalaStateEval;

    fn evaluate_new_state(
        &self,
        state: &Mancala,
        moves: &Vec<MancalaMove>,
        _: Option<SearchHandle<MancalaConfig>>,
    ) -> (Vec<()>, MancalaStateEval) {
        (vec![(); moves.len()], Self::compute(state))
    }

    fn interpret_evaluation_for_player(&self, evaln: &MancalaStateEval, player: &u8) -> i64 {
        let p = *player as usize;
        let n = evaln.num_players as usize;
        let mine = evaln.scores[p];
        let max_other = (0..n)
            .filter(|&q| q != p)
            .map(|q| evaln.scores[q])
            .max()
            .unwrap_or(0);
        (mine - max_other) as i64
    }

    fn evaluate_existing_state(
        &self,
        state: &Mancala,
        _evaln: &MancalaStateEval,
        _: SearchHandle<MancalaConfig>,
    ) -> MancalaStateEval {
        Self::compute(state)
    }
}

// --- MCTS Config ---

#[derive(Default)]
pub struct MancalaConfig;

impl MCTS for MancalaConfig {
    type State = Mancala;
    type Eval = MancalaEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
}

// --- WASM API ---

#[wasm_bindgen]
pub struct MancalaWasm {
    manager: MCTSManager<MancalaConfig>,
    pits: usize,
    stones: u8,
    num_players: usize,
}

impl MancalaWasm {
    fn create(pits: usize, stones: u8, num_players: usize) -> Self {
        let pits = pits.clamp(2, MAX_PITS);
        let num_players = if num_players == 4 { 4 } else { 2 };
        let stones = stones.clamp(1, 8);
        Self {
            manager: MCTSManager::new(
                Mancala::new(pits, stones, num_players),
                MancalaConfig,
                MancalaEval,
                UCTPolicy::new(1.4),
                (),
            ),
            pits,
            stones,
            num_players,
        }
    }
}

#[wasm_bindgen]
impl MancalaWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(pits: u32, stones: u32, num_players: u32) -> Self {
        Self::create(pits as usize, stones as u8, num_players as usize)
    }

    pub fn pits(&self) -> u32 {
        self.pits as u32
    }
    pub fn stones(&self) -> u32 {
        self.stones as u32
    }
    pub fn num_players(&self) -> u32 {
        self.num_players as u32
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        let stats = types::build_stats(&self.manager, |_| None);
        serde_wasm_bindgen::to_value(&stats).unwrap()
    }

    pub fn get_tree(&self, max_depth: u32) -> JsValue {
        let tree = types::export_tree::<MancalaConfig>(
            self.manager.tree().root_node(),
            max_depth,
            &|_| None,
        );
        serde_wasm_bindgen::to_value(&tree).unwrap()
    }

    /// Comma-separated stone count per ring cell, in ring order.
    /// Layout: P0 pit 0..pits-1, P0 store, P1 pit 0..pits-1, P1 store, ...
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        (0..s.ring_len())
            .map(|i| s.board[i].to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current as u32
    }

    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().is_terminal()
    }

    /// Returns "P{N}" (winner, 1-indexed), "Draw", or "" (game in progress).
    pub fn result(&self) -> String {
        let state = self.manager.tree().root_state();
        if !state.is_terminal() {
            return String::new();
        }
        match state.winner() {
            Some(p) => format!("P{}", p + 1),
            None => "Draw".into(),
        }
    }

    /// Comma-separated scores (store stones), one per player.
    pub fn scores(&self) -> String {
        let s = self.manager.tree().root_state();
        (0..s.num_players)
            .map(|p| s.score(p as u8).to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Comma-separated legal local-pit indices for the current player.
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

    pub fn apply_move(&mut self, pit_local: &str) -> bool {
        let pit_num: u8 = match pit_local.parse() {
            Ok(n) if (n as usize) < self.pits => n,
            _ => return false,
        };
        let m = MancalaMove(pit_num);
        if self.manager.advance(&m).is_ok() {
            return true;
        }
        self.manager.playout_n(100);
        self.manager.advance(&m).is_ok()
    }

    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(
            Mancala::new(self.pits, self.stones, self.num_players),
            MancalaConfig,
            MancalaEval,
            UCTPolicy::new(1.4),
            (),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_board_two_player() {
        let g = Mancala::new(6, 4, 2);
        assert_eq!(g.ring_len(), 14);
        // P0 pits at 0..5, P0 store at 6, P1 pits at 7..12, P1 store at 13
        for j in 0..6 {
            assert_eq!(g.board[j], 4);
            assert_eq!(g.board[7 + j], 4);
        }
        assert_eq!(g.board[6], 0);
        assert_eq!(g.board[13], 0);
        assert_eq!(g.current, 0);
    }

    #[test]
    fn basic_sow_no_bonus_no_capture() {
        let mut g = Mancala::new(6, 4, 2);
        // P0 plays pit 2 (4 stones): sows into pits 3, 4, 5, 6 (own store)
        // 4 stones lands the last one in own store → bonus turn
        g.make_move(&MancalaMove(2));
        assert_eq!(g.board[2], 0);
        assert_eq!(g.board[3], 5);
        assert_eq!(g.board[4], 5);
        assert_eq!(g.board[5], 5);
        assert_eq!(g.board[6], 1); // own store
        assert_eq!(g.current, 0); // bonus turn — same player
    }

    #[test]
    fn sow_past_own_store_skips_opponent_store() {
        let mut g = Mancala::new(6, 4, 2);
        // Hand-craft: put 12 stones in P0's pit 0; should sow 0..12 wrapping
        // past own store, into P1's pits, skipping P1's store at index 13.
        g.board = [0; MAX_RING];
        g.board[0] = 12;
        g.make_move(&MancalaMove(0));
        // Sow into 1..12 (12 cells), then would hit 13 (P1 store) — skip — but
        // we only have 12 stones, so last lands in 12 (P1's last pit).
        for i in 1..=6 {
            assert_eq!(g.board[i], 1);
        }
        for i in 7..=12 {
            assert_eq!(g.board[i], 1);
        }
        assert_eq!(g.board[13], 0); // opponent's store NOT touched
    }

    #[test]
    fn sow_skips_opponent_store_on_long_throw() {
        let mut g = Mancala::new(6, 4, 2);
        g.board = [0; MAX_RING];
        g.board[0] = 14; // enough to wrap past P1 store
        g.make_move(&MancalaMove(0));
        // Sow visits indices 1,2,3,4,5,6,7,8,9,10,11,12, skips 13, then 0,1.
        // 14 stones spread: 1..12 (12) skip 13 to 0,1 (2 more) = 14.
        assert_eq!(g.board[0], 1);
        assert_eq!(g.board[1], 2);
        for i in 2..=12 {
            assert_eq!(g.board[i], 1);
        }
        assert_eq!(g.board[6], 1); // own store
        assert_eq!(g.board[13], 0); // opponent's store skipped
    }

    #[test]
    fn capture_two_player() {
        let mut g = Mancala::new(6, 4, 2);
        g.board = [0; MAX_RING];
        // Set up: P0 has 1 stone in pit 1; P1's opposite (pit 11) has 5 stones.
        // P0 plays pit 1 → sows 1 stone to pit 2 (P0's own empty pit, was 0, now 1).
        // Capture: take pit 2's 1 stone + opposite (pit 10) into P0's store.
        // But pit 10 must have stones. Let me set pit 10 = 3.
        g.board[1] = 1;
        g.board[10] = 3;
        g.make_move(&MancalaMove(1));
        assert_eq!(g.board[1], 0);
        assert_eq!(g.board[2], 0); // captured
        assert_eq!(g.board[10], 0); // opposite captured
        assert_eq!(g.board[6], 4); // 1 (landed) + 3 (opposite) = 4
        assert_eq!(g.current, 1); // turn ended, switch
    }

    fn fill_row(g: &mut Mancala, player: usize) {
        let start = g.player_pit_start(player);
        for i in start..start + g.pits {
            g.board[i] = 1;
        }
    }

    #[test]
    fn no_capture_if_opposite_empty() {
        let mut g = Mancala::new(6, 4, 2);
        g.board = [0; MAX_RING];
        // P0 has 1 stone in pit 1; opposite of landing pit 2 is pit 10.
        // Fill P1's row so game doesn't auto-end, but explicitly leave pit 10 empty.
        g.board[1] = 1;
        fill_row(&mut g, 1);
        g.board[10] = 0; // opposite of pit 2: explicitly empty
        g.make_move(&MancalaMove(1));
        // No capture; just sow normally.
        assert_eq!(g.board[2], 1);
        assert_eq!(g.board[6], 0);
        assert_eq!(g.board[10], 0);
    }

    #[test]
    fn no_capture_if_own_pit_already_had_stones() {
        let mut g = Mancala::new(6, 4, 2);
        g.board = [0; MAX_RING];
        // P0 plays pit 1 with 1 stone → lands in pit 2 which already has stones.
        g.board[1] = 1;
        g.board[2] = 3; // already has stones
        g.board[10] = 5;
        g.make_move(&MancalaMove(1));
        // No capture.
        assert_eq!(g.board[2], 4);
        assert_eq!(g.board[10], 5);
        assert_eq!(g.board[6], 0);
    }

    #[test]
    fn game_ends_when_row_empty_sweeps_remaining() {
        let mut g = Mancala::new(6, 4, 2);
        g.board = [0; MAX_RING];
        // P0 has 1 stone in pit 5, sows into own store (bonus).
        // After move, P0's row is all empty.
        // P1 still has 4 stones in each of their pits.
        g.board[5] = 1;
        for i in 7..=12 {
            g.board[i] = 4;
        }
        g.make_move(&MancalaMove(5));
        // After sweep: P0's pits all 0, P0's store has 1 (just sown).
        // P1's pits all 0, P1's store has 24 (swept).
        for i in 0..6 {
            assert_eq!(g.board[i], 0);
        }
        for i in 7..=12 {
            assert_eq!(g.board[i], 0);
        }
        assert_eq!(g.board[6], 1);
        assert_eq!(g.board[13], 24);
        assert!(g.is_terminal());
        assert_eq!(g.winner(), Some(1)); // P1 wins
    }

    #[test]
    fn four_player_initial_board() {
        let g = Mancala::new(4, 3, 4);
        assert_eq!(g.ring_len(), 20); // 4 * 5
                                      // Each player owns 4 pits + 1 store = 5 ring cells
        for player in 0..4 {
            for j in 0..4 {
                assert_eq!(g.board[player * 5 + j], 3);
            }
            assert_eq!(g.board[player * 5 + 4], 0); // store
        }
    }

    #[test]
    fn four_player_skips_other_three_stores() {
        // 4-player layout with pits=3:
        //   P0: 0,1,2; store 3
        //   P1: 4,5,6; store 7
        //   P2: 8,9,10; store 11
        //   P3: 12,13,14; store 15
        // Ring length = 16. Each non-P0 store at indices {7, 11, 15}.
        let mut g = Mancala::new(3, 0, 4);
        g.board = [0; MAX_RING];
        g.board[0] = 18;
        // Fill other rows so game doesn't auto-end after the move.
        for p in 1..4 {
            for j in 0..3 {
                g.board[p * 4 + j] = 1;
            }
        }
        g.make_move(&MancalaMove(0));
        // Stones must NOT land in opponent stores.
        assert_eq!(g.board[7], 0); // P1 store skipped
        assert_eq!(g.board[11], 0); // P2 store skipped
        assert_eq!(g.board[15], 0); // P3 store skipped
                                    // P0 store visited twice during the 18-stone sow.
        assert_eq!(g.board[3], 2);
        // Last stone lands in pit 5 (P1 pit) — not own store → switch player.
        assert_eq!(g.current, 1);
    }

    #[test]
    fn four_player_capture_geometry() {
        // 4-player layout with pits=4:
        //   P0: 0..=3; store 4
        //   P1: 5..=8; store 9
        //   P2: 10..=13; store 14
        //   P3: 15..=18; store 19
        // 4-player rule: P0[j] ↔ P2[j].
        let mut g = Mancala::new(4, 0, 4);
        g.board = [0; MAX_RING];
        g.board[2] = 1; // P0 pit 2
        g.board[13] = 5; // P2 pit 3 — opposite of P0 pit 3
                         // Filler so game doesn't auto-end.
        for p in [1, 3] {
            for j in 0..4 {
                g.board[p * 5 + j] = 1;
            }
        }
        g.make_move(&MancalaMove(2));
        // P0 sows: pit 2 → pit 3 (now has 1, owned by P0, was empty). Capture.
        assert_eq!(g.board[3], 0);
        assert_eq!(g.board[13], 0); // opposite captured
        assert_eq!(g.board[4], 6); // P0 store: 1 + 5
    }

    #[test]
    fn four_player_no_capture_for_p0_p1_pairs() {
        // Verify P0 doesn't capture from P1 or P3 — only from P2 (the true
        // opposite). Setup: P0 plays pit 2 (lands in pit 3). P1 and P3 have
        // stones; P2 pit 3 (the real opposite, ring 13) is empty.
        let mut g = Mancala::new(4, 0, 4);
        g.board = [0; MAX_RING];
        g.board[0] = 1; // P0 filler so its row stays non-empty after the sow
        g.board[2] = 1; // The pit we'll play
        g.board[7] = 5; // P1 pit 2 (NOT opposite of pit 3)
        g.board[18] = 5; // P3 pit 3 (NOT opposite of pit 3)
                         // P2 row needs filler (so its row isn't empty), but pit 13 stays 0.
        g.board[10] = 1;
        g.board[11] = 1;
        g.board[12] = 1;
        g.make_move(&MancalaMove(2));
        // No capture: true opposite (pit 13) is empty.
        assert_eq!(g.board[3], 1);
        assert_eq!(g.board[4], 0); // P0 store untouched
        assert_eq!(g.board[7], 5); // P1 pit unaffected
        assert_eq!(g.board[18], 5); // P3 pit unaffected
    }

    #[test]
    fn extra_turn_chains() {
        // Two consecutive bonus turns by P0.
        let mut g = Mancala::new(6, 4, 2);
        // P0 plays pit 2 (4 stones) → lands in own store, bonus.
        g.make_move(&MancalaMove(2));
        assert_eq!(g.current, 0);
        // P0 plays pit 5 (5 stones now — 4 original + 1 sown) → 5 stones into 6,7,8,9,10.
        // Last lands in pit 10 (P1's pit 4), not own store → no bonus.
        g.make_move(&MancalaMove(5));
        assert_eq!(g.current, 1);
    }

    #[test]
    fn legal_moves_filter_empty_pits() {
        let mut g = Mancala::new(6, 4, 2);
        g.board = [0; MAX_RING];
        g.board[1] = 3;
        g.board[4] = 1;
        fill_row(&mut g, 1); // P1 needs stones so game isn't terminal
        let moves = g.available_moves();
        let indices: Vec<u8> = moves.iter().map(|m| m.0).collect();
        assert_eq!(indices, vec![1, 4]);
    }
}
