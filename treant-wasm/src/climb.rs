//! Climb — the press-your-luck "push up the mountain" dice game (the public,
//! non-trademarked take on Can't Stop). Roll four dice, pair them into two
//! column sums, and advance temporary runners up the columns. STOP to bank your
//! runners; BUST (a roll with no legal pairing) wipes everything you gained this
//! turn. Reach the top of a column and bank it to CLAIM it — claimed columns are
//! dead for everyone. First to claim the target number of columns wins.
//!
//! Showcases treant's chance nodes: each Roll opens a chance node over the 4-dice
//! outcomes. Search is open-loop (the engine samples an outcome per playout), so
//! the pending-roll state is never a stored decision node — the AI plays a full
//! multi-step turn (Roll → pick pairing → Roll/Stop …) via repeated `apply_move`.
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

/// Columns are the eleven sums 2..=12. Standard Can't Stop heights (steps to the
/// top): the middle sums are long, the extremes short. Indexed by `sum - 2`.
const HEIGHTS: [u8; 11] = [3, 5, 7, 9, 11, 13, 11, 9, 7, 5, 3];
const NUM_COLS: usize = 11;
/// Distinct runner columns allowed per turn.
const MAX_RUNNERS: usize = 3;
/// Global safety cap: Climb terminates probabilistically (busts reset progress),
/// so we bound the game. If no one has claimed enough columns by this many
/// completed turns, the game is adjudicated: most claimed columns wins, tie = Draw.
const TURN_CAP: u32 = 500;

#[inline]
fn col_index(sum: u8) -> usize {
    (sum as usize) - 2
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// The player chooses Roll (or Stop, if they have runner progress this turn).
    Act,
    /// A roll is in flight — a chance node over the four dice (never a decision root).
    Roll,
    /// The four dice are on the table; the player chooses a pairing.
    Pair,
}

#[derive(Clone, Debug, PartialEq)]
enum ClimbMove {
    Roll,
    Stop,
    /// Chance outcome: the four dice, sorted ascending.
    Dice([u8; 4]),
    /// A pairing choice. Advance column `first` once; if `second` is `Some(s)`,
    /// advance column `s` once more. `second == Some(first)` means advance the
    /// same column twice (from a doubles pairing).
    Pair(u8, Option<u8>),
}
impl std::fmt::Display for ClimbMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ClimbMove::Roll => write!(f, "Roll"),
            ClimbMove::Stop => write!(f, "Stop"),
            ClimbMove::Dice([a, b, c, d]) => write!(f, "Dice({a},{b},{c},{d})"),
            ClimbMove::Pair(a, Some(b)) => write!(f, "P{a}-{b}"),
            ClimbMove::Pair(a, None) => write!(f, "P{a}"),
        }
    }
}

#[derive(Clone, Debug)]
struct Climb {
    num_players: u8,
    current: u8,
    to_win: u8,
    /// Per player, per column: banked (permanent) progress, 0..=height.
    banked: Vec<[u8; NUM_COLS]>,
    /// Who has claimed each column (-1 = unclaimed), else seat index.
    claimed: [i8; NUM_COLS],
    /// This turn's temporary runners: (column index, absolute position). ≤ 3.
    runners: Vec<(usize, u8)>,
    phase: Phase,
    /// The dice currently on the table (last roll). Kept for the pairing decision
    /// and for display (including the busting roll). `None` at a fresh turn start.
    dice: Option<[u8; 4]>,
    /// True when the immediately preceding action was a bust (for UI display).
    busted: bool,
    /// Completed turns (a turn ends on Stop or Bust). Drives the turn cap.
    turns: u32,
    /// Set when a player claims their `to_win`-th column.
    winner: Option<u8>,
}

impl Climb {
    fn new(num_players: u8, to_win: u8) -> Self {
        Self {
            num_players,
            current: 0,
            to_win,
            banked: vec![[0u8; NUM_COLS]; num_players as usize],
            claimed: [-1i8; NUM_COLS],
            runners: Vec::new(),
            phase: Phase::Act,
            dice: None,
            busted: false,
            turns: 0,
            winner: None,
        }
    }

    fn claimed_count(&self, p: u8) -> usize {
        self.claimed.iter().filter(|&&c| c == p as i8).count()
    }

    fn is_terminal(&self) -> bool {
        self.winner.is_some() || self.turns >= TURN_CAP
    }

