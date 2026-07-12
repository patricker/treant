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
//!   *Mine* is removed from the game." The quoted rule fixes only the ATTACKER's
//!   fate (removed); it is SILENT on the Mine's — the text neither confirms nor
//!   denies that the Mine survives the attack. Bomb PERSISTENCE is therefore a
//!   convention we ADOPT (the standard family reading), not something the source
//!   states: a non-Sapper attacker is removed and the Bomb stays on its square
//!   (now revealed); only a Sapper captures the Bomb without being destroyed.
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

/// Terminal magnitude for the PIMC negamax: capturing the enemy Standard (or
/// immobilising them) scores ±`WIN`, dwarfing any material term. Kept well below
/// `i32::MAX` so `2 * WIN` alpha-beta bounds and negation never overflow.
const WIN: i32 = 1_000_000;

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
    // Per-owner tally of that owner's OWN pieces that have LEFT the board, by kind.
    // Every removal happens through combat, which publicly reveals the piece's
    // kind, so this is PUBLIC information — the determinized sampler reads it to
    // enforce multiset conservation (a captured kind is drawn out of the pool).
    removed: [[usize; N_KINDS]; 2],
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
            removed: [[0; N_KINDS]; 2],
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
                    self.removed[def.owner as usize][Kind::Standard as usize] += 1;
                    attacker.revealed = true;
                    self.board[to] = Some(attacker);
                    self.winner = Some(seat as u8);
                    self.reason = EndReason::Standard;
                    self.last_summary = format!("c:{from}:{to}:{ag}:{ar}:{dg}:{dr}:sc");
                    return true;
                } else if def.kind == Kind::Bomb {
                    if attacker.kind == Kind::Sapper {
                        // Sapper defuses the Bomb (captures it) and takes the square.
                        self.removed[def.owner as usize][Kind::Bomb as usize] += 1;
                        attacker.revealed = true;
                        self.board[to] = Some(attacker);
                        self.last_summary = format!("c:{from}:{to}:{ag}:{ar}:{dg}:{dr}:bd");
                    } else {
                        // Bomb destroys any non-Sapper attacker and PERSISTS (revealed).
                        self.removed[attacker.owner as usize][attacker.kind as usize] += 1;
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
                            self.removed[def.owner as usize][def.kind as usize] += 1;
                            attacker.revealed = true;
                            self.board[to] = Some(attacker); // defender removed
                            self.last_summary = format!("c:{from}:{to}:{ag}:{ar}:{dg}:{dr}:aw");
                        }
                        -1 => {
                            self.removed[attacker.owner as usize][attacker.kind as usize] += 1;
                            def.revealed = true;
                            self.board[to] = Some(def); // attacker removed
                            self.last_summary = format!("c:{from}:{to}:{ag}:{ar}:{dg}:{dr}:dw");
                        }
                        _ => {
                            // both removed — square left empty
                            self.removed[def.owner as usize][def.kind as usize] += 1;
                            self.removed[attacker.owner as usize][attacker.kind as usize] += 1;
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

    // ---- the determinized-PIMC opponent (never peeks) -----------------------
    //
    // The AI never reads a hidden enemy rank. In the play phase it draws K
    // rank-assignments consistent with PUBLIC info only (revealed identities,
    // movement evidence, multiset conservation via the `removed` tally + own
    // pieces), turns each into a perfect-information world, runs a depth-limited
    // alpha-beta negamax with a material + Standard-safety eval to score every
    // ROOT move, AVERAGES the scores across the K worlds, and selects via
    // top-k/temperature — the hand-set difficulty ladder (nim / Bulls & Cows
    // precedent; hidden info makes win-rate calibration meaningless). This is
    // ensemble determinization / PIMC (Cowling et al. 2012), which sits on the
    // favourable side of the applicability line here because combat publicly
    // reveals both pieces and steadily collapses the hidden state (Long et al.
    // AAAI 2010). Placement is a stable deterministic plan (Standard on the back
    // rank, Bombs hugging it, the rest shuffled) — the play-phase search is the
    // strength story.

    /// Value weights for the material eval (centi-units). The Spy is prized above
    /// its rank (it alone kills a Marshal); the Bomb is a defensive asset; the
    /// Standard is 0 here because its capture is handled as a terminal win, not a
    /// material term.
    fn piece_value(k: Kind) -> i32 {
        match k {
            Kind::Marshal => 100,
            Kind::Captain => 70,
            Kind::Sergeant => 50,
            Kind::Trooper => 40,
            Kind::Scout => 25,
            Kind::Sapper => 35,
            Kind::Spy => 55,
            Kind::Bomb => 40,
            Kind::Standard => 0,
        }
    }

    fn orth_neighbors(&self, cell: usize) -> Vec<usize> {
        let (w, h) = (self.w, self.h);
        let (r, c) = (cell / w, cell % w);
        let mut out = Vec::new();
        for (dr, dc) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let (nr, nc) = (r as i32 + dr, c as i32 + dc);
            if nr >= 0 && nc >= 0 && nr < h as i32 && nc < w as i32 {
                out.push(nr as usize * w + nc as usize);
            }
        }
        out
    }

    // ---- sampler: a consistent hidden-rank assignment (reads ONLY public info) --

    /// Backtracking assignment of remaining hidden ranks to `hidden` positions
    /// (tightest-constrained first). Randomised kind order per level gives varied
    /// samples across seeds; guaranteed to find the consistent assignment if one
    /// exists (degenerates to the unique one when evidence pins everything).
    fn assign_kinds(
        &self,
        hidden: &[(usize, bool, bool)],
        pool: &mut [usize; N_KINDS],
        idx: usize,
        out: &mut [usize],
        rng: &mut SmallRng,
    ) -> bool {
        if idx == hidden.len() {
            return true;
        }
        let (_, has_moved, moved_long) = hidden[idx];
        let mut order: [usize; N_KINDS] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
        for i in (1..N_KINDS).rev() {
            order.swap(i, rng.gen_range(0..=i));
        }
        for &k in &order {
            if pool[k] == 0 {
                continue;
            }
            let kind = Kind::from_index(k).unwrap();
            // Public inference: a 2+-straight mover MUST be a Scout; any moved
            // piece cannot be an immovable Bomb/Standard.
            if moved_long && kind != Kind::Scout {
                continue;
            }
            if has_moved && !kind.movable() {
                continue;
            }
            pool[k] -= 1;
            out[idx] = k;
            if self.assign_kinds(hidden, pool, idx + 1, out, rng) {
                return true;
            }
            pool[k] += 1;
        }
        false
    }

    /// Draw ONE perfect-information world for `seat`: clone the board, then
    /// OVERWRITE every hidden enemy piece with a sampled rank consistent with the
    /// public record. Never reads a hidden enemy piece's true kind — the pool is
    /// `army − removed[opp] − revealed-on-board`, and the per-position constraints
    /// are the public movement flags. Returns `None` if the public state admits no
    /// consistent assignment (never expected in a real game).
    fn determinize(&self, seat: usize, rng: &mut SmallRng) -> Option<Vec<Option<Piece>>> {
        let opp = 1 - seat;
        let mut board = self.board.clone();
        let mut known = [0usize; N_KINDS];
        let mut hidden: Vec<(usize, bool, bool)> = Vec::new();
        for (cell, sq) in board.iter().enumerate() {
            if let Some(p) = sq {
                if p.owner as usize == opp {
                    if p.revealed {
                        known[p.kind as usize] += 1; // revealed ⇒ its kind is PUBLIC
                    } else {
                        hidden.push((cell, p.has_moved, p.moved_long));
                    }
                }
            }
        }
        let mut pool = [0usize; N_KINDS];
        let mut pool_total = 0usize;
        for k in 0..N_KINDS {
            let avail = self.army[k] as i64 - self.removed[opp][k] as i64 - known[k] as i64;
            if avail < 0 {
                return None;
            }
            pool[k] = avail as usize;
            pool_total += avail as usize;
        }
        if pool_total != hidden.len() {
            return None;
        }
        // Tightest constraint first: moved_long (Scout-pinned) < has_moved < free.
        hidden.sort_by_key(|&(_, hm, ml)| if ml { 0 } else if hm { 1 } else { 2 });
        let mut assign = vec![0usize; hidden.len()];
        if !self.assign_kinds(&hidden, &mut pool, 0, &mut assign, rng) {
            return None;
        }
        for (i, &(cell, _, _)) in hidden.iter().enumerate() {
            if let Some(p) = &mut board[cell] {
                p.kind = Kind::from_index(assign[i]).unwrap();
            }
        }
        Some(board)
    }

    // ---- per-sample perfect-information search -------------------------------

    /// Legal `(from, to)` moves on an arbitrary board (the search's move-gen).
    /// Mirrors `play_moves` minus the anti-shuffle rule (irrelevant inside a
    /// bounded search). Attack legality never depends on the enemy's rank, so the
    /// root move list is identical across samples.
    fn moves_on(&self, board: &[Option<Piece>], seat: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let (w, h) = (self.w, self.h);
        for from in 0..w * h {
            let p = match &board[from] {
                Some(p) if p.owner as usize == seat && p.kind.movable() => *p,
                _ => continue,
            };
            let (r, c) = (from / w, from % w);
            let long = p.kind == Kind::Scout && self.scout_long;
            let maxstep = if long { w.max(h) as i32 } else { 1 };
            for (dr, dc) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                for step in 1..=maxstep {
                    let (nr, nc) = (r as i32 + dr * step, c as i32 + dc * step);
                    if nr < 0 || nc < 0 || nr >= h as i32 || nc >= w as i32 {
                        break;
                    }
                    let ncell = nr as usize * w + nc as usize;
                    if self.lakes[ncell] {
                        break;
                    }
                    match &board[ncell] {
                        None => out.push((from, ncell)),
                        Some(occ) => {
                            if occ.owner as usize != seat {
                                out.push((from, ncell));
                            }
                            break;
                        }
                    }
                }
            }
        }
        out
    }

    /// Resolve one perfect-information move on `board` (both kinds known). Returns
    /// [`WIN`] iff the moved piece captured the enemy Standard (a terminal win for
    /// the mover), else 0. Mirrors `apply_play`'s combat, minus bookkeeping.
    fn pi_apply(&self, board: &mut [Option<Piece>], from: usize, to: usize) -> i32 {
        let att = board[from].take().unwrap();
        match board[to].take() {
            None => {
                board[to] = Some(att);
                0
            }
            Some(def) => {
                if def.kind == Kind::Standard {
                    board[to] = Some(att);
                    return WIN;
                }
                if def.kind == Kind::Bomb {
                    if att.kind == Kind::Sapper {
                        board[to] = Some(att); // defuse
                    } else {
                        board[to] = Some(def); // Bomb persists, attacker gone
                    }
                    return 0;
                }
                let (ar, dr) = (att.kind.rank().unwrap(), def.kind.rank().unwrap());
                let att_wins = (att.kind == Kind::Spy && def.kind == Kind::Marshal) || ar > dr;
                if att_wins {
                    board[to] = Some(att);
                } else if ar < dr {
                    board[to] = Some(def);
                }
                // equal ⇒ both removed (square already emptied by the takes)
                0
            }
        }
    }

    /// Is `target` capturable next move by a movable `by`-seat piece? The first
    /// piece met walking outward in each orthogonal ray is the only candidate:
    /// adjacent ⇒ any movable reaches it; further ⇒ only a long-move Scout. Used
    /// by the leaf eval for Standard-safety.
    fn attackable_by(&self, board: &[Option<Piece>], target: usize, by: usize) -> bool {
        let (w, h) = (self.w, self.h);
        let (r, c) = (target / w, target % w);
        for (dr, dc) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let mut step = 1i32;
            loop {
                let (nr, nc) = (r as i32 + dr * step, c as i32 + dc * step);
                if nr < 0 || nc < 0 || nr >= h as i32 || nc >= w as i32 {
                    break;
                }
                let ncell = nr as usize * w + nc as usize;
                if self.lakes[ncell] {
                    break;
                }
                match &board[ncell] {
                    None => {
                        step += 1;
                        continue;
                    }
                    Some(p) => {
                        if p.owner as usize == by && p.kind.movable() {
                            if step == 1 {
                                return true;
                            }
                            if p.kind == Kind::Scout && self.scout_long {
                                return true;
                            }
                        }
                        break; // first piece blocks the ray
                    }
                }
            }
        }
        false
    }

    /// Static leaf eval from the side-to-move's view: material difference plus a
    /// Standard-safety term (a Standard a piece can step onto next move is nearly
    /// lost — any attacker beats it).
    fn eval_stm(&self, board: &[Option<Piece>], stm: usize) -> i32 {
        let mut mat = [0i32; 2];
        let mut std_cell = [None, None];
        for (cell, sq) in board.iter().enumerate() {
            if let Some(p) = sq {
                let o = p.owner as usize;
                mat[o] += Self::piece_value(p.kind);
                if p.kind == Kind::Standard {
                    std_cell[o] = Some(cell);
                }
                // Advancement gradient: reward movable pieces for pushing toward the
                // enemy back rank. Small vs material (so it never invites a losing
                // trade), but enough to break the passivity that otherwise stalls
                // every game at the no-combat cap — it drives both armies into
                // contact, where skill (deeper search / more samples) tells.
                if p.kind.movable() {
                    let r = (cell / self.w) as i32;
                    let adv = if o == 0 { (self.h as i32 - 1) - r } else { r };
                    mat[o] += adv * 3;
                }
            }
        }
        let opp = 1 - stm;
        let mut score = mat[stm] - mat[opp];
        if let Some(sc) = std_cell[stm] {
            if self.attackable_by(board, sc, opp) {
                score -= 400;
            }
        }
        if let Some(sc) = std_cell[opp] {
            if self.attackable_by(board, sc, stm) {
                score += 400;
            }
        }
        score
    }

    /// Depth-limited alpha-beta negamax over a perfect-information world. Returns
    /// the value from `stm`'s perspective. Capturing the enemy Standard is a
    /// terminal [`WIN`]; being unable to move is a terminal loss.
    fn negamax(&self, board: &[Option<Piece>], stm: usize, depth: u32, mut alpha: i32, beta: i32) -> i32 {
        let moves = self.moves_on(board, stm);
        if moves.is_empty() {
            return -WIN; // immobilised ⇒ the side to move loses
        }
        if depth == 0 {
            return self.eval_stm(board, stm);
        }
        let mut best = i32::MIN;
        for (f, t) in moves {
            let mut nb = board.to_vec();
            let term = self.pi_apply(&mut nb, f, t);
            let v = if term >= WIN {
                WIN
            } else {
                -self.negamax(&nb, 1 - stm, depth - 1, -beta, -alpha)
            };
            if v > best {
                best = v;
            }
            if best > alpha {
                alpha = best;
            }
            if alpha >= beta {
                break; // fail-high cutoff
            }
        }
        best
    }

    /// The play-phase move: score every root move by AVERAGING its negamax value
    /// across `k_samples` determinized worlds, then pick via top-k/temperature.
    /// `seat` = the current player. Non-cheating by construction (samples read only
    /// public info + own pieces).
    fn play_ai(&self, k_samples: usize, depth: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        let seat = self.current as usize;
        let root_moves = self.play_moves(seat);
        if root_moves.is_empty() {
            return None;
        }
        let mut score = vec![0f64; root_moves.len()];
        let mut n_samples = 0usize;
        for s in 0..k_samples {
            let mut rng = SmallRng::seed_from_u64(
                (seed as u64) ^ 0x007A_11ED ^ ((seat as u64) << 40) ^ ((s as u64) << 8),
            );
            let world = match self.determinize(seat, &mut rng) {
                Some(w) => w,
                None => continue,
            };
            n_samples += 1;
            for (i, &(f, t)) in root_moves.iter().enumerate() {
                let mut nb = world.clone();
                let term = self.pi_apply(&mut nb, f, t);
                let v = if term >= WIN {
                    WIN
                } else {
                    -self.negamax(&nb, 1 - seat, depth.saturating_sub(1), -2 * WIN, 2 * WIN)
                };
                score[i] += v as f64;
            }
        }
        if n_samples == 0 {
            // Sampler found no consistent world (not expected in a real game) —
            // fall back to a legal move rather than stalling.
            let (f, t) = root_moves[0];
            return Some(Self::format_move(f, t));
        }
        for sc in &mut score {
            *sc /= n_samples as f64;
        }
        let pick = Self::select_topk_temp(&score, top_k, temp, seed);
        let (f, t) = root_moves[pick];
        Some(Self::format_move(f, t))
    }

    /// Index into `scores` (higher = better) chosen by top-k softmax at temperature
    /// `temp`. `temp ≤ 0` (or a singleton top-k) is argmax; ties break to the lower
    /// index for determinism. Scores are in ~centi-material units, so the softmax
    /// divides by `temp * 100` to make `temp ≈ 1` a sensible spread.
    fn select_topk_temp(scores: &[f64], top_k: usize, temp: f64, seed: u32) -> usize {
        let mut idx: Vec<usize> = (0..scores.len()).collect();
        idx.sort_by(|&a, &b| scores[b].partial_cmp(&scores[a]).unwrap().then(a.cmp(&b)));
        let k = top_k.clamp(1, idx.len());
        let top = &idx[..k];
        if temp <= 1e-4 || k == 1 {
            return top[0];
        }
        let maxs = scores[top[0]];
        let weights: Vec<f64> = top.iter().map(|&i| ((scores[i] - maxs) / (temp * 100.0)).exp()).collect();
        let sum: f64 = weights.iter().sum();
        if sum <= 0.0 {
            return top[0];
        }
        let mut rng = SmallRng::seed_from_u64((seed as u64) ^ 0x007A_115E);
        let mut r = rng.gen::<f64>() * sum;
        for (n, &i) in top.iter().enumerate() {
            r -= weights[n];
            if r <= 0.0 {
                return i;
            }
        }
        top[0]
    }

    /// Difficulty ladder: map the arcade `playouts` knob to (K samples, search
    /// depth). Hand-set (nim / Bulls & Cows precedent); the spike-validated Hard
    /// config is K=16 × depth 2 (see the task report's timing table). Easy is a
    /// noisy 2-sample depth-1 peek; the top-k/temp knobs (from the same ladder)
    /// supply the softmax spread.
    fn ladder(playouts: u32) -> (usize, u32) {
        match playouts {
            0..=19 => (2, 1),
            20..=199 => (6, 2),
            200..=799 => (10, 2),
            _ => (16, 2),
        }
    }

    /// A stable deterministic deployment plan for `seat` (fixed per game config,
    /// like Salvo's placement plan): Standard on the back rank, Bombs hugging it,
    /// every other kind shuffled across the remaining zone squares. Returned in
    /// placement order so successive `weak_move` calls deploy `plan[placed]`.
    fn placement_plan(&self, seat: usize) -> Vec<(usize, usize)> {
        let army_sig = self.army.iter().enumerate().fold(0u64, |a, (i, &c)| a ^ ((c as u64) << (i * 3)));
        let seedbase =
            0x07A1_1B0D ^ (self.w as u64) ^ ((self.h as u64) << 8) ^ ((seat as u64) << 16) ^ army_sig;
        let mut rng = SmallRng::seed_from_u64(seedbase);
        let zone: Vec<usize> = (0..self.w * self.h).filter(|&c| self.zone[seat][c]).collect();
        let back_row = if seat == 0 { self.h - 1 } else { 0 };
        let mut used = vec![false; self.w * self.h];
        let mut plan: Vec<(usize, usize)> = Vec::new();
        // 1. Standard on a back-rank cell (interior if possible).
        let back: Vec<usize> = zone.iter().copied().filter(|&c| c / self.w == back_row).collect();
        let std_cell = if back.is_empty() { zone[0] } else { back[rng.gen_range(0..back.len())] };
        plan.push((Kind::Standard as usize, std_cell));
        used[std_cell] = true;
        // 2. Bombs on squares next to the Standard (fall back to any free square).
        let adj: Vec<usize> = self
            .orth_neighbors(std_cell)
            .into_iter()
            .filter(|&c| self.zone[seat][c])
            .collect();
        for _ in 0..self.army[Kind::Bomb as usize] {
            let cell = adj
                .iter()
                .copied()
                .find(|&c| !used[c])
                .or_else(|| zone.iter().copied().find(|&c| !used[c]));
            match cell {
                Some(c) => {
                    plan.push((Kind::Bomb as usize, c));
                    used[c] = true;
                }
                None => break,
            }
        }
        // 3. Everything else, shuffled across the remaining zone squares.
        let mut rest: Vec<usize> = zone.iter().copied().filter(|&c| !used[c]).collect();
        for i in (1..rest.len()).rev() {
            rest.swap(i, rng.gen_range(0..=i));
        }
        let mut ci = 0usize;
        for k in 0..N_KINDS {
            if k == Kind::Standard as usize || k == Kind::Bomb as usize {
                continue;
            }
            for _ in 0..self.army[k] {
                if ci >= rest.len() {
                    break;
                }
                plan.push((k, rest[ci]));
                ci += 1;
            }
        }
        plan
    }

    fn placement_move(&self) -> Option<String> {
        let seat = self.current as usize;
        let plan = self.placement_plan(seat);
        plan.get(self.placed_total(seat)).map(|&(k, c)| Self::format_place(k, c))
    }

    /// A uniform-random legal move (seeded) — the pure-random baseline used by the
    /// termination fuzzer and the never-peeks smoke test. Reads only public/own
    /// state + the seed, so it never peeks at hidden enemy ranks.
    #[cfg(test)]
    fn random_move(&self, seed: u32) -> Option<String> {
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

    /// The arcade opponent: a stable deployment during placement, else the
    /// determinized-PIMC play move at the ladder rung `playouts` selects.
    fn weak_move(&self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        if self.over() {
            return None;
        }
        if !self.both_placed() {
            return self.placement_move();
        }
        let (k, depth) = Self::ladder(playouts);
        self.play_ai(k, depth, top_k, temp, seed)
    }

    /// The strongest move (the "watch-AI" / hint selector): the Hard rung, argmax.
    fn best_move(&self) -> Option<String> {
        if self.over() {
            return None;
        }
        if !self.both_placed() {
            return self.placement_move();
        }
        self.play_ai(16, 2, 1, 0.0, 0)
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

    /// No-op by design: the determinized-PIMC `weak_move` samples and searches on
    /// demand, so there is no persistent tree to grow between calls.
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

    /// The determinized-PIMC opponent. `playouts` selects the difficulty rung
    /// (mapped to K samples × search depth), `top_k`/`temp` shape the softmax over
    /// root-move scores, `seed` makes the choice reproducible. Never peeks at a
    /// hidden enemy rank.
    pub fn weak_move(&self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        self.g.weak_move(playouts, top_k, temp, seed)
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
                    let mv = match g.random_move(rng.gen()) {
                        Some(m) => m,
                        None => panic!("random_move None on a live position (preset {preset})"),
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
    fn random_move_never_reads_hidden_state_shape() {
        // A light non-cheating smoke check on the random baseline: `random_move` is
        // a pure function of the public/own state + seed, so the SAME seed yields
        // the SAME move regardless of the (hidden) enemy piece kinds. Build two
        // play-stage games identical for seat 0 but with different hidden seat-1
        // ranks; seat 0's move must match. (The PIMC opponent gets its own, deeper
        // `vanguard_ai_never_peeks` test below.)
        let mut a = play_stage(1, true);
        let mut b = play_stage(1, true);
        put(&mut a, 40, Kind::Trooper, 0);
        put(&mut b, 40, Kind::Trooper, 0);
        put(&mut a, 8, Kind::Marshal, 1); // different hidden ranks, same square
        put(&mut b, 8, Kind::Spy, 1);
        for seed in [1u32, 7, 99, 4242] {
            assert_eq!(a.random_move(seed), b.random_move(seed), "random_move peeked at hidden ranks (seed {seed})");
        }
    }

    // ---------- determinized-PIMC AI ----------

    /// Deploy both default 12-piece armies (via the AI's own placement plan) then
    /// play random moves until the board is down to `<= target` pieces with some
    /// combat behind us — a representative mid-game (some reveals, some hidden).
    fn midgame_position(seed_base: u64, target: usize) -> Vanguard {
        for attempt in 0..64u64 {
            let mut g = Vanguard::new(1, DEFAULT, true);
            for (k, c) in g.placement_plan(0) {
                assert!(g.apply(&Vanguard::format_place(k, c)));
            }
            for (k, c) in g.placement_plan(1) {
                assert!(g.apply(&Vanguard::format_place(k, c)));
            }
            let mut rng = SmallRng::seed_from_u64(seed_base ^ (attempt << 32));
            for _ in 0..400 {
                if g.is_terminal() {
                    break;
                }
                let count = g.board.iter().filter(|c| c.is_some()).count();
                if count <= target {
                    return g;
                }
                match g.random_move(rng.gen()) {
                    Some(m) => {
                        g.apply(&m);
                    }
                    None => break,
                }
            }
            if !g.is_terminal() && g.board.iter().filter(|c| c.is_some()).count() <= target {
                return g;
            }
        }
        panic!("could not build a mid-game position");
    }

    /// The go/no-go perf spike. IGNORED in normal runs; execute explicitly with:
    ///   cargo test -p treant-wasm --release vanguard_timing_spike -- --ignored --nocapture
    /// Prints native ms/move at a grid of (K samples × depth) on representative
    /// mid-game positions. Wasm runs ~3-5× slower; the Hard config must keep the
    /// wasm estimate < 1s/move.
    #[test]
    #[ignore]
    fn vanguard_timing_spike() {
        use std::time::Instant;
        let positions: Vec<Vanguard> = (0..6u64).map(|s| midgame_position(0xABCD ^ s, 16)).collect();
        for (i, g) in positions.iter().enumerate() {
            let pc = g.board.iter().filter(|c| c.is_some()).count();
            let rev = g.board.iter().flatten().filter(|p| p.revealed).count();
            let moves = g.play_moves(g.current as usize).len();
            eprintln!("  position {i}: {pc} pieces on board, {rev} revealed, {moves} root moves");
        }
        eprintln!("\n  native ms/move (avg over {} mid-game positions):", positions.len());
        eprintln!("  {:>6} | {:>8} {:>8} {:>8}", "K\\depth", "d=1", "d=2", "d=3");
        for &k in &[2usize, 6, 10, 16, 24] {
            let mut row = format!("  {k:>6} |");
            for &depth in &[1u32, 2, 3] {
                let t0 = Instant::now();
                let reps = 3u32;
                for _ in 0..reps {
                    for g in &positions {
                        let _ = g.play_ai(k, depth, 1, 0.0, 0x1234);
                    }
                }
                let per = t0.elapsed().as_secs_f64() * 1000.0 / (reps as f64 * positions.len() as f64);
                row.push_str(&format!(" {per:>7.2}ms"));
            }
            eprintln!("{row}");
        }
    }

    /// THE non-cheating property for the PIMC opponent. Two games with DIFFERENT
    /// hidden enemy armies but IDENTICAL public history (no combat, nothing
    /// revealed) and IDENTICAL own pieces must yield the SAME sample set and the
    /// SAME move at a fixed seed — the AI cannot have read the hidden ranks.
    #[test]
    fn vanguard_ai_never_peeks() {
        // Build two play-stage games. Seat 0 (the AI to move) is identical in both.
        // Seat 1's hidden pieces sit on the SAME squares with the SAME public flags
        // (all un-moved, un-revealed) but WILDLY DIFFERENT true ranks. Because the
        // sampler reads only public info (the symmetric `army` vector, `removed`,
        // revealed-on-board, positions + flags) and OVERWRITES every hidden enemy
        // piece before searching, the true ranks must not influence anything.
        let mut a = play_stage(1, true);
        let mut b = play_stage(1, true);
        // A 4-piece symmetric army (public), so the seat-1 hidden pool totals 4.
        let army4: [usize; N_KINDS] = [0, 0, 0, 1, 0, 0, 1, 1, 1]; // Trooper,Spy,Bomb,Standard
        for g in [&mut a, &mut b] {
            g.army = army4;
            g.placed = [army4, army4]; // keep both_placed() true after the swap
        }
        // Seat 0: a small identical force (real kinds; legitimately known to the AI).
        for (cell, kind) in [(48, Kind::Marshal), (49, Kind::Captain), (50, Kind::Sapper), (56, Kind::Standard)] {
            put(&mut a, cell, kind, 0);
            put(&mut b, cell, kind, 0);
        }
        // Seat 1: four hidden pieces on identical squares, but different true ranks
        // (game b's are not even drawn from `army4` — proving the AI never reads them).
        let a1 = [Kind::Spy, Kind::Trooper, Kind::Bomb, Kind::Standard];
        let b1 = [Kind::Marshal, Kind::Scout, Kind::Sergeant, Kind::Standard];
        for (i, &cell) in [8usize, 9, 10, 16].iter().enumerate() {
            put(&mut a, cell, a1[i], 1);
            put(&mut b, cell, b1[i], 1);
        }
        for seed in [1u32, 7, 42, 1000, 55555] {
            // Same sampled candidate set (as multisets of assigned kinds).
            let mut ra = SmallRng::seed_from_u64(seed as u64);
            let mut rb = SmallRng::seed_from_u64(seed as u64);
            let wa = a.determinize(0, &mut ra);
            let wb = b.determinize(0, &mut rb);
            // Both consistent worlds exist and the assigned enemy-kind multiset is
            // identical (the sampler drew from the same public pool).
            assert!(wa.is_some() && wb.is_some(), "sampler failed (seed {seed})");
            let kinds = |w: &Vec<Option<Piece>>| {
                let mut ks: Vec<Kind> =
                    w.iter().flatten().filter(|p| p.owner == 1).map(|p| p.kind).collect();
                ks.sort_by_key(|k| *k as usize);
                ks
            };
            assert_eq!(
                kinds(&wa.unwrap()),
                kinds(&wb.unwrap()),
                "sampled enemy multiset diverged ⇒ AI peeked (seed {seed})"
            );
            // And the chosen move is identical.
            assert_eq!(
                a.weak_move(2000, 1, 0.0, seed),
                b.weak_move(2000, 1, 0.0, seed),
                "PIMC move diverged ⇒ AI peeked (seed {seed})"
            );
        }
    }

    /// Every sampled world must (a) reproduce all revealed enemy identities on the
    /// same squares, (b) respect movement evidence (moved ⇒ movable; moved_long ⇒
    /// Scout), and (c) conserve the multiset (army − removed − revealed).
    #[test]
    fn sampler_respects_all_public_constraints() {
        let mut g = play_stage(1, true);
        // A 5-piece symmetric army matching the seat-1 force below (public).
        let army5: [usize; N_KINDS] = [0, 1, 1, 0, 1, 0, 1, 0, 1]; // Captain,Sergeant,Scout,Spy,Standard
        g.army = army5;
        g.placed = [army5, army5];
        // Seat 0 mover.
        put(&mut g, 48, Kind::Trooper, 0);
        put(&mut g, 56, Kind::Standard, 0);
        // Seat 1 enemy pieces with a mix of evidence.
        let rev = put(&mut g, 8, Kind::Captain, 1); // will mark revealed
        put(&mut g, 9, Kind::Scout, 1); // will mark moved_long
        put(&mut g, 10, Kind::Sergeant, 1); // will mark has_moved
        put(&mut g, 16, Kind::Spy, 1); // fully hidden
        put(&mut g, 17, Kind::Standard, 1); // fully hidden (the Standard)
        let _ = rev;
        // Apply public flags directly.
        g.board[8].as_mut().unwrap().revealed = true;
        g.board[9].as_mut().unwrap().moved_long = true;
        g.board[9].as_mut().unwrap().has_moved = true;
        g.board[10].as_mut().unwrap().has_moved = true;
        for seed in 0..200u64 {
            let mut rng = SmallRng::seed_from_u64(seed);
            let world = g.determinize(0, &mut rng).expect("consistent sample");
            // (a) revealed identity preserved.
            assert_eq!(world[8].unwrap().kind, Kind::Captain, "revealed identity changed (seed {seed})");
            // (b) movement evidence.
            assert_eq!(world[9].unwrap().kind, Kind::Scout, "moved_long piece must be a Scout (seed {seed})");
            assert!(world[10].unwrap().kind.movable(), "moved piece must be movable (seed {seed})");
            // (c) multiset conservation: enemy kinds on board == army (nothing
            // removed here), for owner 1.
            let mut got = [0usize; N_KINDS];
            for p in world.iter().flatten().filter(|p| p.owner == 1) {
                got[p.kind as usize] += 1;
            }
            assert_eq!(got, g.army, "enemy multiset not conserved (seed {seed})");
        }
    }

    /// The sampler honours the `removed` tally: a captured (and thus publicly
    /// revealed) enemy kind is drawn OUT of the hidden pool, so it can never be
    /// re-sampled onto the board.
    #[test]
    fn sampler_conserves_multiset_after_captures() {
        let mut g = play_stage(1, true);
        put(&mut g, 48, Kind::Marshal, 0);
        put(&mut g, 56, Kind::Standard, 0);
        // Two hidden enemy pieces remain on board; the rest of the army has been
        // captured. Mark those captures in `removed` (as combat would).
        put(&mut g, 8, Kind::Spy, 1);
        put(&mut g, 16, Kind::Standard, 1);
        // Army = DEFAULT (12). On board: 2 hidden. So removed must account for 10.
        // Removed set: Marshal1,Captain1,Sergeant2,Trooper2,Scout1,Sapper2,Bomb1 = 10.
        g.removed[1] = [1, 1, 2, 2, 1, 2, 0, 1, 0];
        for seed in 0..100u64 {
            let mut rng = SmallRng::seed_from_u64(seed);
            let world = g.determinize(0, &mut rng).expect("consistent sample");
            let mut got = [0usize; N_KINDS];
            for p in world.iter().flatten().filter(|p| p.owner == 1) {
                got[p.kind as usize] += 1;
            }
            // Only the two survivors may appear, and only from the un-removed pool:
            // Spy (1 left) and Standard (1 left).
            assert_eq!(got[Kind::Spy as usize], 1, "seed {seed}: Spy count");
            assert_eq!(got[Kind::Standard as usize], 1, "seed {seed}: Standard count");
            assert_eq!(got.iter().sum::<usize>(), 2, "seed {seed}: only survivors on board");
        }
    }

    /// Determinism: the same public position + knobs + seed always yields the same
    /// move.
    #[test]
    fn pimc_move_is_deterministic() {
        let g = midgame_position(0x1111, 18);
        for &(p, k, t) in &[(8u32, 6usize, 3.0f64), (100, 4, 1.0), (2000, 1, 0.0)] {
            let seat = g.current as usize;
            let m1 = g.play_ai(Vanguard::ladder(p).0, Vanguard::ladder(p).1, k, t, 777);
            let m2 = g.play_ai(Vanguard::ladder(p).0, Vanguard::ladder(p).1, k, t, 777);
            assert_eq!(m1, m2, "non-deterministic at (p{p},k{k},t{t}) seat {seat}");
            assert!(m1.is_some());
        }
    }

    /// Behavioural sanity: with the enemy Marshal PUBLICLY revealed adjacent to my
    /// weaker Captain, and a safe retreat available, Hard must NOT throw the Captain
    /// onto the Marshal — it should choose a non-suicidal move.
    #[test]
    fn hard_ai_does_not_attack_a_stronger_revealed_piece() {
        let mut g = play_stage(1, true);
        // Enemy army (public): a Marshal (revealed) + a hidden Standard.
        let army2: [usize; N_KINDS] = [1, 0, 0, 0, 0, 0, 0, 0, 1]; // Marshal,Standard
        g.army = army2;
        g.placed = [army2, army2];
        // My Captain (rank 5) at 33, with empty squares around to retreat to.
        put(&mut g, 33, Kind::Captain, 0);
        put(&mut g, 63, Kind::Standard, 0); // my Standard, tucked in a corner
        put(&mut g, 40, Kind::Trooper, 0); // a spare so I'm not down to one piece
        // Enemy Marshal (rank 6) revealed, directly above my Captain (25 = row3col1,
        // 33 = row4col1). Attacking up (33->25) would lose the Captain.
        put(&mut g, 25, Kind::Marshal, 1);
        g.board[25].as_mut().unwrap().revealed = true;
        // Enemy also has a (hidden) Standard so it isn't immobilised / trivially lost.
        put(&mut g, 0, Kind::Standard, 1);
        let mv = g.play_ai(16, 2, 1, 0.0, 4242).expect("a move");
        assert_ne!(mv, "m:33:25", "Hard threw the Captain onto the revealed Marshal");
    }

    /// Behavioural sanity: with a winning Standard capture available, Hard takes it.
    #[test]
    fn hard_ai_captures_a_reachable_standard() {
        let mut g = play_stage(1, true);
        // Enemy army (public): a Trooper (revealed spare) + a hidden Standard.
        let army2: [usize; N_KINDS] = [0, 0, 0, 1, 0, 0, 0, 0, 1]; // Trooper,Standard
        g.army = army2;
        g.placed = [army2, army2];
        put(&mut g, 40, Kind::Trooper, 0); // row5col0
        put(&mut g, 63, Kind::Standard, 0);
        put(&mut g, 32, Kind::Standard, 1); // enemy Standard directly above (row4col0), hidden
        put(&mut g, 0, Kind::Trooper, 1); // enemy spare, revealed ⇒ pool pins cell 32 = Standard
        g.board[0].as_mut().unwrap().revealed = true;
        let mv = g.play_ai(16, 2, 1, 0.0, 9).expect("a move");
        assert_eq!(mv, "m:40:32", "Hard failed to capture the reachable enemy Standard");
    }

    /// Ladder monotonicity smoke: Hard should beat Easy over a small match. Kept
    /// tiny (few games, mini board) so it stays CI-cheap.
    #[test]
    fn hard_beats_easy_over_a_small_match() {
        // Easy vs Hard, alternating who moves first to net out any seat edge.
        let easy = (2usize, 1u32, 6usize, 3.0f64); // (K, depth, top_k, temp)
        let hard = (16usize, 2u32, 1usize, 0.0f64);
        let mut hard_wins = 0i32;
        let mut games = 0i32;
        for seed in 0..6u64 {
            for hard_seat in 0..2u8 {
                let mut g = Vanguard::new(0, DEFAULT, true); // mini board = faster
                let mut rng = SmallRng::seed_from_u64(0xF00D ^ seed ^ ((hard_seat as u64) << 20));
                let mut plies = 0;
                while !g.is_terminal() && plies < 600 {
                    let seat = g.current;
                    let mv = if !g.both_placed() {
                        g.weak_move(0, 1, 0.0, rng.gen())
                    } else if seat == hard_seat {
                        g.play_ai(hard.0, hard.1, hard.2, hard.3, rng.gen())
                    } else {
                        g.play_ai(easy.0, easy.1, easy.2, easy.3, rng.gen())
                    };
                    match mv {
                        Some(m) => {
                            assert!(g.apply(&m), "illegal AI move {m}");
                        }
                        None => break,
                    }
                    plies += 1;
                }
                games += 1;
                let r = g.result();
                let hard_win_str = format!("{}", hard_seat + 1);
                if r == hard_win_str {
                    hard_wins += 1;
                }
            }
        }
        // 16 games; Hard should win a clear majority. Require > half.
        assert!(
            hard_wins * 2 > games,
            "Hard only won {hard_wins}/{games} vs Easy — ladder not monotone"
        );
    }
}
