//! Draughts (checkers) — ONE flag-driven engine backing six national variants.
//!
//! Six wave-A variants ship as config-only tiles over this single engine (the
//! per-variant flag table is asserted in the tests below):
//!
//! | Variant       | size | rows | flying | menBack | maxCap | midChain | misère |
//! |---------------|------|------|--------|---------|--------|----------|--------|
//! | American ⭐    |  8   |  3   |  no    |  no     |  no    |  n/a     |  no    |
//! | International |  10  |  4   |  yes   |  yes    |  yes   |  no      |  no    |
//! | Brazilian     |  8   |  3   |  yes   |  yes    |  yes   |  no      |  no    |
//! | Pool          |  8   |  3   |  yes   |  yes    |  no    |  no      |  no    |
//! | Russian       |  8   |  3   |  yes   |  yes    |  no    |  YES     |  no    |
//! | Giveaway 🙃    |  8   |  3   |  no    |  no     |  no    |  n/a     |  YES   |
//!
//! Table verified 2026-07-06 against the per-variant Wikipedia pages (English/
//! International/Brazilian/Russian draughts, Pool checkers, Poddavki). Load-
//! bearing quotes live at each decision point below.
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
    /// Plies since the last capture or man-move (for the 40-ply draw cap).
    no_progress: u32,
}

impl Draughts {
    fn new(n: usize, men_rows: usize, flying: bool, men_back: bool, max_capture: bool, promote_mid: bool, misere: bool) -> Self {
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
        Self { n, grid, current: 0, flying, men_back, max_capture, promote_mid, misere, no_progress: 0 }
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
        if self.no_progress >= DRAW_PLIES {
            return Some(ProvenValue::Draw);
        }
        if self.gen().is_empty() {
            // The player to move has no legal move. Normally that's a loss; under
            // misère (Giveaway) it's a WIN: "A player with no valid move remaining
            // win[s]. This is the case if the player either has no pieces left or
            // if a player's pieces are obstructed from making a legal move by the
            // pieces of the opponent." (Poddavki).
            return Some(if self.misere { ProvenValue::Win } else { ProvenValue::Loss });
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
}

#[wasm_bindgen]
impl DraughtsWasm {
    /// `size` 8|10|12, `men_rows` 2–4 typically; the remaining args are the seven
    /// rule flags (0/1). See the variant table in the module docs.
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(constructor)]
    pub fn new(size: u32, men_rows: u32, flying_kings: u32, men_capture_back: u32, max_capture: u32, promote_mid_chain: u32, misere: u32) -> Self {
        let (n, men_rows) = clamp_cfg(size, men_rows);
        let flying = flying_kings != 0;
        let men_back = men_capture_back != 0;
        let max_capture = max_capture != 0;
        let promote_mid = promote_mid_chain != 0;
        let misere = misere != 0;
        let state = Draughts::new(n, men_rows, flying, men_back, max_capture, promote_mid, misere);
        Self {
            manager: MCTSManager::new(state, DrCfg, DrEval, UCTPolicy::new(1.4), ()),
            n,
            men_rows,
            flying,
            men_back,
            max_capture,
            promote_mid,
            misere,
        }
    }

    fn fresh_state(&self) -> Draughts {
        Draughts::new(self.n, self.men_rows, self.flying, self.men_back, self.max_capture, self.promote_mid, self.misere)
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

    // Flag presets for the six wave-A variants (size, men_rows, flying, men_back,
    // max_capture, promote_mid, misere) — mirrors the module-doc table.
    const AMERICAN: (usize, usize, bool, bool, bool, bool, bool) = (8, 3, false, false, false, false, false);
    const INTERNATIONAL: (usize, usize, bool, bool, bool, bool, bool) = (10, 4, true, true, true, false, false);
    const BRAZILIAN: (usize, usize, bool, bool, bool, bool, bool) = (8, 3, true, true, true, false, false);
    const POOL: (usize, usize, bool, bool, bool, bool, bool) = (8, 3, true, true, false, false, false);
    const RUSSIAN: (usize, usize, bool, bool, bool, bool, bool) = (8, 3, true, true, false, true, false);
    const GIVEAWAY: (usize, usize, bool, bool, bool, bool, bool) = (8, 3, false, false, false, false, true);

    fn mk(p: (usize, usize, bool, bool, bool, bool, bool)) -> Draughts {
        Draughts::new(p.0, p.1, p.2, p.3, p.4, p.5, p.6)
    }
    /// An empty board with the given flags (for hand-built positions).
    fn empty(n: usize, flying: bool, men_back: bool, max_capture: bool, promote_mid: bool, misere: bool) -> Draughts {
        let mut g = Draughts::new(n, 1, flying, men_back, max_capture, promote_mid, misere);
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
        assert_eq!(AMERICAN, (8, 3, false, false, false, false, false));
        assert_eq!(INTERNATIONAL, (10, 4, true, true, true, false, false));
        assert_eq!(BRAZILIAN, (8, 3, true, true, true, false, false));
        assert_eq!(POOL, (8, 3, true, true, false, false, false));
        assert_eq!(RUSSIAN, (8, 3, true, true, false, true, false));
        assert_eq!(GIVEAWAY, (8, 3, false, false, false, false, true));
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
        let mut g = DraughtsWasm::new(8, 3, 0, 0, 0, 0, 0); // American
        let first = g.legal_moves().split(',').next().unwrap().to_string();
        assert!(g.apply_move(&first));
        assert!(!g.apply_move("999-998"), "illegal path rejected");
    }

    #[test]
    fn ai_plays_american_and_international() {
        for (n, mr, fly, back, max, mid) in [(8u32, 3u32, 0u32, 0u32, 0u32, 0u32), (10, 4, 1, 1, 1, 0)] {
            let mut g = DraughtsWasm::new(n, mr, fly, back, max, mid, 0);
            g.playout_n(200);
            let m = g.best_move().expect("AI returns a move");
            assert!(g.apply_move(&m));
        }
    }
}