    /// The winning seat if there is one, else `None` (game unfinished, or a
    /// turn-cap draw with no unique leader).
    fn result_seat(&self) -> Option<u8> {
        if let Some(w) = self.winner {
            return Some(w);
        }
        if self.turns >= TURN_CAP {
            let counts: Vec<usize> = (0..self.num_players).map(|p| self.claimed_count(p)).collect();
            let mx = counts.iter().copied().max().unwrap_or(0);
            let leaders: Vec<usize> = counts.iter().enumerate().filter(|(_, &c)| c == mx).map(|(i, _)| i).collect();
            if leaders.len() == 1 {
                return Some(leaders[0] as u8);
            }
        }
        None
    }

    fn has_runner(&self, idx: usize) -> Option<u8> {
        self.runners.iter().find(|(c, _)| *c == idx).map(|(_, p)| *p)
    }

    /// The current player's effective position in a column: the runner if one is
    /// placed there this turn, otherwise their banked marker.
    fn cur_pos(&self, idx: usize) -> u8 {
        self.has_runner(idx).unwrap_or(self.banked[self.current as usize][idx])
    }

    fn slots_used(&self) -> usize {
        self.runners.len()
    }

    /// Can the current player advance column `idx` by one step right now?
    fn can_advance(&self, idx: usize) -> bool {
        self.claimed[idx] < 0
            && (self.cur_pos(idx) as usize) < HEIGHTS[idx] as usize
            && (self.has_runner(idx).is_some() || self.slots_used() < MAX_RUNNERS)
    }

    /// Advance the current player's runner in column `idx` by one step, creating
    /// the runner (at banked + 1) if this is its first advance this turn.
    fn step(&mut self, idx: usize) {
        if let Some(r) = self.runners.iter_mut().find(|(c, _)| *c == idx) {
            r.1 += 1;
        } else {
            let start = self.banked[self.current as usize][idx];
            self.runners.push((idx, start + 1));
        }
    }

    /// Legal pairing moves given `self.dice`. Empty ⇒ the roll is a bust.
    fn legal_pairings(&self) -> Vec<ClimbMove> {
        let Some([a, b, c, d]) = self.dice else {
            return Vec::new();
        };
        let pairings = [(a + b, c + d), (a + c, b + d), (a + d, b + c)];
        let mut out: Vec<ClimbMove> = Vec::new();
        let push = |m: ClimbMove, out: &mut Vec<ClimbMove>| {
            if !out.contains(&m) {
                out.push(m);
            }
        };
        for (x, y) in pairings {
            if x == y {
                // Doubles: advance the one column twice if possible, else once.
                let idx = col_index(x);
                if self.claimed[idx] < 0 {
                    let start = self.cur_pos(idx) as usize;
                    let h = HEIGHTS[idx] as usize;
                    let slot_ok = self.has_runner(idx).is_some() || self.slots_used() < MAX_RUNNERS;
                    if slot_ok && start + 2 <= h {
                        push(ClimbMove::Pair(x, Some(x)), &mut out);
                    } else if slot_ok && start < h {
                        push(ClimbMove::Pair(x, None), &mut out);
                    }
                }
                continue;
            }
            let (ix, iy) = (col_index(x), col_index(y));
            // Do both sums fit together, respecting the 3-runner limit?
            let mut new_needed = 0usize;
            if self.has_runner(ix).is_none() {
                new_needed += 1;
            }
            if self.has_runner(iy).is_none() {
                new_needed += 1;
            }
            let both = self.claimed[ix] < 0
                && self.claimed[iy] < 0
                && (self.cur_pos(ix) as usize) < HEIGHTS[ix] as usize
                && (self.cur_pos(iy) as usize) < HEIGHTS[iy] as usize
                && self.slots_used() + new_needed <= MAX_RUNNERS;
            if both {
                // Standard rule: you must take both sums when able.
                push(ClimbMove::Pair(x, Some(y)), &mut out);
            } else {
                if self.can_advance(ix) {
                    push(ClimbMove::Pair(x, None), &mut out);
                }
                if self.can_advance(iy) {
                    push(ClimbMove::Pair(y, None), &mut out);
                }
            }
        }
        out
    }

    fn has_progress(&self) -> bool {
        !self.runners.is_empty()
    }

    /// End the current turn without banking (a bust): clear runners, advance.
    fn advance_player(&mut self) {
        self.runners.clear();
        self.turns += 1;
        self.current = (self.current + 1) % self.num_players;
        self.phase = Phase::Act;
    }

