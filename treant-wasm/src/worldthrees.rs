//! World Threes — one engine over eight traditional tiny three-in-a-row games,
//! selected by `board`. Each board is a per-board static config: a set of
//! points with drawn-line adjacency (for sliding), a set of winning lines, a
//! piece count, a start layout (placement phase, or pre-placed), and a win
//! mode (line, line-except-your-home-row, or blockade). One `GameState` runs
//! them all. Boards are tiny, so the exact solver plays them perfectly.
//!
//! RULES ARE LOAD-BEARING — every board's adjacency and win rules were verified
//! against its cited Wikipedia page (URL in a comment per board) before
//! encoding; edge counts are hand-counted and asserted in the tests.
//!
//! Moves: placement `"<point>"`, slide/jump `"<from>-<to>"` (single-sourced
//! Display/parse, round-trip tested). Sliding games can cycle, so a no-progress
//! ply cap declares a Draw — this also guarantees every random rollout ends.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

/// Plies without a placement before a stalled position is declared a draw.
/// Placements are finite and each resets the counter, so at most this many pure
/// slides/jumps can pass in a row — guaranteeing every rollout terminates.
const DRAW_PLY_CAP: u32 = 40;

/// `from` sentinel marking a placement rather than a slide/jump.
const PLACE: u16 = u16::MAX;

