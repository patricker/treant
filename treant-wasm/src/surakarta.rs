//! Surakarta — the arc-capture game (Indonesia; also called "Roundabouts"). Two
//! players of 12 pieces each on a 6×6 grid of points. A quiet move is a single
//! step to an empty adjacent point (8 directions). A CAPTURE is the signature
//! mechanic: a piece travels along its row/column, around at least one of the
//! board's eight corner loops, and lands on the first enemy piece it meets —
//! removing it and taking its place. All points travelled over must be empty.
//!
//! SOURCES (rules verified against, quoted verbatim; the load-bearing sentences):
//!
//! Wikipedia, "Surakarta (game)" — https://en.wikipedia.org/wiki/Surakarta_(game):
//!   "Pieces always rest on the points of intersection of the board's grid lines."
//!   "Players begin the game with 12 pieces each."
//!   Quiet move: "a player either moves one of their pieces a single step in any
//!     direction (forwards, backwards, sideways, or diagonally) to an unoccupied
//!     point".
//!   Capture: "A capturing move consists of traversing along an inner or outer
//!     circuit ... around at least one of the eight corner loops of the board,
//!     followed by landing on an enemy piece, capturing it." "Only unoccupied
//!     points may be travelled over; jumping over pieces is not permitted." "Any
//!     number of unoccupied points may be travelled over, before or after
//!     traversing a loop. An unoccupied point may be travelled over more than
//!     once during the capturing piece's journey."
//!   Win: "A game is won when a player captures all 12 of the opponent's pieces."
//!
//! Wikipedia does NOT verbatim pin WHICH rows the pieces start on (it only
//! confirms 12 each). SECOND SOURCE for the starting rows — Masters of Games,
//! "Rules of Surakarta or Roundabouts"
//! (https://www.mastersofgames.com/rules/surakarta-rules.htm):
//!   "The game of Surakarta ... is played on a special board of 6 x 6 points
//!     connected orthogonally to form a grid. Additionally, eight further loops
//!     extend out from the board."
//!   "The points second in at each corner are connected by a three-quarter circle
//!     and the points third in from each corner are connected by a larger
//!     three-quarter circle concentrically outside the first."
//!   STARTING SETUP: "The pieces are set up with the shells on the first two rows
//!     nearest the player who will player them and the stones on the first two
//!     rows nearest the opponent."
//!   Capture: "Capturing is done by moving along a line over any number of
//!     unoccupied points including a loop until an opponents piece is reached
//!     whereupon the opponents piece is captured and the attacking piece takes
//!     its place. At least one loop MUST be travelled around in order to capture."
//!
//! ⇒ Each player's 12 pieces occupy that player's two NEAREST rows. We put
//!    player 0 on rows 0–1 and player 1 on rows 4–5; rows 2–3 start empty. (The
//!    board is symmetric, so the seat↔row assignment is a labelling choice only.)
//!
//! ## THE LOOP GEOMETRY (hand-derived from the diagram below; the reviewer should
//! re-derive independently — this is the Picaria lesson)
//!
//! Points are a 6×6 grid, `r`∈0..6 rows top→bottom, `c`∈0..6 cols left→right,
//! flat index `idx = r*6 + c`. The board has 6 horizontal and 6 vertical grid
//! lines. The eight corner loops form TWO concentric circuits. There are NO arcs
//! on the outermost lines (row 0, row 5, col 0, col 5) — which is exactly why the
//! four extreme corner points (and border points along a border line) can never
//! capture along that border line.
//!
//! There are two concentric circuits (no markdown list — avoids clippy noise):
//!   INNER circuit — the closed loop through the lines 1-in from each edge:
//!   rows {1, 4} and cols {1, 4}; 4 corner arcs (one per corner).
//!   OUTER circuit — the closed loop through the lines 2-in from each edge:
//!   rows {2, 3} and cols {2, 3}; 4 corner arcs.
//!   Inner + outer = the eight corner loops.
//!
//! ASCII diagram (arcs drawn OUTSIDE the board; `·` = a point, arcs labelled by
//! the two lines they join). Only lines 1..4 carry arcs; lines 0 and 5 are bare.
//!
//! ```text
//!        c0   c1   c2   c3   c4   c5
//!       ┌────────────────────────────┐
//!  r0   │ ·    ·    ·    ·    ·    ·  │
//!       │    ╭─inner(r1)     inner─╮  │   TL-inner: row1 ↔ col1   TR-inner: row1 ↔ col4
//!  r1   │ ·  │ ·    ·    ·    · │  ·  │
//!       │  ╭─┼─outer(r2)  outer─┼─╮   │   TL-outer: row2 ↔ col2   TR-outer: row2 ↔ col3
//!  r2   │ ·││ ·    ·    ·    · ││ ·  │
//!  r3   │ ·││ ·    ·    ·    · ││ ·  │
//!       │  ╰─┼─outer(r3)  outer─┼─╯   │   BL-outer: row3 ↔ col2   BR-outer: row3 ↔ col3
//!  r4   │ ·  │ ·    ·    ·    · │  ·  │
//!       │    ╰─inner(r4)     inner─╯  │   BL-inner: row4 ↔ col1   BR-inner: row4 ↔ col4
//!  r5   │ ·    ·    ·    ·    ·    ·  │
//!       └────────────────────────────┘
//! ```
//!
//! Tracing the INNER circuit as one continuous curve confirms the arc endpoints:
//!   row1 (going right) ─TR─▶ col4 (top, going down) ─BR─▶ row4 (right, going
//!   left) ─BL─▶ col1 (bottom, going up) ─TL─▶ row1 (left, going right). Closed.
//! Likewise OUTER: row2 ─TR─▶ col3 ─BR─▶ row3 ─BL─▶ col2 ─TL─▶ row2.
//!
//! TRACK-WALKING TABLE (derived from the diagram; each arc is a bidirectional
//! connection between two line-ends). "Exit P heading D → re-enter at P′ heading
//! D′" where P′ is the first on-board point of the perpendicular line:
//!
//!   INNER  TL: row1 exit-left  (1,0)◀ ─▶ col1 exit-up   (0,1)▲   [(1,0)L↔(0,1)U]
//!   INNER  TR: row1 exit-right (1,5)▶ ─▶ col4 exit-up   (0,4)▲   [(1,5)R↔(0,4)U]
//!   INNER  BR: row4 exit-right (4,5)▶ ─▶ col4 exit-down (5,4)▼   [(4,5)R↔(5,4)D]
//!   INNER  BL: row4 exit-left  (4,0)◀ ─▶ col1 exit-down (5,1)▼   [(4,0)L↔(5,1)D]
//!   OUTER  TL: row2 exit-left  (2,0)◀ ─▶ col2 exit-up   (0,2)▲   [(2,0)L↔(0,2)U]
//!   OUTER  TR: row2 exit-right (2,5)▶ ─▶ col3 exit-up   (0,3)▲   [(2,5)R↔(0,3)U]
//!   OUTER  BR: row3 exit-right (3,5)▶ ─▶ col3 exit-down (5,3)▼   [(3,5)R↔(5,3)D]
//!   OUTER  BL: row3 exit-left  (3,0)◀ ─▶ col2 exit-down (5,2)▼   [(3,0)L↔(5,2)D]
//!
//! Worked example (asserted in tests): a piece at (1,0) travelling LEFT steps off
//! the left edge, the TL-inner arc turns it onto col1 at the top (0,1) heading
//! down — so it can capture an enemy at (0,1) with exactly one loop. A piece at a
//! true corner e.g. (0,0) sits only on border lines (row 0, col 0), neither of
//! which carries an arc, so it can NEVER capture.
//!
//! ## Move encoding
//! `(from, to)` as `"from-to"` over flat point indices 0..35 (the same `idx-idx`
//! encoding kōnane uses). A `(from, to)` capture is COMPLETE and UNAMBIGUOUS:
//! the captured piece is exactly the piece at `to`, and the arc path is a derived
//! rendering artifact (recomputed via `canonical_arc` for the UI animation), not
//! part of the move. Two distinct clear arcs from `from` to the same enemy `to`
//! (inner-vs-outer, or two corners) are therefore the SAME move — generation
//! dedups by `to`.
//!
//! ## Termination / draws
//! Win = the opponent has 0 pieces (capture-all). Non-capturing steps can shuffle
//! forever, so — following the exact `draughts.rs` precedent (a 40-ply
//! no-progress cap; see its module docs) — we declare a DRAW after 40 plies with
//! no capture. A player with no legal move (a near-impossible stalemate on this
//! sparse board) loses; guarded so MCTS never sees an empty non-terminal node.
//!
//! ## MCTS
//! Pure UCT + a cheap MATERIAL evaluator (piece-count difference from seat 0's
//! perspective), mirroring `draughts.rs` (material + mobility) — captures matter,
//! so material is the natural cheap heuristic and needs no new machinery. Solver
//! OFF (like draughts: long, cycle-prone games rarely converge and it only adds
//! overhead).
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