    /// Per-player heuristic score: claimed columns dominate, fractional progress
    /// (banked, plus the current player's live runners) breaks ties. Padded to
    /// four seats with `i64::MIN` so unused seats never win `best_other`.
    fn score_array(&self) -> [i64; 4] {
        let mut out = [i64::MIN; 4];
        for p in 0..self.num_players {
            let claimed = self.claimed_count(p) as i64 * 10_000;
            let mut prog = 0i64;
            for (idx, &h) in HEIGHTS.iter().enumerate() {
                if self.claimed[idx] >= 0 {
                    continue;
                }
                let pos = if p == self.current {
                    self.has_runner(idx).unwrap_or(self.banked[p as usize][idx])
                } else {
                    self.banked[p as usize][idx]
                };
                prog += (pos as i64 * 100) / h as i64;
            }
            out[p as usize] = claimed + prog;
        }
        out
    }
}

impl GameState for Climb {
    type Move = ClimbMove;
    type Player = u8;
    type MoveList = Vec<ClimbMove>;

    fn current_player(&self) -> u8 {
        self.current
    }

    fn available_moves(&self) -> Vec<ClimbMove> {
        if self.is_terminal() {
            return vec![];
        }
        match self.phase {
            Phase::Act => {
                let mut v = vec![ClimbMove::Roll];
                if self.has_progress() {
                    v.push(ClimbMove::Stop);
                }
                v
            }
            Phase::Roll => vec![], // chance node — outcomes come from chance_outcomes()
            Phase::Pair => self.legal_pairings(),
        }
    }

    fn make_move(&mut self, m: &ClimbMove) {
        match m {
            ClimbMove::Roll => {
                self.busted = false;
                self.phase = Phase::Roll;
            }
            ClimbMove::Dice(d) => {
                self.dice = Some(*d);
                self.phase = Phase::Pair;
                if self.legal_pairings().is_empty() {
                    // Bust: no legal pairing. Lose all runner progress this turn.
                    self.busted = true;
                    self.advance_player();
                }
            }
            ClimbMove::Pair(first, second) => {
                self.step(col_index(*first));
                if let Some(s) = second {
                    self.step(col_index(*s));
                }
                self.busted = false;
                self.phase = Phase::Act;
            }
            ClimbMove::Stop => {
                // Bank every runner; claim any column filled to its top.
                let cur = self.current as usize;
                let mut runners = std::mem::take(&mut self.runners);
                for (idx, pos) in runners.drain(..) {
                    self.banked[cur][idx] = pos;
                    if self.claimed[idx] < 0 && pos == HEIGHTS[idx] {
                        self.claimed[idx] = self.current as i8;
                    }
                }
                if self.claimed_count(self.current) >= self.to_win as usize {
                    self.winner = Some(self.current);
                    self.phase = Phase::Act;
                    // Terminal: is_terminal() now true, available_moves() empty.
                } else {
                    self.dice = None;
                    self.busted = false;
                    self.turns += 1;
                    self.current = (self.current + 1) % self.num_players;
                    self.phase = Phase::Act;
                }
            }
        }
    }

    fn chance_outcomes(&self) -> Option<Vec<(ClimbMove, f64)>> {
        if self.phase != Phase::Roll || self.is_terminal() {
            return None;
        }
        // Enumerate the distinct sorted 4-dice multisets with their probability
        // weights (multiplicity / 6^4 = / 1296). There are 126 such multisets;
        // weights sum to 1.
        let mut out: Vec<(ClimbMove, f64)> = Vec::with_capacity(126);
        for a in 1..=6u8 {
            for b in a..=6 {
                for c in b..=6 {
                    for d in c..=6 {
                        let mult = multiplicity(a, b, c, d) as f64;
                        out.push((ClimbMove::Dice([a, b, c, d]), mult / 1296.0));
                    }
                }
            }
        }
        Some(out)
    }
}

/// Number of ordered permutations of a sorted 4-dice multiset (sums to 1296
/// over all 126 multisets). `4! / prod(count!)`.
fn multiplicity(a: u8, b: u8, c: u8, d: u8) -> u32 {
    let vals = [a, b, c, d];
    let mut counts = [0u32; 7]; // faces 1..=6
    for &v in &vals {
        counts[v as usize] += 1;
    }
    let fact = |n: u32| (1..=n).product::<u32>().max(1);
    24 / counts.iter().map(|&c| fact(c)).product::<u32>()
}

// --- Evaluator (heuristic; no solver — chance game like Pig/Dice) ---