/// One board's complete static definition.
struct BoardDef {
    /// Point coordinates in a 0..100 viewBox (the renderer's single source of
    /// truth — exported verbatim via `get_layout`, so drawn edges can never
    /// diverge from the legal moves).
    coords: &'static [(f32, f32)],
    /// Drawn-line adjacency: `adj[p]` = points reachable from `p` by a one-step
    /// slide. Symmetric.
    adj: &'static [&'static [u8]],
    /// Winning lines (each exactly three collinear points).
    lines: &'static [&'static [u8]],
    /// Collinear triples `[a, b, c]` over which an endpoint may JUMP the middle
    /// (occupied by anyone, not captured) to land on the far empty endpoint.
    /// Empty for boards without jumping (Tsoro Yematatu is the only jumper).
    jumps: &'static [&'static [u8]],
    /// Pieces per player.
    pieces: u8,
    /// Pre-placed start positions per player; empty ⇒ a placement phase first.
    start: [&'static [u8]; 2],
    /// Per-player "home" line that does NOT count as a win for that player
    /// (Tant Fant). Empty ⇒ every line counts.
    home: [&'static [u8]; 2],
    /// Win by blockade (opponent has no move) rather than by a line — Pong Hau
    /// K'i. When true, `lines` is ignored.
    blockade: bool,
}

// ── Shared 3×3 geometry ──────────────────────────────────────────────────────
// Points 0..8 laid out row-major:  0 1 2 / 3 4 5 / 6 7 8.
const GRID9: [(f32, f32); 9] =
    [(15.0, 15.0), (50.0, 15.0), (85.0, 15.0), (15.0, 50.0), (50.0, 50.0), (85.0, 50.0), (15.0, 85.0), (50.0, 85.0), (85.0, 85.0)];
// 3×3 with the two centre diagonals (0-4-8, 2-4-6) — the Three Men's Morris /
// Tapatan / Achi / Tant Fant graph.
const ADJ_DIAG: [&[u8]; 9] =
    [&[1, 3, 4], &[0, 2, 4], &[1, 4, 5], &[0, 4, 6], &[0, 1, 2, 3, 5, 6, 7, 8], &[2, 4, 8], &[3, 4, 7], &[4, 6, 8], &[4, 5, 7]];
// 3×3 with NO diagonals — Nine Holes.
const ADJ_ORTH: [&[u8]; 9] =
    [&[1, 3], &[0, 2, 4], &[1, 5], &[0, 4, 6], &[1, 3, 5, 7], &[2, 4, 8], &[3, 7], &[4, 6, 8], &[5, 7]];
// Eight straight lines of the 3×3 (3 rows, 3 cols, 2 diagonals).
const LINES_DIAG: [&[u8]; 8] =
    [&[0, 1, 2], &[3, 4, 5], &[6, 7, 8], &[0, 3, 6], &[1, 4, 7], &[2, 5, 8], &[0, 4, 8], &[2, 4, 6]];
// Six straight lines of the 3×3 with diagonals excluded (Nine Holes).
const LINES_ORTH: [&[u8]; 6] = [&[0, 1, 2], &[3, 4, 5], &[6, 7, 8], &[0, 3, 6], &[1, 4, 7], &[2, 5, 8]];

// ── Shisima: octagon rim 0..7 + centre 8 ─────────────────────────────────────
const SHISIMA_COORDS: [(f32, f32); 9] = [
    (50.0, 10.0),  // 0  top
    (78.3, 21.7),  // 1
    (90.0, 50.0),  // 2  right
    (78.3, 78.3),  // 3
    (50.0, 90.0),  // 4  bottom
    (21.7, 78.3),  // 5
    (10.0, 50.0),  // 6  left
    (21.7, 21.7),  // 7
    (50.0, 50.0),  // 8  centre
];
// Rim i ↔ its two rim neighbours (octagon edge) and ↔ the centre (diameter).
const SHISIMA_ADJ: [&[u8]; 9] =
    [&[7, 1, 8], &[0, 2, 8], &[1, 3, 8], &[2, 4, 8], &[3, 5, 8], &[4, 6, 8], &[5, 7, 8], &[6, 0, 8], &[0, 1, 2, 3, 4, 5, 6, 7]];
// Only the four diameters (rim–centre–opposite rim) count as three-in-a-row.
const SHISIMA_LINES: [&[u8]; 4] = [&[0, 8, 4], &[1, 8, 5], &[2, 8, 6], &[3, 8, 7]];

// ── Tsoro Yematatu: 7-point triangle ─────────────────────────────────────────
// 0 apex; 1 2 3 the mid "breadth" line; 4 5 6 the base.
const TSORO_COORDS: [(f32, f32); 7] =
    [(50.0, 10.0), (31.0, 45.0), (50.0, 45.0), (69.0, 45.0), (12.0, 80.0), (50.0, 80.0), (88.0, 80.0)];
const TSORO_ADJ: [&[u8]; 7] = [&[1, 2, 3], &[0, 2, 4], &[0, 1, 3, 5], &[0, 2, 6], &[1, 5], &[2, 4, 6], &[3, 5]];
// Five drawn lines: two sides, the central axis, the breadth line, the base.
const TSORO_LINES: [&[u8]; 5] = [&[0, 1, 4], &[0, 3, 6], &[0, 2, 5], &[1, 2, 3], &[4, 5, 6]];

// ── Picaria: 3×3 grid (0..8) + four INTERIOR quadrant-centre points (9..12) ────
// The documented Zuni board: the outer square is split into four small squares
// (the 3×3 grid, points 0..8), and each small square is split into four
// triangles by BOTH of its diagonals. Those diagonals cross at the small
// squares' centres, adding four INTERIOR playing points — 9 top-left, 10
// top-right, 11 bottom-left, 12 bottom-right — each at the centre of its
// quadrant (degree-4). Sources agree on this interior-point form: the cited
// Wikipedia page ("four additional spaces … at the intersection of the four
// additional diagonal lines with those of the larger diagonal lines"), and the
// independent depictions at auntannie.com / whatdowedoallday.com / bead.game,
// which all describe the 9-point variant as "leaving out the four inner points
// that form a square". (The prior encoding placed these points EXTERIOR to the
// grid — a degree-2 diamond — which no source supports.)
//
// Each interior point is the midpoint of both of its small square's diagonals,
// e.g. 9 = mid(0,4) = mid(1,3). Grid points use a full 15/50/85 spread.
const PICARIA_COORDS: [(f32, f32); 13] = [
    (15.0, 15.0), (50.0, 15.0), (85.0, 15.0), // 0 1 2  grid top row
    (15.0, 50.0), (50.0, 50.0), (85.0, 50.0), // 3 4 5  grid middle row
    (15.0, 85.0), (50.0, 85.0), (85.0, 85.0), // 6 7 8  grid bottom row
    (32.5, 32.5), // 9  top-left  quadrant centre  (mid of 0-4 and of 1-3)
    (67.5, 32.5), // 10 top-right quadrant centre  (mid of 2-4 and of 1-5)
    (32.5, 67.5), // 11 bot-left  quadrant centre  (mid of 4-6 and of 3-7)
    (67.5, 67.5), // 12 bot-right quadrant centre  (mid of 4-8 and of 5-7)
];
// Drawn-line adjacency. The interior points sit ON the two main diagonals, which
// therefore no longer join a corner straight to the grid centre: the diagonals
// read 0-9-4-12-8 and 2-10-4-11-6. Each interior point joins the 4 grid points
// of its small square (that square's two diagonals); the grid rows/cols keep
// their orthogonal adjacencies. 28 undirected edges (verified in tests):
//   0:1,3,9  1:0,2,4,9,10  2:1,5,10  3:0,4,6,9,11  4:1,3,5,7,9,10,11,12
//   5:2,4,8,10,12  6:3,7,11  7:4,6,8,11,12  8:5,7,12  → deg-sum 56 / 2 = 28.
const PICARIA_ADJ: [&[u8]; 13] = [
    &[1, 3, 9],                   // 0
    &[0, 2, 4, 9, 10],            // 1
    &[1, 5, 10],                  // 2
    &[0, 4, 6, 9, 11],            // 3
    &[1, 3, 5, 7, 9, 10, 11, 12], // 4  grid centre — joins the 4 quadrant centres, not the corners
    &[2, 4, 8, 10, 12],           // 5
    &[3, 7, 11],                  // 6
    &[4, 6, 8, 11, 12],           // 7
    &[5, 7, 12],                  // 8
    &[0, 1, 3, 4],                // 9  TL centre → its 4 surrounding grid points
    &[1, 2, 4, 5],                // 10 TR centre
    &[3, 4, 6, 7],                // 11 BL centre
    &[4, 5, 7, 8],                // 12 BR centre
];
// Winning three-in-a-rows: every gapless collinear triple lying on a drawn line
// (middle point second, per the line-adjacency invariant). The board has 12
// DRAWN straight lines — 3 rows, 3 cols, 2 main diagonals, 4 small-square
// anti-diagonals — which is the "12 winning lines" the sources cite. But the two
// main diagonals are 5-point lines (corner-centre-centre-centre-corner), so each
// yields THREE consecutive triples rather than one; the other 10 lines are
// 3-point lines yielding one triple each. Total = 10 + 2·3 = 16 winning triples:
//   rows (3)  cols (3)
//   main diag 0-9-4-12-8 → [0,9,4] [9,4,12] [4,12,8]
//   main diag 2-10-4-11-6 → [2,10,4] [10,4,11] [4,11,6]
//   small-square anti-diagonals (4): [1,9,3] [1,10,5] [3,11,7] [5,12,7]
const PICARIA_LINES: [&[u8]; 16] = [
    &[0, 1, 2], &[3, 4, 5], &[6, 7, 8], // rows
    &[0, 3, 6], &[1, 4, 7], &[2, 5, 8], // cols
    &[0, 9, 4], &[9, 4, 12], &[4, 12, 8], // main diagonal through 9 (TL) and 12 (BR)
    &[2, 10, 4], &[10, 4, 11], &[4, 11, 6], // main diagonal through 10 (TR) and 11 (BL)
    &[1, 9, 3], &[1, 10, 5], &[3, 11, 7], &[5, 12, 7], // small-square anti-diagonals
];

// ── Pong Hau K'i: 4 corners (0 TL, 1 TR, 2 BL, 3 BR) + centre 4 ──────────────
// Square top/left/right sides drawn, bottom side NOT; both diagonals cross at
// the centre → 7 edges. A player who cannot move loses.
const PONG_COORDS: [(f32, f32); 5] = [(20.0, 20.0), (80.0, 20.0), (20.0, 80.0), (80.0, 80.0), (50.0, 50.0)];
const PONG_ADJ: [&[u8]; 5] = [&[1, 2, 4], &[0, 3, 4], &[0, 4], &[1, 4], &[0, 1, 2, 3]];

const NO_LINES: &[&[u8]] = &[];
const NO_JUMPS: &[&[u8]] = &[];
const NO_HOME: [&[u8]; 2] = [&[], &[]];
const NO_START: [&[u8]; 2] = [&[], &[]];

/// The eight boards, in preset order.
const BOARDS: [BoardDef; 8] = [
    // 0 — Achi (Ghana): https://en.wikipedia.org/wiki/Achi_(game)
    // 3×3 + centre diagonals, FOUR men each, place then slide, any 3-in-line.
    BoardDef { coords: &GRID9, adj: &ADJ_DIAG, lines: &LINES_DIAG, jumps: NO_JUMPS, pieces: 4, start: NO_START, home: NO_HOME, blockade: false },
    // 1 — Tapatan (Philippines): https://en.wikipedia.org/wiki/Three_men%27s_morris
    // 3×3 + centre diagonals, three men each, place then slide, any 3-in-line.
    BoardDef { coords: &GRID9, adj: &ADJ_DIAG, lines: &LINES_DIAG, jumps: NO_JUMPS, pieces: 3, start: NO_START, home: NO_HOME, blockade: false },
    // 2 — Shisima (Kenya): https://en.wikipedia.org/wiki/Shisima
    // Octagon rim + centre. PRE-PLACED (3 successive rim points each, a gap on
    // both ends — the sourced form; the plan table's "place then slide" is the
    // less-cited variant). Only lines THROUGH the centre win.
    BoardDef { coords: &SHISIMA_COORDS, adj: &SHISIMA_ADJ, lines: &SHISIMA_LINES, jumps: NO_JUMPS, pieces: 3, start: [&[0, 1, 2], &[4, 5, 6]], home: NO_HOME, blockade: false },
    // 3 — Tant Fant (India): https://en.wikipedia.org/wiki/Three_men%27s_morris
    // 3×3 + centre diagonals, PRE-PLACED on the two home rows, slide only. A
    // line on your OWN home row does not win.
    BoardDef { coords: &GRID9, adj: &ADJ_DIAG, lines: &LINES_DIAG, jumps: NO_JUMPS, pieces: 3, start: [&[0, 1, 2], &[6, 7, 8]], home: [&[0, 1, 2], &[6, 7, 8]], blockade: false },
    // 4 — Nine Holes (England): https://en.wikipedia.org/wiki/Nine_holes
    // 3×3, NO diagonals, three men each, place then slide (adjacent — the
    // sourced "standard" rule; a move-to-any-vacant variant is attested). Only
    // orthogonal rows/columns win.
    BoardDef { coords: &GRID9, adj: &ADJ_ORTH, lines: &LINES_ORTH, jumps: NO_JUMPS, pieces: 3, start: NO_START, home: NO_HOME, blockade: false },
    // 5 — Tsoro Yematatu (Zimbabwe): https://en.wikipedia.org/wiki/Tsoro_Yematatu
    // 7-point triangle, three men each, place then slide OR jump (no capture),
    // any 3-in-line.
    BoardDef { coords: &TSORO_COORDS, adj: &TSORO_ADJ, lines: &TSORO_LINES, jumps: &TSORO_LINES, pieces: 3, start: NO_START, home: NO_HOME, blockade: false },
    // 6 — Picaria (Zuni): https://en.wikipedia.org/wiki/Picaria
    // 13-point board: 3×3 grid + four INTERIOR quadrant-centre points, three men
    // each, place then slide. The main diagonals run through the quadrant
    // centres, so the diagonal wins are the small-square diagonals (16 gapless
    // three-in-a-row triples over the 12 drawn lines — see PICARIA_LINES).
    BoardDef { coords: &PICARIA_COORDS, adj: &PICARIA_ADJ, lines: &PICARIA_LINES, jumps: NO_JUMPS, pieces: 3, start: NO_START, home: NO_HOME, blockade: false },
    // 7 — Pong Hau K'i (China): https://en.wikipedia.org/wiki/Pong_Hau_K%27i
    // 5 points, two men each, PRE-PLACED on opposite corners, slide only. You
    // WIN when your opponent is blockaded (no legal move).
    BoardDef { coords: &PONG_COORDS, adj: &PONG_ADJ, lines: NO_LINES, jumps: NO_JUMPS, pieces: 2, start: [&[0, 3], &[1, 2]], home: NO_HOME, blockade: true },
];

/// A placement (`from == PLACE`) or a slide/jump (`from`→`to`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WtMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for WtMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.from == PLACE {
            write!(f, "{}", self.to)
        } else {
            write!(f, "{}-{}", self.from, self.to)
        }
    }
}
/// Parse `"<point>"` (placement) or `"<from>-<to>"` (slide/jump).
fn parse_move(s: &str) -> Option<WtMove> {
    match s.split_once('-') {
        Some((a, b)) => Some(WtMove { from: a.parse().ok()?, to: b.parse().ok()? }),
        None => Some(WtMove { from: PLACE, to: s.parse().ok()? }),
    }
}