/// Draw after this many plies without a capture (the `draughts.rs` precedent).
const DRAW_PLIES: u32 = 40;

/// Points per side of the (fixed) 6×6 board.
const N: usize = 6;

/// The four orthogonal capture directions, indexed 0..4: up, down, left, right.
/// (Diagonals never carry a loop, so captures travel only along rows/columns.)
const CAP_DIRS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// The eight quiet-move directions (orthogonal + diagonal single step).
const STEP_DIRS: [(i32, i32); 8] =
    [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];

/// One corner arc: `(portA_point, portA_outward_dir, portB_point, portB_outward_dir)`.
type CornerArc = ((usize, usize), (i32, i32), (usize, usize), (i32, i32));

/// The eight corner arcs, each a bidirectional connection between two line-ends.
/// See the module-header track-walking table — derived from the ASCII diagram.
const ARCS: [CornerArc; 8] = [
    // INNER circuit (lines 1 and 4)
    ((1, 0), (0, -1), (0, 1), (-1, 0)), // TL-inner: row1-left  ↔ col1-top
    ((1, 5), (0, 1), (0, 4), (-1, 0)),  // TR-inner: row1-right ↔ col4-top
    ((4, 5), (0, 1), (5, 4), (1, 0)),   // BR-inner: row4-right ↔ col4-bottom
    ((4, 0), (0, -1), (5, 1), (1, 0)),  // BL-inner: row4-left  ↔ col1-bottom
    // OUTER circuit (lines 2 and 3)
    ((2, 0), (0, -1), (0, 2), (-1, 0)), // TL-outer: row2-left  ↔ col2-top
    ((2, 5), (0, 1), (0, 3), (-1, 0)),  // TR-outer: row2-right ↔ col3-top
    ((3, 5), (0, 1), (5, 3), (1, 0)),   // BR-outer: row3-right ↔ col3-bottom
    ((3, 0), (0, -1), (5, 2), (1, 0)),  // BL-outer: row3-left  ↔ col2-bottom
];

