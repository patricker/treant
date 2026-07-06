//! Gale — the grid form of the Shannon switching game (David Gale, 1958). Two
//! players build bridges on interleaved dot grids: Red joins its LEFT and RIGHT
//! sides, Gold joins its TOP and BOTTOM. Like Hex it can never draw — on a full
//! board exactly one player has connected.
//!
//! NAME: we ship it as "Gale" (the academic name). The 1960s commercial version
//! was trademarked "Bridg-It" — that name is deliberately kept out of all
//! user-facing strings.
//!
//! SOURCES (rules verified against, encoded below):
//! Wikipedia, "Shannon switching game" (Gale variant): two interleaved, offset
//! dot grids (the 1960 commercial set used interlocking 5×6 grids); "one player
//! links the top of their grid to the bottom, the other links left to right";
//! "no draw can result". hexwiki, "Bridg-It": the two players' lines are OFFSET
//! so "their centre points are the same", and "lines must not cross — every
//! centre point can only be occupied by one of the players." That shared centre
//! point is the crossing lattice this engine claims.
//!
//! THE ELEGANT ENCODING ("claim a crossing point"). Every potential Red edge
//! crosses exactly one potential Gold edge at a shared centre point, and vice
//! versa; the boundary edges that cross nothing are strategically dead (a Red
//! move can always be replayed one column in), so we model ONLY the crossings.
//! A move claims a crossing: for Red it draws the Red edge there and forbids the
//! Gold edge; for Gold the reverse. That single move set is a faithful, complete
//! Bridg-It.
//!
//! GEOMETRY (n = dots-per-side; classic Bridg-It is n = 5 → Red 5×6, Gold 6×5).
//! World coordinates on the square [0, 2n] × [0, 2n]:
//! Red dot (r, c) at (x = 2c, y = 2r+1), r ∈ 0..n, c ∈ 0..=n (n rows × (n+1) cols).
//! Gold dot (r, c) at (x = 2c+1, y = 2r), r ∈ 0..=n, c ∈ 0..n ((n+1) rows × n cols).
//! Red joins x=0 (c=0) to x=2n (c=n); Gold joins y=0 (r=0) to y=2n (r=n).
//!
//! CROSSING LATTICE (two interleaved sets — hand-derived, asserted in tests).
//! Type H (odd,odd centres (2c+1, 2r+1)), r,c ∈ 0..n: n² crossings; Red edge =
//! red horizontal (r,c)-(r,c+1), Gold edge = gold vertical (r,c)-(r+1,c).
//! Type V (even,even centres (2c+2, 2r+2)), r,c ∈ 0..n-1: (n-1)² crossings; Red
//! edge = red vertical (r,c+1)-(r+1,c+1), Gold edge = gold horizontal
//! (r+1,c)-(r+1,c+1). TOTAL = n² + (n-1)² = 2n² - 2n + 1 (classic n=5 → 41).
//!
//! Move index: Type H uses idx = r*n + c; Type V uses idx = n² + r*(n-1) + c.
//! Winner via union-find over each player's dots plus two virtual side nodes —
//! the same connection pattern hex.rs / y.rs use, adapted to the dual dot grids.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

/// Move sentinel for the pie-rule swap. `u16::MAX` is out of range of every real
/// crossing index (max 2·7²-2·7+1 = 85 at n=7), so it never collides.
const SWAP: u16 = u16::MAX;

/// Render an engine move as the string the UI / apply_move speak: real crossings
/// are their numeric index, the swap sentinel is the literal `"swap"`.
fn fmt_move(m: u16) -> String {
    if m == SWAP {
        "swap".to_string()
    } else {
        format!("{m}")
    }
}

/// A dot-graph edge (two dot indices) that a claimed crossing draws.
struct Edge {
    a: usize,
    b: usize,
}

#[derive(Clone)]
struct Gale {
    /// Ownership per crossing: -1 empty, 0 = Red, 1 = Gold. Indexed 0..n_cross.
    cross: Vec<i8>,
    n: usize,
    current: u8,
    /// Pie (swap) rule: the second player may answer the opening bridge with a
    /// single `swap` instead of claiming a crossing.
    pie: bool,
    /// Half-moves played so far (used to offer `swap` only at ply 1).
    ply: u32,
}