#[derive(Clone)]
struct WorldThrees {
    board: usize,
    cells: Vec<i8>, // -1 empty, else owner 0/1
    current: u8,
    placed: [u8; 2],   // men placed so far, per player
    progress_ply: u32, // slides/jumps since the last placement
}
impl WorldThrees {
    fn new(board: usize) -> Self {
        let def = &BOARDS[board];
        let mut cells = vec![-1i8; def.coords.len()];
        let mut placed = [0u8, 0u8];
        // Pre-placed boards start with their men on the board and no men in hand.
        for (p, pts) in def.start.iter().enumerate() {
            for &pt in *pts {
                cells[pt as usize] = p as i8;
            }
            if !pts.is_empty() {
                placed[p] = def.pieces;
            }
        }
        Self { board, cells, current: 0, placed, progress_ply: 0 }
    }

    fn def(&self) -> &'static BoardDef {
        &BOARDS[self.board]
    }

    fn placing(&self) -> bool {
        self.placed[self.current as usize] < self.def().pieces
    }

    fn gen(&self) -> Vec<WtMove> {
        let def = self.def();
        let me = self.current as i8;
        let mut v = Vec::new();
        if self.placing() {
            // Placement phase: drop a man on any empty point.
            for (i, &c) in self.cells.iter().enumerate() {
                if c < 0 {
                    v.push(WtMove { from: PLACE, to: i as u16 });
                }
            }
            return v;
        }
        // Slide phase: step each man to an adjacent empty point…
        for from in 0..self.cells.len() {
            if self.cells[from] != me {
                continue;
            }
            for &to in def.adj[from] {
                if self.cells[to as usize] < 0 {
                    v.push(WtMove { from: from as u16, to: to as u16 });
                }
            }
        }
        // …and, on jumping boards, leap an endpoint over the (occupied) middle
        // of a line onto the far empty endpoint (no capture).
        for line in def.jumps {
            let (a, b, c) = (line[0] as usize, line[1] as usize, line[2] as usize);
            if self.cells[b] < 0 {
                continue; // nothing to jump over
            }
            if self.cells[a] == me && self.cells[c] < 0 {
                v.push(WtMove { from: a as u16, to: c as u16 });
            }
            if self.cells[c] == me && self.cells[a] < 0 {
                v.push(WtMove { from: c as u16, to: a as u16 });
            }
        }
        v
    }

    /// The owner (0/1) who has completed a winning line, honouring Tant Fant's
    /// home-row exclusion. `None` if no line is complete.
    fn line_winner(&self) -> Option<i8> {
        let def = self.def();
        for line in def.lines {
            let o = self.cells[line[0] as usize];
            if o < 0 || !line.iter().all(|&p| self.cells[p as usize] == o) {
                continue;
            }
            // A completed line on the owner's own home row does not count.
            let home = def.home[o as usize];
            if !home.is_empty() && home == *line {
                continue;
            }
            return Some(o);
        }
        None
    }

    fn term(&self) -> Option<ProvenValue> {
        let def = self.def();
        // A long placement-free stretch (or a blockade stand-off) is a draw.
        if self.progress_ply >= DRAW_PLY_CAP {
            return Some(ProvenValue::Draw);
        }
        if !def.blockade {
            if let Some(o) = self.line_winner() {
                // Only the player who just moved can have completed a line, so
                // from the mover-to-play's view this is a Loss (Win is
                // unreachable but handled for safety).
                return Some(if o == self.current as i8 { ProvenValue::Win } else { ProvenValue::Loss });
            }
        }
        // No legal move → the player to move loses (blockade / stalemate).
        if self.gen().is_empty() {
            return Some(ProvenValue::Loss);
        }
        None
    }
}
impl GameState for WorldThrees {
    type Move = WtMove;
    type Player = u8;
    type MoveList = Vec<WtMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<WtMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &WtMove) {
        if m.from == PLACE {
            self.cells[m.to as usize] = self.current as i8;
            self.placed[self.current as usize] += 1;
            self.progress_ply = 0; // placements are progress (and finite)
        } else {
            self.cells[m.from as usize] = -1;
            self.cells[m.to as usize] = self.current as i8;
            self.progress_ply += 1;
        }
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}

