//! Vanguard — a hidden-rank capture-the-Standard duel on the public-domain
//! mechanics of L'Attaque (patented 1908 by Hermance Edan; the design has been
//! public domain for ~a century). Two armies of rank-hidden pieces deploy behind
//! their own lines, then manoeuvre; a piece's rank is revealed to the opponent
//! only when it fights. You win by capturing the enemy Standard, or by leaving
//! the enemy with no legal move. The commercial descendant's trademarked name is
//! held by a modern publisher and is NEVER surfaced in user strings — only the
//! 1908 public-domain mechanics are used, and every piece name here
//! (Marshal / Captain / Sergeant / Trooper / Scout / Sapper / Spy / Bomb /
//! Standard) is our own original set.
//!
//! ## Rules provenance (Wikipedia, "L'Attaque", https://en.wikipedia.org/wiki/L%27Attaque)
//! Load-bearing sentences quoted verbatim (see the task report for full context):
//! - **Combat:** "the one with the lower rank is captured and removed from the
//!   board. If the pieces are of equal rank, both are removed."
//! - **Spy:** the Spy "can capture the *Commander-in-chief*" (the top rank) — and,
//!   per the family rule the article attests, only when the Spy is the attacker;
//!   the top piece is otherwise "vulnerable only to a *Mine* or attacking *Spy*".
//! - **Mine (our Bomb):** "Immobile; any piece (except a *Sapper*) attacking a
//!   *Mine* is removed from the game." The Mine itself PERSISTS — only a Sapper
//!   captures it without being destroyed. We follow this exactly: a non-Sapper
//!   attacker is removed and the Bomb stays on its square (now revealed).
//! - **Scout:** "Can move any distance in a non-diagonal straight line without
//!   leaping over pieces or lakes." (The article's further Scout embellishments —
//!   "cannot capture another *Scout*; an attacking *Scout* returns to their space
//!   after attacking" — are edition-specific flourishes we deliberately OMIT: our
//!   Scout fights by plain rank, documented as our design choice within the
//!   attested long-move mechanic. The long move is a knob, `scout_long`.)
//! - **Immovability:** the "*Flag*" (our Standard) and the "*Mine*" (our Bomb) are
//!   both "Immobile".
//! - **Winning:** "The game continues until a player captures either their
//!   opponent's *Flag* or all of their moveable pieces." We implement both: a
//!   Standard capture, or the opponent having no legal move at the start of its
//!   turn (immobilisation — all movables gone or boxed in).
//!
//! ## Board geometry (our design choice within attested L'Attaque lake mechanics)
//! Row-major cell indices, `cell = row * w + col`. Seat 0 deploys along the BOTTOM
//! rows, seat 1 along the TOP rows. `size_preset 0` = 6×7 "mini" (one 1×2 lake);
//! `size_preset 1` = 8×8 "standard" (two 1×2 lakes). Setup zones exclude the lake
//! rows by construction, so a placement can never land on a lake.
//!
//! ```text
//!   MINI 6×7 (preset 0)              STANDARD 8×8 (preset 1)
//!   col 0 1 2 3 4 5                  col 0 1 2 3 4 5 6 7
//!  r0  . . . . . .  seat1 zone      r0  . . . . . . . .  seat1 zone
//!  r1  . . . . . .  (rows 0-1)      r1  . . . . . . . .  (rows 0-2)
//!  r2  . . . . . .                  r2  . . . . . . . .
//!  r3  . . # # . .  lake @20,21     r3  . . # . . # . .  lakes @26,29
//!  r4  . . . . . .                  r4  . . # . . # . .        @34,37
//!  r5  . . . . . .  seat0 zone      r5  . . . . . . . .  seat0 zone
//!  r6  . . . . . .  (rows 5-6)      r6  . . . . . . . .  (rows 5-7)
//!                                   r7  . . . . . . . .
//! ```
//! Mini zone capacity = 2 rows × 6 = 12 squares; standard = 3 rows × 8 = 24.
//!
//! ## Secrecy is the product — where the (placeholder) AI lives
//! Position is public (you see the enemy's piece-backs once play begins); RANK is
//! secret until combat reveals it. `get_board_for(seat)` renders the queried
//! seat's own pieces with full information and every enemy piece as an anonymous
//! back (`?`) EXCEPT those a fight has publicly revealed (or at game over). The
//! public movement flags (`has_moved`, `moved_long ⇒ Scout`) travel with every
//! piece and are shown for both sides — they are legitimate public inference. A
//! hidden enemy rank NEVER appears in the wrong seat's board string, in
//! `legal_moves`, or in `last_move_summary_for` (which is a purely public combat/
//! move beat, identical for both seats — safe by construction). `weak_move` here
//! is a PLACEHOLDER: a uniform-random legal move (seeded). The real determinized
//! PIMC opponent is Task 7-2; this engine ships rules-complete and audit-
//! registrable now (the calibration ladder is likewise deferred to 7-2 — the
//! entry in `calibrate.rs` exists only so the audit fuzzer exercises the rules).

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use wasm_bindgen::prelude::*;

/// Number of distinct piece kinds (the ctor's per-type composition knobs).
const N_KINDS: usize = 9;

/// Plies (half-moves) with NO combat before the game is declared a positional
/// Draw. Draughts uses ~40 plies without a capture; Vanguard's armies are larger
/// (up to 24 pieces total) and manoeuvring into contact takes longer, so we allow
/// 2× that — 80 plies (40 full moves) of bloodless shuffling before a draw. Any
/// combat (a piece removed) resets the counter. This, together with the
/// two-squares rule, guarantees termination.
const NO_COMBAT_CAP: u32 = 80;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Marshal,
    Captain,
    Sergeant,
    Trooper,
    Scout,
    Sapper,
    Spy,
    Bomb,
    Standard,
}

impl Kind {
    fn from_index(i: usize) -> Option<Kind> {
        Some(match i {
            0 => Kind::Marshal,
            1 => Kind::Captain,
            2 => Kind::Sergeant,
            3 => Kind::Trooper,
            4 => Kind::Scout,
            5 => Kind::Sapper,
            6 => Kind::Spy,
            7 => Kind::Bomb,
            8 => Kind::Standard,
            _ => return None,
        })
    }

    /// Single-char render glyph (internal code the UI maps to a label). Distinct
    /// per kind so a seat's board string is unambiguous and greppable; the enemy
    /// back is rendered `?` instead (see `render`).
    fn glyph(self) -> char {
        match self {
            Kind::Marshal => 'M',
            Kind::Captain => 'C',
            Kind::Sergeant => 'G', // seRGeant — 'S' is the Scout
            Kind::Trooper => 'T',
            Kind::Scout => 'S',
            Kind::Sapper => 'P', // saPper
            Kind::Spy => 'Y',
            Kind::Bomb => 'B',
            Kind::Standard => 'D', // stanDard
        }
    }

