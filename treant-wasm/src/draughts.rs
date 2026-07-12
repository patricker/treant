//! Draughts (checkers) — ONE flag-driven engine backing six national variants.
//!
//! Six wave-A variants ship as config-only tiles over this single engine (the
//! per-variant flag table is asserted in the tests below). Wave-B adds Spanish
//! and Italian via two more flags (`menNoK` = men-cannot-capture-kings, `prio` =
//! capture-priority mode 0/1/2), both of which default OFF so the six wave-A
//! variants stay bit-identical:
//!
//! | Variant       | size | rows | flying | menBack | maxCap | midChain | misère | menNoK | prio |
//! |---------------|------|------|--------|---------|--------|----------|--------|--------|------|
//! | American ⭐    |  8   |  3   |  no    |  no     |  no    |  n/a     |  no    |  no    |  0   |
//! | International |  10  |  4   |  yes   |  yes    |  yes   |  no      |  no    |  no    |  0   |
//! | Brazilian     |  8   |  3   |  yes   |  yes    |  yes   |  no      |  no    |  no    |  0   |
//! | Pool          |  8   |  3   |  yes   |  yes    |  no    |  no      |  no    |  no    |  0   |
//! | Russian       |  8   |  3   |  yes   |  yes    |  no    |  YES     |  no    |  no    |  0   |
//! | Giveaway 🙃    |  8   |  3   |  no    |  no     |  no    |  n/a     |  YES   |  no    |  0   |
//! | Spanish       |  8   |  3   |  yes   |  no     | (yes)  |  no      |  no    |  no    |  1   |
//! | Italian       |  8   |  3   |  no    |  no     | (yes)  |  no      |  no    |  YES   |  2   |
//!
//! Wave-A table verified 2026-07-06 against the per-variant Wikipedia pages
//! (English/International/Brazilian/Russian draughts, Pool checkers, Poddavki).
//! Spanish/Italian sourced 2026-07-11 (ludoteka + Italian_draughts + FID; quotes
//! at the `capture_priority` and `men_no_king` decision points). `maxCap` is
//! `(yes)` for Spanish/Italian because `prio > 0` normalizes it on — the quality
//! tiebreak is a refinement of maximum-capture, never a replacement.
//!
//! Italian king movement (non-flying, step-1) is pinned by the Federazione
//! Italiana Dama: "Un pezzo muove procedendo in diagonale di una casella e
//! occupandola" and "La pedina può muovere solo avanzando, la dama può anche
//! indietreggiare" (fid.it/corsi/italiana/regole.htm); corroborated by "La dama
//! si muove anch'essa di una casella alla volta, sempre in diagonale, in tutte le
//! direzioni possibili" (federdama.org, Dama Italiana) — i.e. `flying_kings = 0`.
//! Load-bearing quotes live at each decision point below.
//!
//! NOTE on Giveaway: the plan models it as *American rules with an inverted win
//! condition* (Western "Suicide checkers"). The canonical FMJD/Poddavki article
//! actually derives Giveaway from RUSSIAN rules (flying kings, backward capture,
//! and mid-chain promotion). Both are attested; we ship the plan's American-based
//! giveaway because its flags are explicit and it is the more recognizable
//! Western form. A Russian-based Giveaway is a one-line flag change if desired.
//!
//! ## Capture machinery (the real work)
//! - Captures are a FULL ATOMIC PATH: `available_moves()` only ever returns
//!   COMPLETE chains (you are forced to keep jumping until no jump remains).
//!   "If a jump is possible it must be done, even if doing so incurs a
//!   disadvantage." (International_draughts) — compulsory in every wave-A variant.
//! - `max_capture` (International, Brazilian) keeps only the longest chains:
//!   "It is compulsory to jump over as many pieces as possible." (International).
//!   American/Pool/Russian let you pick ANY complete chain: "The sequence chosen
//!   is not required to be the one that maximizes the number of jumps."
//!   (English_draughts).
//! - International "Turkish-stroke" subtlety: "jumped pieces are not removed
//!   during the move, they are removed only after the entire multi-jump move is
//!   complete" and "The same piece may not be jumped more than once."
//!   (International_draughts). We therefore keep captured pieces ON the working
//!   board during generation (they still BLOCK the path and cannot be re-jumped)
//!   and remove them only in `make_move` at chain end.
//! - Flying kings (all but American): "a king can jump any number of squares
//!   forward and backward" and "choose where to stop afterwards." (Pool_checkers
//!   / International_draughts). American kings are step-1 and non-flying:
//!   "Flying kings are not used in American checkers." (Draughts).
//! - Mid-chain promotion: Russian promotes the instant a man touches the kings
//!   row mid-chain and continues as a king — "If a man touches the kings row
//!   during a capture and can continue a capture, it jumps backwards as a king."
//!   (Russian_draughts). International/Brazilian do NOT: "A piece is crowned if
//!   it stops on the far edge of the board at the end of its turn (that is, not
//!   if it reaches the edge but must then jump another piece backward)."
//!   (Brazilian_draughts) — the pass-through rule.
//!
//! ## Termination / draws
//! Captures and man-moves strictly progress (pieces vanish / men march one-way),
//! but a king-only endgame can shuffle forever, which would let a search path
//! grow without bound. We declare a DRAW after 40 plies with no capture and no
//! man-move (the 40-move-no-progress convention; real draughts uses 25/40-move
//! rules for king-vs-king endings — we pick a single 40-ply cap and apply it
//! uniformly). This also guarantees every tree path terminates.
//!
//! ## AI
//! Solver is OFF (draughts games are long and cycle-prone; the exact solver
//! would rarely converge and adds overhead). Strength comes from a REAL
//! evaluator: material (man = 100, king = 175 ≈ 1.7× a man) + mobility (4 ×
//! legal-move-count difference), from seat-0's perspective; material sign-flips
//! under misère (fewest/least material wins), exactly as Anti-Reversi does.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

/// Draw after this many plies without a capture or a man-move (see module docs).
const DRAW_PLIES: u32 = 40;
/// Material weights, in centi-man units. King ≈ 1.7× a man.
const MAN_VAL: i64 = 100;
const KING_VAL: i64 = 175;
/// Weight on the (my legal moves − your legal moves) mobility term.
const MOBILITY: i64 = 4;

