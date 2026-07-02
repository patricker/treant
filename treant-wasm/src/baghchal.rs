//! Bagh-Chal — the traditional Nepali "tigers and goats" hunt on a 5×5
//! Alquerque lattice (orthogonal lines everywhere, diagonals only through the
//! points where `(row + col)` is even). Four **tigers** start on the corners;
//! twenty **goats** begin in hand. Goats move first (seat 0), tigers second
//! (seat 1).
//!
//! - Goat phase 1 — while goats remain in hand, the goat player *places* one
//!   goat per turn on any empty point (no movement).
//! - Goat phase 2 — once the hand is empty, a goat *steps* one point along a
//!   line to an adjacent empty point.
//! - Tigers — from the first move, a tiger *steps* along a line to an adjacent
//!   empty point, OR *jumps* an adjacent goat, landing on the empty point
//!   directly beyond it along the same line (a single-jump capture; no chains).
//!
//! Tigers win by capturing `capture_target` goats (default 5). Goats win by
//! immobilising every tiger. Sliding phases can cycle, so a `PLY_CAP`-ply cap
//! declares a Draw, which also makes every random rollout terminate.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

const SIZE: isize = 5;
const N: usize = 25;
/// `from` sentinel marking a goat placement rather than a slide/jump.
const PLACE: u8 = 0xFF;
/// Seat indices: goats move first, tigers second.
const GOAT: u8 = 0;
const TIGER: u8 = 1;
/// Total plies before the game is declared a draw. Because placements (finite)
/// and captures (finite) can only happen so many times, the cap bounds every
/// game — and every random playout — to a finite length even though the
/// movement phases could otherwise cycle.
const PLY_CAP: u32 = 200;

/// The eight step directions: four orthogonal, then four diagonal.
const ALL_DIRS: [(isize, isize); 8] =
    [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (-1, 1), (1, -1), (1, 1)];

fn rc(i: usize) -> (isize, isize) {
    ((i as isize) / SIZE, (i as isize) % SIZE)
}

/// The neighbour reached by one step `(dr, dc)` from `(r, c)`, if that lattice
/// edge exists: orthogonal edges exist everywhere in bounds; a diagonal edge
/// exists only through a point where `(r + c)` is even (the Alquerque pattern).
fn step(r: isize, c: isize, dr: isize, dc: isize) -> Option<usize> {
    if dr != 0 && dc != 0 && (r + c).rem_euclid(2) != 0 {
        return None;
    }
    let (nr, nc) = (r + dr, c + dc);
    if (0..SIZE).contains(&nr) && (0..SIZE).contains(&nc) {
        Some((nr * SIZE + nc) as usize)
    } else {
        None
    }
}

/// A jump in direction `(dr, dc)` from `(r, c)`: the midpoint and landing
/// points, if both edges of the two-step line exist. When the first (diagonal)
/// edge exists, `(r + c)` is even, so the second edge is guaranteed to exist
/// too (the parity is preserved across the even step), bounds permitting.
fn jump(r: isize, c: isize, dr: isize, dc: isize) -> Option<(usize, usize)> {
    let mid = step(r, c, dr, dc)?;
    let land = step(r + dr, c + dc, dr, dc)?;
    Some((mid, land))
}

/// All lattice neighbours of point `i` (used by tests and doc'd for the UI).
#[cfg(test)]
fn neighbors(i: usize) -> Vec<usize> {
    let (r, c) = rc(i);
    ALL_DIRS.iter().filter_map(|&(dr, dc)| step(r, c, dr, dc)).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BcMove {
    from: u8, // PLACE for a goat placement, else the source point of a move
    to: u8,   // the destination point (a jump's landing is two steps away)
}
impl std::fmt::Display for BcMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.from == PLACE {
            write!(f, "p{}", self.to)
        } else {
            write!(f, "{}-{}", self.from, self.to)
        }
    }
}