#[inline]
fn dir_index(dr: i32, dc: i32) -> usize {
    match (dr, dc) {
        (-1, 0) => 0,
        (1, 0) => 1,
        (0, -1) => 2,
        (0, 1) => 3,
        _ => unreachable!("non-orthogonal capture direction"),
    }
}

/// Arc lookup: stepping off the board from point `(r,c)` heading `(dr,dc)`, does a
/// corner arc catch it? Returns the re-entry `(point, dir_index)` — the first
/// on-board point of the perpendicular line and the inward direction of travel —
/// or `None` (a bare border line / dead end).
fn arc(r: usize, c: usize, dr: i32, dc: i32) -> Option<(usize, usize)> {
    for &((ar, ac), (adr, adc), (br, bc), (bdr, bdc)) in ARCS.iter() {
        if r == ar && c == ac && dr == adr && dc == adc {
            // Re-enter at port B heading INWARD (opposite of B's outward dir).
            return Some((br * N + bc, dir_index(-bdr, -bdc)));
        }
        if r == br && c == bc && dr == bdr && dc == bdc {
            return Some((ar * N + ac, dir_index(-adr, -adc)));
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for SMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

#[derive(Clone)]
struct Surakarta {
    /// -1 empty, else owner (0 or 1). Indexed `r*6 + c`.
    grid: [i8; N * N],
    current: u8,
    /// Plies since the last capture (for the 40-ply draw cap).
    no_capture: u32,
}

impl Surakarta {
    fn new() -> Self {
        let mut grid = [-1i8; N * N];
        // Player 0 on its two nearest rows (0,1); player 1 on rows (4,5).
        for c in 0..N {
            grid[c] = 0; // row 0
            grid[N + c] = 0; // row 1
            grid[4 * N + c] = 1; // row 4
            grid[5 * N + c] = 1; // row 5
        }
        Self { grid, current: 0, no_capture: 0 }
    }

    #[inline]
    fn count(&self, player: u8) -> usize {
        self.grid.iter().filter(|&&v| v == player as i8).count()
    }

    /// Walk a capturing traversal from `from` heading `CAP_DIRS[dir_idx]`. Returns
    /// the full point path (inclusive of `from` and the captured enemy) if the
    /// journey passes at least one corner arc and lands on the first enemy piece
    /// over an otherwise-empty route; else `None`.
    ///
    /// The mover's OWN start point `from` is treated as empty while it is in
    /// transit (it may be re-crossed) — the source's "an unoccupied point may be
    /// travelled over more than once". Any other occupied point (friend, or an
    /// enemy reached before completing a loop) is a hard block: no jumping.
    fn walk(&self, from: usize, dir_idx: usize) -> Option<Vec<usize>> {
        let me = self.grid[from];
        let enemy = 1 - me;
        let (mut r, mut c) = (from / N, from % N);
        let (mut dr, mut dc) = CAP_DIRS[dir_idx];
        let mut di = dir_idx;
        let mut path = vec![from];
        let mut arcs_taken = 0u32;
        // Deterministic traversal ⇒ revisiting any (point, dir) state means we
        // have looped without capturing. Bounded at 36 points × 4 dirs = 144.
        let mut visited = [false; N * N * 4];
        loop {
            let state = (r * N + c) * 4 + di;
            if visited[state] {
                return None; // cycle without capture
            }
            visited[state] = true;

            let nr = r as i32 + dr;
            let nc = c as i32 + dc;
            if nr < 0 || nr >= N as i32 || nc < 0 || nc >= N as i32 {
                // Off-board: take an arc, or dead-end.
                match arc(r, c, dr, dc) {
                    Some((p, ndi)) => {
                        arcs_taken += 1;
                        r = p / N;
                        c = p % N;
                        di = ndi;
                        let (ndr, ndc) = CAP_DIRS[di];
                        dr = ndr;
                        dc = ndc;
                    }
                    None => return None,
                }
            } else {
                r = nr as usize;
                c = nc as usize;
            }

            let p = r * N + c;
            if p == from {
                // Own origin — empty in transit; travel over it.
                path.push(p);
                continue;
            }
            match self.grid[p] {
                -1 => {
                    path.push(p); // empty point travelled over
                }
                v if v == enemy => {
                    if arcs_taken >= 1 {
                        path.push(p);
                        return Some(path); // legal capture of the first enemy
                    }
                    return None; // enemy reached before any loop ⇒ blocked
                }
                _ => return None, // friendly piece ⇒ blocked
            }
        }
    }

    fn captures_from(&self, from: usize) -> Vec<usize> {
        // Dedup targets: distinct arcs to the same enemy are one move.
        let mut targets: Vec<usize> = Vec::new();
        for d in 0..4 {
            if let Some(path) = self.walk(from, d) {
                let to = *path.last().unwrap();
                if !targets.contains(&to) {
                    targets.push(to);
                }
            }
        }
        targets
    }

    fn gen_moves(&self) -> Vec<SMove> {
        let me = self.current as i8;
        let mut out = Vec::new();
        for from in 0..N * N {
            if self.grid[from] != me {
                continue;
            }
            // Captures (land on an enemy via a clear loop).
            for to in self.captures_from(from) {
                out.push(SMove { from: from as u16, to: to as u16 });
            }
            // Quiet single steps to an empty point (8 directions).
            let (r, c) = (from / N, from % N);
            for (dr, dc) in STEP_DIRS {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if nr < 0 || nr >= N as i32 || nc < 0 || nc >= N as i32 {
                    continue;
                }
                let to = nr as usize * N + nc as usize;
                if self.grid[to] == -1 {
                    out.push(SMove { from: from as u16, to: to as u16 });
                }
            }
        }
        out
    }

    /// Shortest clear capturing route `from → to` as a list of point indices
    /// (inclusive). Tie-break: iterate the 4 capture directions in `CAP_DIRS`
    /// order (up, down, left, right) and keep the first path of minimal length —
    /// fully deterministic. Empty when `(from, to)` is not a legal capture.
    fn canonical_arc(&self, from: usize, to: usize) -> Vec<usize> {
        let mut best: Option<Vec<usize>> = None;
        for d in 0..4 {
            if let Some(path) = self.walk(from, d) {
                if *path.last().unwrap() == to
                    && best.as_ref().is_none_or(|b| path.len() < b.len())
                {
                    best = Some(path);
                }
            }
        }
        best.unwrap_or_default()
    }

    fn term(&self) -> Option<ProvenValue> {
        let me = self.count(self.current);
        let opp = self.count(1 - self.current);
        if me == 0 {
            return Some(ProvenValue::Loss); // I have no pieces left
        }
        if opp == 0 {
            return Some(ProvenValue::Win); // opponent wiped out
        }
        if self.no_capture >= DRAW_PLIES {
            return Some(ProvenValue::Draw);
        }
        if self.gen_moves().is_empty() {
            return Some(ProvenValue::Loss); // stalemate: cannot move ⇒ lose
        }
        None
    }

    /// Static evaluation from seat 0's perspective (positive = seat 0 better):
    /// material (piece-count difference).
    fn eval0(&self) -> i64 {
        self.count(0) as i64 - self.count(1) as i64
    }
}

impl GameState for Surakarta {
    type Move = SMove;
    type Player = u8;
    type MoveList = Vec<SMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<SMove> {
        if self.term().is_some() {
            return vec![];
        }
        self.gen_moves()
    }
    fn make_move(&mut self, m: &SMove) {
        let (from, to) = (m.from as usize, m.to as usize);
        let captured = self.grid[to] != -1; // a non-empty target ⇒ capture
        self.grid[to] = self.current as i8;
        self.grid[from] = -1;
        if captured {
            self.no_capture = 0;
        } else {
            self.no_capture += 1;
        }
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}

struct SEval;
impl Evaluator<SCfg> for SEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &Surakarta, m: &Vec<SMove>, _: Option<SearchHandle<SCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], s.eval0())
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 {
            *e
        } else {
            -*e
        }
    }
    fn evaluate_existing_state(&self, s: &Surakarta, _: &i64, _: SearchHandle<SCfg>) -> i64 {
        s.eval0()
    }
}