impl Gale {
    fn new(n: usize) -> Self {
        Self { cross: vec![-1; 2 * n * n - 2 * n + 1], n, current: 0, pie: false, ply: 0 }
    }

    #[inline]
    fn n_cross(&self) -> usize {
        self.cross.len()
    }

    // --- dot indexing -------------------------------------------------------
    /// Red dots: n rows (r ∈ 0..n) × (n+1) cols (c ∈ 0..=n).
    #[inline]
    fn red_idx(&self, r: usize, c: usize) -> usize {
        r * (self.n + 1) + c
    }
    #[inline]
    fn red_count(&self) -> usize {
        self.n * (self.n + 1)
    }
    /// Gold dots: (n+1) rows (r ∈ 0..=n) × n cols (c ∈ 0..n).
    #[inline]
    fn gold_idx(&self, r: usize, c: usize) -> usize {
        r * self.n + c
    }
    #[inline]
    fn gold_count(&self) -> usize {
        (self.n + 1) * self.n
    }

    /// Decode a crossing index into `(is_type_v, r, c)`.
    fn decode(&self, idx: usize) -> (bool, usize, usize) {
        let h = self.n * self.n;
        if idx < h {
            (false, idx / self.n, idx % self.n)
        } else {
            let j = idx - h;
            (true, j / (self.n - 1), j % (self.n - 1))
        }
    }

    /// The Red edge (dot pair) a crossing draws when claimed by Red.
    fn red_edge(&self, idx: usize) -> Edge {
        let (v, r, c) = self.decode(idx);
        if !v {
            // Type H: red horizontal (r,c)-(r,c+1).
            Edge { a: self.red_idx(r, c), b: self.red_idx(r, c + 1) }
        } else {
            // Type V: red vertical (r,c+1)-(r+1,c+1).
            Edge { a: self.red_idx(r, c + 1), b: self.red_idx(r + 1, c + 1) }
        }
    }

    /// The Gold edge (dot pair) a crossing draws when claimed by Gold.
    fn gold_edge(&self, idx: usize) -> Edge {
        let (v, r, c) = self.decode(idx);
        if !v {
            // Type H: gold vertical (r,c)-(r+1,c).
            Edge { a: self.gold_idx(r, c), b: self.gold_idx(r + 1, c) }
        } else {
            // Type V: gold horizontal (r+1,c)-(r+1,c+1).
            Edge { a: self.gold_idx(r + 1, c), b: self.gold_idx(r + 1, c + 1) }
        }
    }

    /// Transpose a crossing across the main diagonal (world (x,y)→(y,x)). The
    /// board's diagonal symmetry swaps Red's left↔right role with Gold's
    /// top↔bottom role, so the transposed crossing claimed for the OTHER colour
    /// is the mirror-equivalent of the original — exactly what the pie swap needs.
    fn transpose(&self, idx: usize) -> usize {
        let (v, r, c) = self.decode(idx);
        if !v {
            // Type H (r,c) → Type H (c,r).
            c * self.n + r
        } else {
            // Type V (r,c) → Type V (c,r).
            self.n * self.n + c * (self.n - 1) + r
        }
    }

    /// Union-find connection test for `pl` (0 = Red L↔R, 1 = Gold T↔B) over that
    /// player's dot grid plus two virtual side nodes. Reads crossing ownership.
    fn connected(&self, pl: u8) -> bool {
        let (root_lo, root_hi) = self.side_roots(pl);
        root_lo == root_hi
    }

    /// Build the union-find and return the roots of the two virtual side nodes
    /// for `pl`. They are equal iff `pl` has connected.
    fn side_roots(&self, pl: u8) -> (usize, usize) {
        let (dot_count, lo_virtual, hi_virtual) = self.uf_layout(pl);
        let mut parent: Vec<usize> = (0..dot_count + 2).collect();
        // Seed: attach each dot on the low/high goal edge to its virtual node.
        self.seed_edges(pl, dot_count, lo_virtual, hi_virtual, &mut parent);
        // Draw every edge this player has claimed.
        for idx in 0..self.n_cross() {
            if self.cross[idx] == pl as i8 {
                let e = if pl == 0 { self.red_edge(idx) } else { self.gold_edge(idx) };
                uf_union(&mut parent, e.a, e.b);
            }
        }
        (uf_find(&mut parent, lo_virtual), uf_find(&mut parent, hi_virtual))
    }