struct ClimbEval;
impl Evaluator<ClimbConfig> for ClimbEval {
    type StateEvaluation = [i64; 4];
    fn evaluate_new_state(&self, s: &Climb, m: &Vec<ClimbMove>, _: Option<SearchHandle<ClimbConfig>>) -> (Vec<()>, [i64; 4]) {
        (vec![(); m.len()], s.score_array())
    }
    fn interpret_evaluation_for_player(&self, e: &[i64; 4], p: &u8) -> i64 {
        let mine = e[*p as usize];
        let best_other = e
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != *p as usize)
            .map(|(_, &s)| s)
            .max()
            .unwrap_or(0);
        mine - best_other
    }
    fn evaluate_existing_state(&self, s: &Climb, _: &[i64; 4], _: SearchHandle<ClimbConfig>) -> [i64; 4] {
        s.score_array()
    }
}

#[derive(Default)]
struct ClimbConfig;
impl MCTS for ClimbConfig {
    type State = Climb;
    type Eval = ClimbEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
}

// --- WASM surface ---

#[wasm_bindgen]
pub struct ClimbWasm {
    manager: MCTSManager<ClimbConfig>,
    num_players: u8,
    to_win: u8,
    rng: SmallRng,
}

#[wasm_bindgen]
impl ClimbWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(num_players: u32, to_win: u32) -> Self {
        let np = (num_players as u8).clamp(2, 4);
        let to_win = (to_win as u8).clamp(2, 5);
        Self {
            manager: MCTSManager::new(Climb::new(np, to_win), ClimbConfig, ClimbEval, UCTPolicy::new(0.7), ()),
            num_players: np,
            to_win,
            rng: SmallRng::from_entropy(),
        }
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

    /// Compact board string, `|`-separated. Parse it in the UI (see `climb.tsx`).
    ///
    /// ```text
    /// <meta> | <dice> | <col2> | <col3> | … | <col12>
    /// ```
    /// — 13 fields total (1 meta + 1 dice + 11 columns).
    ///
    /// - **meta**: `current,numPlayers,toWin,turns,turnCap,phase,busted`
    ///   - `current`  — seat to act, 0-based
    ///   - `phase`    — `act` | `pair` | `over`
    ///   - `busted`   — `1` if the previous action busted (show "💥 Bust!"), else `0`
    /// - **dice**: `d0,d1,d2,d3` (sorted ascending) or `-` when no roll is shown.
    /// - **col&lt;n&gt;** (one per sum 2..=12, in order): `height:claimedBy:runner:banked0,banked1,…`
    ///   - `height`    — steps to the top of this column
    ///   - `claimedBy` — `-1` unclaimed, else the owning seat
    ///   - `runner`    — the current player's runner position here, or `-1` if none
    ///   - `banked…`   — each seat's banked marker position (comma-separated)
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        let phase = if s.is_terminal() {
            "over"
        } else {
            match s.phase {
                Phase::Pair => "pair",
                _ => "act",
            }
        };
        let meta = format!("{},{},{},{},{},{},{}", s.current, s.num_players, s.to_win, s.turns, TURN_CAP, phase, s.busted as u8);
        let dice = match s.dice {
            Some([a, b, c, d]) => format!("{a},{b},{c},{d}"),
            None => "-".to_string(),
        };
        let mut parts = vec![meta, dice];
        for (idx, &h) in HEIGHTS.iter().enumerate() {
            let runner = s.has_runner(idx).map(|p| p as i32).unwrap_or(-1);
            let banked = (0..s.num_players as usize).map(|p| s.banked[p][idx].to_string()).collect::<Vec<_>>().join(",");
            parts.push(format!("{}:{}:{}:{}", h, s.claimed[idx], runner, banked));
        }
        parts.join("|")
    }

    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current as u32
    }

    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().is_terminal()
    }

    pub fn result(&self) -> String {
        let s = self.manager.tree().root_state();
        if !s.is_terminal() {
            return String::new();
        }
        match s.result_seat() {
            Some(w) => format!("{}", w + 1),
            None => "Draw".to_string(),
        }
    }

    /// Comma-joined legal move encodings for the current decision (Roll/Stop, or
    /// pairing options like `P4-9`, `P4`, `P8-8`). Chance outcomes are never listed.
    pub fn legal_moves(&self) -> String {
        let s = self.manager.tree().root_state();
        s.available_moves().iter().map(|m| format!("{m}")).collect::<Vec<_>>().join(",")
    }

    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }

    /// Validate `mov` against the current decision and apply it, rebuilding the
    /// search tree from the new root. A `Roll` resolves its dice here (via this
    /// instance's RNG) and lands the state on either a pairing choice or a bust.
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let mut s = self.manager.tree().root_state().clone();
        if s.is_terminal() {
            return false;
        }
        let legal: Vec<String> = s.available_moves().iter().map(|m| format!("{m}")).collect();
        if !legal.iter().any(|l| l == mov) {
            return false;
        }
        match mov {
            "Roll" => {
                s.make_move(&ClimbMove::Roll);
                let mut d = [0u8; 4];
                for v in d.iter_mut() {
                    *v = self.rng.gen_range(1..=6);
                }
                d.sort_unstable();
                s.make_move(&ClimbMove::Dice(d));
            }
            "Stop" => s.make_move(&ClimbMove::Stop),
            _ => {
                // A pairing: "P<a>" or "P<a>-<b>".
                let Some(m) = parse_pair(mov) else {
                    return false;
                };
                s.make_move(&m);
            }
        }
        self.manager = MCTSManager::new(s, ClimbConfig, ClimbEval, UCTPolicy::new(0.7), ());
        true
    }

    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Climb::new(self.num_players, self.to_win), ClimbConfig, ClimbEval, UCTPolicy::new(0.7), ());
    }
}