    /// Movable pieces are everything except the immobile Bomb and Standard.
    fn movable(self) -> bool {
        !matches!(self, Kind::Bomb | Kind::Standard)
    }

    /// Combat rank for movable pieces (Spy weakest = 0 … Marshal strongest = 6);
    /// `None` for the immobile Bomb/Standard, which never attack.
    fn rank(self) -> Option<i32> {
        Some(match self {
            Kind::Spy => 0,
            Kind::Sapper => 1,
            Kind::Scout => 2,
            Kind::Trooper => 3,
            Kind::Sergeant => 4,
            Kind::Captain => 5,
            Kind::Marshal => 6,
            Kind::Bomb | Kind::Standard => return None,
        })
    }

    fn rank_str(self) -> String {
        match self.rank() {
            Some(r) => r.to_string(),
            None => "x".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Piece {
    kind: Kind,
    owner: u8,
    id: u32,          // stable per-piece id (for the two-squares shuttle tracker)
    revealed: bool,   // rank is public (a fight exposed it) — permanent
    has_moved: bool,  // has ever moved — public (⇒ cannot be a Bomb/Standard)
    moved_long: bool, // has ever moved 2+ in a straight line — public (⇒ Scout)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EndReason {
    None,
    Standard,     // enemy Standard captured
    Immobilised,  // opponent had no legal move at turn start
    Draw,         // no-combat ply cap reached
}

#[derive(Clone)]
struct Vanguard {
    w: usize,
    h: usize,
    scout_long: bool,
    army: [usize; N_KINDS],  // per-kind counts (Standard forced to 1); PUBLIC + symmetric
    lakes: Vec<bool>,        // impassable cells
    zone: [Vec<bool>; 2],    // per-seat setup cells
    board: Vec<Option<Piece>>,
    placed: [[usize; N_KINDS]; 2],
    next_id: u32,
    current: u8,
    winner: Option<u8>,
    draw: bool,
    reason: EndReason,
    last_summary: String,
    // Two-squares rule: per seat, the last-moved piece id + the {min,max} square
    // pair it shuttled over + how many consecutive own-turns it stayed on that
    // pair. A move that would make the count reach 3 on the same pair is illegal.
    shuttle: [Option<(u32, usize, usize, u32)>; 2],
    no_combat_plies: u32,
}

impl Vanguard {
    /// `counts` are per-kind piece counts in Kind order (Marshal … Standard).
    /// Feasibility policy (documented, tested at extremes):
    /// - The Standard count is FORCED to exactly 1, whatever is passed.
    /// - Every other kind is capped to 0..=6.
    /// - If the total exceeds the setup-zone capacity, pieces are dropped one at a
    ///   time in this deterministic priority — Bomb, Sapper, Scout, Trooper,
    ///   Sergeant, Spy, Captain, Marshal (each kind drained before the next) —
    ///   until it fits. The Standard is NEVER dropped, so a game is always
    ///   playable.
    fn new(preset: u32, counts: [usize; N_KINDS], scout_long: bool) -> Self {
        let (w, h, lakes, zone0, zone1, cap) = Self::geometry(preset);
        let mut army = [0usize; N_KINDS];
        for k in 0..N_KINDS {
            army[k] = counts[k].min(6);
        }
        army[Kind::Standard as usize] = 1; // exactly one Standard, always
        // Drop overflow deterministically (Standard excluded).
        const DROP: [usize; 8] = [7, 5, 4, 3, 2, 6, 1, 0]; // Bomb,Sapper,Scout,Trooper,Sergeant,Spy,Captain,Marshal
        loop {
            let total: usize = army.iter().sum();
            if total <= cap {
                break;
            }
            let mut dropped = false;
            for &k in DROP.iter() {
                if army[k] > 0 {
                    army[k] -= 1;
                    dropped = true;
                    break;
                }
            }
            if !dropped {
                break; // only the Standard remains (always ≤ cap)
            }
        }
        let n = w * h;
        Vanguard {
            w,
            h,
            scout_long,
            army,
            lakes,
            zone: [zone0, zone1],
            board: vec![None; n],
            placed: [[0; N_KINDS]; 2],
            next_id: 0,
            current: 0,
            winner: None,
            draw: false,
            reason: EndReason::None,
            last_summary: String::new(),
            shuttle: [None, None],
            no_combat_plies: 0,
        }
    }

    /// (w, h, lakes, seat0 zone, seat1 zone, zone capacity) for a preset.
    #[allow(clippy::type_complexity)]
    fn geometry(preset: u32) -> (usize, usize, Vec<bool>, Vec<bool>, Vec<bool>, usize) {
        if preset == 0 {
            // Mini 6×7: one horizontal 1×2 lake at row 3, cols 2-3 (cells 20,21).
            let (w, h) = (6usize, 7usize);
            let mut lakes = vec![false; w * h];
            lakes[20] = true;
            lakes[21] = true;
            let mut z0 = vec![false; w * h]; // seat 0: rows 5-6
            let mut z1 = vec![false; w * h]; // seat 1: rows 0-1
            for c in 0..w {
                z1[c] = true; // row 0
                z1[w + c] = true; // row 1
                z0[5 * w + c] = true; // row 5
                z0[6 * w + c] = true; // row 6
            }
            (w, h, lakes, z0, z1, 2 * w)
        } else {
            // Standard 8×8: two vertical 1×2 lakes over rows 3-4, at cols 2 and 5.
            let (w, h) = (8usize, 8usize);
            let mut lakes = vec![false; w * h];
            for &c in &[26usize, 29, 34, 37] {
                lakes[c] = true;
            }
            let mut z0 = vec![false; w * h]; // seat 0: rows 5-7
            let mut z1 = vec![false; w * h]; // seat 1: rows 0-2
            for c in 0..w {
                for r in 0..3 {
                    z1[r * w + c] = true;
                }
                for r in 5..8 {
                    z0[r * w + c] = true;
                }
            }
            (w, h, lakes, z0, z1, 3 * w)
        }
    }

    fn army_total(&self) -> usize {
        self.army.iter().sum()
    }

    fn placed_total(&self, seat: usize) -> usize {
        self.placed[seat].iter().sum()
    }

    fn both_placed(&self) -> bool {
        self.placed_total(0) == self.army_total() && self.placed_total(1) == self.army_total()
    }

    fn over(&self) -> bool {
        self.winner.is_some() || self.draw
    }

    fn is_terminal(&self) -> bool {
        self.over()
    }

    /// `""` in progress, `"Draw"`, or the 1-indexed winner seat digit.
    fn result(&self) -> String {
        if self.draw {
            "Draw".to_string()
        } else if let Some(w) = self.winner {
            format!("{}", w + 1)
        } else {
            String::new()
        }
    }

    /// The `resultFlavor` seam: why the game ended (empty while in progress).
    fn result_flavor(&self) -> &'static str {
        match self.reason {
            EndReason::Standard => "standard",
            EndReason::Immobilised => "immobilised",
            EndReason::Draw => "draw",
            EndReason::None => "",
        }
    }

    // ---- move parse/format (single-sourced; round-trip tested) --------------

    fn format_place(kind: usize, cell: usize) -> String {
        format!("p:{kind}:{cell}")
    }

    fn format_move(from: usize, to: usize) -> String {
        format!("m:{from}:{to}")
    }

    fn parse_place(&self, s: &str) -> Option<(usize, usize)> {
        let mut it = s.split(':');
        if it.next()? != "p" {
            return None;
        }
        let k: usize = it.next()?.parse().ok()?;
        let cell: usize = it.next()?.parse().ok()?;
        if it.next().is_some() || k >= N_KINDS || cell >= self.w * self.h {
            return None;
        }
        Some((k, cell))
    }

    fn parse_play(&self, s: &str) -> Option<(usize, usize)> {
        let mut it = s.split(':');
        if it.next()? != "m" {
            return None;
        }
        let f: usize = it.next()?.parse().ok()?;
        let t: usize = it.next()?.parse().ok()?;
        if it.next().is_some() || f >= self.w * self.h || t >= self.w * self.h {
            return None;
        }
        Some((f, t))
    }

    // ---- two-squares (anti-shuffle) rule ------------------------------------

    fn two_squares_ok(&self, seat: usize, id: u32, a: usize, b: usize) -> bool {
        let pair = if a < b { (a, b) } else { (b, a) };
        !matches!(
            self.shuttle[seat],
            Some((sid, pa, pb, cnt)) if sid == id && (pa, pb) == pair && cnt >= 2
        )
    }

    fn record_shuttle(&mut self, seat: usize, id: u32, a: usize, b: usize) {
        let pair = if a < b { (a, b) } else { (b, a) };
        self.shuttle[seat] = match self.shuttle[seat] {
            Some((sid, pa, pb, cnt)) if sid == id && (pa, pb) == pair => Some((sid, pa, pb, cnt + 1)),
            _ => Some((id, pair.0, pair.1, 1)),
        };
    }

    // ---- movement / legal move generation -----------------------------------

    /// Every legal `(from, to)` play move for `seat` (empty during placement or at
    /// game over). Encodes movement (1 orthogonal step; Scout any straight
    /// orthogonal distance through empties when `scout_long`), lake/own-piece
    /// blocking, attacks onto enemy squares, and the two-squares ban.
    fn play_moves(&self, seat: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        if !self.both_placed() || self.over() {
            return out;
        }
        let (w, h) = (self.w, self.h);
        for from in 0..w * h {
            let p = match &self.board[from] {
                Some(p) if p.owner as usize == seat && p.kind.movable() => *p,
                _ => continue,
            };
            let (r, c) = (from / w, from % w);
            let long = p.kind == Kind::Scout && self.scout_long;
            let maxstep = if long { w.max(h) as i32 } else { 1 };
            for (dr, dc) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                for step in 1..=maxstep {
                    let nr = r as i32 + dr * step;
                    let nc = c as i32 + dc * step;
                    if nr < 0 || nc < 0 || nr >= h as i32 || nc >= w as i32 {
                        break;
                    }
                    let ncell = nr as usize * w + nc as usize;
                    if self.lakes[ncell] {
                        break; // cannot enter or leap a lake
                    }
                    match &self.board[ncell] {
                        None => {
                            if self.two_squares_ok(seat, p.id, from, ncell) {
                                out.push((from, ncell));
                            }
                            // a Scout may continue through this empty square
                        }
                        Some(occ) => {
                            if occ.owner as usize != seat
                                && self.two_squares_ok(seat, p.id, from, ncell)
                            {
                                out.push((from, ncell)); // attack, then stop
                            }
                            break; // own piece blocks; enemy stops the ray
                        }
                    }
                }
            }
        }
        out
    }

    // ---- applying moves -----------------------------------------------------

    fn apply(&mut self, mov: &str) -> bool {
        if self.over() {
            return false;
        }
        if let Some(rest) = mov.strip_prefix("p:") {
            let _ = rest;
            self.apply_place(mov)
        } else if mov.starts_with("m:") {
            self.apply_play(mov)
        } else {
            false
        }
    }

    fn apply_place(&mut self, mov: &str) -> bool {
        if self.both_placed() || self.over() {
            return false;
        }
        let (k, cell) = match self.parse_place(mov) {
            Some(v) => v,
            None => return false,
        };
        let seat = self.current as usize;
        if self.army[k] == 0 || self.placed[seat][k] >= self.army[k] {
            return false; // no such piece left to place
        }
        if !self.zone[seat][cell] || self.board[cell].is_some() {
            return false; // wrong zone / lake / occupied
        }
        let kind = Kind::from_index(k).unwrap();
        let id = self.next_id;
        self.next_id += 1;
        self.board[cell] = Some(Piece {
            kind,
            owner: seat as u8,
            id,
            revealed: false,
            has_moved: false,
            moved_long: false,
        });
        self.placed[seat][k] += 1;
        if self.placed_total(seat) == self.army_total() {
            if seat == 0 {
                self.current = 1; // seat 1 now deploys its whole army
            } else {
                self.current = 0; // both deployed → play begins with seat 0
                                  // Entering play: seat 0 might already be immobilised
                                  // (e.g. an all-Standard army).
                if self.play_moves(0).is_empty() {
                    self.winner = Some(1);
                    self.reason = EndReason::Immobilised;
                }
            }
        }
        true
    }

    fn apply_play(&mut self, mov: &str) -> bool {
        if self.over() || !self.both_placed() {
            return false;
        }
        let (from, to) = match self.parse_play(mov) {
            Some(v) => v,
            None => return false,
        };
        let seat = self.current as usize;
        if !self.play_moves(seat).contains(&(from, to)) {
            return false;
        }
        let mut attacker = self.board[from].take().unwrap();
        let id = attacker.id;
        let dist = ((from / self.w) as i32 - (to / self.w) as i32).abs()
            + ((from % self.w) as i32 - (to % self.w) as i32).abs();
        attacker.has_moved = true;
        if attacker.kind == Kind::Scout && dist >= 2 {
            attacker.moved_long = true;
        }
        self.record_shuttle(seat, id, from, to);

        match self.board[to].take() {
            None => {
                // Quiet move. Public summary carries the glyph ONLY if the piece is
                // already publicly known (revealed by a past fight, or a long Scout
                // move which is itself public); otherwise `?`.
                let g = if attacker.revealed || attacker.moved_long {
                    attacker.kind.glyph()
                } else {
                    '?'
                };
                self.last_summary =
                    format!("q:{from}:{to}:{g}:{}", if attacker.moved_long { 1 } else { 0 });
                self.board[to] = Some(attacker);
                self.no_combat_plies += 1;
            }
            Some(mut def) => {
                self.no_combat_plies = 0; // a fight happened
                let ag = attacker.kind.glyph();
                let dg = def.kind.glyph();
                let ar = attacker.kind.rank_str();
                let dr = def.kind.rank_str();
                if def.kind == Kind::Standard {
                    // Capture the enemy Standard ⇒ immediate win.
                    attacker.revealed = true;
                    self.board[to] = Some(attacker);
                    self.winner = Some(seat as u8);
                    self.reason = EndReason::Standard;
                    self.last_summary = format!("c:{from}:{to}:{ag}:{ar}:{dg}:{dr}:sc");
                    return true;
                } else if def.kind == Kind::Bomb {
                    if attacker.kind == Kind::Sapper {
                        // Sapper defuses the Bomb (captures it) and takes the square.
                        attacker.revealed = true;
                        self.board[to] = Some(attacker);
                        self.last_summary = format!("c:{from}:{to}:{ag}:{ar}:{dg}:{dr}:bd");
                    } else {
                        // Bomb destroys any non-Sapper attacker and PERSISTS (revealed).
                        def.revealed = true;
                        self.board[to] = Some(def);
                        self.last_summary = format!("c:{from}:{to}:{ag}:{ar}:{dg}:{dr}:bh");
                    }
                } else {
                    // Both movable: rank duel, with the Spy's attacking special.
                    let arank = attacker.kind.rank().unwrap();
                    let drank = def.kind.rank().unwrap();
                    // Spy beats the Marshal — but ONLY when the Spy is the attacker;
                    // otherwise plain rank decides.
                    let att_wins =
                        (attacker.kind == Kind::Spy && def.kind == Kind::Marshal) || arank > drank;
                    let outcome = if att_wins {
                        1
                    } else if arank < drank {
                        -1
                    } else {
                        0 // equal ⇒ both removed
                    };
                    match outcome {
                        1 => {
                            attacker.revealed = true;
                            self.board[to] = Some(attacker); // defender removed
                            self.last_summary = format!("c:{from}:{to}:{ag}:{ar}:{dg}:{dr}:aw");
                        }
                        -1 => {
                            def.revealed = true;
                            self.board[to] = Some(def); // attacker removed
                            self.last_summary = format!("c:{from}:{to}:{ag}:{ar}:{dg}:{dr}:dw");
                        }
                        _ => {
                            // both removed — square left empty
                            self.last_summary = format!("c:{from}:{to}:{ag}:{ar}:{dg}:{dr}:mm");
                        }
                    }
                }
            }
        }

        // Post-move: hand off and check end conditions (Standard win already
        // returned above).
        if self.winner.is_none() && !self.draw {
            self.current = 1 - self.current; // switch to the opponent
            let nc = self.current as usize;
            if self.play_moves(nc).is_empty() {
                // Opponent has no legal move ⇒ the mover wins by immobilisation.
                self.winner = Some(1 - self.current);
                self.reason = EndReason::Immobilised;
            } else if self.no_combat_plies >= NO_COMBAT_CAP {
                self.draw = true;
                self.reason = EndReason::Draw;
            }
        }
        true
    }

    // ---- placeholder move selection (7-2 replaces with determinized PIMC) ----

    /// PLACEHOLDER: a uniform-random legal move (seeded). During placement it
    /// deploys one random remaining piece into a random empty zone square; during
    /// play it picks a random legal move. Ignores playout/temperature knobs — the
    /// real strength ladder is Task 7-2.
    fn weak_move(&self, seed: u32) -> Option<String> {
        if self.over() {
            return None;
        }
        let seat = self.current as usize;
        let mut rng = SmallRng::seed_from_u64((seed as u64) ^ 0xC0_1A_A5 ^ ((seat as u64) << 24));
        if !self.both_placed() {
            let types: Vec<usize> =
                (0..N_KINDS).filter(|&k| self.placed[seat][k] < self.army[k]).collect();
            let cells: Vec<usize> = (0..self.w * self.h)
                .filter(|&c| self.zone[seat][c] && self.board[c].is_none())
                .collect();
            if types.is_empty() || cells.is_empty() {
                return None;
            }
            let k = types[rng.gen_range(0..types.len())];
            let c = cells[rng.gen_range(0..cells.len())];
            Some(Self::format_place(k, c))
        } else {
            let moves = self.play_moves(seat);
            if moves.is_empty() {
                return None;
            }
            let (f, t) = moves[rng.gen_range(0..moves.len())];
            Some(Self::format_move(f, t))
        }
    }

    /// PLACEHOLDER "best" move: the first legal move deterministically (7-2 gives
    /// this real strength). Never reads hidden enemy ranks.
    fn best_move(&self) -> Option<String> {
        if self.over() {
            return None;
        }
        let seat = self.current as usize;
        if !self.both_placed() {
            let cell = (0..self.w * self.h)
                .find(|&c| self.zone[seat][c] && self.board[c].is_none())?;
            for k in 0..N_KINDS {
                if self.placed[seat][k] < self.army[k] {
                    return Some(Self::format_place(k, cell));
                }
            }
            None
        } else {
            let mut moves = self.play_moves(seat);
            if moves.is_empty() {
                return None;
            }
            moves.sort_unstable();
            let (f, t) = moves[0];
            Some(Self::format_move(f, t))
        }
    }

    /// A sample of legal moves as strings (placement options or play moves) — for
    /// the UI's highlighting and the random-move fallback. Never leaks enemy rank.
    fn legal_moves_list(&self) -> Vec<String> {
        if self.over() {
            return Vec::new();
        }
        let seat = self.current as usize;
        if !self.both_placed() {
            let mut out = Vec::new();
            let cells: Vec<usize> = (0..self.w * self.h)
                .filter(|&c| self.zone[seat][c] && self.board[c].is_none())
                .collect();
            for k in 0..N_KINDS {
                if self.placed[seat][k] < self.army[k] {
                    for &c in &cells {
                        out.push(Self::format_place(k, c));
                        if out.len() >= 64 {
                            return out;
                        }
                    }
                }
            }
            out
        } else {
            self.play_moves(seat)
                .into_iter()
                .map(|(f, t)| Self::format_move(f, t))
                .collect()
        }
    }

    /// Deterministic full-army deployment for the CURRENT seat (the "deploy for
    /// me" helper): army pieces in Kind order into the seat's zone squares in
    /// index order. Every move is legal on a fresh setup (zone capacity ≥ army).
    fn random_placement(&self) -> Vec<String> {
        let seat = self.current as usize;
        let cells: Vec<usize> = (0..self.w * self.h).filter(|&c| self.zone[seat][c]).collect();
        let mut out = Vec::new();
        let mut idx = 0usize;
        for k in 0..N_KINDS {
            for _ in 0..self.army[k] {
                if idx >= cells.len() {
                    return out;
                }
                out.push(Self::format_place(k, cells[idx]));
                idx += 1;
            }
        }
        out
    }

    // ---- per-seat board rendering (the secrecy boundary) --------------------

    /// Board string for `seat`'s eyes only. Fields joined by `|`:
    /// `phase|w|h|scoutLong|current|winner|reason|army|myRemaining|mine|theirs`.
    /// `mine` lists the seat's OWN pieces `cell.glyph.rev.moved.long`; `theirs`
    /// lists enemy pieces the same way but with glyph `?` UNLESS the piece has been
    /// revealed by combat (or the game is over). Enemy pieces are omitted entirely
    /// until both armies are deployed (deployment is secret). The load-bearing
    /// guarantee: a hidden enemy rank never appears in `theirs` — see
    /// `wrong_seat_never_sees_hidden_ranks`.
    fn render(&self, seat: usize) -> String {
        let seat = seat.min(1);
        let phase = if self.over() {
            "over"
        } else if self.both_placed() {
            "play"
        } else {
            "place"
        };
        let winner = match self.winner {
            Some(w) => w.to_string(),
            None => String::new(),
        };
        let army = self.army.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
        let myrem = (0..N_KINDS)
            .map(|k| (self.army[k] - self.placed[seat][k]).to_string())
            .collect::<Vec<_>>()
            .join(",");
        let show_enemy = self.both_placed();
        let over = self.over();
        let mut mine = Vec::new();
        let mut theirs = Vec::new();
        for cell in 0..self.w * self.h {
            if let Some(p) = &self.board[cell] {
                let rev = if p.revealed { 1 } else { 0 };
                let mv = if p.has_moved { 1 } else { 0 };
                let ml = if p.moved_long { 1 } else { 0 };
                if p.owner as usize == seat {
                    mine.push(format!("{}.{}.{}.{}.{}", cell, p.kind.glyph(), rev, mv, ml));
                } else if show_enemy {
                    let g = if p.revealed || over { p.kind.glyph() } else { '?' };
                    theirs.push(format!("{}.{}.{}.{}.{}", cell, g, rev, mv, ml));
                }
            }
        }
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            phase,
            self.w,
            self.h,
            if self.scout_long { 1 } else { 0 },
            self.current,
            winner,
            self.result_flavor(),
            army,
            myrem,
            mine.join(";"),
            theirs.join(";"),
        )
    }
}

