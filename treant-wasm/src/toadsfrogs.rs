//! Toads & Frogs — John Conway's one-dimensional hopping game, a classic of
//! combinatorial game theory. A single row of squares holds Toads (Left player)
//! and Frogs (Right player). Toads only ever move RIGHT, Frogs only ever move
//! LEFT. The row only ever "progresses" (every piece's coordinate moves strictly
//! toward the far end), so every playout terminates — and the board is tiny, so
//! treant's exact solver plays it perfectly.
//!
//! SOURCE (rules verified against, encoded below):
//! * Wikipedia, "Toads and Frogs", https://en.wikipedia.org/wiki/Toads_and_Frogs
//!   — "played on a strip of squares"; toads start on the leftmost squares,
//!   frogs on the rightmost, with empty squares between.
//!   MOVES (quoted): a toad may "move a toad one square to the right, into an
//!   empty square" (SLIDE) or "hop a toad two squares to the right, over a frog,
//!   into an empty square" (HOP); a frog moves/hops symmetrically to the LEFT
//!   over a toad. PROHIBITED (quoted): "Hops over an empty square, a toad, or
//!   more than one square are not allowed" — i.e. no backward moves, no hopping
//!   your own kind, no captures (the hopped piece stays put). WIN (quoted):
//!   "Under the normal play rule … the first player to be unable to move on
//!   their turn loses."
//!
//! SOURCED RULE DECISIONS (each with a test below):
//! * DIRECTION: Toads (seat 0) move only right (+1); Frogs (seat 1) move only
//!   left (−1). No backward moves. → tests `toad_slides_right` / `frog_slides_left`
//!   / `no_backward_move`.
//! * HOP: over exactly one ADJACENT opposing piece into the empty square
//!   immediately beyond; you may not hop your own kind, and the hopped piece is
//!   NOT captured. → tests `toad_hops_frog` / `frog_hops_toad` / `no_friendly_hop`.
//! * NO-MOVE RULE: the player to move with no legal move LOSES (normal play). →
//!   test `blocked_mover_loses`.
//!
//! SOURCED SOLVED VALUES (the CGT showcase — the solver reproduces them):
//! Wikipedia lists exact game values for small positions. Value 0 ⇒ the SECOND
//! player wins (the mover loses); a positive value ⇒ Left/Toads wins no matter
//! who moves first. We assert three of them against an independent minimax over
//! this engine's own move generator (`solved_value_*` tests):
//!   * "T□□F = 0"   → mover (Toads) loses; Frogs win.               (`new(1,1,2)`)
//!   * "TF□□ = 1"   → Toads win regardless of who moves first.
//!   * "T□F□ = 1/2" → Toads win regardless of who moves first.
//!
//! We also drive the wasm solver through best-play self-play from T□□F,
//! asserting it lands on the Frogs win (`solver_plays_toadsdoubleframe_perfectly`).
//!
//! DEFERRED — MULTI-ROW "SUMS": the knob bible's multi-row disjunctive-sum
//! variant (play any one row per turn) is deferred; we ship the single-row game.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

const EMPTY: i8 = -1;
const TOAD: i8 = 0; // owned by seat 0 (Left, faces/moves right)
const FROG: i8 = 1; // owned by seat 1 (Right, faces/moves left)

#[derive(Clone)]
struct Toads {
    /// One entry per square: EMPTY / TOAD / FROG. A cell's piece value equals the
    /// seat that owns it, so `cells[i] == current` marks a mover-owned piece.
    cells: Vec<i8>,
    current: u8,
    // Retained for `reset`.
    toads: usize,
    frogs: usize,
    gaps: usize,
}

impl Toads {
    /// Canonical start: toads packed left, frogs packed right, gaps between.
    /// Length is DERIVED as `toads + gaps + frogs` — the tile exposes toads/
    /// frogs/gaps knobs and shows the derived length in its preview, so the UI
    /// can never request an impossible (length ≠ toads+frogs+gaps) combo. (The
    /// phase plan offered "derive length" or "clamp counts"; we derive.)
    fn new(toads: usize, frogs: usize, gaps: usize) -> Self {
        let toads = toads.clamp(1, 6);
        let frogs = frogs.clamp(1, 6);
        let gaps = gaps.clamp(1, 3);
        let mut cells = Vec::with_capacity(toads + gaps + frogs);
        cells.extend(std::iter::repeat_n(TOAD, toads));
        cells.extend(std::iter::repeat_n(EMPTY, gaps));
        cells.extend(std::iter::repeat_n(FROG, frogs));
        Self { cells, current: 0, toads, frogs, gaps }
    }