#[derive(Default)]
struct SCfg;
impl MCTS for SCfg {
    type State = Surakarta;
    type Eval = SEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    // Solver OFF: long, cycle-prone (draw-cap) game; the exact solver would
    // seldom converge and only adds overhead — the draughts.rs rationale.
}

#[wasm_bindgen]
pub struct SurakartaWasm {
    manager: MCTSManager<SCfg>,
}

#[wasm_bindgen]
impl SurakartaWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { manager: MCTSManager::new(Surakarta::new(), SCfg, SEval, UCTPolicy::new(1.4), ()) }
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }

    /// 36 chars, row-major: ' ' empty, 'X' player 0, 'O' player 1.
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

    /// The comma-joined point path (flat indices) of the shortest clear capturing
    /// route `from → to`, for the UI's capture animation. Empty if not a capture.
    pub fn canonical_arc(&self, from: u32, to: u32) -> String {
        let s = self.manager.tree().root_state();
        s.canonical_arc(from as usize, to as usize)
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn apply_move(&mut self, mov: &str) -> bool {
        let parts: Vec<&str> = mov.split('-').collect();
        if parts.len() != 2 {
            return false;
        }
        let (from, to) = match (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
            (Ok(f), Ok(t)) if f < N * N && t < N * N => (f, t),
            _ => return false,
        };
        let m = SMove { from: from as u16, to: to as u16 };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, SCfg, SEval, UCTPolicy::new(1.4), ());
        true
    }

    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Surakarta::new(), SCfg, SEval, UCTPolicy::new(1.4), ());
    }
}

