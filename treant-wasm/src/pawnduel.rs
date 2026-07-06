//! Pawn Duel — a chess-pawns-only duel on an N-files × M-ranks board. One rank of
//! pawns each; reach the far rank (promote), capture every enemy pawn, or leave
//! the opponent with no move to win. Two rule knobs turn it into a family of
//! games — most famously **Hexapawn** (Martin Gardner, 1962): a 3×3 board with
//! double-step and en-passant OFF, a classic teaching game whose full game tree
//! the solver proves out.
//!
//! GEOMETRY: pawns start on the two OUTERMOST ranks (seat 0 on the bottom edge
//! row `rows-1`, seat 1 on the top edge row 0), one pawn per file, mirroring
//! Frontline's orientation (seat 0 moves UP toward row 0; seat 1 moves DOWN
//! toward row `rows-1`). The phase plan's prose ("row 2 and row M-1") describes
//! chess's second-rank start, but a single rank on each *edge* is what makes the
//! 3-rank Hexapawn preset fall out cleanly (pawns on rows 0 and 2 with an empty
//! middle) AND keeps the geometry identical to Frontline — so we reuse its board.
//! The "starting rank" for the double-step is therefore each side's edge row.
//!
//! RULES — the pawn subset of the FIDE Laws of Chess (Handbook E.I.01, Article
//! 3.7 "The pawn"), cited inline against `gen`/`term`:
//!
//! - 3.7.1 move one square straight forward to an empty square (`gen` forward-one).
//! - 3.7.2 from its starting square, optionally advance TWO squares if both are
//!   empty — the `double_step` knob (`gen` forward-two).
//! - 3.7.3 capture one square diagonally forward onto an enemy pawn (`gen` capture).
//! - 3.7.4.1 EN PASSANT — the `en_passant` knob: a pawn attacking the square a
//!   just-double-stepped enemy pawn CROSSED may capture it as though it had moved
//!   only one square; "this capture is only legal on the move following this
//!   advance" — a one-ply window (`ep_target`, `gen` ep branch).
//! - 3.7.5.1 PROMOTION — reaching the far rank; here promotion is an instant WIN,
//!   there being no other pieces to promote to (`term` promotion).
//!
//! WIN (any of): promote a pawn to the far rank, capture ALL enemy pawns, or the
//! opponent (to move) has no legal move.
//!
//! ⚠️ DELIBERATE DEVIATION FROM CHESS — STALEMATE IS A LOSS, NOT A DRAW. In
//! standard chess a player with no legal move (and not in check) is stalemated
//! and the game is a DRAW (FIDE Art. 5.2.1 / 9.6). Pawn Duel instead scores a
//! stuck mover as a LOSS (`term` returns `ProvenValue::Loss` for the player to
//! move). This is the Breakthrough/Hexapawn convention: it keeps every game
//! decisive (no draws), which the exact solver relies on and which matches the
//! known Hexapawn result. This deviation is ALSO surfaced in the player-facing
//! rules text (docs `rules.ts` / the tile's `rules`).
//!
//! TERMINATION (why every MCTS playout ends): each move either advances a pawn
//! strictly forward (a bounded, monotone-increasing potential — total pawn
//! advancement, capped at `pawns × (rows-1)`) or captures (strictly reducing the
//! pawn count). Under the lexicographic measure (count ↓, then advancement ↑) no
//! line can cycle, so random rollouts always terminate — Watch-AI mode is safe.
use crate::gridlib::idx;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

const EMPTY: i8 = -1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PdMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for PdMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