/// Parse a pairing move encoding: `"P4"` → `Pair(4, None)`, `"P4-9"` →
/// `Pair(4, Some(9))`, `"P8-8"` → `Pair(8, Some(8))`.
fn parse_pair(s: &str) -> Option<ClimbMove> {
    let body = s.strip_prefix('P')?;
    match body.split_once('-') {
        Some((a, b)) => Some(ClimbMove::Pair(a.parse().ok()?, Some(b.parse().ok()?))),
        None => Some(ClimbMove::Pair(body.parse().ok()?, None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_dice(g: &mut Climb, d: [u8; 4]) {
        g.make_move(&ClimbMove::Roll);
        g.make_move(&ClimbMove::Dice(d));
    }

    #[test]
    fn chance_weights_sum_to_one_over_1296() {
        let mut g = Climb::new(2, 3);
        g.make_move(&ClimbMove::Roll);
        let outcomes = g.chance_outcomes().expect("pending roll is a chance node");
        assert_eq!(outcomes.len(), 126, "126 distinct sorted 4-dice multisets");
        let total: f64 = outcomes.iter().map(|(_, p)| p).sum();
        assert!((total - 1.0).abs() < 1e-9, "weights sum to 1, got {total}");
        // Multiplicities are integers over 1296 and sum to exactly 1296.
        let mult_sum: i64 = outcomes.iter().map(|(_, p)| (p * 1296.0).round() as i64).sum();
        assert_eq!(mult_sum, 1296);
    }

    #[test]
    fn multiplicity_counts() {
        assert_eq!(multiplicity(1, 1, 1, 1), 1); // quad
        assert_eq!(multiplicity(1, 2, 3, 4), 24); // all distinct
        assert_eq!(multiplicity(1, 1, 2, 3), 12); // one pair
        assert_eq!(multiplicity(1, 1, 2, 2), 6); // two pair
        assert_eq!(multiplicity(1, 1, 1, 2), 4); // triple
    }

    #[test]
    fn pairing_respects_three_runner_limit() {
        let mut g = Climb::new(2, 3);
        // Occupy three distinct runner columns.
        g.runners = vec![(col_index(4), 1), (col_index(5), 1), (col_index(6), 1)];
        g.phase = Phase::Pair;
        // Dice 1,1,6,6 → pairings (2,12),(7,7),(7,7). Columns 2 and 12 are new
        // (no slot left), column 7 is new too → nothing is playable → bust.
        g.dice = Some([1, 1, 6, 6]);
        assert!(g.legal_pairings().is_empty(), "no 4th runner column allowed");
    }

    #[test]
    fn pairing_can_extend_an_existing_runner_when_full() {
        let mut g = Climb::new(2, 3);
        g.runners = vec![(col_index(4), 1), (col_index(6), 1), (col_index(8), 1)];
        g.phase = Phase::Pair;
        // 2,2,4,4 → pairings (4,8?) wait compute: (2+2,4+4)=(4,8); (2+4,2+4)=(6,6);
        // (2+4,2+4)=(6,6). Column 4 & 8 already have runners → can extend both.
        g.dice = Some([2, 2, 4, 4]);
        let moves = g.legal_pairings();
        assert!(moves.contains(&ClimbMove::Pair(4, Some(8))), "extend both existing runners: {moves:?}");
    }

    #[test]
    fn bust_detected_and_turn_passes() {
        let mut g = Climb::new(2, 3);
        g.runners = vec![(col_index(2), 3), (col_index(3), 5), (col_index(12), 3)];
        // All three runner columns are at their tops (maxed), no slot free.
        // Any roll can only hit those or open a 4th column → bust.
        set_dice(&mut g, [1, 1, 6, 6]); // sums land on 2/12 (maxed) or 7 (new, no slot)
        assert!(g.busted, "should bust");
        assert_eq!(g.current, 1, "turn passed to player 2");
        assert!(g.runners.is_empty(), "runner progress wiped");
    }

    #[test]
    fn stop_banks_and_claims_column_at_top() {
        let mut g = Climb::new(2, 3);
        // Runner in column 2 (height 3) sitting at the top.
        g.runners = vec![(col_index(2), 3)];
        g.phase = Phase::Act;
        g.make_move(&ClimbMove::Stop);
        assert_eq!(g.banked[0][col_index(2)], 3, "banked to the top");
        assert_eq!(g.claimed[col_index(2)], 0, "column 2 claimed by seat 0");
        assert_eq!(g.current, 1, "turn passed");
    }

    #[test]
    fn win_at_target_columns_correct_seat() {
        let mut g = Climb::new(2, 3);
        // Seat 0 already owns columns 2 and 12; now claims column 3 to reach 3.
        g.claimed[col_index(2)] = 0;
        g.claimed[col_index(12)] = 0;
        g.runners = vec![(col_index(3), 5)]; // height 5, at top
        g.make_move(&ClimbMove::Stop);
        assert!(g.is_terminal(), "reaching 3 columns ends the game");
        assert_eq!(g.result_seat(), Some(0), "seat 0 wins");
        assert_eq!(g.winner, Some(0));
    }

    #[test]
    fn turn_cap_adjudicates_by_claimed_count() {
        let mut g = Climb::new(2, 3);
        g.claimed[col_index(4)] = 1; // seat 1 leads 1-0
        g.turns = TURN_CAP;
        assert!(g.is_terminal());
        assert_eq!(g.result_seat(), Some(1), "most claimed columns wins the cap");
        // A tie at the cap is a draw.
        let mut h = Climb::new(2, 3);
        h.turns = TURN_CAP;
        assert!(h.is_terminal());
        assert_eq!(h.result_seat(), None, "0-0 at the cap is a draw");
    }

    #[test]
    fn doubles_advance_same_column_twice() {
        let mut g = Climb::new(2, 3);
        g.phase = Phase::Pair;
        g.dice = Some([3, 3, 3, 3]); // every pairing is (6,6)
        let moves = g.legal_pairings();
        assert!(moves.contains(&ClimbMove::Pair(6, Some(6))), "doubles step twice: {moves:?}");
        g.make_move(&ClimbMove::Pair(6, Some(6)));
        assert_eq!(g.cur_pos(col_index(6)), 2, "column 6 runner advanced two steps");
    }

    #[test]
    fn must_roll_before_stopping() {
        let g = Climb::new(2, 3);
        let moves = g.available_moves();
        assert!(moves.contains(&ClimbMove::Roll));
        assert!(!moves.contains(&ClimbMove::Stop), "cannot stop with no progress");
    }

    #[test]
    fn ai_plays_a_full_turn() {
        let mut g = ClimbWasm::new(2, 3);
        g.playout_n(200);
        let m = g.best_move().unwrap();
        assert!(m == "Roll" || m == "Stop" || m.starts_with('P'));
        // Drive a couple of real moves through the wasm surface.
        assert!(g.apply_move("Roll"), "roll should apply");
        let board = g.get_board();
        assert_eq!(board.split('|').count(), 13, "13 board fields");
    }

    #[test]
    fn ai_can_finish_a_game_within_the_cap() {
        // A full self-play game must terminate (winner or cap) — guards playout
        // termination for Watch-AI mode.
        let mut g = ClimbWasm::new(2, 2);
        for _ in 0..5000 {
            if g.is_terminal() {
                break;
            }
            g.playout_n(30);
            let m = g.best_move().or_else(|| g.legal_moves().split(',').next().map(|s| s.to_string())).unwrap();
            assert!(g.apply_move(&m), "move {m} should apply");
        }
        assert!(g.is_terminal(), "game reached a terminal state");
        assert!(!g.result().is_empty(), "terminal has a result string");
    }
}