impl Default for SurakartaWasm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[inline]
    fn idx(r: usize, c: usize) -> usize {
        r * N + c
    }

    /// Build a board with only the listed pieces: (point, owner).
    fn board(current: u8, pieces: &[(usize, i8)]) -> Surakarta {
        let mut grid = [-1i8; N * N];
        for &(p, o) in pieces {
            grid[p] = o;
        }
        Surakarta { grid, current, no_capture: 0 }
    }

    // ---- setup ------------------------------------------------------------
    #[test]
    fn starting_position_is_twelve_each_on_nearest_rows() {
        let s = Surakarta::new();
        assert_eq!(s.count(0), 12, "player 0 has 12 pieces");
        assert_eq!(s.count(1), 12, "player 1 has 12 pieces");
        assert_eq!(s.current, 0);
        for c in 0..N {
            assert_eq!(s.grid[idx(0, c)], 0);
            assert_eq!(s.grid[idx(1, c)], 0);
            assert_eq!(s.grid[idx(4, c)], 1);
            assert_eq!(s.grid[idx(5, c)], 1);
            assert_eq!(s.grid[idx(2, c)], -1, "rows 2-3 start empty");
            assert_eq!(s.grid[idx(3, c)], -1);
        }
        assert!(s.term().is_none());
    }

    // ---- geometry: arc track-walking table --------------------------------
    #[test]
    fn arc_table_matches_hand_derivation() {
        // INNER TR: row1 exit-right ↔ col4 exit-up.
        assert_eq!(arc(1, 5, 0, 1), Some((idx(0, 4), 1)), "(1,5)▶ → (0,4)▼");
        assert_eq!(arc(0, 4, -1, 0), Some((idx(1, 5), 2)), "(0,4)▲ → (1,5)◀");
        // INNER TL: row1 exit-left ↔ col1 exit-up.
        assert_eq!(arc(1, 0, 0, -1), Some((idx(0, 1), 1)), "(1,0)◀ → (0,1)▼");
        assert_eq!(arc(0, 1, -1, 0), Some((idx(1, 0), 3)), "(0,1)▲ → (1,0)▶");
        // OUTER BR: row3 exit-right ↔ col3 exit-down.
        assert_eq!(arc(3, 5, 0, 1), Some((idx(5, 3), 0)), "(3,5)▶ → (5,3)▲");
        assert_eq!(arc(5, 3, 1, 0), Some((idx(3, 5), 2)), "(5,3)▼ → (3,5)◀");
        // OUTER TL: row2 exit-left ↔ col2 exit-up.
        assert_eq!(arc(2, 0, 0, -1), Some((idx(0, 2), 1)), "(2,0)◀ → (0,2)▼");
        // Bare border lines (0 and 5) carry NO arc.
        assert_eq!(arc(0, 0, -1, 0), None, "top-left corner up: no arc");
        assert_eq!(arc(0, 0, 0, -1), None, "top-left corner left: no arc");
        assert_eq!(arc(0, 5, 0, 1), None, "row 0 exit-right: no arc");
        assert_eq!(arc(5, 0, 1, 0), None, "col 0 exit-down: no arc");
    }

    // ---- captures ---------------------------------------------------------
    #[test]
    fn simple_capture_around_one_corner() {
        // Mover(0) at (1,0); enemy(1) at (0,1). Going LEFT wraps the TL-inner arc
        // straight onto (0,1) — one loop, no intermediate points.
        let s = board(0, &[(idx(1, 0), 0), (idx(0, 1), 1)]);
        let path = s.walk(idx(1, 0), 2).expect("left wraps TL to (0,1)");
        assert_eq!(path, vec![idx(1, 0), idx(0, 1)]);
        let mv = SMove { from: idx(1, 0) as u16, to: idx(0, 1) as u16 };
        assert!(s.gen_moves().contains(&mv), "the capture is a legal move");
        assert_eq!(s.canonical_arc(idx(1, 0), idx(0, 1)), vec![idx(1, 0), idx(0, 1)]);
    }

    #[test]
    fn enemy_reached_before_any_loop_is_not_a_capture() {
        // Mover(0) at (1,3), enemy(1) at (1,1) on the same row to the left. Going
        // LEFT hits the enemy with zero arcs — blocked, no capture that way.
        let s = board(0, &[(idx(1, 3), 0), (idx(1, 1), 1)]);
        assert_eq!(s.walk(idx(1, 3), 2), None, "adjacent-ish enemy, no loop ⇒ no capture");
    }

    #[test]
    fn multi_loop_route_captures_the_long_way_around() {
        // Same pieces: going RIGHT from (1,3) rides the whole inner circuit and
        // reaches (1,1) from behind (multiple arcs) — a legal capture.
        let s = board(0, &[(idx(1, 3), 0), (idx(1, 1), 1)]);
        let path = s.walk(idx(1, 3), 3).expect("right wraps the inner circuit to (1,1)");
        assert_eq!(*path.last().unwrap(), idx(1, 1));
        assert!(path.len() > 3, "a genuine multi-loop route, not a straight shot");
    }

    #[test]
    fn corner_piece_can_never_capture() {
        // Mover(0) at (0,0), enemies packed around it. Both its lines (row 0,
        // col 0) are bare borders ⇒ no capture in any direction.
        let s = board(
            0,
            &[(idx(0, 0), 0), (idx(0, 1), 1), (idx(1, 0), 1), (idx(1, 1), 1), (idx(0, 5), 1), (idx(5, 0), 1)],
        );
        for d in 0..4 {
            assert_eq!(s.walk(idx(0, 0), d), None, "corner captures nothing (dir {d})");
        }
        assert!(
            !s.gen_moves().iter().any(|m| m.from == idx(0, 0) as u16 && s.grid[m.to as usize] != -1),
            "no capture move originates at the corner"
        );
    }

    #[test]
    fn blocked_arc_route_yields_no_capture() {
        // Mover(0) at (1,2); enemy(1) at (0,1). LEFT would go (1,2)→(1,1)→(1,0)→
        // arc→(0,1). Put a friendly stone on (1,1): the route is blocked there.
        let clear = board(0, &[(idx(1, 2), 0), (idx(0, 1), 1)]);
        assert!(clear.walk(idx(1, 2), 2).is_some(), "clear route captures");
        let blocked = board(0, &[(idx(1, 2), 0), (idx(1, 1), 0), (idx(0, 1), 1)]);
        assert_eq!(blocked.walk(idx(1, 2), 2), None, "own stone on the path blocks it");
    }

    #[test]
    fn two_clear_arcs_to_one_target_are_a_single_move() {
        // Mover(0) at (1,1); enemy(1) at (4,4) — the inner circuit's opposite
        // corner. Both RIGHT (along row 1) and DOWN (along col 1) wrap to (4,4).
        let s = board(0, &[(idx(1, 1), 0), (idx(4, 4), 1)]);
        assert!(s.walk(idx(1, 1), 3).is_some(), "right route exists");
        assert!(s.walk(idx(1, 1), 1).is_some(), "down route exists");
        let n = s
            .gen_moves()
            .iter()
            .filter(|m| m.from == idx(1, 1) as u16 && m.to == idx(4, 4) as u16)
            .count();
        assert_eq!(n, 1, "two arcs to the same target ⇒ exactly one (from,to) move");
    }

    #[test]
    fn canonical_arc_is_itself_a_legal_traversal() {
        // Validate the returned path: consecutive points connected by a single
        // orthogonal step OR a corner arc, every intermediate point empty, ending
        // on the enemy at `to`.
        let s = board(0, &[(idx(1, 1), 0), (idx(4, 4), 1)]);
        let path = s.canonical_arc(idx(1, 1), idx(4, 4));
        assert!(path.len() >= 2);
        assert_eq!(*path.first().unwrap(), idx(1, 1));
        assert_eq!(*path.last().unwrap(), idx(4, 4));
        for w in path.windows(2) {
            let (a, b) = (w[0], w[1]);
            let (ar, ac) = (a / N, a % N);
            let (br, bc) = (b / N, b % N);
            let step = (br as i32 - ar as i32, bc as i32 - ac as i32);
            let ortho_step = CAP_DIRS.contains(&step);
            // Or an arc from a in some direction lands on b.
            let via_arc = CAP_DIRS.iter().any(|&(dr, dc)| arc(ar, ac, dr, dc).map(|(p, _)| p) == Some(b));
            assert!(ortho_step || via_arc, "{a}→{b} must be a step or an arc");
        }
        // Every intermediate point empty (endpoints excluded).
        for &p in &path[1..path.len() - 1] {
            assert_eq!(s.grid[p], -1, "intermediate point {p} must be empty");
        }
    }

    // ---- quiet moves ------------------------------------------------------
    #[test]
    fn quiet_moves_are_eight_directional_single_steps_to_empty() {
        // Lone mover(0) at (2,2): all 8 neighbours empty ⇒ 8 quiet moves, incl.
        // diagonals; none land on an occupied point.
        let s = board(0, &[(idx(2, 2), 0), (idx(2, 3), 1)]);
        let quiets: Vec<usize> = s
            .gen_moves()
            .iter()
            .filter(|m| m.from == idx(2, 2) as u16 && s.grid[m.to as usize] == -1)
            .map(|m| m.to as usize)
            .collect();
        assert_eq!(quiets.len(), 7, "7 empty neighbours ((2,3) is the enemy)");
        assert!(quiets.contains(&idx(1, 1)), "diagonal step allowed");
        assert!(quiets.contains(&idx(3, 2)), "orthogonal step allowed");
        assert!(!quiets.contains(&idx(2, 3)), "quiet move never lands on a piece");
    }

    // ---- move strings -----------------------------------------------------
    #[test]
    fn round_trip_move_strings() {
        let mut g = SurakartaWasm::new();
        let legal: Vec<String> = g.legal_moves().split(',').map(|s| s.to_string()).collect();
        assert!(!legal.is_empty());
        let before = g.get_board();
        assert!(g.apply_move(&legal[0]), "a listed legal move applies");
        assert_ne!(g.get_board(), before);
        // best_move round-trips through apply_move.
        g.playout_n(80);
        let bm = g.best_move().unwrap();
        assert!(g.apply_move(&bm), "best_move {bm} must be applicable");
    }

    #[test]
    fn apply_move_rejects_illegal_strings() {
        let mut g = SurakartaWasm::new();
        assert!(!g.apply_move("garbage"), "malformed");
        assert!(!g.apply_move("0"), "missing dash");
        assert!(!g.apply_move("0-99"), "out-of-range target");
        assert!(!g.apply_move("0-35"), "corner→corner is not a legal move");
        assert!(!g.apply_move("2-8"), "own-piece hop onto occupied (2,2) illegal from start");
    }

    // ---- termination ------------------------------------------------------
    #[test]
    fn capture_all_is_terminal_with_correct_sign() {
        // Only player 0 pieces remain. Whoever is to move, player 0 (seat 0) wins.
        let a = board(1, &[(idx(2, 2), 0)]); // player 1 to move, has nothing
        assert_eq!(a.term(), Some(ProvenValue::Loss), "no pieces ⇒ Loss");
        let mut w = SurakartaWasm::new();
        w.manager = MCTSManager::new(a, SCfg, SEval, UCTPolicy::new(1.4), ());
        assert!(w.is_terminal());
        assert_eq!(w.result(), "1", "player 0 (seat 0) is the winner");

        let b = board(0, &[(idx(2, 2), 0)]); // player 0 to move, opp wiped
        assert_eq!(b.term(), Some(ProvenValue::Win), "opponent wiped ⇒ Win");
    }

    #[test]
    fn forty_ply_no_capture_is_a_draw() {
        let mut s = board(0, &[(idx(2, 2), 0), (idx(3, 3), 1)]);
        assert!(s.term().is_none());
        s.no_capture = DRAW_PLIES;
        assert_eq!(s.term(), Some(ProvenValue::Draw), "40 quiet plies ⇒ Draw");
        // A capture resets the counter (accounting sanity).
        let mut t = Surakarta::new();
        t.no_capture = 17;
        // find any quiet move; it should bump the counter, not reset it.
        let quiet = t.gen_moves().into_iter().find(|m| t.grid[m.to as usize] == -1).unwrap();
        t.make_move(&quiet);
        assert_eq!(t.no_capture, 18, "a quiet move is not progress");
    }

    // ---- AI / self-play ---------------------------------------------------
    #[test]
    fn ai_plays() {
        let mut g = SurakartaWasm::new();
        g.playout_n(300);
        assert!(g.best_move().is_some());
    }

    #[test]
    fn self_play_terminates() {
        // Full games with the weakening AI on both seats must always reach a
        // well-formed terminal (win or the 40-ply draw), never hang.
        for seed in 0..4u32 {
            let mut g = SurakartaWasm::new();
            let mut plies = 0;
            while !g.is_terminal() && plies < 4000 {
                let m = g.weak_move(40, 4, 1.5, seed.wrapping_mul(31) + plies).or_else(|| g.best_move());
                match m {
                    Some(mv) => assert!(g.apply_move(&mv), "engine emitted an unplayable move: {mv}"),
                    None => break,
                }
                plies += 1;
            }
            assert!(g.is_terminal(), "game (seed {seed}) terminated");
            assert!(!g.result().is_empty(), "terminal result is well-formed");
        }
    }
}