#[wasm_bindgen]
pub struct VanguardWasm {
    g: Vanguard,
    preset: u32,
    counts: [u32; N_KINDS],
    scout_long: bool,
}

#[wasm_bindgen]
impl VanguardWasm {
    /// `size_preset` 0 = 6×7 mini (one lake), 1 = 8×8 standard (two lakes). The
    /// nine count args are the per-kind army composition (`marshal` … `standard`,
    /// each 0..=6; `standard` is forced to exactly 1). `scout_long` (0/1) toggles
    /// the Scout's any-distance straight move. The army is clamped to a deployable
    /// one (see `Vanguard::new`).
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(constructor)]
    pub fn new(
        size_preset: u32,
        marshal: u32,
        captain: u32,
        sergeant: u32,
        trooper: u32,
        scout: u32,
        sapper: u32,
        spy: u32,
        bomb: u32,
        standard: u32,
        scout_long: u32,
    ) -> Self {
        let counts = [marshal, captain, sergeant, trooper, scout, sapper, spy, bomb, standard];
        let g = Vanguard::new(size_preset, counts.map(|x| x as usize), scout_long != 0);
        Self { g, preset: size_preset, counts, scout_long: scout_long != 0 }
    }

    /// No-op by design: `weak_move` is an on-demand selector, not a persistent
    /// tree search (and 7-2 will replace it with determinized PIMC).
    pub fn playout_n(&mut self, _n: u32) {}

    /// Board for the CURRENT seat's eyes. Prefer `get_board_for`.
    pub fn get_board(&self) -> String {
        self.g.render(self.g.current as usize)
    }

    /// Per-seat secret view — never reveals a hidden enemy rank.
    pub fn get_board_for(&self, seat: u32) -> String {
        self.g.render(seat as usize)
    }

    /// Public combat/move summary for the pre-handoff reveal beat. It is identical
    /// for both seats (combat reveals are public), so the `seat` argument is
    /// accepted for API uniformity but the result is safe for either seat.
    pub fn last_move_summary_for(&self, _seat: u32) -> String {
        self.g.last_summary.clone()
    }

    pub fn current_player(&self) -> u32 {
        self.g.current as u32
    }

    pub fn is_terminal(&self) -> bool {
        self.g.is_terminal()
    }

    pub fn result(&self) -> String {
        self.g.result()
    }

    /// Why the game ended: "standard" / "immobilised" / "draw" / "".
    pub fn result_flavor(&self) -> String {
        self.g.result_flavor().to_string()
    }

    pub fn legal_moves(&self) -> String {
        self.g.legal_moves_list().join(",")
    }

    pub fn best_move(&self) -> Option<String> {
        self.g.best_move()
    }

    /// PLACEHOLDER random-legal opponent (Task 7-2 gives it real strength). The
    /// playout/top-k/temperature knobs are accepted for interface parity but
    /// ignored; only `seed` matters.
    pub fn weak_move(&self, _playouts: u32, _top_k: usize, _temp: f64, seed: u32) -> Option<String> {
        self.g.weak_move(seed)
    }

    /// Comma-joined full-army deployment for the current seat (the UI's "deploy
    /// for me"). Deterministic.
    pub fn random_placement(&self) -> String {
        self.g.random_placement().join(",")
    }

    pub fn apply_move(&mut self, mov: &str) -> bool {
        self.g.apply(mov)
    }

    pub fn reset(&mut self) {
        self.g = Vanguard::new(self.preset, self.counts.map(|x| x as usize), self.scout_long);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Default 12-piece army (Marshal1,Captain1,Sergeant2,Trooper2,Scout1,Sapper2,
    // Spy1,Bomb1,Standard1) in Kind order.
    const DEFAULT: [usize; N_KINDS] = [1, 1, 2, 2, 1, 2, 1, 1, 1];

    /// A game forced into the play phase with an EMPTY board, so tests can drop
    /// specific pieces and exercise one mechanic in isolation. Both seats are
    /// marked fully deployed (the engine does not cross-check board vs counts
    /// during play).
    fn play_stage(preset: u32, scout_long: bool) -> Vanguard {
        let mut g = Vanguard::new(preset, DEFAULT, scout_long);
        g.placed = [g.army, g.army];
        g.current = 0;
        g
    }

    fn put(g: &mut Vanguard, cell: usize, kind: Kind, owner: u8) -> u32 {
        let id = g.next_id;
        g.next_id += 1;
        g.board[cell] = Some(Piece {
            kind,
            owner,
            id,
            revealed: false,
            has_moved: false,
            moved_long: false,
        });
        id
    }

    fn at(g: &Vanguard, cell: usize) -> Option<Kind> {
        g.board[cell].map(|p| p.kind)
    }

    // ---------- geometry & feasibility ----------

    #[test]
    fn geometry_mini_and_standard() {
        let mini = Vanguard::new(0, DEFAULT, true);
        assert_eq!((mini.w, mini.h), (6, 7));
        assert!(mini.lakes[20] && mini.lakes[21]);
        assert_eq!(mini.lakes.iter().filter(|&&l| l).count(), 2);
        assert_eq!(mini.zone[0].iter().filter(|&&z| z).count(), 12);
        assert_eq!(mini.zone[1].iter().filter(|&&z| z).count(), 12);

        let std = Vanguard::new(1, DEFAULT, true);
        assert_eq!((std.w, std.h), (8, 8));
        for c in [26usize, 29, 34, 37] {
            assert!(std.lakes[c], "lake missing at {c}");
        }
        assert_eq!(std.lakes.iter().filter(|&&l| l).count(), 4);
        assert_eq!(std.zone[0].iter().filter(|&&z| z).count(), 24);
        // Zones never overlap lakes.
        for c in 0..std.w * std.h {
            assert!(!(std.lakes[c] && (std.zone[0][c] || std.zone[1][c])));
        }
    }

    #[test]
    fn ctor_forces_exactly_one_standard() {
        // Passing 0 or 5 Standards both normalise to exactly 1.
        let g0 = Vanguard::new(1, [1, 1, 2, 2, 1, 2, 1, 1, 0], true);
        assert_eq!(g0.army[Kind::Standard as usize], 1);
        let g5 = Vanguard::new(1, [1, 1, 2, 2, 1, 2, 1, 1, 5], true);
        assert_eq!(g5.army[Kind::Standard as usize], 1);
    }

    #[test]
    fn ctor_caps_each_count_at_six() {
        let g = Vanguard::new(1, [9, 9, 9, 0, 0, 0, 0, 0, 9], true);
        assert_eq!(g.army[Kind::Marshal as usize], 6);
        assert_eq!(g.army[Kind::Captain as usize], 6);
        assert_eq!(g.army[Kind::Sergeant as usize], 6);
        assert_eq!(g.army[Kind::Standard as usize], 1);
    }

    #[test]
    fn ctor_clamps_total_to_zone_capacity_and_keeps_standard() {
        // Mini zone = 12. An absurd army (6 of everything + standard) must clamp to
        // ≤ 12 with the Standard preserved.
        let g = Vanguard::new(0, [6, 6, 6, 6, 6, 6, 6, 6, 6], true);
        assert!(g.army_total() <= 12, "army {} exceeds mini capacity", g.army_total());
        assert_eq!(g.army[Kind::Standard as usize], 1, "Standard was dropped");
        // Bombs are dropped first (drop-order head).
        assert_eq!(g.army[Kind::Bomb as usize], 0);
        // A fully-empty count array still yields a lone Standard.
        let e = Vanguard::new(0, [0; N_KINDS], true);
        assert_eq!(e.army_total(), 1);
        assert_eq!(e.army[Kind::Standard as usize], 1);
    }

    // ---------- placement ----------

    #[test]
    fn placement_flow_and_phase_transition() {
        let mut g = Vanguard::new(1, DEFAULT, true);
        // Seat 0 deploys its whole army via the deterministic helper.
        assert_eq!(g.current, 0);
        for m in g.random_placement() {
            assert!(g.apply(&m), "seat0 placement {m} rejected");
        }
        assert_eq!(g.current, 1, "after seat0 finishes, seat1 deploys");
        assert!(!g.both_placed());
        for m in g.random_placement() {
            assert!(g.apply(&m), "seat1 placement {m} rejected");
        }
        assert!(g.both_placed());
        assert_eq!(g.current, 0, "play begins with seat 0");
        assert!(!g.over());
    }

    #[test]
    fn placement_rejections() {
        let mut g = Vanguard::new(1, DEFAULT, true);
        // Wrong zone: cell 40 is in seat 0's zone but seat 0 placing there is fine;
        // placing in seat 1's zone (cell 0) must be rejected for seat 0.
        assert!(!g.apply("p:0:0"), "seat0 may not deploy into seat1's zone");
        // Lake / out of zone: cell 26 is a lake and not in either zone.
        assert!(!g.apply("p:0:26"));
        // Valid deploy for seat 0 at cell 40 (row 5).
        assert!(g.apply("p:0:40"));
        // Occupied.
        assert!(!g.apply("p:1:40"));
        // Exhausted type: only 1 Marshal — a second is rejected.
        assert!(!g.apply("p:0:41"));
        // Bad parse forms.
        assert!(!g.apply("p:9:40")); // kind index out of range
        assert!(!g.apply("p:0:999")); // cell out of range
        assert!(!g.apply("p:0:40:x")); // trailing token
    }

    #[test]
    fn move_encoding_round_trips_and_rejects() {
        let g = Vanguard::new(1, DEFAULT, true);
        for (k, cell) in [(0usize, 0usize), (8, 63), (4, 40)] {
            let s = Vanguard::format_place(k, cell);
            assert_eq!(g.parse_place(&s), Some((k, cell)));
        }
        for (f, t) in [(0usize, 1usize), (63, 62), (40, 32)] {
            let s = Vanguard::format_move(f, t);
            assert_eq!(g.parse_play(&s), Some((f, t)));
        }
        assert_eq!(g.parse_place("p:9:0"), None); // kind out of range
        assert_eq!(g.parse_place("p:0:64"), None); // cell one past the last
        assert_eq!(g.parse_place("p:0:0:extra"), None);
        assert_eq!(g.parse_play("m:64:0"), None);
        assert_eq!(g.parse_play("m:0:0:extra"), None);
        assert_eq!(g.parse_play("x:0:1"), None);
    }

    // ---------- combat ----------

    #[test]
    fn combat_higher_rank_wins() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Marshal, 0); // rank 6
        put(&mut g, 32, Kind::Sergeant, 1); // rank 4, adjacent (row above)
        put(&mut g, 0, Kind::Trooper, 1); // spare so seat1 isn't immobilised
        assert!(g.apply("m:40:32"));
        assert_eq!(at(&g, 32), Some(Kind::Marshal), "Marshal should occupy the square");
        assert!(g.board[40].is_none());
    }

    #[test]
    fn combat_equal_rank_is_mutual() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Sergeant, 0);
        put(&mut g, 32, Kind::Sergeant, 1);
        put(&mut g, 0, Kind::Trooper, 1);
        assert!(g.apply("m:40:32"));
        assert!(g.board[40].is_none() && g.board[32].is_none(), "both removed on equal rank");
    }

    #[test]
    fn combat_spy_beats_marshal_when_attacking() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Spy, 0); // rank 0
        put(&mut g, 32, Kind::Marshal, 1); // rank 6
        put(&mut g, 0, Kind::Trooper, 1);
        assert!(g.apply("m:40:32"));
        assert_eq!(at(&g, 32), Some(Kind::Spy), "attacking Spy captures the Marshal");
        assert!(g.board[40].is_none());
    }

    #[test]
    fn combat_marshal_beats_spy_when_marshal_attacks() {
        // The Spy only wins when IT attacks; a Marshal attacking a Spy wins.
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Marshal, 0);
        put(&mut g, 32, Kind::Spy, 1);
        put(&mut g, 0, Kind::Trooper, 1);
        assert!(g.apply("m:40:32"));
        assert_eq!(at(&g, 32), Some(Kind::Marshal));
        assert!(g.board[40].is_none());
    }

    #[test]
    fn combat_sapper_defuses_bomb() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Sapper, 0);
        put(&mut g, 32, Kind::Bomb, 1);
        put(&mut g, 0, Kind::Trooper, 1);
        assert!(g.apply("m:40:32"));
        assert_eq!(at(&g, 32), Some(Kind::Sapper), "Sapper captures the Bomb and takes the square");
        assert!(g.board[40].is_none());
    }

    #[test]
    fn combat_bomb_destroys_nonsapper_and_persists() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Marshal, 0);
        let bomb_id = put(&mut g, 32, Kind::Bomb, 1);
        put(&mut g, 0, Kind::Trooper, 1);
        assert!(g.apply("m:40:32"));
        assert_eq!(at(&g, 32), Some(Kind::Bomb), "Bomb persists after the explosion");
        assert_eq!(g.board[32].unwrap().id, bomb_id, "same Bomb, still there");
        assert!(g.board[32].unwrap().revealed, "the Bomb is now revealed");
        assert!(g.board[40].is_none(), "the non-Sapper attacker is removed");
    }

    #[test]
    fn combat_reveals_both_participants() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Captain, 0);
        put(&mut g, 32, Kind::Trooper, 1);
        put(&mut g, 0, Kind::Trooper, 1);
        assert!(g.apply("m:40:32"));
        // Winner (Captain) is now on the square, revealed and moved.
        let p = g.board[32].unwrap();
        assert!(p.revealed && p.has_moved);
    }

    #[test]
    fn moved_long_flag_marks_a_scout_publicly() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Scout, 0); // row 5, col 0
        put(&mut g, 0, Kind::Trooper, 1);
        // Long straight move up the empty column: 40 -> 8 (rows 5..1), dist 4.
        assert!(g.apply("m:40:8"));
        let p = g.board[8].unwrap();
        assert!(p.moved_long && p.has_moved, "a 2+ straight move sets moved_long");
        assert!(!p.revealed, "a quiet long move does not set the combat-reveal flag");
    }

    // ---------- scout movement ----------

    #[test]
    fn scout_long_move_and_blocking() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Scout, 0); // (row5,col0)
        put(&mut g, 16, Kind::Trooper, 0); // own piece at (row2,col0) blocks the ray
        put(&mut g, 0, Kind::Trooper, 1);
        let moves = g.play_moves(0);
        // Can reach 32 (row4) and 24 (row3) but NOT past its own piece at 16.
        assert!(moves.contains(&(40, 32)));
        assert!(moves.contains(&(40, 24)));
        assert!(!moves.contains(&(40, 8)), "cannot leap own piece at 16");
        assert!(!moves.contains(&(40, 16)), "cannot land on own piece");
    }

    #[test]
    fn scout_cannot_leap_a_lake() {
        let mut g = play_stage(1, true);
        // Standard lakes at cols 2 & 5 over rows 3-4. A Scout on col 2 (cell 42,
        // row5) moving up hits the lake at 34 (row4,col2) and stops.
        put(&mut g, 42, Kind::Scout, 0);
        put(&mut g, 0, Kind::Trooper, 1);
        let moves = g.play_moves(0);
        assert!(!moves.iter().any(|&(_, t)| t == 34 || t == 26), "cannot enter/leap the lake column");
    }

    #[test]
    fn scout_long_disabled_moves_one_step() {
        let mut g = play_stage(1, false); // scout_long OFF
        put(&mut g, 40, Kind::Scout, 0);
        put(&mut g, 0, Kind::Trooper, 1);
        let moves: Vec<usize> = g.play_moves(0).into_iter().filter(|&(f, _)| f == 40).map(|(_, t)| t).collect();
        // Only the single orthogonal step to 32 (up) and 41 (right) — never 24.
        assert!(moves.contains(&32) && moves.contains(&41));
        assert!(!moves.contains(&24), "with scout_long off, no multi-square move");
    }

    // ---------- anti-shuffle & draw ----------

    #[test]
    fn two_squares_rule_bans_the_third_shuttle() {
        let mut g = play_stage(1, true);
        let _t0 = put(&mut g, 40, Kind::Trooper, 0); // seat0 shuttler, cols/rows clear
        put(&mut g, 0, Kind::Trooper, 1); // seat1 has its own piece to move
        // Seat 0: 40 -> 41
        assert!(g.apply("m:40:41"));
        // Seat 1 moves (breaks nothing for seat 0's tracker).
        assert!(g.apply("m:0:1"));
        // Seat 0: 41 -> 40 (2nd move on the {40,41} pair)
        assert!(g.apply("m:41:40"));
        // Seat 1 moves again.
        assert!(g.apply("m:1:0"));
        // Seat 0: 40 -> 41 would be the 3rd consecutive shuttle → illegal.
        assert!(!g.play_moves(0).contains(&(40, 41)), "third shuttle must be banned");
        assert!(!g.apply("m:40:41"), "engine must reject the banned shuttle move");
    }

    #[test]
    fn no_combat_ply_cap_ends_in_a_draw() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Trooper, 0);
        put(&mut g, 0, Kind::Trooper, 1);
        // Jump right to the edge of the cap, then one quiet move tips it over.
        g.no_combat_plies = NO_COMBAT_CAP - 1;
        assert!(g.apply("m:40:41")); // quiet ⇒ counter hits the cap
        assert!(g.over());
        assert_eq!(g.result(), "Draw");
        assert_eq!(g.result_flavor(), "draw");
    }

    // ---------- winning ----------

    #[test]
    fn win_by_capturing_the_standard() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Trooper, 0);
        put(&mut g, 32, Kind::Standard, 1);
        assert!(g.apply("m:40:32"));
        assert!(g.over());
        assert_eq!(g.result(), "1", "seat 0 captured the Standard");
        assert_eq!(g.result_flavor(), "standard");
    }

    #[test]
    fn win_by_immobilisation() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Trooper, 0); // seat0 movable
        put(&mut g, 32, Kind::Standard, 1); // seat1 has ONLY an immobile Standard
        // Seat 0 makes any quiet move; seat 1 then has no legal move.
        assert!(g.apply("m:40:41"));
        assert!(g.over());
        assert_eq!(g.result(), "1");
        assert_eq!(g.result_flavor(), "immobilised");
    }

    #[test]
    fn immobilisation_detected_when_play_begins() {
        // An all-Standard army: seat 0 is immobilised the instant play starts.
        let mut g = Vanguard::new(0, [0; N_KINDS], true); // army = one Standard each
        // Seat 0 deploys its lone Standard.
        let z0 = (0..g.w * g.h).find(|&c| g.zone[0][c]).unwrap();
        assert!(g.apply(&Vanguard::format_place(Kind::Standard as usize, z0)));
        assert_eq!(g.current, 1);
        let z1 = (0..g.w * g.h).find(|&c| g.zone[1][c]).unwrap();
        assert!(g.apply(&Vanguard::format_place(Kind::Standard as usize, z1)));
        assert!(g.over(), "both deployed; seat 0 (to move) cannot move");
        assert_eq!(g.result(), "2", "seat 1 wins by seat 0's immobilisation");
        assert_eq!(g.result_flavor(), "immobilised");
    }

    // ---------- secrecy ----------

    #[test]
    fn wrong_seat_never_sees_hidden_ranks() {
        // THE secrecy property. Deploy both armies, enter play, and assert that in
        // each seat's board string the `theirs` field carries ONLY `?` glyphs — no
        // enemy rank leaks — while the `mine` field shows real glyphs.
        let mut g = Vanguard::new(1, DEFAULT, true);
        for m in g.random_placement() {
            g.apply(&m);
        }
        for m in g.random_placement() {
            g.apply(&m);
        }
        assert!(g.both_placed());
        let theirs_glyphs = |seat: usize| -> Vec<char> {
            let s = g.render(seat);
            let theirs = s.split('|').nth(10).unwrap_or("");
            theirs
                .split(';')
                .filter(|e| !e.is_empty())
                .map(|e| e.split('.').nth(1).unwrap().chars().next().unwrap())
                .collect()
        };
        for seat in [0usize, 1] {
            let gs = theirs_glyphs(seat);
            assert!(!gs.is_empty(), "seat {seat} should see enemy backs");
            assert!(gs.iter().all(|&c| c == '?'), "seat {seat} leaked an enemy rank: {gs:?}");
        }
        // And `mine` DOES carry real glyphs (own info).
        let mine0 = g.render(0);
        let mine_field = mine0.split('|').nth(9).unwrap();
        assert!(mine_field.chars().any(|c| c != '?' && c.is_ascii_uppercase()));
    }

    #[test]
    fn combat_reveals_only_the_participants_to_the_opponent() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Captain, 0); // seat0 attacker
        put(&mut g, 32, Kind::Trooper, 1); // seat1 defender (loses, removed)
        put(&mut g, 24, Kind::Marshal, 1); // seat1 hidden bystander (stays hidden)
        assert!(g.apply("m:40:32"));
        // Seat 1's view of seat 0: the Captain that fought is now revealed ('C');
        // there are no OTHER seat-0 pieces, so just confirm the revealed glyph.
        let s1 = g.render(1);
        let theirs = s1.split('|').nth(10).unwrap();
        assert!(theirs.contains(".C."), "the fighting Captain must be revealed to seat 1: {theirs}");
        // Seat 0's view of seat 1: the surviving Marshal bystander stays `?`.
        let s0 = g.render(0);
        let theirs0 = s0.split('|').nth(10).unwrap();
        assert!(theirs0.contains("24.?."), "the untouched Marshal must stay hidden: {theirs0}");
        assert!(!theirs0.contains(".M."), "no hidden enemy rank may leak: {theirs0}");
    }

    #[test]
    fn last_move_summary_is_public_and_seat_independent() {
        let mut g = play_stage(1, true);
        put(&mut g, 40, Kind::Marshal, 0);
        put(&mut g, 32, Kind::Sergeant, 1);
        put(&mut g, 0, Kind::Trooper, 1);
        assert!(g.apply("m:40:32"));
        // Combat summary names both participants (public) and is identical for
        // both seats by construction.
        assert!(g.last_summary.starts_with("c:40:32:M:6:G:4:aw"), "summary: {}", g.last_summary);
    }

    // ---------- integration ----------

    #[test]
    fn legal_moves_are_all_applyable() {
        let mut g = Vanguard::new(1, DEFAULT, true);
        // Placement phase moves.
        for m in g.legal_moves_list() {
            let mut c = g.clone();
            assert!(c.apply(&m), "placement legal move rejected: {m}");
        }
        for m in g.random_placement() {
            g.apply(&m);
        }
        for m in g.random_placement() {
            g.apply(&m);
        }
        assert!(g.both_placed());
        for m in g.legal_moves_list() {
            let mut c = g.clone();
            assert!(c.apply(&m), "play legal move rejected: {m}");
        }
    }

    #[test]
    fn random_play_always_terminates_both_presets() {
        for preset in [0u32, 1] {
            for seed_base in 0..24u64 {
                let mut g = Vanguard::new(preset, DEFAULT, true);
                let mut rng = SmallRng::seed_from_u64(0x5EED ^ seed_base ^ ((preset as u64) << 40));
                let mut plies = 0;
                while !g.is_terminal() {
                    assert!(plies < 4000, "did not terminate (preset {preset}, seed {seed_base})");
                    let mv = match g.weak_move(rng.gen()) {
                        Some(m) => m,
                        None => panic!("weak_move None on a live position (preset {preset})"),
                    };
                    assert!(g.apply(&mv), "engine offered an illegal move: {mv}");
                    plies += 1;
                }
                let r = g.result();
                assert!(r == "Draw" || r == "1" || r == "2", "bad result string: {r:?}");
            }
        }
    }

    #[test]
    fn weak_move_never_reads_hidden_state_shape() {
        // A light non-cheating smoke check: weak_move is a pure function of the
        // public/own state + seed, so the SAME seed yields the SAME move regardless
        // of the (hidden) enemy piece kinds. Build two play-stage games identical
        // for seat 0 but with different hidden seat-1 ranks; seat 0's move must match.
        let mut a = play_stage(1, true);
        let mut b = play_stage(1, true);
        put(&mut a, 40, Kind::Trooper, 0);
        put(&mut b, 40, Kind::Trooper, 0);
        put(&mut a, 8, Kind::Marshal, 1); // different hidden ranks, same square
        put(&mut b, 8, Kind::Spy, 1);
        for seed in [1u32, 7, 99, 4242] {
            assert_eq!(a.weak_move(seed), b.weak_move(seed), "weak_move peeked at hidden ranks (seed {seed})");
        }
    }
}