struct WtEval;
impl Evaluator<WtCfg> for WtEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &WorldThrees, m: &Vec<WtMove>, _: Option<SearchHandle<WtCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &WorldThrees, _: &i64, _: SearchHandle<WtCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct WtCfg;
impl MCTS for WtCfg {
    type State = WorldThrees;
    type Eval = WtEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct WorldThreesWasm {
    manager: MCTSManager<WtCfg>,
    board: usize,
}
#[wasm_bindgen]
impl WorldThreesWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(board: u32) -> Self {
        let board = (board as usize).min(BOARDS.len() - 1);
        Self { manager: MCTSManager::new(WorldThrees::new(board), WtCfg, WtEval, UCTPolicy::new(1.4), ()), board }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// One char per point: ' '=empty, 'X'=p0, 'O'=p1.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .cells
            .iter()
            .map(|&v| match v {
                0 => 'X',
                1 => 'O',
                _ => ' ',
            })
            .collect()
    }
    /// Render layout (the board's single source of truth, shared with the
    /// engine's adjacency): `"x,y x,y …|a-b,c-d,…"` — points then undirected
    /// edges. The React board draws exactly these; legality still comes from
    /// `legal_moves`, so drawn edges cannot diverge from legal slides.
    pub fn get_layout(&self) -> String {
        let def = &BOARDS[self.board];
        let pts: Vec<String> = def.coords.iter().map(|(x, y)| format!("{x},{y}")).collect();
        let mut edges: Vec<String> = Vec::new();
        for (a, ns) in def.adj.iter().enumerate() {
            for &b in *ns {
                if (a as u8) < b {
                    edges.push(format!("{a}-{b}"));
                }
            }
        }
        format!("{}|{}", pts.join(" "), edges.join(","))
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
        self.manager = MCTSManager::new(s, WtCfg, WtEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(WorldThrees::new(self.board), WtCfg, WtEval, UCTPolicy::new(1.4), ());
    }
}

impl Default for WorldThreesWasm {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Undirected edge count for a board (each edge counted once).
    fn edge_count(board: usize) -> usize {
        let def = &BOARDS[board];
        let mut n = 0;
        for (a, ns) in def.adj.iter().enumerate() {
            for &b in *ns {
                if (a as u8) < b {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn adjacency_is_symmetric_on_every_board() {
        for (bi, def) in BOARDS.iter().enumerate() {
            for (a, ns) in def.adj.iter().enumerate() {
                for &b in *ns {
                    assert!(def.adj[b as usize].contains(&(a as u8)), "board {bi}: {a}->{b} not mirrored");
                }
            }
        }
    }

    #[test]
    fn hand_counted_edge_counts() {
        // Verified against each board's cited Wikipedia diagram.
        assert_eq!(edge_count(0), 16, "Achi 3×3+diag"); // 6 rows + 6 cols + 4 diag
        assert_eq!(edge_count(1), 16, "Tapatan 3×3+diag");
        assert_eq!(edge_count(2), 16, "Shisima octagon"); // 8 rim + 8 spokes
        assert_eq!(edge_count(3), 16, "Tant Fant 3×3+diag");
        assert_eq!(edge_count(4), 12, "Nine Holes 3×3 orth"); // 6 rows + 6 cols
        assert_eq!(edge_count(5), 10, "Tsoro Yematatu triangle");
        assert_eq!(edge_count(6), 28, "Picaria 13-point interior-point"); // see PICARIA_ADJ enumeration
        assert_eq!(edge_count(7), 7, "Pong Hau K'i"); // 5 vertices, 7 edges
    }

    #[test]
    fn line_points_are_all_adjacent_pairs_or_share_a_middle() {
        // Every winning line's endpoints must be reachable along drawn edges via
        // its middle (so a completed line is a real board line, not a phantom).
        for (bi, def) in BOARDS.iter().enumerate() {
            for line in def.lines {
                assert_eq!(line.len(), 3, "board {bi}: line not length 3");
                let (a, b, c) = (line[0], line[1], line[2]);
                assert!(def.adj[a as usize].contains(&b), "board {bi}: line {line:?} a-b not an edge");
                assert!(def.adj[b as usize].contains(&c), "board {bi}: line {line:?} b-c not an edge");
            }
        }
    }

    #[test]
    fn move_encoding_round_trips() {
        assert_eq!(parse_move("5"), Some(WtMove { from: PLACE, to: 5 }));
        assert_eq!(parse_move("3-7"), Some(WtMove { from: 3, to: 7 }));
        assert_eq!(WtMove { from: PLACE, to: 5 }.to_string(), "5");
        assert_eq!(WtMove { from: 3, to: 7 }.to_string(), "3-7");
        assert!(parse_move("nope").is_none());
        assert!(parse_move("3-x").is_none());
    }

    #[test]
    fn achi_starts_in_placement_and_transitions_to_slide() {
        let mut g = WorldThrees::new(0);
        // Opening: 9 empty points, all placements, no dashes.
        let opening = g.gen();
        assert_eq!(opening.len(), 9);
        assert!(opening.iter().all(|m| m.from == PLACE));
        // Fill all 8 men (4 each) → the 9th point stays empty → slide phase.
        let seq = [0, 8, 1, 7, 2, 6, 3, 5]; // p0: 0,1,2,3  p1: 8,7,6,5  (no line)
        for &p in &seq {
            let m = WtMove { from: PLACE, to: p };
            assert!(g.gen().contains(&m), "placement {p} unavailable");
            g.make_move(&m);
        }
        assert!(!g.placing(), "should be in slide phase after 8 placements");
        let slides = g.gen();
        assert!(!slides.is_empty() && slides.iter().all(|m| m.from != PLACE), "slide phase must offer only slides");
    }

    #[test]
    fn tapatan_scripted_line_win() {
        // p0 places a winning column 0-3-6 in three moves while p1 sits idle
        // elsewhere; the win registers during placement.
        let mut g = WorldThrees::new(1);
        for (mv, _) in [(0u16, 0u8), (5, 1), (3, 0), (8, 1)] {
            g.make_move(&WtMove { from: PLACE, to: mv });
        }
        // p0 to move: place at 6 to complete column {0,3,6}.
        g.make_move(&WtMove { from: PLACE, to: 6 });
        assert_eq!(g.line_winner(), Some(0));
        // From p1's perspective (now to move) it's a Loss → result seat 1.
        assert_eq!(g.term(), Some(ProvenValue::Loss));
        assert_eq!(g.current, 1);
    }

    #[test]
    fn nine_holes_diagonal_does_not_win() {
        // A diagonal 0-4-8 owned by one player is NOT a win on Nine Holes.
        let mut g = WorldThrees::new(4);
        g.cells[0] = 0;
        g.cells[4] = 0;
        g.cells[8] = 0;
        assert_eq!(g.line_winner(), None, "Nine Holes has no diagonal wins");
        // …but an orthogonal row is.
        let mut g2 = WorldThrees::new(4);
        g2.cells[0] = 0;
        g2.cells[1] = 0;
        g2.cells[2] = 0;
        assert_eq!(g2.line_winner(), Some(0));
    }

    #[test]
    fn shisima_rim_only_line_does_not_win() {
        // Three successive RIM points (no centre) is not a diameter → no win.
        let mut g = WorldThrees::new(2);
        g.cells = vec![-1; 9];
        g.cells[0] = 0;
        g.cells[1] = 0;
        g.cells[2] = 0;
        assert_eq!(g.line_winner(), None, "rim-only run must not win");
        // A diameter rim–centre–opposite-rim DOES win.
        let mut g2 = WorldThrees::new(2);
        g2.cells = vec![-1; 9];
        g2.cells[0] = 1;
        g2.cells[8] = 1;
        g2.cells[4] = 1;
        assert_eq!(g2.line_winner(), Some(1));
    }

    #[test]
    fn shisima_is_pre_placed() {
        let g = WorldThrees::new(2);
        assert_eq!(g.placed, [3, 3], "Shisima starts fully placed");
        assert_eq!(g.cells[0], 0);
        assert_eq!(g.cells[4], 1);
        assert_eq!(g.cells[8], -1, "centre empty at start");
        assert!(g.gen().iter().all(|m| m.from != PLACE), "no placements — slide only");
    }

    #[test]
    fn tant_fant_home_row_does_not_win_but_other_lines_do() {
        let g = WorldThrees::new(3);
        assert_eq!(g.placed, [3, 3]);
        // Start: p0 owns home row {0,1,2}, p1 owns {6,7,8}. Neither counts.
        assert_eq!(g.line_winner(), None, "home rows must not count as wins");
        // p0 forming the OPPONENT's row {6,7,8} would win (not p0's home row).
        let mut g2 = WorldThrees::new(3);
        g2.cells = vec![-1; 9];
        g2.cells[6] = 0;
        g2.cells[7] = 0;
        g2.cells[8] = 0;
        assert_eq!(g2.line_winner(), Some(0), "a non-home line wins");
        // …and a column does too.
        let mut g3 = WorldThrees::new(3);
        g3.cells = vec![-1; 9];
        g3.cells[0] = 1;
        g3.cells[3] = 1;
        g3.cells[6] = 1;
        assert_eq!(g3.line_winner(), Some(1));
    }

    #[test]
    fn tsoro_jump_over_a_piece_is_legal() {
        // Slide phase: p0 at apex 0, a piece at middle 2, far point 5 empty →
        // 0 may jump over 2 to 5 along the central axis {0,2,5}.
        let mut g = WorldThrees::new(5);
        g.placed = [3, 3];
        g.cells = vec![-1; 7];
        g.cells[0] = 0; // apex (p0, to move)
        g.cells[2] = 1; // middle occupied (enemy — jumps don't capture)
        g.current = 0;
        let moves = g.gen();
        assert!(moves.contains(&WtMove { from: 0, to: 5 }), "expected a jump 0->5, got {moves:?}");
        // The jumped piece is NOT removed.
        g.make_move(&WtMove { from: 0, to: 5 });
        assert_eq!(g.cells[2], 1, "jumped piece stays on the board");
        assert_eq!(g.cells[5], 0);
        assert_eq!(g.cells[0], -1);
    }

    #[test]
    fn picaria_quadrant_centre_line_wins() {
        // A small-square diagonal through quadrant-centre 9: corner 0, centre 9,
        // grid centre 4 — a winning three-in-a-row.
        let mut g = WorldThrees::new(6);
        g.cells = vec![-1; 13];
        g.cells[0] = 0;
        g.cells[9] = 0;
        g.cells[4] = 0;
        assert_eq!(g.line_winner(), Some(0), "corner–quadrant-centre–grid-centre diagonal must win");
        // The interior–grid-centre–interior triple 9-4-12 (the middle of the main
        // diagonal) is also a gapless three-in-a-row.
        let mut g2 = WorldThrees::new(6);
        g2.cells = vec![-1; 13];
        g2.cells[9] = 1;
        g2.cells[4] = 1;
        g2.cells[12] = 1;
        assert_eq!(g2.line_winner(), Some(1), "interior–centre–interior diagonal must win");
        // The full grid corner-to-corner diagonal 0-4-8 is NOT a win: interior
        // points 9 and 12 sit between the corners, so 0,4,8 have gaps.
        let mut g3 = WorldThrees::new(6);
        g3.cells = vec![-1; 13];
        g3.cells[0] = 0;
        g3.cells[4] = 0;
        g3.cells[8] = 0;
        assert_eq!(g3.line_winner(), None, "0-4-8 has interior gaps (9,12) — not a line");
    }

    #[test]
    fn picaria_has_sixteen_winning_lines() {
        // 6 rows/cols + 4 small-square anti-diagonals (one triple each) + 2 main
        // diagonals (three gapless triples each) = 16. See PICARIA_LINES comment.
        assert_eq!(BOARDS[6].lines.len(), 16, "Picaria: 12 drawn lines → 16 gapless three-in-a-row triples");
    }

    #[test]
    fn pong_hau_ki_blockade_is_a_loss() {
        // Construct a blockade: empty point is a bottom corner (2), whose only
        // neighbours (0 and centre 4) are held so the player to move cannot
        // reach it. Pieces: p0 at 0,4; p1 at 1,3. Point 2 empty, p1 to move.
        let mut g = WorldThrees::new(7);
        g.cells = vec![0, 1, -1, 1, 0];
        g.current = 1; // p1 to move: pieces at 1 and 3
        // 1's neighbours: 0,3,4 (all occupied). 3's neighbours: 1,4 (occupied).
        assert!(g.gen().is_empty(), "p1 must be blockaded");
        assert_eq!(g.term(), Some(ProvenValue::Loss), "blockaded player loses");
    }

    #[test]
    fn pong_hau_ki_is_pre_placed_and_moves() {
        let g = WorldThrees::new(7);
        assert_eq!(g.cells, vec![0, 1, 1, 0, -1], "opposite-corner start, centre empty");
        // p0 (at 0 and 3) can slide either man into the empty centre 4.
        let moves = g.gen();
        assert!(moves.contains(&WtMove { from: 0, to: 4 }));
        assert!(moves.contains(&WtMove { from: 3, to: 4 }));
    }

    #[test]
    fn every_board_ai_plays_and_terminates() {
        for board in 0..BOARDS.len() {
            let mut g = WorldThreesWasm::new(board as u32);
            for _ in 0..200 {
                if g.is_terminal() {
                    break;
                }
                g.playout_n(60);
                let mv = match g.best_move() {
                    Some(m) => m,
                    None => break,
                };
                assert!(g.apply_move(&mv), "board {board}: engine rejected its own move {mv}");
            }
            assert!(g.is_terminal() || !g.legal_moves().is_empty(), "board {board}: stuck non-terminal");
        }
    }

    #[test]
    fn constructor_clamps_out_of_range_board() {
        let g = WorldThreesWasm::new(99);
        assert_eq!(g.board, BOARDS.len() - 1);
    }

    #[test]
    fn layout_edges_match_adjacency() {
        for (board, spec) in BOARDS.iter().enumerate() {
            let g = WorldThreesWasm::new(board as u32);
            let layout = g.get_layout();
            let (pts, edges) = layout.split_once('|').unwrap();
            assert_eq!(pts.split(' ').count(), spec.coords.len(), "board {board}: point count");
            assert_eq!(edges.split(',').count(), edge_count(board), "board {board}: edge count");
        }
    }
}