    fn len(&self) -> usize {
        self.cells.len()
    }

    /// Movement direction for a seat: toads step +1 (right), frogs −1 (left).
    fn dir(player: u8) -> isize {
        if player == TOAD as u8 {
            1
        } else {
            -1
        }
    }

    /// The forced destination for a legal move originating at `i` (slide first,
    /// else hop) — the two are mutually exclusive, so the origin index alone
    /// encodes the move. Returns None if `i` has no legal move.
    fn dest(&self, i: usize) -> Option<usize> {
        let p = self.cells[i];
        if p != TOAD && p != FROG {
            return None;
        }
        let dir = Self::dir(p as u8);
        let l = self.len() as isize;
        let opp = if p == TOAD { FROG } else { TOAD };
        let slide = i as isize + dir;
        if slide >= 0 && slide < l && self.cells[slide as usize] == EMPTY {
            return Some(slide as usize); // SLIDE one square forward into empty
        }
        let over = i as isize + dir;
        let hop = i as isize + 2 * dir;
        if hop >= 0
            && hop < l
            && self.cells[over as usize] == opp // exactly one adjacent OPPOSING piece
            && self.cells[hop as usize] == EMPTY
        {
            return Some(hop as usize); // HOP over it into the empty square beyond
        }
        None
    }

    /// Legal moves = origin indices of mover-owned pieces that can slide or hop.
    fn gen(&self) -> Vec<u16> {
        let cur = self.current as i8;
        (0..self.len())
            .filter(|&i| self.cells[i] == cur && self.dest(i).is_some())
            .map(|i| i as u16)
            .collect()
    }

    fn term(&self) -> Option<ProvenValue> {
        // Normal play: the player to move with no legal move loses.
        if self.gen().is_empty() {
            Some(ProvenValue::Loss)
        } else {
            None
        }
    }
}

impl GameState for Toads {
    type Move = u16;
    type Player = u8;
    type MoveList = Vec<u16>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<u16> {
        self.gen()
    }
    fn make_move(&mut self, m: &u16) {
        let i = *m as usize;
        let to = self.dest(i).expect("apply_move validates legality before make_move");
        self.cells[to] = self.cells[i];
        self.cells[i] = EMPTY;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}

struct TfEval;
impl Evaluator<TfCfg> for TfEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Toads, m: &Vec<u16>, _: Option<SearchHandle<TfCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Toads, _: &i64, _: SearchHandle<TfCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct TfCfg;
impl MCTS for TfCfg {
    type State = Toads;
    type Eval = TfEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true // tiny, CGT-solved game — exact solving is the showcase
    }
}