#[derive(Clone)]
struct PawnDuel {
    board: Vec<i8>, // -1 empty, 0 = seat 0 (moves up), 1 = seat 1 (moves down)
    cols: usize,
    rows: usize,
    current: u8,
    /// The en-passant target square: the empty square a pawn CROSSED on its last
    /// double-step, capturable *only* on the immediately following ply. `None`
    /// whenever the previous move was not a double-step — this is the one-ply
    /// window (FIDE 3.7.4.1). Part of the game state, so replaying the move log
    /// on a fresh engine reconstructs it by construction.
    ep_target: Option<u16>,
    double_step: bool,
    en_passant: bool,
}
impl PawnDuel {
    fn new(cols: usize, rows: usize, double_step: bool, en_passant: bool) -> Self {
        let mut board = vec![EMPTY; cols * rows];
        for c in 0..cols {
            board[idx(rows - 1, c, cols)] = 0; // seat 0 on the bottom edge
            board[idx(0, c, cols)] = 1; // seat 1 on the top edge
        }
        Self { board, cols, rows, current: 0, ep_target: None, double_step, en_passant }
    }
    fn count(&self, p: i8) -> usize {
        self.board.iter().filter(|&&v| v == p).count()
    }
    /// Forward row-step for a seat: seat 0 moves up (−1), seat 1 down (+1).
    fn fwd(p: u8) -> i32 {
        if p == 0 {
            -1
        } else {
            1
        }
    }
    /// Each seat's starting (edge) rank — where the double-step is allowed from.
    fn start_rank(&self, p: u8) -> usize {
        if p == 0 {
            self.rows - 1
        } else {
            0
        }
    }
    /// Each seat's promotion (far) rank.
    fn promo_rank(&self, p: u8) -> usize {
        if p == 0 {
            0
        } else {
            self.rows - 1
        }
    }
    fn gen(&self) -> Vec<PdMove> {
        let mut v = Vec::new();
        let p = self.current;
        let opp = (1 - p) as i8;
        let fwd = Self::fwd(p);
        for r in 0..self.rows {
            for c in 0..self.cols {
                if self.board[idx(r, c, self.cols)] != p as i8 {
                    continue;
                }
                let nr = r as i32 + fwd;
                if nr < 0 || nr >= self.rows as i32 {
                    continue; // a pawn on the far rank has already won; nothing to gen
                }
                let nr = nr as usize;
                let from = idx(r, c, self.cols) as u16;
                // 3.7.1 forward one — into an empty square only.
                if self.board[idx(nr, c, self.cols)] == EMPTY {
                    v.push(PdMove { from, to: idx(nr, c, self.cols) as u16 });
                    // 3.7.2 forward two — from the starting rank, both squares empty.
                    if self.double_step && r == self.start_rank(p) {
                        let nr2 = r as i32 + 2 * fwd;
                        if nr2 >= 0 && nr2 < self.rows as i32 && self.board[idx(nr2 as usize, c, self.cols)] == EMPTY {
                            v.push(PdMove { from, to: idx(nr2 as usize, c, self.cols) as u16 });
                        }
                    }
                }
                // 3.7.3 diagonal capture + 3.7.4.1 en passant.
                for dc in [-1i32, 1] {
                    let nc = c as i32 + dc;
                    if nc < 0 || nc >= self.cols as i32 {
                        continue;
                    }
                    let tgt = idx(nr, nc as usize, self.cols) as u16;
                    let occ = self.board[tgt as usize];
                    if occ == opp {
                        v.push(PdMove { from, to: tgt }); // ordinary diagonal capture
                    } else if self.en_passant && occ == EMPTY && self.ep_target == Some(tgt) {
                        // The crossed square is empty and flagged; the enemy pawn to
                        // remove sits beside us at (r, nc) — validated in make_move.
                        v.push(PdMove { from, to: tgt });
                    }
                }
            }
        }
        v
    }
    fn term(&self) -> Option<ProvenValue> {
        let s0_promoted = (0..self.cols).any(|c| self.board[idx(self.promo_rank(0), c, self.cols)] == 0);
        let s1_promoted = (0..self.cols).any(|c| self.board[idx(self.promo_rank(1), c, self.cols)] == 1);
        let s0_wins = s0_promoted || self.count(1) == 0;
        let s1_wins = s1_promoted || self.count(0) == 0;
        if s0_wins {
            return Some(if self.current == 0 { ProvenValue::Win } else { ProvenValue::Loss });
        }
        if s1_wins {
            return Some(if self.current == 1 { ProvenValue::Win } else { ProvenValue::Loss });
        }
        // STALEMATE-AS-LOSS (deviation from chess, see header): the mover with no
        // legal move loses rather than drawing.
        if self.gen().is_empty() {
            return Some(ProvenValue::Loss);
        }
        None
    }
    /// The cell to glow on a promotion win — the pawn that reached the far rank.
    /// `None` for an elimination/stalemate win (no line to highlight) or mid-game.
    fn winning_cell(&self) -> Option<usize> {
        self.term()?;
        if let Some(c) = (0..self.cols).find(|&c| self.board[idx(self.promo_rank(0), c, self.cols)] == 0) {
            return Some(idx(self.promo_rank(0), c, self.cols));
        }
        if let Some(c) = (0..self.cols).find(|&c| self.board[idx(self.promo_rank(1), c, self.cols)] == 1) {
            return Some(idx(self.promo_rank(1), c, self.cols));
        }
        None
    }
    fn eval0(&self) -> i64 {
        // Reward advancement toward the far rank (mirrors Frontline): seat 0 gains
        // as it climbs toward row 0, seat 1 as it descends toward row rows-1.
        let mut s = 0i64;
        for r in 0..self.rows {
            for c in 0..self.cols {
                match self.board[idx(r, c, self.cols)] {
                    0 => s += 10 + (self.rows as i64 - 1 - r as i64),
                    1 => s -= 10 + r as i64,
                    _ => {}
                }
            }
        }
        s
    }
}
impl GameState for PawnDuel {
    type Move = PdMove;
    type Player = u8;
    type MoveList = Vec<PdMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<PdMove> {
        if self.term().is_some() {
            return vec![];
        }
        self.gen()
    }
    fn make_move(&mut self, m: &PdMove) {
        let (from, to) = (m.from as usize, m.to as usize);
        let (fr, fc) = (from / self.cols, from % self.cols);
        let (tr, tc) = (to / self.cols, to % self.cols);
        let p = self.current;
        // En passant: a DIAGONAL move (fc != tc) into the flagged empty crossed
        // square removes the enemy pawn that is BESIDE us — same rank fr, file tc.
        let is_ep = self.en_passant && self.ep_target == Some(m.to) && self.board[to] == EMPTY && fc != tc;
        // A fresh double-step re-arms the one-ply window; every other move clears it.
        let new_ep = if fc == tc && (tr as i32 - fr as i32).abs() == 2 {
            Some(idx((fr + tr) / 2, fc, self.cols) as u16)
        } else {
            None
        };
        self.board[to] = p as i8;
        self.board[from] = EMPTY;
        if is_ep {
            self.board[idx(fr, tc, self.cols)] = EMPTY;
        }
        self.ep_target = new_ep;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct PdEval;
impl Evaluator<PdCfg> for PdEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &PawnDuel, m: &Vec<PdMove>, _: Option<SearchHandle<PdCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], s.eval0())
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 {
            *e
        } else {
            -*e
        }
    }
    fn evaluate_existing_state(&self, s: &PawnDuel, _: &i64, _: SearchHandle<PdCfg>) -> i64 {
        s.eval0()
    }
}
#[derive(Default)]
struct PdCfg;
impl MCTS for PdCfg {
    type State = PawnDuel;
    type Eval = PdEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true // no draws (stalemate-as-loss) — the solver proves small boards exactly
    }
}