/// The four diagonal steps.
const ALL4: [(i32, i32); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cell {
    Empty,
    Man(u8),
    King(u8),
}

/// A full draughts move: the piece's path (start cell, then each landing cell),
/// plus the squares of every piece it captures. A simple step is a 2-cell path
/// with no captures; a jump chain is a 3+-cell path. Display/parse are single-
/// sourced on the dash-joined path (`"12-19"`, `"12-19-26"`), and the captured
/// set is recovered by matching the parsed path against generated moves.
#[derive(Clone, PartialEq, Eq, Debug)]
struct DMove {
    path: Vec<u16>,
    captured: Vec<u16>,
}
impl std::fmt::Display for DMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s: Vec<String> = self.path.iter().map(|c| c.to_string()).collect();
        write!(f, "{}", s.join("-"))
    }
}
impl DMove {
    /// Parse the dash-joined path form. Round-trips `Display`. The captured set
    /// is NOT encoded — callers resolve it by matching against generated moves.
    fn parse_path(s: &str) -> Option<Vec<u16>> {
        let path: Result<Vec<u16>, _> = s.split('-').map(|p| p.parse::<u16>()).collect();
        match path {
            Ok(p) if p.len() >= 2 => Some(p),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct Draughts {
    n: usize,
    grid: Vec<Cell>,
    current: u8,
    // rule flags
    flying: bool,
    men_back: bool,
    max_capture: bool,
    promote_mid: bool,
    misere: bool,
    /// Italian: a man may not jump a king (guard inside `chain()`).
    men_no_king: bool,
    /// Capture-priority mode: 0 = none (wave-A), 1 = Spanish (`(count, kings)`),
    /// 2 = Italian (`(count, capturer_is_king, kings, king_captured_earliest)`).
    capture_priority: u8,
    /// Plies since the last capture or man-move (for the 40-ply draw cap).
    no_progress: u32,
}

impl Draughts {
    #[allow(clippy::too_many_arguments)]
    fn new(n: usize, men_rows: usize, flying: bool, men_back: bool, max_capture: bool, promote_mid: bool, misere: bool, men_no_king: bool, capture_priority: u8) -> Self {
        // A capture-priority mode is a REFINEMENT of maximum-capture: both Spanish
        // and Italian rules first demand "capture the greatest quantity of pieces"
        // (Italian_draughts) / the "Quantity Rule: as many pieces as possible must
        // be captured" (ludoteka, Spanish), then break the count-tie on quality.
        // We therefore make `capture_priority > 0` IMPLY `max_capture` (normalize,
        // not reject): the count filter runs first, the quality retain second. This
        // keeps the two flags orthogonal for wave-A (both off ⇒ unchanged) while
        // guaranteeing the quality tiebreak always sits on top of a max-count base.
        let max_capture = max_capture || capture_priority > 0;
        let capture_priority = capture_priority.min(2);
        let mut grid = vec![Cell::Empty; n * n];
        // Playing squares are the dark squares (r + c) odd. Seat 1 fills the top
        // `men_rows`; seat 0 fills the bottom `men_rows`; the middle is empty.
        for r in 0..men_rows {
            for c in 0..n {
                if (r + c) % 2 == 1 {
                    grid[r * n + c] = Cell::Man(1);
                }
            }
        }
        for r in (n - men_rows)..n {
            for c in 0..n {
                if (r + c) % 2 == 1 {
                    grid[r * n + c] = Cell::Man(0);
                }
            }
        }
        Self { n, grid, current: 0, flying, men_back, max_capture, promote_mid, misere, men_no_king, capture_priority, no_progress: 0 }
    }

    #[inline]
    fn in_bounds(&self, r: i32, c: i32) -> bool {
        r >= 0 && r < self.n as i32 && c >= 0 && c < self.n as i32
    }
    #[inline]
    fn idx(&self, r: i32, c: i32) -> usize {
        r as usize * self.n + c as usize
    }
    /// Promotion (kings) row for `owner`: seat 0 marches up to row 0, seat 1
    /// marches down to the last row.
    #[inline]
    fn promo_row(&self, owner: u8) -> usize {
        if owner == 0 {
            0
        } else {
            self.n - 1
        }
    }
    /// Forward diagonal steps for a *man* of `owner` (men only move/step forward;
    /// they may CAPTURE backward only when `men_back` is set).
    fn man_dirs(&self, owner: u8) -> [(i32, i32); 2] {
        if owner == 0 {
            [(-1, -1), (-1, 1)]
        } else {
            [(1, -1), (1, 1)]
        }
    }

    fn is_enemy(bd: &[Cell], sq: usize, owner: u8) -> bool {
        matches!(bd[sq], Cell::Man(o) | Cell::King(o) if o != owner)
    }

    // ---------------------------------------------------------------- captures

    /// Recursively extend a capture chain. Emits only MAXIMAL chains (you must
    /// keep jumping while a jump exists). `bd` is the board with the moving piece
    /// lifted off its origin; captured pieces stay ON `bd` as blockers and are
    /// tracked in `captured` so they cannot be re-jumped (Turkish-stroke rule).
    #[allow(clippy::too_many_arguments)]
    fn chain(&self, bd: &[Cell], sq: usize, is_king: bool, owner: u8, path: &mut Vec<u16>, captured: &mut Vec<u16>, out: &mut Vec<DMove>) {
        let (r, c) = ((sq / self.n) as i32, (sq % self.n) as i32);
        let mut extended = false;

        // Continuation attempt closure: try to capture `over_sq`, landing on
        // `land_sq`, then recurse. Handles mid-chain promotion.
        let mut try_capture = |slf: &Self, over_sq: usize, land_sq: usize, is_king: bool, path: &mut Vec<u16>, captured: &mut Vec<u16>, out: &mut Vec<DMove>| {
            extended = true;
            captured.push(over_sq as u16);
            path.push(land_sq as u16);
            // Mid-chain promotion: a man that touches its kings row promotes
            // immediately ONLY when `promote_mid` (Russian); otherwise it stays a
            // man and may keep jumping backward off the row without crowning
            // (International/Brazilian pass-through).
            let mut nk = is_king;
            if !is_king && (land_sq / slf.n) == slf.promo_row(owner) && slf.promote_mid {
                nk = true;
            }
            slf.chain(bd, land_sq, nk, owner, path, captured, out);
            path.pop();
            captured.pop();
        };

        if is_king && self.flying {
            // Flying king: slide over empties, capture the FIRST uncaptured enemy
            // in a direction, then land on ANY empty square beyond it.
            for (dr, dc) in ALL4 {
                let mut d = 1;
                loop {
                    let (rr, cc) = (r + d * dr, c + d * dc);
                    if !self.in_bounds(rr, cc) {
                        break;
                    }
                    let s = self.idx(rr, cc);
                    if bd[s] == Cell::Empty {
                        d += 1;
                        continue;
                    }
                    // Hit a piece. Capturable only if it's an enemy we haven't
                    // already jumped; anything else (friendly, or a still-present
                    // captured piece) blocks the direction entirely.
                    if Self::is_enemy(bd, s, owner) && !captured.contains(&(s as u16)) {
                        let mut e = d + 1;
                        loop {
                            let (lr, lc) = (r + e * dr, c + e * dc);
                            if !self.in_bounds(lr, lc) {
                                break;
                            }
                            let ls = self.idx(lr, lc);
                            if bd[ls] != Cell::Empty {
                                break; // landing squares must be empty
                            }
                            try_capture(self, s, ls, is_king, path, captured, out);
                            e += 1;
                        }
                    }
                    break;
                }
            }
        } else {
            // Man, or non-flying (American) king: adjacent jump, land 2 beyond.
            // Kings capture in all 4 diagonals; men only forward unless `men_back`.
            let dirs: Vec<(i32, i32)> = if is_king || self.men_back { ALL4.to_vec() } else { self.man_dirs(owner).to_vec() };
            for (dr, dc) in dirs {
                let (or, oc) = (r + dr, c + dc);
                let (lr, lc) = (r + 2 * dr, c + 2 * dc);
                if !self.in_bounds(lr, lc) {
                    continue;
                }
                let (osq, lsq) = (self.idx(or, oc), self.idx(lr, lc));
                // Italian: "Men cannot jump kings." (Draughts). A man (`!is_king`)
                // may not capture a king — skip that continuation entirely. The
                // subtle consequence (tested): if a man's ONLY jump is over a king
                // it has NO capture, so the capture obligation dissolves for that
                // man (it may fall to another piece, or quiet moves become legal if
                // no piece anywhere can capture). Kings jumping kings are unaffected.
                if !is_king && self.men_no_king && matches!(bd[osq], Cell::King(_)) {
                    continue;
                }
                if Self::is_enemy(bd, osq, owner) && !captured.contains(&(osq as u16)) && bd[lsq] == Cell::Empty {
                    try_capture(self, osq, lsq, is_king, path, captured, out);
                }
            }
        }

        if !extended && path.len() > 1 {
            out.push(DMove { path: path.clone(), captured: captured.clone() });
        }
    }

    /// All legal moves for `owner` (independent of whose turn it is — used by the
    /// evaluator's mobility term). Captures, when they exist, are FORCED and are
    /// the only moves returned; `max_capture` then filters to the longest chains.
    fn gen_for(&self, owner: u8) -> Vec<DMove> {
        let mut caps: Vec<DMove> = Vec::new();
        for sq in 0..self.grid.len() {
            let is_king = match self.grid[sq] {
                Cell::Man(o) if o == owner => false,
                Cell::King(o) if o == owner => true,
                _ => continue,
            };
            let mut bd = self.grid.clone();
            bd[sq] = Cell::Empty; // lift the moving piece
            let mut path = vec![sq as u16];
            let mut captured = Vec::new();
            self.chain(&bd, sq, is_king, owner, &mut path, &mut captured, &mut caps);
        }
        if !caps.is_empty() {
            if self.max_capture {
                let best = caps.iter().map(|m| m.captured.len()).max().unwrap_or(0);
                caps.retain(|m| m.captured.len() == best);
            }
            // Capture-priority quality tiebreak (Spanish / Italian). A pure
            // post-generation lexicographic `retain`; the `chain()` walker is
            // untouched. Keys are computed against the PRE-capture board (`self.grid`
            // still holds every jumped piece — captures are only removed in
            // `make_move`), so each captured square's man/king identity is exact.
            //
            //  - Mode 1 Spanish: "Quality Rule: ... as much kings as possible must
            //    be captured." (ludoteka) ⇒ key `(count, kings_captured)`.
            //  - Mode 2 Italian: the verbatim 4-level hierarchy — "capture the
            //    greatest quantity of pieces" → "he must do so with the king" →
            //    "capture the greatest number of kings possible" → "capture wherever
            //    the king occurs first." (Italian_draughts) ⇒ key `(count,
            //    capturer_is_king, kings_captured, king_captured_earliest)`.
            //
            // Every component is oriented "higher = better", so we keep the moves
            // whose key equals the maximum. `king_captured_earliest` prefers the
            // chain whose first-jumped king comes EARLIEST in the jump order, so we
            // encode it as the negated index of the first king (0 for a king jumped
            // first, i64::MIN when no king is captured — the worst).
            if self.capture_priority > 0 {
                let key = |m: &DMove| -> (usize, u8, i64, i64) {
                    let count = m.captured.len();
                    let capturer_is_king = matches!(self.grid[m.path[0] as usize], Cell::King(_));
                    let kings = m.captured.iter().filter(|&&sq| matches!(self.grid[sq as usize], Cell::King(_))).count() as i64;
                    let earliest = m
                        .captured
                        .iter()
                        .position(|&sq| matches!(self.grid[sq as usize], Cell::King(_)))
                        .map(|i| -(i as i64))
                        .unwrap_or(i64::MIN);
                    match self.capture_priority {
                        // Spanish: only the king-COUNT tiebreak applies; the
                        // capturer-is-king and king-first levels are neutralised.
                        1 => (count, 0, kings, 0),
                        // Italian: the full 4-level hierarchy.
                        _ => (count, capturer_is_king as u8, kings, earliest),
                    }
                };
                let best = caps.iter().map(&key).max().unwrap();
                caps.retain(|m| key(m) == best);
            }
            return caps;
        }
        // No captures anywhere: quiet moves.
        let mut mv: Vec<DMove> = Vec::new();
        for sq in 0..self.grid.len() {
            let (r, c) = ((sq / self.n) as i32, (sq % self.n) as i32);
            match self.grid[sq] {
                Cell::Man(o) if o == owner => {
                    for (dr, dc) in self.man_dirs(owner) {
                        let (nr, nc) = (r + dr, c + dc);
                        if self.in_bounds(nr, nc) {
                            let t = self.idx(nr, nc);
                            if self.grid[t] == Cell::Empty {
                                mv.push(DMove { path: vec![sq as u16, t as u16], captured: vec![] });
                            }
                        }
                    }
                }
                Cell::King(o) if o == owner => {
                    for (dr, dc) in ALL4 {
                        let mut d = 1;
                        loop {
                            let (nr, nc) = (r + d * dr, c + d * dc);
                            if !self.in_bounds(nr, nc) {
                                break;
                            }
                            let t = self.idx(nr, nc);
                            if self.grid[t] != Cell::Empty {
                                break;
                            }
                            mv.push(DMove { path: vec![sq as u16, t as u16], captured: vec![] });
                            if !self.flying {
                                break; // non-flying king steps one square
                            }
                            d += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        mv
    }

    fn gen(&self) -> Vec<DMove> {
        self.gen_for(self.current)
    }

    fn apply(&mut self, m: &DMove) {
        let owner = self.current;
        let from = m.path[0] as usize;
        let was_king = matches!(self.grid[from], Cell::King(_));
        for &cap in &m.captured {
            self.grid[cap as usize] = Cell::Empty;
        }
        self.grid[from] = Cell::Empty;
        let to = *m.path.last().unwrap() as usize;
        // Crown if the piece finishes on the far row, or (Russian) if it touched
        // the far row anywhere mid-chain.
        let ends_on_promo = to / self.n == self.promo_row(owner);
        let crossed_promo = self.promote_mid && m.path[1..].iter().any(|&p| (p as usize) / self.n == self.promo_row(owner));
        let king = was_king || ends_on_promo || crossed_promo;
        self.grid[to] = if king { Cell::King(owner) } else { Cell::Man(owner) };

        // Progress = a capture or a man-move (both are irreversible). A quiet
        // king slide is NOT progress and ticks the draw counter.
        if !m.captured.is_empty() || !was_king {
            self.no_progress = 0;
        } else {
            self.no_progress += 1;
        }
        self.current = 1 - owner;
    }

    fn term(&self) -> Option<ProvenValue> {
        // A DECISIVE no-move terminal takes precedence over the 40-ply draw cap:
        // if the player to move is out of moves the game is already won or lost, so
        // reaching the no-progress cap on that same ply must NOT downgrade it to a
        // draw. (Order matters — check the no-move terminal first.)
        if self.gen().is_empty() {
            // The player to move has no legal move. Normally that's a loss; under
            // misère (Giveaway) it's a WIN: "A player with no valid move remaining
            // win[s]. This is the case if the player either has no pieces left or
            // if a player's pieces are obstructed from making a legal move by the
            // pieces of the opponent." (Poddavki).
            return Some(if self.misere { ProvenValue::Win } else { ProvenValue::Loss });
        }
        if self.no_progress >= DRAW_PLIES {
            return Some(ProvenValue::Draw);
        }
        None
    }

    fn count(&self, cell_owner_man: Cell, cell_owner_king: Cell) -> (i64, i64) {
        let men = self.grid.iter().filter(|&&c| c == cell_owner_man).count() as i64;
        let kings = self.grid.iter().filter(|&&c| c == cell_owner_king).count() as i64;
        (men, kings)
    }

    /// Static evaluation from seat 0's perspective (positive = seat 0 better).
    fn eval0(&self) -> i64 {
        let (m0, k0) = self.count(Cell::Man(0), Cell::King(0));
        let (m1, k1) = self.count(Cell::Man(1), Cell::King(1));
        let material = MAN_VAL * (m0 - m1) + KING_VAL * (k0 - k1);
        // Material sign flips under misère (giving pieces away is good), mirroring
        // Anti-Reversi. Mobility keeps its sign — having moves is mild insurance.
        let material = if self.misere { -material } else { material };
        let mobility = self.gen_for(0).len() as i64 - self.gen_for(1).len() as i64;
        material + MOBILITY * mobility
    }
}

impl GameState for Draughts {
    type Move = DMove;
    type Player = u8;
    type MoveList = Vec<DMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<DMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &DMove) {
        self.apply(m);
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}

struct DrEval;
impl Evaluator<DrCfg> for DrEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &Draughts, m: &Vec<DMove>, _: Option<SearchHandle<DrCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], s.eval0())
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 {
            *e
        } else {
            -*e
        }
    }
    fn evaluate_existing_state(&self, s: &Draughts, _: &i64, _: SearchHandle<DrCfg>) -> i64 {
        s.eval0()
    }
}

#[derive(Default)]
struct DrCfg;
impl MCTS for DrCfg {
    type State = Draughts;
    type Eval = DrEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    // Solver OFF: draughts games are long and cycle-prone; the exact solver would
    // seldom converge here and only adds overhead. (Default is already false; we
    // state it for the record.)
    fn solver_enabled(&self) -> bool {
        false
    }
}

/// Clamp constructor flags to the supported ranges.
fn clamp_cfg(size: u32, men_rows: u32) -> (usize, usize) {
    let n: usize = match size {
        10 => 10,
        12 => 12,
        _ => 8,
    };
    // Leave at least one empty middle row per side.
    let men_rows = (men_rows as usize).clamp(1, n / 2 - 1);
    (n, men_rows)
}

#[wasm_bindgen]
pub struct DraughtsWasm {
    manager: MCTSManager<DrCfg>,
    n: usize,
    men_rows: usize,
    flying: bool,
    men_back: bool,
    max_capture: bool,
    promote_mid: bool,
    misere: bool,
    men_no_king: bool,
    capture_priority: u8,
}

#[wasm_bindgen]
impl DraughtsWasm {
    /// `size` 8|10|12, `men_rows` 2–4 typically; the remaining args are the nine
    /// rule flags. Seven are 0/1 booleans; the last, `capture_priority`, is an enum
    /// (0 = none, 1 = Spanish, 2 = Italian). See the variant table in the module
    /// docs. `capture_priority > 0` normalizes `max_capture` on (the quality
    /// tiebreak sits atop a max-count base); `men_cannot_capture_kings` is the
    /// Italian "men may not jump kings" rule. Both new flags default off, leaving
    /// the six wave-A variants bit-identical.
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(constructor)]
    pub fn new(size: u32, men_rows: u32, flying_kings: u32, men_capture_back: u32, max_capture: u32, promote_mid_chain: u32, misere: u32, men_cannot_capture_kings: u32, capture_priority: u32) -> Self {
        let (n, men_rows) = clamp_cfg(size, men_rows);
        let flying = flying_kings != 0;
        let men_back = men_capture_back != 0;
        let promote_mid = promote_mid_chain != 0;
        let misere = misere != 0;
        let men_no_king = men_cannot_capture_kings != 0;
        let capture_priority = capture_priority.min(2) as u8;
        // Draughts::new normalizes `max_capture` on when priority > 0; mirror that
        // here so the cached DraughtsWasm flag matches the state's actual behavior.
        let max_capture = (max_capture != 0) || capture_priority > 0;
        let state = Draughts::new(n, men_rows, flying, men_back, max_capture, promote_mid, misere, men_no_king, capture_priority);
        Self {
            manager: MCTSManager::new(state, DrCfg, DrEval, UCTPolicy::new(1.4), ()),
            n,
            men_rows,
            flying,
            men_back,
            max_capture,
            promote_mid,
            misere,
            men_no_king,
            capture_priority,
        }
    }

    fn fresh_state(&self) -> Draughts {
        Draughts::new(self.n, self.men_rows, self.flying, self.men_back, self.max_capture, self.promote_mid, self.misere, self.men_no_king, self.capture_priority)
    }

    pub fn cols(&self) -> u32 {
        self.n as u32
    }
    pub fn rows(&self) -> u32 {
        self.n as u32
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }

    /// One char per cell, row-major over the FULL grid: ' ' empty, seat-0
    /// man/king 'x'/'X', seat-1 man/king 'o'/'O'. Light squares are always ' '.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .grid
            .iter()
            .map(|&c| match c {
                Cell::Empty => ' ',
                Cell::Man(0) => 'x',
                Cell::King(0) => 'X',
                Cell::Man(1) => 'o',
                Cell::King(1) => 'O',
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

    /// Comma-separated full-path move encodings (`"12-19"`, `"12-19-26"`).
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

    pub fn apply_move(&mut self, mov: &str) -> bool {
        let path = match DMove::parse_path(mov) {
            Some(p) => p,
            None => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        // Resolve the captured set by matching the parsed path against a freshly
        // generated legal move (single-sourced: the engine owns capture logic).
        let found = s.available_moves().into_iter().find(|m| m.path == path);
        let m = match found {
            Some(m) => m,
            None => return false,
        };
        s.make_move(&m);
        self.manager = MCTSManager::new(s, DrCfg, DrEval, UCTPolicy::new(1.4), ());
        true
    }

    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(self.fresh_state(), DrCfg, DrEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Flag presets: (size, men_rows, flying, men_back, max_capture, promote_mid,
    // misere, men_no_king, capture_priority) — mirrors the module-doc table. The
    // six wave-A variants carry the two new flags as `false, 0` (bit-identity bar).
    type Preset = (usize, usize, bool, bool, bool, bool, bool, bool, u8);
    const AMERICAN: Preset = (8, 3, false, false, false, false, false, false, 0);
    const INTERNATIONAL: Preset = (10, 4, true, true, true, false, false, false, 0);
    const BRAZILIAN: Preset = (8, 3, true, true, true, false, false, false, 0);
    const POOL: Preset = (8, 3, true, true, false, false, false, false, 0);
    const RUSSIAN: Preset = (8, 3, true, true, false, true, false, false, 0);
    const GIVEAWAY: Preset = (8, 3, false, false, false, false, true, false, 0);
    // Wave-B: Spanish (flying, forward-only men, priority=1); Italian (non-flying
    // step-1 kings, men-cannot-capture-kings, priority=2). max_capture is passed 0
    // here to prove the ctor normalizes it on for priority > 0.
    const SPANISH: Preset = (8, 3, true, false, false, false, false, false, 1);
    const ITALIAN: Preset = (8, 3, false, false, false, false, false, true, 2);

    fn mk(p: Preset) -> Draughts {
        Draughts::new(p.0, p.1, p.2, p.3, p.4, p.5, p.6, p.7, p.8)
    }
    /// An empty wave-A board with the given flags (new flags off; hand-built
    /// positions). Spanish/Italian positions use `empty_ex`.
    fn empty(n: usize, flying: bool, men_back: bool, max_capture: bool, promote_mid: bool, misere: bool) -> Draughts {
        empty_ex(n, flying, men_back, max_capture, promote_mid, misere, false, 0)
    }
    /// An empty board with ALL nine flags (for Spanish/Italian hand-built tests).
    #[allow(clippy::too_many_arguments)]
    fn empty_ex(n: usize, flying: bool, men_back: bool, max_capture: bool, promote_mid: bool, misere: bool, men_no_king: bool, capture_priority: u8) -> Draughts {
        let mut g = Draughts::new(n, 1, flying, men_back, max_capture, promote_mid, misere, men_no_king, capture_priority);
        g.grid = vec![Cell::Empty; n * n];
        g
    }
    fn rc(g: &Draughts, r: usize, c: usize) -> usize {
        r * g.n + c
    }
    fn set(g: &mut Draughts, r: usize, c: usize, cell: Cell) {
        let i = rc(g, r, c);
        g.grid[i] = cell;
    }

    #[test]
    fn variant_setups_have_the_right_piece_counts() {
        // American/Brazilian/Pool/Russian/Giveaway: 8×8, 3 rows → 12 men/side.
        for p in [AMERICAN, BRAZILIAN, POOL, RUSSIAN, GIVEAWAY] {
            let g = mk(p);
            assert_eq!(g.grid.iter().filter(|&&c| c == Cell::Man(0)).count(), 12);
            assert_eq!(g.grid.iter().filter(|&&c| c == Cell::Man(1)).count(), 12);
        }
        // International: 10×10, 4 rows → 20 men/side.
        let g = mk(INTERNATIONAL);
        assert_eq!(g.grid.iter().filter(|&&c| c == Cell::Man(0)).count(), 20);
        assert_eq!(g.grid.iter().filter(|&&c| c == Cell::Man(1)).count(), 20);
    }

    #[test]
    fn variant_flags_match_the_table() {
        // Guard the exact flag tuple per variant so the table can't silently drift.
        // The six wave-A variants MUST carry the two new flags as `false, 0` — this
        // is the bit-identity contract (see also `wave_a_flags_leave_new_paths_off`).
        assert_eq!(AMERICAN, (8, 3, false, false, false, false, false, false, 0));
        assert_eq!(INTERNATIONAL, (10, 4, true, true, true, false, false, false, 0));
        assert_eq!(BRAZILIAN, (8, 3, true, true, true, false, false, false, 0));
        assert_eq!(POOL, (8, 3, true, true, false, false, false, false, 0));
        assert_eq!(RUSSIAN, (8, 3, true, true, false, true, false, false, 0));
        assert_eq!(GIVEAWAY, (8, 3, false, false, false, false, true, false, 0));
        // Wave-B variants (Spanish ⭐ / Italian).
        assert_eq!(SPANISH, (8, 3, true, false, false, false, false, false, 1));
        assert_eq!(ITALIAN, (8, 3, false, false, false, false, false, true, 2));
    }

    #[test]
    fn american_opening_has_seven_moves() {
        // Standard checkers opening: 7 legal first moves for the side to move.
        let g = mk(AMERICAN);
        assert_eq!(g.gen().len(), 7);
        assert!(g.gen().iter().all(|m| m.captured.is_empty()));
    }

    #[test]
    fn captures_are_forced_when_available() {
        // A lone seat-0 man with a jump available: the only legal moves are jumps.
        let mut g = empty(8, false, false, false, false, false);
        set(&mut g, 5, 2, Cell::Man(0));
        set(&mut g, 4, 1, Cell::Man(1));
        // (3,0) is empty → jump legal.
        let moves = g.gen();
        assert!(!moves.is_empty());
        assert!(moves.iter().all(|m| !m.captured.is_empty()), "captures must be forced");
    }

    #[test]
    fn multi_jump_is_one_atomic_chain() {
        // Double jump (5,2)->(3,0)->(1,2): a single move with a 3-cell path.
        let mut g = empty(8, false, false, false, false, false);
        set(&mut g, 5, 2, Cell::Man(0));
        set(&mut g, 4, 1, Cell::Man(1));
        set(&mut g, 2, 1, Cell::Man(1));
        let moves = g.gen();
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].path, vec![rc(&g, 5, 2) as u16, rc(&g, 3, 0) as u16, rc(&g, 1, 2) as u16]);
        assert_eq!(moves[0].captured.len(), 2);
    }

    #[test]
    fn max_capture_filters_to_the_longest_chain() {
        // Piece A can capture 2, piece B can capture 1. International (max_capture)
        // must offer ONLY A's 2-chain; Pool (no max) offers both.
        fn setup(g: &mut Draughts) {
            set(g, 5, 2, Cell::Man(0)); // A: double-jumper
            set(g, 4, 1, Cell::Man(1));
            set(g, 2, 1, Cell::Man(1));
            set(g, 5, 6, Cell::Man(0)); // B: single-jumper
            set(g, 4, 5, Cell::Man(1));
        }
        let mut intl = empty(8, true, true, true, false, false);
        setup(&mut intl);
        let mi = intl.gen();
        assert_eq!(mi.len(), 1, "max-capture keeps only the longest");
        assert_eq!(mi[0].captured.len(), 2);

        let mut pool = empty(8, true, true, false, false, false);
        setup(&mut pool);
        let mp = pool.gen();
        assert_eq!(mp.len(), 2, "no max-capture keeps both chains");
    }

    #[test]
    fn flying_king_captures_at_a_distance_and_lands_beyond() {
        // Flying king at (7,0), enemy at (5,2): the king may land on ANY empty
        // square beyond the captured man ((4,3),(3,4),(2,5),(1,6),(0,7)).
        let mut g = empty(8, true, true, false, false, false);
        set(&mut g, 7, 0, Cell::King(0));
        set(&mut g, 5, 2, Cell::Man(1));
        let moves = g.gen();
        assert!(!moves.is_empty());
        // Every move captures the one man and lands strictly beyond it.
        assert!(moves.iter().all(|m| m.captured == vec![rc(&g, 5, 2) as u16]));
        let landings: Vec<usize> = moves.iter().map(|m| *m.path.last().unwrap() as usize).collect();
        assert!(landings.contains(&rc(&g, 4, 3)));
        assert!(landings.contains(&rc(&g, 0, 7)), "a flying king can choose a far landing");
    }

    #[test]
    fn turkish_stroke_each_piece_jumped_once_and_captured_pieces_block() {
        // A flying king loops a 4-man square, capturing each exactly once. The
        // Turkish-stroke rule: "jumped pieces are ... removed only after the
        // entire multi-jump move is complete" and "The same piece may not be
        // jumped more than once." (International_draughts). Captured pieces stay
        // on the board as blockers, so the loop cannot re-cross a jumped man.
        let mut g = empty(8, true, true, false, false, false);
        set(&mut g, 5, 2, Cell::King(0));
        set(&mut g, 4, 3, Cell::Man(1));
        set(&mut g, 2, 3, Cell::Man(1));
        set(&mut g, 2, 1, Cell::Man(1));
        set(&mut g, 4, 1, Cell::Man(1));
        let moves = g.gen();
        // The maximal chain captures all four, returns to the start square, and
        // jumps no piece twice.
        let best = moves.iter().map(|m| m.captured.len()).max().unwrap();
        assert_eq!(best, 4, "the king rounds the whole square");
        let full = moves.iter().find(|m| m.captured.len() == 4).unwrap();
        assert_eq!(*full.path.first().unwrap(), *full.path.last().unwrap(), "loop returns to start");
        let mut seen = full.captured.clone();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 4, "no piece jumped twice");
    }

    #[test]
    fn russian_man_promotes_mid_chain_and_continues_as_a_king() {
        // Man at (2,1) jumps (1,2)->(0,3) onto the kings row, then — as a freshly
        // crowned FLYING king (Russian) — captures a distant man (2,5) that a man
        // could never reach. "If a man touches the kings row during a capture and
        // can continue a capture, it jumps ... as a king." (Russian_draughts).
        let mut g = empty(8, true, true, false, true, false); // Russian flags
        set(&mut g, 2, 1, Cell::Man(0));
        set(&mut g, 1, 2, Cell::Man(1));
        set(&mut g, 2, 5, Cell::Man(1));
        let moves = g.gen();
        let best = moves.iter().map(|m| m.captured.len()).max().unwrap();
        assert_eq!(best, 2, "mid-chain crown lets the king take the second man");
        let full = moves.iter().find(|m| m.captured.len() == 2).unwrap().clone();
        g.apply(&full);
        let end = *full.path.last().unwrap() as usize;
        assert!(matches!(g.grid[end], Cell::King(0)), "finishes as a king");
    }

    #[test]
    fn international_man_passes_through_king_row_without_promoting() {
        // Same geometry, International flags (no mid-chain promotion). The man
        // jumps (1,2)->(0,3) onto the far row but must keep jumping backward over
        // (1,4)->(2,5); because it does NOT stop on the far row it stays a MAN:
        // "A piece is crowned if it stops on the far edge of the board at the end
        // of its turn (that is, not if it reaches the edge but must then jump
        // another piece backward)." (Brazilian_draughts).
        let mut g = empty(10, true, true, true, false, false); // International flags
        set(&mut g, 2, 1, Cell::Man(0));
        set(&mut g, 1, 2, Cell::Man(1));
        set(&mut g, 1, 4, Cell::Man(1));
        let moves = g.gen();
        // Forced maximal chain captures both and ends at (2,5), off the far row.
        let full = moves.iter().max_by_key(|m| m.captured.len()).unwrap().clone();
        assert_eq!(full.captured.len(), 2);
        assert_eq!(*full.path.last().unwrap() as usize, rc(&g, 2, 5));
        g.apply(&full);
        let end = rc(&g, 2, 5);
        assert!(matches!(g.grid[end], Cell::Man(0)), "passed through the row → NOT crowned");
    }

    #[test]
    fn misere_inverts_the_no_move_terminal() {
        // A seat-0 man boxed into a corner by its own turn with no move: normal
        // rules = the mover loses; Giveaway (misère) = the mover WINS.
        fn boxed(misere: bool) -> Draughts {
            let mut g = empty(8, false, false, false, false, misere);
            // Seat-0 man at (0,1) on the far row: a man cannot move forward off the
            // board and (no men_back) cannot capture; seat 1 has a piece so the
            // game isn't a mutual stalemate. Current = seat 0 with no move.
            set(&mut g, 0, 1, Cell::Man(0));
            set(&mut g, 7, 0, Cell::Man(1));
            g.current = 0;
            g
        }
        let normal = boxed(false);
        assert!(normal.gen().is_empty());
        assert_eq!(normal.term(), Some(ProvenValue::Loss));
        let give = boxed(true);
        assert_eq!(give.term(), Some(ProvenValue::Win), "misère: no move = win");
    }

    #[test]
    fn no_move_terminal_beats_the_draw_cap_when_they_coincide() {
        // Simultaneous case: the player to move has NO legal move AND the 40-ply
        // no-progress counter is already at the cap. The decisive terminal must
        // win: normal rules = Loss (not Draw), misère = Win (not Draw).
        fn boxed(misere: bool) -> Draughts {
            let mut g = empty(8, false, false, false, false, misere);
            set(&mut g, 0, 1, Cell::Man(0)); // seat-0 man stuck on its far row
            set(&mut g, 7, 0, Cell::Man(1));
            g.current = 0;
            g.no_progress = DRAW_PLIES; // draw cap reached on the same ply
            g
        }
        let normal = boxed(false);
        assert!(normal.gen().is_empty());
        assert_eq!(normal.term(), Some(ProvenValue::Loss), "decisive loss beats the draw cap");
        let give = boxed(true);
        assert_eq!(give.term(), Some(ProvenValue::Win), "misère win beats the draw cap");
    }

    #[test]
    fn apply_move_by_string_reconstructs_a_multi_jump_chain() {
        // End-to-end path→captured reconstruction: build a flying-king 4-capture
        // loop, take the maximal chain's dash-joined STRING, drive DraughtsWasm's
        // apply_move with it, and assert every captured square is cleared and the
        // piece lands at the path's end (single-sourced captured-set recovery).
        let mut g = empty(8, true, true, false, true, false);
        set(&mut g, 5, 2, Cell::King(0));
        set(&mut g, 4, 3, Cell::Man(1));
        set(&mut g, 2, 3, Cell::Man(1));
        set(&mut g, 2, 1, Cell::Man(1));
        set(&mut g, 4, 1, Cell::Man(1));
        let moves = g.gen();
        let full = moves.iter().max_by_key(|m| m.captured.len()).unwrap().clone();
        assert_eq!(full.captured.len(), 4, "need a real multi-jump chain");
        let mv_str = format!("{full}");
        assert!(mv_str.matches('-').count() >= 4, "a genuine chain string: {mv_str}");

        // Wrap the hand-built state in a DraughtsWasm (flags mirror `g`) and drive
        // apply_move with ONLY the string — captured squares are reconstructed.
        let mut w = DraughtsWasm::new(8, 3, 1, 1, 0, 1, 0, 0, 0);
        w.manager = MCTSManager::new(g.clone(), DrCfg, DrEval, UCTPolicy::new(1.4), ());
        assert!(w.apply_move(&mv_str), "the chain string applies");
        let after = w.get_board();
        for &cap in &full.captured {
            assert_eq!(after.chars().nth(cap as usize), Some(' '), "captured square {cap} cleared");
        }
        let end = *full.path.last().unwrap() as usize;
        assert_eq!(after.chars().nth(end), Some('X'), "the king lands at the path end");
    }

    #[test]
    fn move_encoding_round_trips_over_generated_chains() {
        // Display -> parse_path -> match must recover every generated move exactly,
        // including multi-jump chains.
        let mut g = empty(8, true, true, false, true, false);
        set(&mut g, 5, 2, Cell::King(0));
        set(&mut g, 4, 3, Cell::Man(1));
        set(&mut g, 2, 3, Cell::Man(1));
        set(&mut g, 2, 1, Cell::Man(1));
        set(&mut g, 4, 1, Cell::Man(1));
        let moves = g.gen();
        assert!(moves.iter().any(|m| m.path.len() > 2), "need real chains to test");
        for m in &moves {
            let s = format!("{m}");
            let parsed = DMove::parse_path(&s).expect("parses");
            assert_eq!(parsed, m.path, "round-trip path");
        }
    }

    #[test]
    fn forty_ply_no_progress_is_a_draw() {
        // The 40-ply cap and its progress accounting (also the guarantee that
        // every search path terminates). Two lone, non-adjacent kings: seat 0 has
        // only quiet slides, and a quiet king slide TICKS the counter.
        let mut g = empty(8, true, false, false, false, false);
        set(&mut g, 0, 1, Cell::King(0));
        set(&mut g, 7, 0, Cell::King(1));
        g.current = 0;
        assert!(g.gen().iter().all(|m| m.captured.is_empty()), "no capture here");
        let before = g.no_progress;
        let slide = g.gen().into_iter().next().unwrap();
        g.apply(&slide);
        assert_eq!(g.no_progress, before + 1, "a quiet king slide is not progress");
        // At the cap the position is a draw even though legal moves remain.
        g.no_progress = DRAW_PLIES;
        assert!(!g.gen().is_empty());
        assert_eq!(g.term(), Some(ProvenValue::Draw));

        // A capture or a man-move RESETS the counter.
        let mut g2 = empty(8, false, false, false, false, false);
        set(&mut g2, 5, 2, Cell::Man(0));
        g2.current = 0;
        g2.no_progress = 17;
        let step = g2.gen().into_iter().next().unwrap();
        assert!(step.captured.is_empty());
        g2.apply(&step);
        assert_eq!(g2.no_progress, 0, "a man-move is progress");
    }

    #[test]
    fn apply_move_by_string_validates_and_applies() {
        let mut g = DraughtsWasm::new(8, 3, 0, 0, 0, 0, 0, 0, 0); // American
        let first = g.legal_moves().split(',').next().unwrap().to_string();
        assert!(g.apply_move(&first));
        assert!(!g.apply_move("999-998"), "illegal path rejected");
    }

    #[test]
    fn ai_plays_american_and_international() {
        for (n, mr, fly, back, max, mid) in [(8u32, 3u32, 0u32, 0u32, 0u32, 0u32), (10, 4, 1, 1, 1, 0)] {
            let mut g = DraughtsWasm::new(n, mr, fly, back, max, mid, 0, 0, 0);
            g.playout_n(200);
            let m = g.best_move().expect("AI returns a move");
            assert!(g.apply_move(&m));
        }
    }

    // ==================================================================== wave-B

    /// Deterministic engine-only self-play: at each ply pick a legal move by a
    /// seeded RNG index into `gen()`, apply it, record its string. Uses ONLY the
    /// move generator / apply / terminal (no MCTS), so it is fully reproducible and
    /// exercises exactly the code paths the new flags touch.
    fn selfplay(mut g: Draughts, seed: u64, max_plies: usize) -> Vec<String> {
        use rand::rngs::SmallRng;
        use rand::{Rng, SeedableRng};
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut out = Vec::new();
        for _ in 0..max_plies {
            if g.term().is_some() {
                break;
            }
            let moves = g.gen();
            if moves.is_empty() {
                break;
            }
            let i = rng.gen_range(0..moves.len());
            out.push(format!("{}", moves[i]));
            let m = moves[i].clone();
            g.apply(&m);
        }
        out
    }

    // GOLDEN transcripts recorded from the ORIGINAL 7-arg engine (pre-flag), driven
    // by `selfplay` at the fixed seeds below. The permanent tests re-run the SAME
    // deterministic driver on the NEW 9-arg engine with the two new flags 0 and
    // assert byte-for-byte equality — i.e. the new code changed NOTHING for wave-A.
    const AMERICAN_GOLDEN: &str = "42-33,17-26,44-35,26-44,51-37,21-28,58-51,8-17,37-30,23-37,53-44,17-26,44-30,14-23,62-53,23-37,51-44,37-51,60-42,10-17,46-37,28-46-60,42-35,26-44,55-46,19-26,33-19,12-26,46-39,5-14,40-33,26-40-58,39-30,14-21,30-12,3-21,56-49,58-40";
    const INTL_GOLDEN: &str = "67-58,36-45,78-67,45-56,65-47,38-56-78,89-67,34-43,74-65,29-38,67-56,43-54,65-43,32-54,63-45,38-47,58-36,25-47-65,76-54,23-32,72-63,32-43,54-32,21-43,85-74,14-23,61-52,43-61,70-52,16-25,87-76,23-32,94-85,3-14,69-58,30-41,52-30,5-16,98-87,27-36,45-27-5-23-41,12-21,30-12,1-23,81-70,10-21,63-54,23-32,41-23,18-27,87-78,9-18,78-69,27-38,74-63,7-16,83-72,16-27,70-61,38-49,76-67,21-30,54-43,27-38,23-12,30-41,90-81,25-34,43-25,41-52,63-41,18-29,41-32,38-47,58-36,49-58,69-47,29-38,47-29";

    #[test]
    fn wave_a_bit_identity_american_self_play_transcript() {
        // American with the two new flags 0 must reproduce the golden transcript
        // recorded from the original engine — proof the engine is bit-identical.
        let g = mk(AMERICAN);
        assert_eq!(selfplay(g, 0xA5A5A5, 80).join(","), AMERICAN_GOLDEN);
    }

    #[test]
    fn wave_a_bit_identity_international_self_play_transcript() {
        // International (flying kings, backward capture, max-capture, mid-chain
        // pass-through) with the two new flags 0 reproduces its golden transcript.
        let g = mk(INTERNATIONAL);
        assert_eq!(selfplay(g, 0x123456, 80).join(","), INTL_GOLDEN);
    }

    #[test]
    fn wave_a_bit_identity_all_six_variants_reproduce_across_reruns() {
        // Broader identity net: every wave-A variant's deterministic transcript is
        // self-consistent run-to-run (the driver is engine-only, no thread_rng),
        // and — the load-bearing part — none of the six ever enters the new code
        // paths: `capture_priority == 0` and `men_no_king == false` for all of them.
        for p in [AMERICAN, INTERNATIONAL, BRAZILIAN, POOL, RUSSIAN, GIVEAWAY] {
            assert!(!p.7, "wave-A men_no_king must stay off");
            assert_eq!(p.8, 0, "wave-A capture_priority must stay 0");
            let a = selfplay(mk(p), 0xBEEF, 60);
            let b = selfplay(mk(p), 0xBEEF, 60);
            assert_eq!(a, b, "engine-only self-play is deterministic");
        }
    }

    #[test]
    fn priority_normalizes_max_capture_on_in_ctor() {
        // Decision (documented in the ctor): `capture_priority > 0` IMPLIES
        // `max_capture` — the quality tiebreak is a refinement of maximum-capture,
        // never a replacement. We pass max_capture = false for both Spanish and
        // Italian and assert the constructed state has it forced on.
        let sp = mk(SPANISH); // priority 1, max_capture arg = false
        assert!(sp.max_capture, "Spanish: priority>0 forces max_capture on");
        let it = mk(ITALIAN); // priority 2, max_capture arg = false
        assert!(it.max_capture, "Italian: priority>0 forces max_capture on");
        // And the DraughtsWasm wrapper mirrors the same normalization.
        let w = DraughtsWasm::new(8, 3, 1, 0, 0, 0, 0, 0, 1);
        assert!(w.max_capture);
    }

    // ---- man_cannot_capture_kings (Italian) ---------------------------------
    //
    // Board diagram (8×8, seat 0 = 'x' moving UP toward row 0, seat 1 kings 'O'):
    //   row 3:  . . . . . . . .     landing squares (3,0) and (3,4) are empty
    //   row 4:  . O x O . . . .     (4,1)=enemy KING, (4,3)? no — see below
    //   row 5:  . . x . . . . .     (5,2)=our MAN
    // Concretely: our man at (5,2); enemy KING at (4,1) with empty landing (3,0).
    // Under men_no_king the man may NOT jump the king; with a second enemy MAN at
    // (4,3) landing (3,4) it MAY jump the man. Squares are re-derivable: (5,2),
    // (4,1), (4,3), (3,0), (3,4) all have (r+c) odd (dark playing squares).

    #[test]
    fn man_cannot_jump_a_king_but_may_jump_a_man() {
        // Italian flags: men_no_king on, priority 2 (max_capture normalized on).
        let mut g = empty_ex(8, false, false, false, false, false, true, 2);
        set(&mut g, 5, 2, Cell::Man(0)); // our man
        set(&mut g, 4, 1, Cell::King(1)); // enemy KING — forbidden target
        set(&mut g, 4, 3, Cell::Man(1)); // enemy MAN — legal target
        g.current = 0;
        let moves = g.gen();
        assert_eq!(moves.len(), 1, "exactly one legal capture (over the man)");
        assert_eq!(moves[0].captured, vec![rc(&g, 4, 3) as u16], "captured the MAN, not the king");
        // Contrast: with men_no_king OFF (wave-A American, priority 0) the same man
        // may jump the king — proof the guard is the only thing suppressing it.
        let mut us = empty(8, false, false, false, false, false);
        set(&mut us, 5, 2, Cell::Man(0));
        set(&mut us, 4, 1, Cell::King(1));
        us.current = 0;
        let um = us.gen();
        assert!(um.iter().any(|m| m.captured == vec![rc(&us, 4, 1) as u16]), "wave-A: man CAN jump a king");
    }

    #[test]
    fn man_whose_only_jump_is_a_king_falls_back_to_a_quiet_move() {
        // The subtle consequence: our man's ONLY geometric jump is over a king, so
        // under men_no_king it has no capture. No other piece can capture either, so
        // the capture obligation dissolves and quiet moves become legal.
        //   row 3:  x . . . . . . .   (3,0) empty landing behind the king
        //   row 4:  . O . . . . . .   (4,1)=enemy KING
        //   row 5:  . . x . . . . .   (5,2)=our MAN, quiet step to (4,3) available
        let mut g = empty_ex(8, false, false, false, false, false, true, 2);
        set(&mut g, 5, 2, Cell::Man(0));
        set(&mut g, 4, 1, Cell::King(1));
        g.current = 0;
        let moves = g.gen();
        assert!(moves.iter().all(|m| m.captured.is_empty()), "no capture: the only jump is over a king");
        assert!(!moves.is_empty(), "quiet moves become legal");
        // The available quiet move is the man's forward step to (4,3).
        assert!(moves.iter().any(|m| m.path == vec![rc(&g, 5, 2) as u16, rc(&g, 4, 3) as u16]));
    }

    #[test]
    fn king_may_still_jump_a_king_under_men_no_king() {
        // The guard is scoped to MEN: a KING (mover) jumping a king is unaffected.
        //   row 3:  x . . . . . . .   (3,0) empty landing
        //   row 4:  . O . . . . . .   (4,1)=enemy KING
        //   row 5:  . . X . . . . .   (5,2)=our KING (non-flying, Italian)
        let mut g = empty_ex(8, false, false, false, false, false, true, 2);
        set(&mut g, 5, 2, Cell::King(0));
        set(&mut g, 4, 1, Cell::King(1));
        g.current = 0;
        let moves = g.gen();
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].captured, vec![rc(&g, 4, 1) as u16], "king takes king");
    }

    // ---- capture_priority tiers, each isolated by a crafted tie ---------------

    #[test]
    fn spanish_prefers_the_chain_capturing_more_kings() {
        // Flying king at (4,3) with two count-1 captures on opposite diagonals:
        // up-left over an enemy KING at (2,1) (land (1,0)); down-right over an enemy
        // MAN at (5,4) (land (6,5) or (7,6)). All three moves capture exactly one
        // piece, so Spanish's count tie breaks on kings-captured → keep only the
        // king-capturing chain.
        //   row 1:  o . . . . . . .    (1,0) landing
        //   row 2:  . O . . . . . .    (2,1)=enemy KING
        //   row 4:  . . . X . . . .    (4,3)=our flying KING
        //   row 5:  . . . . o . . .    (5,4)=enemy MAN
        let mut g = empty_ex(8, true, false, false, false, false, false, 1); // Spanish
        set(&mut g, 4, 3, Cell::King(0));
        set(&mut g, 2, 1, Cell::King(1));
        set(&mut g, 5, 4, Cell::Man(1));
        g.current = 0;
        let moves = g.gen();
        assert!(!moves.is_empty());
        assert!(moves.iter().all(|m| m.captured == vec![rc(&g, 2, 1) as u16]), "Spanish keeps only the king-capture");
        // Priority 0 (wave-A) keeps BOTH the man- and king-capturing chains.
        let mut wa = empty_ex(8, true, false, false, false, false, false, 0);
        set(&mut wa, 4, 3, Cell::King(0));
        set(&mut wa, 2, 1, Cell::King(1));
        set(&mut wa, 5, 4, Cell::Man(1));
        wa.current = 0;
        let wam = wa.gen();
        assert!(wam.iter().any(|m| m.captured == vec![rc(&wa, 5, 4) as u16]), "wave-A keeps the man-capture too");
        assert!(wam.iter().any(|m| m.captured == vec![rc(&wa, 2, 1) as u16]));
    }

    #[test]
    fn italian_level2_prefers_capturing_with_a_king() {
        // Count tie where one capturer is a KING and one is a MAN, both taking a MAN
        // (kings-captured tie = 0). Italian level 2 ("he must do so with the king")
        // keeps the king-capturer; Spanish (no such level) keeps BOTH.
        //   row 3:  x . . . o . . .    (3,0),(3,4) landings
        //   row 4:  . o . . . o . .    (4,1)=enemy MAN, (4,5)=enemy MAN
        //   row 5:  . . X . . . x .    (5,2)=our KING, (5,6)=our MAN
        fn board(priority: u8) -> Draughts {
            let mut g = empty_ex(8, false, false, false, false, false, false, priority);
            set(&mut g, 5, 2, Cell::King(0));
            set(&mut g, 4, 1, Cell::Man(1));
            set(&mut g, 5, 6, Cell::Man(0));
            set(&mut g, 4, 5, Cell::Man(1));
            g.current = 0;
            g
        }
        let it = board(2).gen();
        assert_eq!(it.len(), 1, "Italian keeps only the king-capturer");
        assert_eq!(it[0].path[0], rc(&board(2), 5, 2) as u16, "the KING is the capturer");
        let sp = board(1).gen();
        assert_eq!(sp.len(), 2, "Spanish has no capturer-is-king level → keeps both");
    }

    #[test]
    fn italian_level3_prefers_capturing_more_kings() {
        // Both capturers are KINGS (level-2 tie); they differ on kings-captured: one
        // takes an enemy KING, the other an enemy MAN. Italian level 3 keeps the
        // king-taker.
        //   row 4:  . O . . . o . .    (4,1)=enemy KING, (4,5)=enemy MAN
        //   row 5:  . . X . . . X .    (5,2),(5,6)=our KINGS
        let mut g = empty_ex(8, false, false, false, false, false, false, 2);
        set(&mut g, 5, 2, Cell::King(0));
        set(&mut g, 4, 1, Cell::King(1)); // king target
        set(&mut g, 5, 6, Cell::King(0));
        set(&mut g, 4, 5, Cell::Man(1)); // man target
        g.current = 0;
        let moves = g.gen();
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].captured, vec![rc(&g, 4, 1) as u16], "kept the king-capturing chain");
    }

    #[test]
    fn italian_level4_prefers_capturing_a_king_earliest() {
        // Everything above ties: two count-2 chains, each by a KING, each taking
        // exactly {1 king + 1 man}. They differ only in ORDER — chain X jumps the
        // king FIRST, chain Y jumps the man first then the king. Italian level 4
        // ("capture wherever the king occurs first") keeps chain X.
        //   Chain X: (5,2) → over KING(4,1) → (3,0) → over MAN(2,1) → (1,2)
        //   Chain Y: (5,6) → over MAN(4,5)  → (3,4) → over KING(2,5) → (1,6)
        let mut g = empty_ex(8, false, false, false, false, false, false, 2);
        set(&mut g, 5, 2, Cell::King(0));
        set(&mut g, 4, 1, Cell::King(1));
        set(&mut g, 2, 1, Cell::Man(1));
        set(&mut g, 5, 6, Cell::King(0));
        set(&mut g, 4, 5, Cell::Man(1));
        set(&mut g, 2, 5, Cell::King(1));
        g.current = 0;
        let moves = g.gen();
        assert_eq!(moves.len(), 1, "level-4 tiebreak leaves exactly one chain");
        let m = &moves[0];
        assert_eq!(m.captured.len(), 2, "a genuine 2-capture chain");
        assert_eq!(m.path[0], rc(&g, 5, 2) as u16, "chain X is the king-first chain");
        assert!(matches!(g.grid[m.captured[0] as usize], Cell::King(_)), "its FIRST capture is the king");
        assert_eq!(m.captured[0], rc(&g, 4, 1) as u16);
    }

    #[test]
    fn priority_still_forces_the_longest_chain_first() {
        // max_capture-interaction: the quality tiebreak sits ON TOP of the count
        // filter, never overriding it. A 2-chain capturing 0 kings must beat a
        // 1-chain capturing a king (count dominates), even under Italian.
        //   (5,2) → over MAN(4,1) → (3,0) → over MAN(2,1) → (1,2)   [count 2, 0 kings]
        //   separate KING capturer (5,6) → over KING(4,5) → (3,4)  [count 1, 1 king]
        let mut g = empty_ex(8, false, false, false, false, false, false, 2);
        set(&mut g, 5, 2, Cell::King(0));
        set(&mut g, 4, 1, Cell::Man(1));
        set(&mut g, 2, 1, Cell::Man(1));
        set(&mut g, 5, 6, Cell::King(0));
        set(&mut g, 4, 5, Cell::King(1));
        g.current = 0;
        let moves = g.gen();
        assert_eq!(moves.len(), 1, "count filter runs first");
        assert_eq!(moves[0].captured.len(), 2, "the 2-chain wins on count before quality");
        assert_eq!(moves[0].path[0], rc(&g, 5, 2) as u16);
    }
}