#[wasm_bindgen]
pub struct ToadsFrogsWasm {
    manager: MCTSManager<TfCfg>,
    toads: usize,
    frogs: usize,
    gaps: usize,
}
#[wasm_bindgen]
impl ToadsFrogsWasm {
    /// `toads`/`frogs` are clamped 1..=6, `gaps` 1..=3; the row LENGTH is derived
    /// as `toads + gaps + frogs`. Toads pack the left, frogs the right, gaps
    /// between (the canonical start).
    #[wasm_bindgen(constructor)]
    pub fn new(toads: u32, frogs: u32, gaps: u32) -> Self {
        let s = Toads::new(toads as usize, frogs as usize, gaps as usize);
        let (t, f, g) = (s.toads, s.frogs, s.gaps);
        Self { manager: MCTSManager::new(s, TfCfg, TfEval, UCTPolicy::new(1.4), ()), toads: t, frogs: f, gaps: g }
    }
    /// Derived row length (toads + gaps + frogs).
    pub fn length(&self) -> u32 {
        self.manager.tree().root_state().len() as u32
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// One char per square: ' '=empty, 'T'=toad, 'F'=frog.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .cells
            .iter()
            .map(|&v| match v {
                TOAD => 'T',
                FROG => 'F',
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
            // Mover has no move ⇒ mover loses ⇒ the OTHER seat wins (1-indexed).
            Some(ProvenValue::Loss) => format!("{}", (1 - s.current) + 1),
            _ => String::new(),
        }
    }
    /// Comma-joined origin indices of the mover's legal moves (destination is
    /// forced, so the origin alone is the move encoding).
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
        self.manager.best_move().map(|m| m.to_string())
    }
    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m: u16 = match mov.parse() {
            Ok(v) => v,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, TfCfg, TfEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        let s = Toads::new(self.toads, self.frogs, self.gaps);
        self.manager = MCTSManager::new(s, TfCfg, TfEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an arbitrary position from a compact string ("T", "F", "_"/" ")
    /// with a stated player to move — for encoding source positions like T□□F.
    fn pos(s: &str, current: u8) -> Toads {
        let cells: Vec<i8> = s
            .chars()
            .map(|c| match c {
                'T' => TOAD,
                'F' => FROG,
                _ => EMPTY,
            })
            .collect();
        Toads { cells, current, toads: 0, frogs: 0, gaps: 0 }
    }

    #[test]
    fn start_position_packs_toads_left_frogs_right() {
        let s = Toads::new(3, 3, 2); // Classic TTT__FFF
        assert_eq!(s.len(), 8);
        assert_eq!(s.cells, vec![TOAD, TOAD, TOAD, EMPTY, EMPTY, FROG, FROG, FROG]);
        assert_eq!(s.current, 0, "Toads (seat 0) move first");
    }

    #[test]
    fn constructor_clamps_and_derives_length() {
        let s = Toads::new(99, 0, 99); // toads→6, frogs→1, gaps→3
        assert_eq!((s.toads, s.frogs, s.gaps), (6, 1, 3));
        assert_eq!(s.len(), 10);
    }

    #[test]
    fn toad_slides_right() {
        // T_ …  toad at 0 slides into the empty cell 1.
        let s = pos("T_F", 0);
        assert_eq!(s.dest(0), Some(1), "toad slides one square RIGHT");
    }

    #[test]
    fn frog_slides_left() {
        // …_F  frog at 2 slides into the empty cell 1.
        let s = pos("T_F", 1);
        assert_eq!(s.dest(2), Some(1), "frog slides one square LEFT");
    }

    #[test]
    fn no_backward_move() {
        // _T : a toad with only an empty square BEHIND it (to the left) and the
        // wall ahead has no move. Frogs, symmetrically, never move right.
        let s = pos("_T", 0);
        assert_eq!(s.dest(1), None, "a toad never moves left (backward)");
        let s = pos("F_", 1);
        assert_eq!(s.dest(0), None, "a frog never moves right (backward)");
    }

    #[test]
    fn toad_hops_frog() {
        // TF_ : toad at 0 hops over the frog at 1 into empty cell 2.
        let s = pos("TF_", 0);
        assert_eq!(s.dest(0), Some(2), "toad hops the adjacent frog");
        // …but not if the landing square is occupied.
        let s = pos("TFF", 0);
        assert_eq!(s.dest(0), None, "no hop when the square beyond is occupied");
    }

    #[test]
    fn frog_hops_toad() {
        // _TF : frog at 2 hops over the toad at 1 into empty cell 0.
        let s = pos("_TF", 1);
        assert_eq!(s.dest(2), Some(0), "frog hops the adjacent toad");
    }

    #[test]
    fn no_friendly_hop() {
        // TT_ : a toad may not hop its own kind (only an opposing piece).
        let s = pos("TT_", 0);
        assert_eq!(s.dest(0), None, "no hopping a friendly toad");
        let s = pos("_FF", 1);
        assert_eq!(s.dest(2), None, "no hopping a friendly frog");
    }

    #[test]
    fn hopped_piece_is_not_captured() {
        // TF_ : after the toad hops, the frog remains on the board (no capture).
        let mut s = pos("TF_", 0);
        s.make_move(&0);
        assert_eq!(s.cells, vec![EMPTY, FROG, TOAD], "the hopped frog stays put");
    }

    #[test]
    fn blocked_mover_loses() {
        // FT : it is Toads' turn but the toad (at 1) is at the wall with a frog
        // behind — no slide, no forward hop. Toads have no move ⇒ Toads lose,
        // so Frogs (seat 1) win → result "2".
        let s = pos("FT", 0);
        assert!(s.gen().is_empty(), "the mover has no legal move");
        assert_eq!(s.term(), Some(ProvenValue::Loss), "a boxed-in mover loses (normal play)");
    }

    // ---- Independent minimax over this engine's own move generator, used to
    // ---- reproduce Wikipedia's exact solved values. Returns true iff the
    // ---- player to move wins with perfect play (normal-play convention).
    fn mover_wins(s: &Toads) -> bool {
        let moves = s.gen();
        if moves.is_empty() {
            return false; // no move ⇒ the mover loses
        }
        moves.iter().any(|&m| {
            let mut c = s.clone();
            c.make_move(&m);
            !mover_wins(&c) // a move that leaves the opponent losing wins
        })
    }

    #[test]
    fn solved_value_tgapgapf_is_zero_second_player_wins() {
        // Wikipedia: "T□□F = 0" — value 0 ⇒ the SECOND player wins, i.e. whoever
        // moves FIRST loses. Verify both seats-to-move lose the first move.
        assert!(!mover_wins(&pos("T__F", 0)), "Toads-to-move loses T□□F (value 0)");
        assert!(!mover_wins(&pos("T__F", 1)), "Frogs-to-move loses T□□F (value 0)");
        // And it is exactly the canonical `new(1,1,2)` start position.
        assert_eq!(Toads::new(1, 1, 2).cells, pos("T__F", 0).cells);
    }

    #[test]
    fn solved_value_tfgapgap_is_one_left_wins() {
        // Wikipedia: "TF□□ = 1" — positive ⇒ Left/Toads wins regardless of who
        // starts.
        assert!(mover_wins(&pos("TF__", 0)), "Toads-to-move wins TF□□ (value 1)");
        assert!(!mover_wins(&pos("TF__", 1)), "Frogs-to-move still lose TF□□ (Toads win)");
    }

    #[test]
    fn solved_value_t_f_is_half_left_wins() {
        // Wikipedia: "T□F□ = 1/2" — positive ⇒ Left/Toads wins regardless of who
        // starts.
        assert!(mover_wins(&pos("T_F_", 0)), "Toads-to-move wins T□F□ (value 1/2)");
        assert!(!mover_wins(&pos("T_F_", 1)), "Frogs-to-move still lose T□F□ (Toads win)");
    }

    #[test]
    fn solver_plays_toadsdoubleframe_perfectly() {
        // Drive the wasm solver through best-play self-play from T□□F = new(1,1,2).
        // Value 0 ⇒ the first mover (Toads) cannot avoid losing, so the game must
        // end with Frogs (seat 1) winning → result "2".
        let mut g = ToadsFrogsWasm::new(1, 1, 2);
        assert_eq!(g.get_board(), "T  F");
        for _ in 0..16 {
            if g.is_terminal() {
                break;
            }
            g.playout_n(800); // ample for the solver to prove this 4-cell tree
            let m = g.best_move().expect("a non-terminal position has a best move");
            assert!(g.apply_move(&m), "best_move {m} must be applicable");
        }
        assert!(g.is_terminal(), "self-play must terminate");
        assert_eq!(g.result(), "2", "T□□F is a second-player (Frogs) win");
    }

    #[test]
    fn round_trip_move_strings() {
        let mut g = ToadsFrogsWasm::new(3, 3, 2);
        let before = g.get_board();
        let first = g.legal_moves().split(',').next().unwrap().to_string();
        assert!(g.apply_move(&first), "a legal origin index round-trips");
        assert_ne!(g.get_board(), before);
        assert!(!g.apply_move("999"), "an out-of-range index is rejected");
        assert!(!g.apply_move("frog"), "a non-numeric move is rejected");
    }

    #[test]
    fn ai_plays() {
        let mut g = ToadsFrogsWasm::new(3, 3, 2);
        g.playout_n(400);
        assert!(g.best_move().is_some());
    }

    #[test]
    fn full_game_terminates_and_is_decisive() {
        // Self-play across a spread of presets; the game must always end (pieces
        // only ever advance) with a decided, non-empty result (never a draw).
        for (t, f, gp) in [(1, 1, 1), (2, 2, 1), (3, 3, 2), (6, 6, 3), (4, 2, 3)] {
            let mut g = ToadsFrogsWasm::new(t, f, gp);
            let cap = (t + f + gp) * (t + f + gp) + 10;
            for _ in 0..cap {
                if g.is_terminal() {
                    break;
                }
                let m = g.weak_move(100, 2, 0.4, 9).or_else(|| g.best_move());
                match m {
                    Some(mv) => assert!(g.apply_move(&mv), "engine emitted an unplayable move: {mv}"),
                    None => break,
                }
            }
            assert!(g.is_terminal(), "({t},{f},{gp}) game must terminate");
            assert!(!g.result().is_empty(), "Toads & Frogs is decisive — no draws");
        }
    }
}