#[wasm_bindgen]
pub struct PawnDuelWasm {
    manager: MCTSManager<PdCfg>,
    cols: usize,
    rows: usize,
    double_step: bool,
    en_passant: bool,
}
#[wasm_bindgen]
impl PawnDuelWasm {
    /// `files` clamp 3..=10, `ranks` 3..=8 (min 3 admits the Hexapawn 3×3 preset;
    /// the tile's custom knobs stay in the saner 4–10 / 5–8 range). `double_step`
    /// and `en_passant` are 0/1 flags.
    #[wasm_bindgen(constructor)]
    pub fn new(files: u32, ranks: u32, double_step: u32, en_passant: u32) -> Self {
        let cols = (files as usize).clamp(3, 10);
        let rows = (ranks as usize).clamp(3, 8);
        let (ds, ep) = (double_step != 0, en_passant != 0);
        Self {
            manager: MCTSManager::new(PawnDuel::new(cols, rows, ds, ep), PdCfg, PdEval, UCTPolicy::new(1.4), ()),
            cols,
            rows,
            double_step: ds,
            en_passant: ep,
        }
    }
    pub fn cols(&self) -> u32 {
        self.cols as u32
    }
    pub fn rows(&self) -> u32 {
        self.rows as u32
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// One char per cell: ' '=empty, 'X'=seat 0, 'O'=seat 1 (Frontline's encoding,
    /// so the shared pawn board renders it unchanged).
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
    /// The current en-passant target square index, or -1 if none. Exposes the
    /// one-ply capture window as part of the observable state (the UI shows it as
    /// a legal move target via `legal_moves`; this accessor makes it inspectable).
    pub fn en_passant_target(&self) -> i32 {
        match self.manager.tree().root_state().ep_target {
            Some(t) => t as i32,
            None => -1,
        }
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
    /// Comma-separated "from-to" legal moves for the current player (single source
    /// of the move encoding — the UI parses the same strings it plays back).
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
    /// 0-based promotion cell to glow, or "" for an elimination/stalemate win.
    pub fn winning_cells(&self) -> String {
        match self.manager.tree().root_state().winning_cell() {
            Some(i) => i.to_string(),
            None => String::new(),
        }
    }
    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let parts: Vec<&str> = mov.split('-').collect();
        if parts.len() != 2 {
            return false;
        }
        let (from, to): (u16, u16) = match (parts[0].parse(), parts[1].parse()) {
            (Ok(a), Ok(b)) => (a, b),
            _ => return false,
        };
        let m = PdMove { from, to };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, PdCfg, PdEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        let s = PawnDuel::new(self.cols, self.rows, self.double_step, self.en_passant);
        self.manager = MCTSManager::new(s, PdCfg, PdEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn i(r: usize, c: usize, cols: usize) -> u16 {
        idx(r, c, cols) as u16
    }

    #[test]
    fn start_places_one_rank_each_on_the_edges() {
        let g = PawnDuel::new(8, 6, true, true);
        // seat 0 fills the bottom row, seat 1 the top row, nothing between.
        assert!((0..8).all(|c| g.board[idx(5, c, 8)] == 0));
        assert!((0..8).all(|c| g.board[idx(0, c, 8)] == 1));
        assert_eq!(g.count(0), 8);
        assert_eq!(g.count(1), 8);
        assert_eq!(g.current, 0, "seat 0 moves first");
    }

    #[test]
    fn double_step_only_from_the_starting_rank() {
        // With double_step on, a seat-0 edge pawn may go one OR two forward.
        let g = PawnDuel::new(4, 6, true, false);
        let from = i(5, 0, 4);
        let ms = g.gen();
        assert!(ms.contains(&PdMove { from, to: i(4, 0, 4) }), "single step");
        assert!(ms.contains(&PdMove { from, to: i(3, 0, 4) }), "double step from start rank");
        // Move it one square forward; now it is NOT on the start rank — no double.
        let mut g2 = g.clone();
        g2.make_move(&PdMove { from, to: i(4, 0, 4) });
        g2.current = 0; // pretend it is seat 0's turn again for a focused check
        let ms2 = g2.gen();
        assert!(ms2.contains(&PdMove { from: i(4, 0, 4), to: i(3, 0, 4) }), "single step still legal");
        assert!(!ms2.contains(&PdMove { from: i(4, 0, 4), to: i(2, 0, 4) }), "no double step off the start rank");
    }

    #[test]
    fn double_step_disabled_offers_only_single() {
        let g = PawnDuel::new(4, 6, false, false);
        let from = i(5, 0, 4);
        let ms = g.gen();
        assert!(ms.contains(&PdMove { from, to: i(4, 0, 4) }));
        assert!(!ms.contains(&PdMove { from, to: i(3, 0, 4) }), "double step off when knob is 0");
    }

    #[test]
    fn diagonal_capture_only() {
        // seat-0 pawn at (3,1) with enemies on BOTH forward diagonals (2,0)/(2,2)
        // and one directly ahead (2,1). Only the two diagonals are capturable; the
        // head-on enemy also BLOCKS the forward-one move.
        let mut g = PawnDuel::new(4, 6, false, false);
        g.board = vec![EMPTY; 24];
        g.board[idx(3, 1, 4)] = 0;
        g.board[idx(2, 0, 4)] = 1;
        g.board[idx(2, 1, 4)] = 1;
        g.board[idx(2, 2, 4)] = 1;
        g.current = 0;
        let ms: Vec<_> = g.gen().into_iter().filter(|m| m.from == i(3, 1, 4)).collect();
        assert!(ms.contains(&PdMove { from: i(3, 1, 4), to: i(2, 0, 4) }), "capture the left diagonal enemy");
        assert!(ms.contains(&PdMove { from: i(3, 1, 4), to: i(2, 2, 4) }), "capture the right diagonal enemy");
        assert!(!ms.iter().any(|m| m.to == i(2, 1, 4)), "no straight-forward capture");
        assert_eq!(ms.len(), 2, "exactly the two diagonal captures — the head-on enemy blocks the advance");
    }

    #[test]
    fn en_passant_window_is_exactly_one_ply_and_removes_the_passed_pawn() {
        // 5 ranks so a double-step and an adjacent capturer coexist cleanly.
        let mut g = PawnDuel::new(4, 5, true, true);
        g.board = vec![EMPTY; 20];
        g.board[idx(0, 2, 4)] = 1; // seat 1 on its start rank (top)
        g.board[idx(2, 1, 4)] = 0; // seat 0 capturer, adjacent file, one rank down
        g.board[idx(4, 3, 4)] = 0; // spare seat-0 pawn so nobody is eliminated
        g.current = 1;
        // Seat 1 double-steps 0->2, crossing (1,2): that square becomes the ep target.
        g.make_move(&PdMove { from: i(0, 2, 4), to: i(2, 2, 4) });
        assert_eq!(g.ep_target, Some(i(1, 2, 4)), "the crossed square is armed");
        assert_eq!(g.current, 0);
        // Seat 0's pawn at (2,1) may capture en passant INTO the empty (1,2).
        let ep = PdMove { from: i(2, 1, 4), to: i(1, 2, 4) };
        assert!(g.gen().contains(&ep), "en passant offered on the very next ply");
        let mut done = g.clone();
        done.make_move(&ep);
        assert_eq!(done.board[idx(1, 2, 4)], 0, "capturer lands on the crossed square");
        assert_eq!(done.board[idx(2, 2, 4)], EMPTY, "the passed pawn is removed");
        assert_eq!(done.ep_target, None, "window closes after use");
        // Window is EXACTLY one ply: if instead seat 0 plays elsewhere, the chance is gone.
        let mut lapsed = g.clone();
        lapsed.make_move(&PdMove { from: i(4, 3, 4), to: i(3, 3, 4) });
        assert_eq!(lapsed.ep_target, None, "an unrelated move disarms the window");
        lapsed.current = 0; // even if it somehow came back to this pawn…
        assert!(!lapsed.gen().contains(&ep), "en passant is no longer available a ply later");
    }

    #[test]
    fn replay_reconstructs_en_passant_state() {
        // Undo replays the move LOG on a fresh engine — verify an en-passant
        // capture survives replay by construction (the ep window is re-armed by
        // replaying the double-step, so the capture validates on the fresh engine).
        let mut src = PawnDuelWasm::new(4, 5, 1, 1);
        // Hand-build the same position through the public string API so the log is
        // a real sequence: reset the board via a fresh internal state.
        {
            let mut s = PawnDuel::new(4, 5, true, true);
            s.board = vec![EMPTY; 20];
            s.board[idx(0, 2, 4)] = 1;
            s.board[idx(2, 1, 4)] = 0;
            s.board[idx(4, 3, 4)] = 0;
            s.current = 1;
            src.manager = MCTSManager::new(s, PdCfg, PdEval, UCTPolicy::new(1.4), ());
        }
        // seat1 double 0,2->2,2 = "2-10"; seat0 ep (2,1)->(1,2) = "9-6".
        let log = [
            format!("{}-{}", i(0, 2, 4), i(2, 2, 4)),
            format!("{}-{}", i(2, 1, 4), i(1, 2, 4)),
        ];
        assert_eq!(log, ["2-10".to_string(), "9-6".to_string()]);
        // Apply the log on `src`.
        for m in &log {
            assert!(src.apply_move(m), "log move {m} must apply on the live engine");
        }
        let live_board = src.get_board();
        // Now REPLAY the same log on a fresh engine seeded identically.
        let mut fresh = PawnDuelWasm::new(4, 5, 1, 1);
        {
            let mut s = PawnDuel::new(4, 5, true, true);
            s.board = vec![EMPTY; 20];
            s.board[idx(0, 2, 4)] = 1;
            s.board[idx(2, 1, 4)] = 0;
            s.board[idx(4, 3, 4)] = 0;
            s.current = 1;
            fresh.manager = MCTSManager::new(s, PdCfg, PdEval, UCTPolicy::new(1.4), ());
        }
        for m in &log {
            assert!(fresh.apply_move(m), "replay move {m} must re-validate (ep window reconstructed)");
        }
        assert_eq!(fresh.get_board(), live_board, "replay reproduces the post-en-passant board");
        // And the passed pawn really is gone on the replayed board.
        assert_eq!(fresh.get_board().chars().nth(idx(2, 2, 4)), Some(' '));
    }

    #[test]
    fn promotion_wins() {
        // A seat-0 pawn one step from the far rank (row 0) walks in and wins.
        let mut g = PawnDuel::new(4, 6, false, false);
        g.board = vec![EMPTY; 24];
        g.board[idx(1, 2, 4)] = 0;
        g.board[idx(3, 0, 4)] = 1; // a lone enemy so it is promotion, not elimination
        g.current = 0;
        g.make_move(&PdMove { from: i(1, 2, 4), to: i(0, 2, 4) });
        assert_eq!(g.term(), Some(ProvenValue::Loss), "seat 1 to move now, and it has lost");
        assert_eq!(g.winning_cell(), Some(idx(0, 2, 4)), "glow the promotion square");
    }

    #[test]
    fn capturing_all_enemy_pawns_wins() {
        let mut g = PawnDuel::new(4, 6, false, false);
        g.board = vec![EMPTY; 24];
        g.board[idx(3, 1, 4)] = 0;
        g.board[idx(2, 2, 4)] = 1; // seat 1's last pawn
        g.current = 0;
        g.make_move(&PdMove { from: i(3, 1, 4), to: i(2, 2, 4) }); // capture it
        assert_eq!(g.count(1), 0);
        assert_eq!(g.term(), Some(ProvenValue::Loss), "seat 1 to move, wiped out, has lost");
        assert_eq!(g.winning_cell(), None, "elimination has no line to glow");
    }

    #[test]
    fn stalemate_is_a_loss_not_a_draw() {
        // Seat 0 (to move) has a single pawn blocked head-on with no diagonal — no
        // legal move. In chess this is a stalemate DRAW; here it is a LOSS.
        let mut g = PawnDuel::new(3, 5, false, false);
        g.board = vec![EMPTY; 15];
        g.board[idx(2, 0, 3)] = 0; // seat 0 pawn
        g.board[idx(1, 0, 3)] = 1; // enemy directly ahead — blocks the only file
        g.board[idx(4, 2, 3)] = 1; // keep seat 1 with material (no elimination)
        g.current = 0;
        assert!(g.gen().is_empty(), "the mover is stuck (no forward, no diagonal capture)");
        assert_eq!(g.term(), Some(ProvenValue::Loss), "stuck mover LOSES (deviation from chess draw)");
        assert_eq!(g.result_seat(), 2, "seat 1 (the other player) wins → result \"2\"");
    }

    #[test]
    fn round_trip_move_strings() {
        let mut g = PawnDuelWasm::new(4, 6, 1, 1);
        let before = g.get_board();
        let first = g.legal_moves().split(',').next().unwrap().to_string();
        assert!(first.contains('-'), "moves are from-to encoded");
        assert!(g.apply_move(&first), "a legal from-to round-trips");
        assert_ne!(g.get_board(), before);
        assert!(!g.apply_move("999-0"), "an illegal move is rejected");
        assert!(!g.apply_move("pawn"), "a malformed move is rejected");
        assert!(!g.apply_move("1-2-3"), "a three-part move is rejected");
    }

    // Independent minimax over this engine's own move generator: true iff the
    // player to move wins under perfect play (no draws exist here).
    fn mover_wins(s: &PawnDuel) -> bool {
        if s.term().is_some() {
            // Only reachable as Loss (win-for-other) since a win is detected the
            // ply after it is achieved; a terminal for the mover means the mover lost.
            return false;
        }
        s.gen().iter().any(|m| {
            let mut c = s.clone();
            c.make_move(m);
            !mover_wins(&c)
        })
    }

    #[test]
    fn hexapawn_3x3_is_a_second_player_win_minimax() {
        // Martin Gardner's classic result: 3×3 Hexapawn is a win for the SECOND
        // player. Seat 0 moves first, so the first mover must LOSE with best play.
        let g = PawnDuel::new(3, 3, false, false);
        assert_eq!(g.count(0), 3);
        assert_eq!(g.count(1), 3);
        assert!(!mover_wins(&g), "the first player (seat 0) loses 3×3 Hexapawn — second player wins");
    }

    #[test]
    fn solver_proves_hexapawn_second_player_win() {
        // THE showcase test: drive the wasm SOLVER through best-play self-play from
        // the Hexapawn preset and assert it lands on the second player (seat 1 →
        // result "2"). If the solver can't reach this known result, something is
        // wrong with the engine.
        let mut g = PawnDuelWasm::new(3, 3, 0, 0);
        assert_eq!(g.get_board(), "OOO   XXX", "Hexapawn start: seat 1 top, seat 0 bottom");
        for _ in 0..12 {
            if g.is_terminal() {
                break;
            }
            g.playout_n(2000); // ample for the solver to prove this tiny tree
            let m = g.best_move().expect("a non-terminal position has a best move");
            assert!(g.apply_move(&m), "solver best_move {m} must apply");
        }
        assert!(g.is_terminal(), "self-play terminates");
        assert_eq!(g.result(), "2", "Hexapawn 3×3 is a second-player win (Gardner)");
    }

    #[test]
    fn ai_plays() {
        let mut g = PawnDuelWasm::new(8, 6, 1, 1);
        g.playout_n(300);
        assert!(g.best_move().is_some());
        let m = g.best_move().unwrap();
        assert!(g.apply_move(&m));
    }

    #[test]
    fn full_games_terminate_and_are_decisive() {
        for (f, r, ds, ep) in [(3u32, 3u32, 0u32, 0u32), (8, 6, 1, 1), (4, 5, 1, 1), (10, 8, 1, 1), (5, 5, 0, 1)] {
            let mut g = PawnDuelWasm::new(f, r, ds, ep);
            let cap = (f * r) as usize * 4 + 20;
            for _ in 0..cap {
                if g.is_terminal() {
                    break;
                }
                let m = g.weak_move(80, 3, 0.6, 7).or_else(|| g.best_move());
                match m {
                    Some(mv) => assert!(g.apply_move(&mv), "engine emitted an unplayable move: {mv}"),
                    None => break,
                }
            }
            assert!(g.is_terminal(), "({f},{r},{ds},{ep}) must terminate");
            assert!(!g.result().is_empty(), "Pawn Duel is decisive — no draws");
        }
    }
}

// Test-only helper on the private state (kept out of the wasm surface).
#[cfg(test)]
impl PawnDuel {
    fn result_seat(&self) -> u8 {
        match self.term() {
            Some(ProvenValue::Win) => self.current + 1,
            Some(ProvenValue::Loss) => (1 - self.current) + 1,
            _ => 0,
        }
    }
}