    fn uf_layout(&self, pl: u8) -> (usize, usize, usize) {
        let dot_count = if pl == 0 { self.red_count() } else { self.gold_count() };
        (dot_count, dot_count, dot_count + 1)
    }

    fn seed_edges(&self, pl: u8, _dot_count: usize, lo: usize, hi: usize, parent: &mut [usize]) {
        let n = self.n;
        if pl == 0 {
            // Red: LEFT = col 0, RIGHT = col n.
            for r in 0..n {
                uf_union(parent, self.red_idx(r, 0), lo);
                uf_union(parent, self.red_idx(r, n), hi);
            }
        } else {
            // Gold: TOP = row 0, BOTTOM = row n.
            for c in 0..n {
                uf_union(parent, self.gold_idx(0, c), lo);
                uf_union(parent, self.gold_idx(n, c), hi);
            }
        }
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

    /// Side bitmask a dot touches for player `pl`: 1 = the low goal edge
    /// (Red col 0 / Gold row 0), 2 = the high goal edge (Red col n / Gold row n).
    fn dot_side_mask(&self, pl: u8, dot: usize) -> u8 {
        let n = self.n;
        let mut m = 0u8;
        if pl == 0 {
            let (_r, c) = (dot / (n + 1), dot % (n + 1));
            if c == 0 {
                m |= 1;
            }
            if c == n {
                m |= 2;
            }
        } else {
            let (r, _c) = (dot / n, dot % n);
            if r == 0 {
                m |= 1;
            }
            if r == n {
                m |= 2;
            }
        }
        m
    }

    /// Crossing indices of the winning bridge: the winner's crossings whose edges
    /// lie in the single connected component that reaches BOTH goal sides. Empty
    /// when nobody has connected. We flood the winner's edges WITHOUT the virtual
    /// side nodes (those would merge every boundary dot and drag in dead stubs),
    /// accumulate a side mask per component, and keep the component whose mask is
    /// 0b11 — the same "component touching both edges" recovery hex.rs / y.rs use.
    /// The Board maps these to drawn edges and glows them.
    fn winning_bridge(&self) -> Vec<usize> {
        let pl: u8 = if self.connected(0) {
            0
        } else if self.connected(1) {
            1
        } else {
            return Vec::new();
        };
        let (dot_count, _lo, _hi) = self.uf_layout(pl);
        let mut parent: Vec<usize> = (0..dot_count).collect();
        let mask: Vec<u8> = (0..dot_count).map(|dot| self.dot_side_mask(pl, dot)).collect();
        // Union only through drawn edges (no virtual side nodes).
        for idx in 0..self.n_cross() {
            if self.cross[idx] == pl as i8 {
                let e = if pl == 0 { self.red_edge(idx) } else { self.gold_edge(idx) };
                uf_union(&mut parent, e.a, e.b);
            }
        }
        // Accumulate each component's side mask.
        let mut comp_mask = vec![0u8; dot_count];
        for (dot, &m) in mask.iter().enumerate() {
            let root = uf_find(&mut parent, dot);
            comp_mask[root] |= m;
        }
        let mut out = Vec::new();
        for idx in 0..self.n_cross() {
            if self.cross[idx] == pl as i8 {
                let e = if pl == 0 { self.red_edge(idx) } else { self.gold_edge(idx) };
                if comp_mask[uf_find(&mut parent, e.a)] == 0b11 {
                    out.push(idx);
                }
            }
        }
        out
    }
}

fn uf_find(parent: &mut [usize], x: usize) -> usize {
    let mut x = x;
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}
fn uf_union(parent: &mut [usize], a: usize, b: usize) {
    let ra = uf_find(parent, a);
    let rb = uf_find(parent, b);
    if ra != rb {
        parent[ra] = rb;
    }
}

impl GameState for Gale {
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
            (0..self.n_cross()).filter(|&i| self.cross[i] < 0).map(|i| i as u16).collect();
        // Pie rule: the second player's reply may be a swap instead of a claim.
        if self.pie && self.ply == 1 {
            moves.push(SWAP);
        }
        moves
    }
    fn make_move(&mut self, m: &u16) {
        if *m == SWAP {
            // Claim the transposed crossing for the swapping player and clear the
            // opener. The post-swap position is a normal one-bridge position, so
            // solver / win detection need no special case.
            if let Some(i) = self.cross.iter().position(|&v| v >= 0) {
                self.cross[i] = -1;
                let t = self.transpose(i);
                self.cross[t] = self.current as i8;
            }
        } else {
            self.cross[*m as usize] = self.current as i8;
        }
        self.current = 1 - self.current;
        self.ply += 1;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}

struct GaleEval;
impl Evaluator<GaleCfg> for GaleEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Gale, m: &Vec<u16>, _: Option<SearchHandle<GaleCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Gale, _: &i64, _: SearchHandle<GaleCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct GaleCfg;
impl MCTS for GaleCfg {
    type State = Gale;
    type Eval = GaleEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct GaleWasm {
    manager: MCTSManager<GaleCfg>,
    n: usize,
    pie: bool,
}
#[wasm_bindgen]
impl GaleWasm {
    /// `n` = dots-per-side, clamped 3..=7 (default preset 5 = classic Bridg-It).
    /// `pie != 0` enables the pie (swap) rule.
    #[wasm_bindgen(constructor)]
    pub fn new(n: u32, pie: u32) -> Self {
        let n = (n as usize).clamp(3, 7);
        let pie = pie != 0;
        let mut st = Gale::new(n);
        st.pie = pie;
        Self { manager: MCTSManager::new(st, GaleCfg, GaleEval, UCTPolicy::new(1.4), ()), n, pie }
    }
    /// dots-per-side.
    pub fn size(&self) -> u32 {
        self.n as u32
    }
    pub fn playout_n(&mut self, k: u32) {
        self.manager.playout_n(k as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// One char per crossing in index order (Type H then Type V):
    /// ' '=empty, 'X'=Red, 'O'=Gold.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .cross
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

    /// Comma-joined legal move strings: crossing indices, plus the literal
    /// `"swap"` when the pie rule offers it.
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

    /// Comma-joined crossing indices of the winning bridge, or "" when nobody
    /// has connected. The UI glows the drawn edges at these crossings.
    pub fn winning_cells(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .winning_bridge()
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
        let n_cross = self.n_cross_for(self.n);
        let m: u16 = if mov == "swap" {
            SWAP
        } else {
            match mov.parse::<u16>() {
                Ok(v) if v == SWAP => SWAP,
                Ok(v) if (v as usize) < n_cross => v,
                _ => return false,
            }
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, GaleCfg, GaleEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        let mut st = Gale::new(self.n);
        st.pie = self.pie;
        self.manager = MCTSManager::new(st, GaleCfg, GaleEval, UCTPolicy::new(1.4), ());
    }

    #[inline]
    fn n_cross_for(&self, n: usize) -> usize {
        2 * n * n - 2 * n + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-derived crossing-lattice count: n² (Type H) + (n-1)² (Type V) =
    /// 2n²-2n+1. Classic Bridg-It (n=5) has 41.
    #[test]
    fn crossing_lattice_count_matches_hand_derivation() {
        for n in 3..=7 {
            let g = Gale::new(n);
            assert_eq!(g.n_cross(), 2 * n * n - 2 * n + 1);
            assert_eq!(g.n_cross(), n * n + (n - 1) * (n - 1));
        }
        assert_eq!(Gale::new(5).n_cross(), 41); // classic
        assert_eq!(Gale::new(3).n_cross(), 13);
    }

    /// Every crossing pairs exactly one Red edge with one Gold edge, and their
    /// dot-graph midpoints coincide at the crossing's world centre. Claiming the
    /// point is mutually exclusive by construction (one `cross[idx]` slot).
    #[test]
    fn each_crossing_pairs_one_red_and_one_gold_edge_at_a_shared_centre() {
        let g = Gale::new(5);
        // world centre of a dot index in each grid
        let red_world = |i: usize| ((i % (g.n + 1)) as i32 * 2, (i / (g.n + 1)) as i32 * 2 + 1);
        let gold_world = |i: usize| ((i % g.n) as i32 * 2 + 1, (i / g.n) as i32 * 2);
        for idx in 0..g.n_cross() {
            let re = g.red_edge(idx);
            let ge = g.gold_edge(idx);
            let (rax, ray) = red_world(re.a);
            let (rbx, rby) = red_world(re.b);
            let (gax, gay) = gold_world(ge.a);
            let (gbx, gby) = gold_world(ge.b);
            // midpoints (doubled to stay integral)
            let rmid = (rax + rbx, ray + rby);
            let gmid = (gax + gbx, gay + gby);
            assert_eq!(rmid, gmid, "crossing {idx}: red/gold edges must share a centre");
        }
    }

    /// Claiming a crossing for one player blocks the perpendicular opponent edge:
    /// there is exactly one ownership slot per crossing, so a claimed crossing can
    /// never simultaneously carry both edges.
    #[test]
    fn claiming_a_crossing_blocks_the_perpendicular_edge() {
        let mut g = GaleWasm::new(3, 0);
        assert!(g.apply_move("0")); // Red claims crossing 0
        // crossing 0 is no longer offered to Gold (its perpendicular gold edge is dead)
        assert!(!g.legal_moves().split(',').any(|m| m == "0"));
        assert!(!g.apply_move("0"));
    }

    #[test]
    fn red_connects_left_to_right() {
        // n=3: Red dots are 3 rows × 4 cols. A full horizontal run in row 0 uses
        // the Type-H crossings (0,0),(0,1),(0,2) = indices 0,1,2, joining c=0..c=3.
        let mut g = Gale::new(3);
        g.cross[0] = 0;
        g.cross[1] = 0;
        g.cross[2] = 0;
        assert!(g.connected(0), "row-0 horizontal bridge must join left to right");
        assert!(!g.connected(1));
        // Red just moved; it's Gold (current) to answer → Gold sees a Loss.
        g.current = 1;
        assert_eq!(g.term(), Some(ProvenValue::Loss));
        g.current = 0;
        assert_eq!(g.term(), Some(ProvenValue::Win));
    }

    #[test]
    fn gold_connects_top_to_bottom() {
        // n=3: Gold vertical run down col 0 uses Type-H gold-vertical edges at
        // crossings (0,0),(1,0),(2,0) = indices 0, 3, 6, joining r=0..r=3.
        let mut g = Gale::new(3);
        g.cross[0] = 1;
        g.cross[3] = 1;
        g.cross[6] = 1;
        assert!(g.connected(1), "col-0 vertical bridge must join top to bottom");
        assert!(!g.connected(0));
        g.current = 0;
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    /// No-draw property: for ANY 2-colouring of every crossing, exactly one
    /// player has connected (the Shannon-game theorem). Fuzz 200 random fills.
    #[test]
    fn full_board_has_exactly_one_winner() {
        let mut state = 0x1234_5678u32;
        let mut rng = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        for n in 3..=6 {
            for _ in 0..200 {
                let mut g = Gale::new(n);
                for i in 0..g.n_cross() {
                    g.cross[i] = (rng() & 1) as i8;
                }
                assert!(
                    g.connected(0) ^ g.connected(1),
                    "a full Gale board (n={n}) must have exactly one winner"
                );
            }
        }
    }

    #[test]
    fn round_trip_move_strings() {
        let mut g = GaleWasm::new(5, 0);
        // Display of a crossing is its bare index; apply_move parses it back.
        for m in ["0", "13", "40"] {
            let before = g.get_board();
            assert!(g.apply_move(m), "move {m} should be legal on a fresh board");
            assert_ne!(g.get_board(), before);
        }
        // best_move round-trips through apply_move.
        g.playout_n(50);
        let bm = g.best_move().unwrap();
        assert!(g.apply_move(&bm), "best_move {bm} must be applicable");
    }

    #[test]
    fn ai_plays() {
        let mut g = GaleWasm::new(5, 0);
        g.playout_n(300);
        assert!(g.best_move().is_some());
    }

    /// Solver proves the outcome of a tiny endgame. On n=3 with one crossing
    /// left, the forced result must be provable at modest playouts.
    #[test]
    fn solver_resolves_a_small_endgame() {
        // Play a full n=3 game to termination with the AI on both seats; the
        // solver-backed search must always reach a decided (non-empty) result.
        let mut g = GaleWasm::new(3, 0);
        for _ in 0..40 {
            if g.is_terminal() {
                break;
            }
            let m = g.weak_move(200, 1, 0.0, 7).or_else(|| g.best_move());
            match m {
                Some(mv) => assert!(g.apply_move(&mv), "engine emitted an unplayable move: {mv}"),
                None => break,
            }
        }
        assert!(g.is_terminal());
        assert!(!g.result().is_empty(), "n=3 Gale is decisive — no draws");
    }

    #[test]
    fn winning_cells_empty_mid_game() {
        let mut g = GaleWasm::new(5, 0);
        assert_eq!(g.winning_cells(), "", "empty board has no bridge");
        assert!(g.apply_move("0"));
        assert_eq!(g.winning_cells(), "", "one crossing connects nothing");
    }

    #[test]
    fn winning_cells_reports_the_connecting_bridge() {
        let mut g = Gale::new(3);
        g.cross[0] = 0;
        g.cross[1] = 0;
        g.cross[2] = 0;
        // A stray, disconnected Red crossing that must NOT be reported: index 6
        // is Type-H (r=2,c=0) whose red edge is row-2 horizontal, not touching
        // the winning row-0 bridge.
        g.cross[6] = 0;
        assert!(g.connected(0));
        let mut got = g.winning_bridge();
        got.sort_unstable();
        assert_eq!(got, vec![0, 1, 2], "only the row-0 bridge, not the stray crossing");
    }

    // --- pie rule (ships: the diagonal transpose swaps Red↔Gold roles) --------

    #[test]
    fn transpose_is_an_involution_on_the_lattice() {
        for n in 3..=7 {
            let g = Gale::new(n);
            for idx in 0..g.n_cross() {
                assert_eq!(g.transpose(g.transpose(idx)), idx, "transpose must be its own inverse");
                assert!(g.transpose(idx) < g.n_cross(), "transpose stays on the lattice");
            }
        }
    }

    #[test]
    fn pie_swap_recolors_the_mirrored_crossing() {
        let mut g = GaleWasm::new(5, 1);
        assert!(g.apply_move("1")); // Red claims Type-H crossing (0,1)
        assert!(g.legal_moves().split(',').any(|m| m == "swap"));
        assert!(g.apply_move("swap"));
        let b = g.get_board();
        // Original crossing cleared…
        assert_eq!(b.chars().nth(1), Some(' '));
        // …and its transpose (Type-H (1,0) = index 5) is now Gold's.
        assert_eq!(b.chars().nth(5), Some('O'));
        assert_eq!(g.current_player(), 0); // back to Red
    }

    #[test]
    fn decline_swap_plays_normally() {
        let mut g = GaleWasm::new(5, 1);
        assert!(g.apply_move("10"));
        assert!(g.legal_moves().split(',').any(|m| m == "swap"));
        assert!(g.apply_move("11")); // Gold declines, claims a normal crossing
        assert!(!g.legal_moves().split(',').any(|m| m == "swap"));
        assert_eq!(g.current_player(), 0);
        assert!(!g.is_terminal());
    }

    #[test]
    fn no_swap_when_pie_disabled() {
        let mut g = GaleWasm::new(5, 0);
        assert!(g.apply_move("10"));
        assert!(!g.legal_moves().split(',').any(|m| m == "swap"));
        assert!(!g.apply_move("swap"));
    }

    #[test]
    fn full_game_after_swap_terminates() {
        let mut g = GaleWasm::new(5, 1);
        assert!(g.apply_move("7"));
        assert!(g.apply_move("swap"));
        for _ in 0..120 {
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
}