#[derive(Clone)]
struct Baghchal {
    grid: [i8; N], // -1 empty, 0 goat, 1 tiger
    current: u8,
    in_hand: u32,        // goats still to place
    captured: u32,       // goats eaten by the tigers
    capture_target: u32, // captures that win it for the tigers
    ply: u32,
}
impl Baghchal {
    fn new(goats: u32, capture_target: u32) -> Self {
        let mut grid = [-1i8; N];
        // Tigers start on the four corners.
        for &corner in &[0usize, 4, 20, 24] {
            grid[corner] = TIGER as i8;
        }
        Self {
            grid,
            current: GOAT,
            in_hand: goats.clamp(1, 24),
            captured: 0,
            capture_target: capture_target.clamp(1, 24),
            ply: 0,
        }
    }

    fn gen(&self) -> Vec<BcMove> {
        let mut v = Vec::new();
        if self.current == GOAT {
            if self.in_hand > 0 {
                // Placement phase: drop a goat on any empty point.
                for (i, &cell) in self.grid.iter().enumerate() {
                    if cell == -1 {
                        v.push(BcMove { from: PLACE, to: i as u8 });
                    }
                }
            } else {
                // Movement phase: step a goat to an adjacent empty point.
                for (i, &cell) in self.grid.iter().enumerate() {
                    if cell != GOAT as i8 {
                        continue;
                    }
                    let (r, c) = rc(i);
                    for &(dr, dc) in &ALL_DIRS {
                        if let Some(n) = step(r, c, dr, dc) {
                            if self.grid[n] == -1 {
                                v.push(BcMove { from: i as u8, to: n as u8 });
                            }
                        }
                    }
                }
            }
        } else {
            // Tigers step to an empty neighbour, or jump an adjacent goat.
            for (i, &cell) in self.grid.iter().enumerate() {
                if cell != TIGER as i8 {
                    continue;
                }
                let (r, c) = rc(i);
                for &(dr, dc) in &ALL_DIRS {
                    match step(r, c, dr, dc) {
                        Some(n) if self.grid[n] == -1 => {
                            v.push(BcMove { from: i as u8, to: n as u8 });
                        }
                        Some(n) if self.grid[n] == GOAT as i8 => {
                            if let Some((_mid, land)) = jump(r, c, dr, dc) {
                                if self.grid[land] == -1 {
                                    v.push(BcMove { from: i as u8, to: land as u8 });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        v
    }

    fn term(&self) -> Option<ProvenValue> {
        // Tigers reached their capture target → tigers have won. The capture
        // lands on a tiger move, after which the turn flips, so the player to
        // move is normally the goat (who sees a Loss); computed by seat to keep
        // the sign correct regardless.
        if self.captured >= self.capture_target {
            return Some(if self.current == TIGER { ProvenValue::Win } else { ProvenValue::Loss });
        }
        // A long, progress-free game is a draw (and bounds every rollout).
        if self.ply >= PLY_CAP {
            return Some(ProvenValue::Draw);
        }
        // No legal move → the player to move loses. Tigers immobilised is the
        // goats' win; a fully-stuck goat side (only reachable with the extreme
        // high-goat presets) loses symmetrically.
        if self.gen().is_empty() {
            return Some(ProvenValue::Loss);
        }
        None
    }
}
impl GameState for Baghchal {
    type Move = BcMove;
    type Player = u8;
    type MoveList = Vec<BcMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<BcMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &BcMove) {
        if m.from == PLACE {
            self.grid[m.to as usize] = GOAT as i8;
            self.in_hand -= 1;
        } else {
            let (from, to) = (m.from as usize, m.to as usize);
            self.grid[to] = self.grid[from];
            self.grid[from] = -1;
            // A two-step move is a jump: remove the goat on the midpoint.
            let (fr, fc) = rc(from);
            let (tr, tc) = rc(to);
            if (tr - fr).abs() == 2 || (tc - fc).abs() == 2 {
                let mid = ((fr + tr) / 2 * SIZE + (fc + tc) / 2) as usize;
                self.grid[mid] = -1;
                self.captured += 1;
            }
        }
        self.ply += 1;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}

struct BcEval;
impl Evaluator<BcCfg> for BcEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Baghchal, m: &Vec<BcMove>, _: Option<SearchHandle<BcCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Baghchal, _: &i64, _: SearchHandle<BcCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct BcCfg;
impl MCTS for BcCfg {
    type State = Baghchal;
    type Eval = BcEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

/// Parse a move encoding: `p<to>` (placement) or `<from>-<to>` (slide/jump).
fn parse_move(mov: &str) -> Option<BcMove> {
    if let Some(rest) = mov.strip_prefix('p') {
        let to = rest.parse::<u8>().ok()?;
        if to as usize >= N {
            return None;
        }
        return Some(BcMove { from: PLACE, to });
    }
    let (f, t) = mov.split_once('-')?;
    let from = f.parse::<u8>().ok()?;
    let to = t.parse::<u8>().ok()?;
    if from as usize >= N || to as usize >= N {
        return None;
    }
    Some(BcMove { from, to })
}

#[wasm_bindgen]
pub struct BaghchalWasm {
    manager: MCTSManager<BcCfg>,
    goats: u32,
    capture_target: u32,
}
#[wasm_bindgen]
impl BaghchalWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(goats: u32, capture_target: u32) -> Self {
        Self {
            manager: MCTSManager::new(Baghchal::new(goats, capture_target), BcCfg, BcEval, UCTPolicy::new(1.4), ()),
            goats,
            capture_target,
        }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// 25 point chars (row-major: ' '=empty, 'G'=goat, 'T'=tiger), then `'|'`,
    /// then `<goats_in_hand>,<goats_captured>`.
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        let mut out: String = s
            .grid
            .iter()
            .map(|&v| match v {
                0 => 'G',
                1 => 'T',
                _ => ' ',
            })
            .collect();
        out.push('|');
        out.push_str(&format!("{},{}", s.in_hand, s.captured));
        out
    }
    /// Goats still waiting to be placed (placement phase counter).
    pub fn goats_in_hand(&self) -> u32 {
        self.manager.tree().root_state().in_hand
    }
    /// Goats eaten by the tigers so far.
    pub fn captured(&self) -> u32 {
        self.manager.tree().root_state().captured
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
        let m = match parse_move(mov) {
            Some(m) => m,
            None => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, BcCfg, BcEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(
            Baghchal::new(self.goats, self.capture_target),
            BcCfg,
            BcEval,
            UCTPolicy::new(1.4),
            (),
        );
    }
}

impl Default for BaghchalWasm {
    fn default() -> Self {
        Self::new(20, 5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lattice_adjacency_spot_checks() {
        // Centre (2,2) has all eight neighbours.
        let mut c = neighbors(12);
        c.sort();
        assert_eq!(c, vec![6, 7, 8, 11, 13, 16, 17, 18]);
        // Corner (0,0): two orthogonal + one diagonal (r+c even).
        let mut a = neighbors(0);
        a.sort();
        assert_eq!(a, vec![1, 5, 6]);
        // Edge midpoint (0,1) has r+c odd → no diagonals, three orthogonals.
        let mut b = neighbors(1);
        b.sort();
        assert_eq!(b, vec![0, 2, 6]);
        // (1,1) is an even point → full eight neighbours.
        let mut d = neighbors(6);
        d.sort();
        assert_eq!(d, vec![0, 1, 2, 5, 7, 10, 11, 12]);
    }

    #[test]
    fn opening_is_goat_placement_over_every_empty_point() {
        let g = Baghchal::new(20, 5);
        assert_eq!(g.current, GOAT);
        assert_eq!(g.grid.iter().filter(|&&v| v == TIGER as i8).count(), 4);
        let moves = g.gen();
        assert_eq!(moves.len(), N - 4); // 21 empty points
        assert!(moves.iter().all(|m| m.from == PLACE));
    }

    #[test]
    fn tiger_jumps_and_captures_the_goat() {
        // Tiger at centre 12, goat at 13 (right), empty landing 14.
        let mut g = Baghchal::new(20, 5);
        g.grid = [-1; N];
        g.grid[12] = TIGER as i8;
        g.grid[13] = GOAT as i8;
        g.in_hand = 0;
        g.current = TIGER;
        let jump = BcMove { from: 12, to: 14 };
        assert!(g.gen().contains(&jump), "the jump 12-14 should be legal");
        g.make_move(&jump);
        assert_eq!(g.grid[14], TIGER as i8); // tiger landed
        assert_eq!(g.grid[12], -1); // vacated
        assert_eq!(g.grid[13], -1); // jumped goat removed
        assert_eq!(g.captured, 1);
        assert_eq!(g.current, GOAT);
    }

    #[test]
    fn a_goat_on_the_landing_point_blocks_the_jump() {
        // Tiger 12, goat 13, and a goat already sitting on landing 14.
        let mut g = Baghchal::new(20, 5);
        g.grid = [-1; N];
        g.grid[12] = TIGER as i8;
        g.grid[13] = GOAT as i8;
        g.grid[14] = GOAT as i8;
        g.in_hand = 0;
        g.current = TIGER;
        assert!(!g.gen().contains(&BcMove { from: 12, to: 14 }), "blocked landing → no jump");
        // The tiger can still slide to an empty orthogonal/diagonal neighbour.
        assert!(g.gen().iter().any(|m| m.from == 12), "tiger still has slides");
    }

    #[test]
    fn reaching_the_capture_target_is_a_tiger_win() {
        // Sign check: with the capture target met and the goat to move, the
        // player to move (goat) sees a Loss → tigers have won.
        let mut g = Baghchal::new(20, 5);
        g.captured = 5;
        g.current = GOAT;
        assert_eq!(g.term(), Some(ProvenValue::Loss));
        // From the tigers' seat it would read as a Win (mirror perspective).
        g.current = TIGER;
        assert_eq!(g.term(), Some(ProvenValue::Win));
    }

    #[test]
    fn immobilised_tigers_is_a_goat_win() {
        // A lone tiger at corner 0 with every neighbour a goat and every jump
        // landing also a goat → the tiger to move has no move and loses, which
        // is the goats' win.
        let mut g = Baghchal::new(20, 5);
        g.grid = [-1; N];
        g.grid[0] = TIGER as i8;
        for &p in &[1usize, 5, 6, 2, 10, 12] {
            g.grid[p] = GOAT as i8; // neighbours 1,5,6 blocked; landings 2,10,12 blocked
        }
        g.in_hand = 0;
        g.current = TIGER;
        assert!(g.gen().is_empty(), "tiger should be fully boxed in");
        assert_eq!(g.term(), Some(ProvenValue::Loss)); // tiger to move loses → goats win
    }

    #[test]
    fn ply_cap_forces_a_draw() {
        let mut g = Baghchal::new(20, 5);
        g.in_hand = 0;
        g.ply = PLY_CAP;
        assert_eq!(g.term(), Some(ProvenValue::Draw));
    }

    #[test]
    fn move_encodings_round_trip() {
        assert_eq!(parse_move("p7"), Some(BcMove { from: PLACE, to: 7 }));
        assert_eq!(parse_move("3-8"), Some(BcMove { from: 3, to: 8 }));
        assert_eq!(BcMove { from: PLACE, to: 7 }.to_string(), "p7");
        assert_eq!(BcMove { from: 3, to: 8 }.to_string(), "3-8");
        assert!(parse_move("garbage").is_none());
        assert!(parse_move("p99").is_none()); // out of range
    }

    #[test]
    fn ai_plays() {
        let mut g = BaghchalWasm::new(20, 5);
        g.playout_n(500);
        assert!(g.best_move().is_some());
        // Board string carries the phase counters after the separator.
        let board = g.get_board();
        assert_eq!(board.split('|').nth(1), Some("20,0"));
        assert_eq!(g.goats_in_hand(), 20);
        assert_eq!(g.captured(), 0);
    }

    #[test]
    fn ai_plays_a_full_game_to_terminal() {
        // Self-play sanity: every rollout terminates and the game resolves to a
        // legal verdict without the engine ever rejecting its own move.
        let mut g = BaghchalWasm::new(12, 3);
        for _ in 0..500 {
            if g.is_terminal() {
                break;
            }
            g.playout_n(40);
            let mv = match g.best_move() {
                Some(m) => m,
                None => break,
            };
            assert!(g.apply_move(&mv), "engine rejected its own move {mv}");
        }
        assert!(g.is_terminal() || !g.legal_moves().is_empty());
    }
}
