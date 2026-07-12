//! Salvo — the public-domain WWI-era pencil-and-paper naval guessing game (the
//! one whose trademarked commercial NAME belongs to Hasbro; we NEVER surface that
//! name in user strings — the paper game itself is public domain, and "Salvo" is
//! the 1931 paper-era title we ship under). Each player secretly places a fleet on
//! an n×n grid; players alternate firing at grid squares; feedback per shot is
//! miss / hit / (on a ship's last cell) "sunk + which ship" (its length). You win
//! by sinking the opponent's whole fleet.
//!
//! ## Rules provenance (Wikipedia "Battleship (game)" + ultraboardgames rules)
//! Load-bearing sentences quoted in the engine below and in the task report:
//! - History: "Battleship is known worldwide as a pencil and paper game which
//!   dates from World War I." "The first commercial version of the game was Salvo,
//!   published in 1931 in the United States by the Starex company."
//! - Grid: "The grids are typically square, usually 10×10 …"
//! - Classic fleet (1990 Milton Bradley standard): "1 Carrier (5), 2 Battleship (4),
//!   3 Cruiser (3), 4 Submarine (3), 5 Destroyer (2)" — i.e. lengths {5,4,3,3,2}.
//! - Hit report: "the player who is hit … announces what ship was hit." Sunk:
//!   "When all of the squares of a ship have been hit, the ship's owner announces
//!   the sinking of the [ship]." We encode the sunk report as the ship's LENGTH
//!   (the information a paper player conveys — "you sank my 3").
//! - SALVO variant (1931): "players target a specified number of squares at one
//!   time, and all of the squares are attacked simultaneously." Feedback: "The
//!   opponent may either call the result of each shot in turn or simply announce
//!   the hits or misses." We encode the ATTESTED first option — per-shot results
//!   called in turn (miss/hit/sunk) — because it is what the game and a heat-map
//!   gunner both consume, and it is a documented form of the rule. The number of
//!   shots in a volley = the firing player's own SURVIVING ships, recounted each
//!   volley as ships are sunk (the authentic Salvo shot-count rule).
//!
//! ## Secrecy is the product — where the AI lives (and why it CANNOT cheat)
//! Like Bulls & Cows, this engine does not wrap a plain tree search around the
//! TRUE position (that would let every rollout read the opponent's hidden fleet —
//! cheating). Its intelligence is an honest **determinized gunner**: it samples K
//! opponent-fleet placements that are CONSISTENT with the feedback the firing seat
//! has received (rejection over a constraint-respecting constructive placer),
//! builds a hit-probability heat map from those samples, and fires by top-k /
//! temperature sampling over the map (Easy = few noisy samples, wide sampling;
//! Hard = many samples, argmax + parity targeting). The sampler reads ONLY the
//! seat's own feedback log (`shots[seat]`) plus the PUBLIC fleet composition, grid
//! size and touch rule — never `ships[opponent]`. This makes it a pure function of
//! feedback, proven by `gunner_never_cheats` below (two games with DIFFERENT true
//! fleets but IDENTICAL feedback yield identical candidate sets, heat maps and
//! shots). Hunt behaviour (concentrating fire around an unresolved hit) is not
//! hand-coded — it EMERGES because every consistent sample must cover a live hit
//! with a ship that extends through its orthogonal neighbours (`hunt_concentrates`
//! proves it). `playout_n` is a documented no-op: the gunner samples on demand in
//! `weak_move`, it has no persistent tree to grow. This is the master-plan-
//! sanctioned determinization the pass-screen primitive was built to serve.

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use wasm_bindgen::prelude::*;

/// Feasibility budget: total ship cells may occupy at most this fraction of the
/// grid (a fleet denser than this is clamped — see `clamp_fleet`).
const CELL_BUDGET_NUM: usize = 55;
const CELL_BUDGET_DEN: usize = 100;

/// Node budgets for the backtracking placer (bounds worst-case time; the 55% cell
/// cap keeps real fleets far under these).
const FEAS_BUDGET: u32 = 300_000; // ctor feasibility gate (must be thorough)
const PLAN_BUDGET: u32 = 100_000; // in-game full-fleet placement
const SAMPLE_BUDGET: u32 = 20_000; // per gunner-sample proposal

/// Cap on gunner samples per move (perf guard; sharp enough for argmax play).
const K_CAP: usize = 160;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Orient {
    H,
    V,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ShotResult {
    Miss,
    Hit,
    Sunk(u8), // carries the sunk ship's length (the paper "you sank my N")
}

#[derive(Clone)]
struct Ship {
    len: usize,
    cells: Vec<usize>,
    orient: Orient,
}

/// Cells a ship of `len` occupies from `cell` in orientation `o`, or None if it
/// runs off the grid. Row-major cell indices over an `size`×`size` board.
fn ship_cells(cell: usize, o: Orient, len: usize, size: usize) -> Option<Vec<usize>> {
    let (r, c) = (cell / size, cell % size);
    let mut cells = Vec::with_capacity(len);
    for i in 0..len {
        let (rr, cc) = match o {
            Orient::H => (r, c + i),
            Orient::V => (r + i, c),
        };
        if rr >= size || cc >= size {
            return None;
        }
        cells.push(rr * size + cc);
    }
    Some(cells)
}

/// True if `cells` can be placed on `occ` (all empty; and when `!touch`, none of
/// the ship's cells is 8-adjacent to a DIFFERENT ship). `occ[c]` is a ship index
/// or -1 for empty.
fn placement_ok(occ: &[i16], size: usize, cells: &[usize], touch: bool) -> bool {
    for &c in cells {
        if occ[c] != -1 {
            return false;
        }
    }
    if !touch {
        for &c in cells {
            let (r, col) = (c / size, c % size);
            for dr in -1i32..=1 {
                for dc in -1i32..=1 {
                    let nr = r as i32 + dr;
                    let ncol = col as i32 + dc;
                    if nr < 0 || ncol < 0 || nr >= size as i32 || ncol >= size as i32 {
                        continue;
                    }
                    let nc = (nr as usize) * size + ncol as usize;
                    if occ[nc] != -1 && !cells.contains(&nc) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// Every legal (cell, orient) position for a ship of `len` on the current board,
/// avoiding `forbidden` cells. Positions covering a still-uncovered `required`
/// cell are returned FIRST (each group shuffled) — the coverage bias that makes
/// the gunner's consistent-sampling hit rate high and makes hunt behaviour emerge.
fn positions(
    len: usize,
    size: usize,
    touch: bool,
    occ: &[i16],
    forbidden: &[bool],
    required: &[bool],
    rng: &mut SmallRng,
) -> Vec<(usize, Orient, Vec<usize>)> {
    let n2 = size * size;
    let mut covering = Vec::new();
    let mut plain = Vec::new();
    for o in [Orient::H, Orient::V] {
        if len == 1 && o == Orient::V {
            continue; // a 1-cell ship has one orientation
        }
        for cell in 0..n2 {
            if let Some(cells) = ship_cells(cell, o, len, size) {
                if cells.iter().any(|&c| forbidden[c]) {
                    continue;
                }
                if !placement_ok(occ, size, &cells, touch) {
                    continue;
                }
                let covers = cells.iter().any(|&c| required[c] && occ[c] == -1);
                if covers {
                    covering.push((cell, o, cells));
                } else {
                    plain.push((cell, o, cells));
                }
            }
        }
    }
    covering.shuffle(rng);
    plain.shuffle(rng);
    covering.extend(plain);
    covering
}

/// Backtracking placer: place every ship in `fleet` (lengths, any order — larger
/// first is best) onto an empty board, avoiding `forbidden`, biased to cover
/// `required`. Returns one placement or None if the node budget is exhausted /
/// no packing exists. Randomised position order ⇒ repeated calls sample varied
/// placements (the gunner relies on this).
fn place_backtrack(
    fleet: &[usize],
    size: usize,
    touch: bool,
    forbidden: &[bool],
    required: &[bool],
    rng: &mut SmallRng,
    budget: &mut u32,
) -> Option<Vec<Ship>> {
    let n2 = size * size;
    let mut occ = vec![-1i16; n2];
    let mut ships: Vec<Ship> = Vec::with_capacity(fleet.len());
    if bt(0, fleet, size, touch, forbidden, required, &mut occ, &mut ships, rng, budget) {
        Some(ships)
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn bt(
    i: usize,
    fleet: &[usize],
    size: usize,
    touch: bool,
    forbidden: &[bool],
    required: &[bool],
    occ: &mut Vec<i16>,
    ships: &mut Vec<Ship>,
    rng: &mut SmallRng,
    budget: &mut u32,
) -> bool {
    if i == fleet.len() {
        return true;
    }
    if *budget == 0 {
        return false;
    }
    let len = fleet[i];
    let cands = positions(len, size, touch, occ, forbidden, required, rng);
    for (_cell, o, cells) in cands {
        if *budget == 0 {
            return false;
        }
        *budget -= 1;
        for &c in &cells {
            occ[c] = i as i16;
        }
        ships.push(Ship { len, cells: cells.clone(), orient: o });
        if bt(i + 1, fleet, size, touch, forbidden, required, occ, ships, rng, budget) {
            return true;
        }
        ships.pop();
        for &c in &cells {
            occ[c] = -1;
        }
    }
    false
}

/// Clamp a raw fleet (lengths, descending) to a feasible one: at most 55% of the
/// grid in ship cells AND a concrete placement must exist. Policy: if infeasible,
/// drop the LARGEST ship (frees the most cells per drop; fewest ships lost) and
/// retry; guarantee at least one ship. This makes an unplaceable fleet
/// unrepresentable — the engine NEVER accepts one.
fn clamp_fleet(mut fleet: Vec<usize>, size: usize, touch: bool) -> Vec<usize> {
    let n2 = size * size;
    let budget_cells = n2 * CELL_BUDGET_NUM / CELL_BUDGET_DEN;
    fleet.sort_unstable_by(|a, b| b.cmp(a)); // descending
    if fleet.is_empty() {
        fleet.push(2.min(size)); // never zero ships: a lone "boat"
    }
    loop {
        let cells: usize = fleet.iter().sum();
        if cells <= budget_cells {
            let mut rng = SmallRng::seed_from_u64(0x5A1000 ^ size as u64);
            let no = vec![false; n2];
            let mut budget = FEAS_BUDGET;
            if place_backtrack(&fleet, size, touch, &no, &no, &mut rng, &mut budget).is_some() {
                return fleet;
            }
        }
        if fleet.len() <= 1 {
            // A single ship of length ≤ size always fits (size ≥ 6 ≥ max len 5).
            fleet[0] = fleet[0].min(size);
            return fleet;
        }
        fleet.remove(0); // drop the largest ship
    }
}

#[derive(Clone)]
struct Salvo {
    size: usize,
    fleet: Vec<usize>, // ship lengths, descending; PUBLIC composition (same for both seats)
    touch: bool,       // ships may touch (true) / must have a clear border (false)
    salvo: bool,       // volley mode: shots per turn = firing seat's surviving ships
    ships: [Vec<Option<Ship>>; 2], // placed ships per seat (secret); len == fleet.len()
    occ: [Vec<i16>; 2],            // cell -> ship index or -1 (secret)
    hits: [Vec<bool>; 2],          // cells hit on seat s's own board (from opponent shots)
    shots: [Vec<(usize, ShotResult)>; 2], // seat s's shots at the opponent + result (PUBLIC feedback log)
    fired: [Vec<bool>; 2],         // cells seat s has fired at
    current: u8,
    volley_left: u32, // shots remaining in the current seat's volley
    winner: Option<u8>,
    place_seed: u64, // fixed per game ⇒ deterministic AI/"place-for-me" placement (audit's fixed-seed mode)
}

impl Salvo {
    #[allow(clippy::too_many_arguments)]
    fn new(size: usize, counts: [usize; 5], touch: bool, salvo: bool) -> Self {
        let size = size.clamp(6, 15);
        let n2 = size * size;
        // counts[i] = number of ships of length i+1. Build descending (largest first).
        let mut fleet: Vec<usize> = Vec::new();
        for len in (1..=5).rev() {
            let c = counts[len - 1].min(6);
            for _ in 0..c {
                fleet.push(len);
            }
        }
        let fleet = clamp_fleet(fleet, size, touch);
        let nships = fleet.len();
        Self {
            size,
            fleet,
            touch,
            salvo,
            ships: [vec![None; nships], vec![None; nships]],
            occ: [vec![-1i16; n2], vec![-1i16; n2]],
            hits: [vec![false; n2], vec![false; n2]],
            shots: [Vec::new(), Vec::new()],
            fired: [vec![false; n2], vec![false; n2]],
            current: 0,
            volley_left: 0,
            winner: None,
            place_seed: 0x5A1000 ^ (size as u64) ^ ((nships as u64) << 12),
        }
    }

    fn placed(&self, seat: usize) -> usize {
        self.ships[seat].iter().filter(|s| s.is_some()).count()
    }

    fn all_placed(&self, seat: usize) -> bool {
        self.placed(seat) == self.fleet.len()
    }

    fn both_placed(&self) -> bool {
        self.all_placed(0) && self.all_placed(1)
    }

    fn is_terminal(&self) -> bool {
        self.winner.is_some()
    }

    /// Ships of `seat` not yet fully sunk (used for the salvo shot count).
    fn surviving(&self, seat: usize) -> u32 {
        self.ships[seat]
            .iter()
            .filter(|s| match s {
                Some(sh) => !sh.cells.iter().all(|&c| self.hits[seat][c]),
                None => true,
            })
            .count() as u32
    }

    /// Shots the firing `seat` gets this volley: its surviving ships in salvo mode,
    /// else exactly one.
    fn volley_size(&self, seat: usize) -> u32 {
        if self.salvo {
            self.surviving(seat).max(1)
        } else {
            1
        }
    }

    /// `""` in progress, else the 1-indexed winner seat digit. (No draw is
    /// reachable: shots never repeat a cell, so the board is exhausted in finite
    /// moves and firing every cell sinks every ship.)
    fn result(&self) -> String {
        match self.winner {
            Some(w) => format!("{}", w + 1),
            None => String::new(),
        }
    }

    // ---- move parse/format (single-sourced; round-trip tested) --------------

    fn format_place(idx: usize, cell: usize, o: Orient) -> String {
        format!("p:{}:{}:{}", idx, cell, if o == Orient::H { 'h' } else { 'v' })
    }

    fn format_shot(cell: usize) -> String {
        format!("s:{cell}")
    }

    fn parse_place(&self, s: &str) -> Option<(usize, usize, Orient)> {
        let mut it = s.split(':');
        if it.next()? != "p" {
            return None;
        }
        let idx: usize = it.next()?.parse().ok()?;
        let cell: usize = it.next()?.parse().ok()?;
        let o = match it.next()? {
            "h" => Orient::H,
            "v" => Orient::V,
            _ => return None,
        };
        if it.next().is_some() || idx >= self.fleet.len() || cell >= self.size * self.size {
            return None;
        }
        Some((idx, cell, o))
    }

    fn parse_shot(&self, s: &str) -> Option<usize> {
        let rest = s.strip_prefix("s:")?;
        let cell: usize = rest.parse().ok()?;
        if cell >= self.size * self.size {
            return None;
        }
        Some(cell)
    }

    // ---- applying moves -----------------------------------------------------

    fn apply(&mut self, mov: &str) -> bool {
        if self.is_terminal() {
            return false;
        }
        if mov.starts_with("p:") {
            self.apply_place(mov)
        } else if mov.starts_with("s:") {
            self.apply_shot(mov)
        } else {
            false
        }
    }

    fn apply_place(&mut self, mov: &str) -> bool {
        if self.both_placed() {
            return false; // placement phase only
        }
        let seat = self.current as usize;
        let (idx, cell, o) = match self.parse_place(mov) {
            Some(v) => v,
            None => return false,
        };
        if self.ships[seat][idx].is_some() {
            return false; // that ship slot is already placed
        }
        let len = self.fleet[idx];
        let cells = match ship_cells(cell, o, len, self.size) {
            Some(c) => c,
            None => return false,
        };
        if !placement_ok(&self.occ[seat], self.size, &cells, self.touch) {
            return false;
        }
        for &c in &cells {
            self.occ[seat][c] = idx as i16;
        }
        self.ships[seat][idx] = Some(Ship { len, cells, orient: o });
        // Advance the placing seat once its whole fleet is down.
        if self.all_placed(seat) {
            if seat == 0 {
                self.current = 1;
            } else {
                // Both fleets placed → firing begins, seat 0 first.
                self.current = 0;
                self.volley_left = self.volley_size(0);
            }
        }
        true
    }

    fn apply_shot(&mut self, mov: &str) -> bool {
        if !self.both_placed() {
            return false; // no firing before both fleets are down
        }
        let me = self.current as usize;
        let you = 1 - me;
        let cell = match self.parse_shot(mov) {
            Some(c) => c,
            None => return false,
        };
        if self.fired[me][cell] {
            return false; // can't fire the same square twice
        }
        self.fired[me][cell] = true;
        let result = if self.occ[you][cell] == -1 {
            ShotResult::Miss
        } else {
            self.hits[you][cell] = true;
            let sidx = self.occ[you][cell] as usize;
            let ship = self.ships[you][sidx].as_ref().unwrap();
            if ship.cells.iter().all(|&c| self.hits[you][c]) {
                ShotResult::Sunk(ship.len as u8)
            } else {
                ShotResult::Hit
            }
        };
        self.shots[me].push((cell, result));
        // Win check: every one of the opponent's ships fully hit.
        if self.surviving(you) == 0 {
            self.winner = Some(me as u8);
            self.volley_left = 0;
            return true;
        }
        // Volley bookkeeping: hold current through the volley; recount on handoff.
        self.volley_left = self.volley_left.saturating_sub(1);
        if self.volley_left == 0 {
            self.current = you as u8;
            self.volley_left = self.volley_size(you);
        }
        true
    }

    // ---- the gunner (determinized; reads ONLY feedback + public params) -----

    /// Replay `feedback` against a candidate placement and return true iff it
    /// reproduces EVERY recorded result. This is the exact real firing logic, so a
    /// consistent candidate is one that could have produced the observed feedback.
    fn consistent(feedback: &[(usize, ShotResult)], occ: &[i16], ships: &[Ship], n2: usize) -> bool {
        let mut hit = vec![false; n2];
        for &(cell, res) in feedback {
            let got = if occ[cell] == -1 {
                ShotResult::Miss
            } else {
                hit[cell] = true;
                let ship = &ships[occ[cell] as usize];
                if ship.cells.iter().all(|&c| hit[c]) {
                    ShotResult::Sunk(ship.len as u8)
                } else {
                    ShotResult::Hit
                }
            };
            if got != res {
                return false;
            }
        }
        true
    }

    /// Sample up to `k` opponent-fleet placements consistent with `seat`'s
    /// feedback. Reads ONLY `self.shots[seat]` + PUBLIC params (fleet/size/touch);
    /// never the opponent's true ships — this is what makes the gunner non-cheating.
    fn sample_candidates(&self, seat: usize, k: usize, seed: u32) -> Vec<Vec<Ship>> {
        let n2 = self.size * self.size;
        let feedback = &self.shots[seat];
        // Constraints derived purely from feedback:
        let mut forbidden = vec![false; n2]; // MISS cells: no ship may cover them
        let mut required = vec![false; n2]; // HIT/SUNK cells: must be covered
        for &(cell, res) in feedback {
            match res {
                ShotResult::Miss => forbidden[cell] = true,
                ShotResult::Hit | ShotResult::Sunk(_) => required[cell] = true,
            }
        }
        let mut rng = SmallRng::seed_from_u64((seed as u64) ^ 0x5A1060 ^ ((seat as u64) << 20));
        let mut out = Vec::with_capacity(k);
        let max_attempts = (k * 12).max(200);
        for _ in 0..max_attempts {
            if out.len() >= k {
                break;
            }
            let mut budget = SAMPLE_BUDGET;
            let cand = match place_backtrack(
                &self.fleet,
                self.size,
                self.touch,
                &forbidden,
                &required,
                &mut rng,
                &mut budget,
            ) {
                Some(c) => c,
                None => continue,
            };
            // Reindex occ for the consistency replay (ships in fleet order).
            let mut occ = vec![-1i16; n2];
            for (i, sh) in cand.iter().enumerate() {
                for &c in &sh.cells {
                    occ[c] = i as i16;
                }
            }
            if Self::consistent(feedback, &occ, &cand, n2) {
                out.push(cand);
            }
        }
        out
    }

    /// Occupancy frequency (0..1) per cell across `candidates` — the raw heat map.
    fn heat_map(candidates: &[Vec<Ship>], n2: usize) -> Vec<f64> {
        let mut heat = vec![0.0f64; n2];
        if candidates.is_empty() {
            return heat;
        }
        for cand in candidates {
            for sh in cand {
                for &c in &sh.cells {
                    heat[c] += 1.0;
                }
            }
        }
        let n = candidates.len() as f64;
        for h in &mut heat {
            *h /= n;
        }
        heat
    }

    /// Lengths of the opponent ships the firing `seat` has SUNK (from Sunk reports).
    fn sunk_lengths(&self, seat: usize) -> Vec<usize> {
        self.shots[seat]
            .iter()
            .filter_map(|&(_, r)| match r {
                ShotResult::Sunk(l) => Some(l as usize),
                _ => None,
            })
            .collect()
    }

    /// The opponent fleet lengths still afloat (fleet minus sunk), and whether the
    /// seat is in SEARCH mode (no unresolved "live" hits — every hit cell is
    /// accounted for by a sunk ship).
    fn remaining_and_mode(&self, seat: usize) -> (Vec<usize>, bool) {
        let mut remaining = self.fleet.clone();
        for l in self.sunk_lengths(seat) {
            if let Some(pos) = remaining.iter().position(|&x| x == l) {
                remaining.remove(pos);
            }
        }
        let hit_cells = self.shots[seat].iter().filter(|&&(_, r)| r != ShotResult::Miss).count();
        let sunk_cells: usize = self.sunk_lengths(seat).iter().sum();
        let search = hit_cells == sunk_cells; // no live (unresolved) hits
        (remaining, search)
    }

    /// Pick the firing seat's next target cell. `playouts` scales the sample count
    /// K (heat sharpness), `top_k`/`temp` sample among the hottest cells. At low
    /// temperature in SEARCH mode a parity boost concentrates fire on the lattice
    /// that must contain the smallest surviving ship (the classic hunting trick).
    fn pick_shot(&self, seat: usize, playouts: u64, top_k: usize, temp: f64, seed: u32) -> usize {
        let n2 = self.size * self.size;
        let k = (playouts as usize).clamp(1, K_CAP);
        let candidates = self.sample_candidates(seat, k, seed);
        let heat = Self::heat_map(&candidates, n2);
        let (remaining, search) = self.remaining_and_mode(seat);
        let l_min = remaining.iter().copied().min().unwrap_or(1);
        let parity_on = search && temp < 0.5 && l_min >= 2;

        // Score un-fired cells; a small parity boost in search mode.
        let mut scored: Vec<(f64, usize)> = Vec::new();
        for (cell, &h) in heat.iter().enumerate() {
            if self.fired[seat][cell] {
                continue;
            }
            let mut s = h;
            if parity_on {
                let (r, c) = (cell / self.size, cell % self.size);
                if (r + c) % l_min != 0 {
                    s *= 0.2;
                }
            }
            scored.push((s, cell));
        }
        if scored.is_empty() {
            return 0; // unreachable in a live fire phase (there is always an un-fired cell)
        }
        // Highest score first; ties broken by lowest cell index (determinism).
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
        let k_top = top_k.clamp(1, scored.len());
        let top = &scored[..k_top];
        if temp <= 0.0001 || top.len() == 1 {
            return top[0].1;
        }
        // Softmax over the top-k scores at temperature `temp`.
        let best = top[0].0.max(1e-9);
        let weights: Vec<f64> = top.iter().map(|&(s, _)| (s.max(1e-9) / best).powf(1.0 / temp)).collect();
        let sum: f64 = weights.iter().sum();
        if sum <= 0.0 {
            return top[0].1;
        }
        let mut rng = SmallRng::seed_from_u64((seed as u64) ^ 0x5A1071);
        let mut r = rng.gen::<f64>() * sum;
        for (i, w) in weights.iter().enumerate() {
            r -= w;
            if r <= 0.0 {
                return top[i].1;
            }
        }
        top[0].1
    }

    /// A full, deterministic placement plan for `seat` (fixed per game — this is
    /// the audit's fixed-seed placement mode). Returns ships in fleet order so the
    /// AI can place them idx 0,1,2… across successive `weak_move` calls.
    fn placement_plan(&self, seat: usize) -> Option<Vec<Ship>> {
        let n2 = self.size * self.size;
        let no = vec![false; n2];
        let mut rng = SmallRng::seed_from_u64(self.place_seed ^ ((seat as u64) << 32));
        let mut budget = PLAN_BUDGET;
        place_backtrack(&self.fleet, self.size, self.touch, &no, &no, &mut rng, &mut budget)
    }

    /// The current seat's move: next placement in the plan (setup) or the gunner's
    /// shot (firing).
    fn pick_move(&self, playouts: u64, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        if self.is_terminal() {
            return None;
        }
        let seat = self.current as usize;
        if !self.both_placed() {
            let plan = self.placement_plan(seat)?;
            let idx = self.placed(seat); // ships placed in order 0,1,2,…
            let sh = plan.get(idx)?;
            let cell = *sh.cells.iter().min().unwrap();
            return Some(Self::format_place(idx, cell, sh.orient));
        }
        let cell = self.pick_shot(seat, playouts, top_k, temp, seed);
        Some(Self::format_shot(cell))
    }

    /// A small, applyable sample of legal moves (for the random-move fallback and
    /// the audit fuzzer's needs — never the exhaustive space).
    fn legal_sample(&self) -> Vec<String> {
        if self.is_terminal() {
            return Vec::new();
        }
        let seat = self.current as usize;
        let n2 = self.size * self.size;
        let mut out = Vec::new();
        if !self.both_placed() {
            let idx = self.placed(seat);
            let len = self.fleet[idx];
            // A handful of valid positions for the next unplaced ship.
            let mut rng = SmallRng::seed_from_u64(0x5A101E ^ seat as u64 ^ ((idx as u64) << 8));
            let no = vec![false; n2];
            let pos = positions(len, self.size, self.touch, &self.occ[seat], &no, &no, &mut rng);
            for (cell, o, _) in pos.into_iter().take(12) {
                out.push(Self::format_place(idx, cell, o));
            }
        } else {
            for cell in 0..n2 {
                if !self.fired[seat][cell] {
                    out.push(Self::format_shot(cell));
                    if out.len() >= 12 {
                        break;
                    }
                }
            }
        }
        out
    }

    // ---- board rendering (per-seat secret view) -----------------------------

    /// Board string for `seat`'s eyes only. Fields joined by `|`:
    /// `phase|size|touch|salvo|current|volleyLeft|volleySize|winner|fleet|myships|myincoming|myshots|oppsunk|oppfull`.
    /// The ONLY place opponent ship cells ever appear is `oppsunk` (cells of ships
    /// the seat has SUNK — an attested, desirable reveal) and `oppfull` (populated
    /// ONLY at game over). Un-sunk opponent ship cells are structurally absent —
    /// the load-bearing secrecy guarantee (see `wrong_seat_never_sees_fleet`).
    fn render(&self, seat: usize) -> String {
        let seat = seat.min(1);
        let you = 1 - seat;
        let over = self.is_terminal();
        let phase = if over {
            "over"
        } else if self.both_placed() {
            "fire"
        } else {
            "place"
        };
        let winner = match self.winner {
            Some(w) => format!("{w}"),
            None => String::new(),
        };
        let fleet = self.fleet.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(",");
        // Own ships: cell.orient.len.sunkflag
        let ship_str = |ships: &[Option<Ship>], hits: &[bool]| {
            ships
                .iter()
                .flatten()
                .map(|sh| {
                    let sunk = sh.cells.iter().all(|&c| hits[c]);
                    let cell = sh.cells.iter().min().unwrap();
                    format!(
                        "{}.{}.{}.{}",
                        cell,
                        if sh.orient == Orient::H { 'h' } else { 'v' },
                        sh.len,
                        if sunk { 1 } else { 0 }
                    )
                })
                .collect::<Vec<_>>()
                .join(";")
        };
        let myships = ship_str(&self.ships[seat], &self.hits[seat]);
        // Incoming shots on my board (opponent's shots at me): cell:res
        let res_char = |r: ShotResult| match r {
            ShotResult::Miss => "m".to_string(),
            ShotResult::Hit => "h".to_string(),
            ShotResult::Sunk(l) => format!("k{l}"),
        };
        let shot_str = |shots: &[(usize, ShotResult)]| {
            shots
                .iter()
                .map(|&(c, r)| format!("{}:{}", c, res_char(r)))
                .collect::<Vec<_>>()
                .join(";")
        };
        let myincoming = shot_str(&self.shots[you]); // opponent's shots land on MY board
        let myshots = shot_str(&self.shots[seat]); // MY shots + feedback (my target grid)
        // Opponent ships I have SUNK are revealed (cells+len); nothing else pre-sink.
        let oppsunk = self.ships[you]
            .iter()
            .flatten()
            .filter(|sh| sh.cells.iter().all(|&c| self.hits[you][c]))
            .map(|sh| {
                sh.cells.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(".") + &format!(".{}", sh.len)
            })
            .collect::<Vec<_>>()
            .join(";");
        // At game over ONLY: reveal the opponent's full fleet.
        let oppfull = if over {
            ship_str(&self.ships[you], &self.hits[you])
        } else {
            String::new()
        };
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            phase,
            self.size,
            if self.touch { 1 } else { 0 },
            if self.salvo { 1 } else { 0 },
            self.current,
            self.volley_left,
            self.volley_size(self.current as usize),
            winner,
            fleet,
            myships,
            myincoming,
            myshots,
            oppsunk,
            oppfull,
        )
    }

    /// Placement moves for a random full fleet for the CURRENT seat (the "place
    /// for me" helper). Deterministic per game (fixed placement seed).
    fn random_placement(&self) -> Vec<String> {
        let seat = self.current as usize;
        match self.placement_plan(seat) {
            Some(plan) => plan
                .iter()
                .enumerate()
                .map(|(i, sh)| Self::format_place(i, *sh.cells.iter().min().unwrap(), sh.orient))
                .collect(),
            None => Vec::new(),
        }
    }
}

#[wasm_bindgen]
pub struct SalvoWasm {
    g: Salvo,
    size: u32,
    counts: [usize; 5],
    touch: bool,
    salvo: bool,
}

#[wasm_bindgen]
impl SalvoWasm {
    /// `size` 6–15, `s1..s5` = number of ships of length 1..5 (each 0–6), `touch`
    /// = ships may touch (0/1), `salvo` = volley mode (0 = one shot/turn; 1 =
    /// one shot per surviving own ship). The fleet is clamped to a placeable one.
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(constructor)]
    pub fn new(size: u32, s1: u32, s2: u32, s3: u32, s4: u32, s5: u32, touch: u32, salvo: u32) -> Self {
        let counts = [s1 as usize, s2 as usize, s3 as usize, s4 as usize, s5 as usize];
        let g = Salvo::new(size as usize, counts, touch != 0, salvo != 0);
        Self { size: g.size as u32, counts, touch: g.touch, salvo: g.salvo, g }
    }

    /// No-op by design: the gunner is a determinized sampler evaluated on demand in
    /// `weak_move`, not a persistent tree search (see the module header).
    pub fn playout_n(&mut self, _n: u32) {}

    /// Board for the CURRENT player's eyes. Prefer `get_board_for`; this fallback
    /// still only ever reveals the current seat's own fleet.
    pub fn get_board(&self) -> String {
        self.g.render(self.g.current as usize)
    }

    /// Board for a specific seat — the per-seat secret view. Never reveals an
    /// un-sunk opponent ship cell.
    pub fn get_board_for(&self, seat: u32) -> String {
        self.g.render(seat as usize)
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

    pub fn legal_moves(&self) -> String {
        self.g.legal_sample().join(",")
    }

    pub fn best_move(&self) -> Option<String> {
        self.g.pick_move(u64::MAX, 1, 0.0, 0)
    }

    pub fn weak_move(&self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        self.g.pick_move(playouts as u64, top_k, temp, seed)
    }

    /// Comma-joined placement moves for a random full fleet for the current seat
    /// (the UI's "🎲 Place for me"). Deterministic per game.
    pub fn random_placement(&self) -> String {
        self.g.random_placement().join(",")
    }

    pub fn apply_move(&mut self, mov: &str) -> bool {
        self.g.apply(mov)
    }

    pub fn reset(&mut self) {
        self.g = Salvo::new(self.size as usize, self.counts, self.touch, self.salvo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Classic fleet counts (s1..s5) = 0,1,2,1,1 → lengths {5,4,3,3,2} (1990 MB set).
    const CLASSIC: [usize; 5] = [0, 1, 2, 1, 1];

    /// Place a seat's whole fleet from an explicit list of (idx, cell, orient).
    fn place_all(g: &mut Salvo, moves: &[(usize, usize, char)]) {
        for &(idx, cell, o) in moves {
            let ok = g.apply(&format!("p:{}:{}:{}", idx, cell, o));
            assert!(ok, "placement p:{idx}:{cell}:{o} rejected");
        }
    }

    #[test]
    fn ship_cells_bounds() {
        // 8×8: horizontal length-3 from cell 0 → 0,1,2; off-grid rejected.
        assert_eq!(ship_cells(0, Orient::H, 3, 8), Some(vec![0, 1, 2]));
        assert_eq!(ship_cells(6, Orient::H, 3, 8), None); // col 6,7,8 → off grid
        assert_eq!(ship_cells(0, Orient::V, 3, 8), Some(vec![0, 8, 16]));
        assert_eq!(ship_cells(48, Orient::V, 3, 8), None); // row 6,7,8 → off grid
    }

    #[test]
    fn placement_rejects_overlap_and_out_of_bounds() {
        let mut g = Salvo::new(8, [0, 0, 1, 0, 1], true, false); // lengths {5,3}
        assert!(g.apply("p:0:0:h")); // len5 at 0..4
        assert!(!g.apply("p:1:2:h")); // len3 at 2,3,4 overlaps ship 0
        assert!(!g.apply("p:1:6:h")); // len3 off the right edge (col 6,7,8)
        assert!(g.apply("p:1:16:h")); // len3 at 16,17,18 — clear
        assert!(g.all_placed(0));
    }

    #[test]
    fn touch_rule_forbids_adjacency_when_off() {
        // touch = false: a ship abutting another (even diagonally) is illegal.
        let mut h = Salvo::new(8, [0, 1, 1, 0, 0], false, false); // lengths {3,2}
        assert!(h.apply("p:0:0:h")); // len3 at 0,1,2 (row 0)
        assert!(!h.apply("p:1:8:h")); // len2 at 8,9 — cell 8 is below cell 0 (adjacent) → rejected
        assert!(!h.apply("p:1:11:h")); // len2 at 11,12 — 11 diag-adjacent to cell 2 → rejected
        assert!(h.apply("p:1:20:h")); // len2 at 20,21 (row2) — clear of the len3 → ok
        // With touch = true the abutting placement is legal.
        let mut t = Salvo::new(8, [0, 1, 1, 0, 0], true, false);
        assert!(t.apply("p:0:0:h"));
        assert!(t.apply("p:1:8:h")); // adjacent allowed when ships may touch
    }

    #[test]
    fn feedback_miss_hit_and_sunk_report_length() {
        let mut g = Salvo::new(8, [0, 0, 1, 0, 0], true, false); // one len3 ship each
        place_all(&mut g, &[(0, 0, 'h')]); // seat 0: 0,1,2
        place_all(&mut g, &[(0, 16, 'h')]); // seat 1: 16,17,18
        assert!(g.both_placed());
        assert_eq!(g.current, 0);
        // Seat 0 fires at seat 1's ship (16,17,18).
        assert!(g.apply("s:40")); // empty → miss
        assert_eq!(g.shots[0].last().unwrap().1, ShotResult::Miss);
        // (non-salvo: turn passed to seat 1; give seat 1 a wasted shot to return)
        assert_eq!(g.current, 1);
        assert!(g.apply("s:60")); // seat 1 misses
        assert_eq!(g.current, 0);
        assert!(g.apply("s:16"));
        assert_eq!(g.shots[0].last().unwrap().1, ShotResult::Hit);
        assert!(g.apply("s:63")); // seat 1 misses again
        assert!(g.apply("s:17"));
        assert_eq!(g.shots[0].last().unwrap().1, ShotResult::Hit);
        assert!(g.apply("s:62"));
        assert!(g.apply("s:18"));
        assert_eq!(g.shots[0].last().unwrap().1, ShotResult::Sunk(3)); // sunk report = length 3
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1"); // seat 0 sank the whole (1-ship) fleet
    }

    #[test]
    fn salvo_volley_shot_count_is_surviving_ships_and_recounts() {
        // Two ships each; salvo mode. Volley size = firing seat's surviving ships.
        let mut g = Salvo::new(8, [0, 1, 1, 0, 0], true, true); // lengths {3,2}
        // Seat 0 fleet (anywhere; its own survival drives ITS volleys, not tested here).
        place_all(&mut g, &[(0, 40, 'h'), (1, 56, 'h')]); // len3 @40.., len2 @56..
        // Seat 1 fleet at known cells so seat 0 can sink them deterministically.
        place_all(&mut g, &[(0, 0, 'h'), (1, 16, 'h')]); // len3 @0,1,2 ; len2 @16,17
        assert!(g.both_placed());
        assert_eq!(g.current, 0);
        assert_eq!(g.volley_left, 2); // seat 0 has 2 ships → 2 shots
        // Volley 1: two hits on the len3 (not yet sunk).
        assert!(g.apply("s:0"));
        assert_eq!(g.volley_left, 1);
        assert_eq!(g.current, 0); // current held through the volley
        assert!(g.apply("s:1"));
        assert_eq!(g.current, 1); // volley done → handoff
        assert_eq!(g.volley_left, 2); // seat 1 also has 2 ships
        // Seat 1 wastes its volley on empty water.
        assert!(g.apply("s:60"));
        assert!(g.apply("s:61"));
        assert_eq!(g.current, 0);
        assert_eq!(g.volley_left, 2); // seat 0 still has both its ships
        // Volley 2: finish the len3 (sunk) and start on the len2.
        assert!(g.apply("s:2"));
        assert_eq!(g.shots[0].last().unwrap().1, ShotResult::Sunk(3));
        assert!(g.apply("s:16"));
        assert_eq!(g.current, 1);
        // RECOUNT: seat 1 lost its len3 → only 1 surviving ship → volley of 1.
        assert_eq!(g.volley_left, 1);
    }

    #[test]
    fn non_salvo_is_one_shot_per_turn() {
        let mut g = Salvo::new(8, [0, 1, 1, 0, 0], true, false);
        place_all(&mut g, &[(0, 40, 'h'), (1, 56, 'h')]);
        place_all(&mut g, &[(0, 0, 'h'), (1, 16, 'h')]);
        assert_eq!(g.volley_left, 1);
        assert!(g.apply("s:0"));
        assert_eq!(g.current, 1); // strict alternation, one shot each
        assert!(g.apply("s:60"));
        assert_eq!(g.current, 0);
    }

    #[test]
    fn move_encoding_round_trips() {
        let g = Salvo::new(10, CLASSIC, true, false); // 5 ships (idx 0..4), 100 cells (0..99)
        // Placement round-trips, including BOTH corner cells (0 and size*size-1)
        // and both orientations. (parse_place validates only idx/cell bounds, not
        // ship fit, so a corner origin round-trips regardless of length.)
        for (idx, cell, o) in [
            (0usize, 0usize, Orient::H),  // first cell
            (4, 99, Orient::V),           // last cell (size*size - 1)
            (0, 34, Orient::H),
            (3, 77, Orient::V),
        ] {
            let s = Salvo::format_place(idx, cell, o);
            assert_eq!(g.parse_place(&s), Some((idx, cell, o)));
        }
        // Shot round-trips at both edge cells and an interior cell.
        for cell in [0usize, 42, 99] {
            let s = Salvo::format_shot(cell);
            assert_eq!(g.parse_shot(&s), Some(cell));
        }
        // Parse rejections (both move forms).
        assert_eq!(g.parse_shot("s:100"), None); // one past the last cell
        assert_eq!(g.parse_shot("s:999"), None); // far out of a 10×10 board
        assert_eq!(g.parse_place("p:0:5:x"), None); // bad orientation char
        assert_eq!(g.parse_place("p:9:5:h"), None); // ship index 9 ≥ fleet len (5)
        assert_eq!(g.parse_place("p:0:100:h"), None); // cell one past the last
        assert_eq!(g.parse_place("p:0:5:h:extra"), None); // trailing token
    }

    #[test]
    fn feasibility_clamps_an_overfull_fleet() {
        // 6×6 = 36 cells; 55% budget = 19 cells. Six length-5 ships = 30 cells is
        // impossible → the fleet MUST be clamped to something placeable.
        let g = Salvo::new(6, [0, 0, 0, 0, 6], false, false);
        let cells: usize = g.fleet.iter().sum();
        assert!(cells <= 36 * 55 / 100, "fleet not clamped to budget: {} cells", cells);
        assert!(!g.fleet.is_empty(), "clamp removed every ship");
        // And a concrete placement exists (never accept an unplaceable fleet).
        assert!(g.placement_plan(0).is_some(), "clamped fleet is still unplaceable");
    }

    #[test]
    fn empty_fleet_is_clamped_to_a_single_ship() {
        let g = Salvo::new(8, [0, 0, 0, 0, 0], true, false);
        assert_eq!(g.fleet.len(), 1, "an all-zero fleet must become one ship");
        assert!(g.placement_plan(0).is_some());
    }

    #[test]
    fn dinghy_swarm_extreme_is_placeable() {
        // The silly flagship preset: 12×12, six length-1 ships. Trivially feasible.
        let g = Salvo::new(12, [6, 0, 0, 0, 0], true, false);
        assert_eq!(g.fleet.len(), 6);
        assert!(g.placement_plan(0).is_some());
    }

    #[test]
    fn random_placement_is_a_valid_full_fleet() {
        let g = Salvo::new(10, CLASSIC, false, false);
        let moves = g.random_placement();
        assert_eq!(moves.len(), g.fleet.len());
        // Apply them to a fresh game — every one must be legal.
        let mut h = Salvo::new(10, CLASSIC, false, false);
        for m in &moves {
            assert!(h.apply(m), "random placement move rejected: {m}");
        }
        assert!(h.all_placed(0));
    }

    #[test]
    fn gunner_never_cheats() {
        // THE secrecy property. Two games with DIFFERENT true opponent fleets but
        // an IDENTICAL feedback log for the firing seat must yield identical
        // candidate sets, heat maps AND chosen shots — proving the gunner reads
        // only feedback, never the hidden fleet.
        let mut a = Salvo::new(10, CLASSIC, true, false);
        let mut b = Salvo::new(10, CLASSIC, true, false);
        // Give the two games DIFFERENT real opponent fleets (irrelevant — the
        // gunner never reads them; it works purely off the injected feedback).
        place_all(&mut a, &[(0, 0, 'h'), (1, 20, 'h'), (2, 40, 'h'), (3, 60, 'h'), (4, 80, 'h')]); // seat 0
        place_all(&mut a, &[(0, 10, 'h'), (1, 30, 'h'), (2, 50, 'h'), (3, 70, 'h'), (4, 90, 'h')]); // seat 1
        place_all(&mut b, &[(0, 3, 'h'), (1, 23, 'h'), (2, 43, 'h'), (3, 63, 'h'), (4, 83, 'h')]); // seat 0
        place_all(&mut b, &[(0, 13, 'h'), (1, 33, 'h'), (2, 53, 'h'), (3, 73, 'h'), (4, 93, 'h')]); // seat 1
        // Inject the SAME feedback log for seat 0 in both games.
        let log = vec![
            (44, ShotResult::Miss),
            (45, ShotResult::Hit),
            (55, ShotResult::Miss),
            (46, ShotResult::Hit),
            (12, ShotResult::Miss),
        ];
        for g in [&mut a, &mut b] {
            g.shots[0] = log.clone();
            for &(c, _) in &log {
                g.fired[0][c] = true;
            }
        }
        for seed in [1u32, 7, 42, 1000] {
            let ca = a.sample_candidates(0, 80, seed);
            let cb = b.sample_candidates(0, 80, seed);
            let key = |cands: &[Vec<Ship>]| -> Vec<Vec<usize>> {
                cands
                    .iter()
                    .map(|c| {
                        let mut cells: Vec<usize> = c.iter().flat_map(|s| s.cells.clone()).collect();
                        cells.sort_unstable();
                        cells
                    })
                    .collect()
            };
            assert_eq!(key(&ca), key(&cb), "candidate sets diverged (seed {seed}) — gunner peeked");
            let n2 = 100;
            assert_eq!(Salvo::heat_map(&ca, n2), Salvo::heat_map(&cb, n2), "heat diverged (seed {seed})");
            assert_eq!(
                a.pick_shot(0, 80, 2, 0.7, seed),
                b.pick_shot(0, 80, 2, 0.7, seed),
                "chosen shot diverged (seed {seed}) — gunner peeked at the fleet",
            );
        }
    }

    #[test]
    fn hunt_concentrates_after_a_lone_hit() {
        // After a single unresolved hit, the heat map must concentrate on the
        // hit's ORTHOGONAL neighbours (a ship covering the hit extends through
        // them) — the emergent hunt behaviour, verified on the heat map itself.
        let mut g = Salvo::new(10, CLASSIC, true, false);
        // Real fleets are irrelevant; drive the gunner off an injected feedback log.
        let hit = 5 * 10 + 5; // cell (5,5)
        g.shots[0] = vec![(hit, ShotResult::Hit)];
        g.fired[0][hit] = true;
        let cands = g.sample_candidates(0, 400, 12345);
        assert!(!cands.is_empty());
        let heat = Salvo::heat_map(&cands, 100);
        let up = hit - 10;
        let down = hit + 10;
        let left = hit - 1;
        let right = hit + 1;
        let far = 0; // corner, distant from (5,5)
        for &n in &[up, down, left, right] {
            assert!(
                heat[n] > heat[far] + 0.1,
                "neighbour {n} heat {} not concentrated over far {} ({})",
                heat[n],
                far,
                heat[far]
            );
        }
        // Orthogonal neighbours dominate the diagonals (a straight ship through the
        // hit can't cover a diagonal).
        let diag = hit - 11; // (4,4)
        let min_orth = [up, down, left, right].iter().map(|&n| heat[n]).fold(f64::MAX, f64::min);
        assert!(min_orth > heat[diag], "diagonal {diag} ({}) not below orthogonals ({min_orth})", heat[diag]);
        // The argmax target is an orthogonal neighbour.
        let target = g.pick_shot(0, 400, 1, 0.0, 7);
        assert!([up, down, left, right].contains(&target), "hunt target {target} is not an orthogonal neighbour");
    }

    #[test]
    fn hard_gunner_sinks_a_classic_10x10_fleet_within_bound() {
        // Hard = argmax over a sharp heat map + parity search. Honest bound: the
        // gunner must sink the 17-cell classic fleet in FAR fewer than firing the
        // whole 100-cell board — 17 hits + a bounded search. This test drives ONE
        // layout at ONE fixed seed and pins the assert at ≤ 72 shots (measured 57
        // on this all-vertical layout — a hard case). Derivation of the bound:
        // 17 unavoidable ship-cell hits + a parity-limited search of the ≤50
        // same-parity water cells (l_min = 2 ⇒ checkerboard) + a small margin for
        // hunt overshoot ≈ 70. See the task report.
        let layouts: &[&[(usize, usize, char)]] = &[
            &[(0, 5, 'v'), (1, 7, 'v'), (2, 9, 'v'), (3, 50, 'h'), (4, 72, 'h')],
        ];
        for (li, layout) in layouts.iter().enumerate() {
            // Build a real opponent placement, then play Hard against it, feeding
            // TRUE feedback into a gunner that only ever reads its own shot log.
            let mut opp = Salvo::new(10, CLASSIC, true, false);
            for m in opp.random_placement() {
                opp.apply(&m); // seat 0 = an arbitrary valid fleet (unused by the gunner)
            }
            place_all(&mut opp, layout); // seat 1 = the target fleet
            let mut shots = 0;
            loop {
                assert!(shots < 100, "gunner never finished (layout {li})");
                let cell = opp.pick_shot(0, 300, 1, 0.0, 1); // Hard settings, fixed seed
                assert!(opp.apply(&Salvo::format_shot(cell)), "gunner fired an illegal cell");
                shots += 1;
                if opp.is_terminal() {
                    break;
                }
                // Non-salvo: seat 1 must pass a (wasted) shot back so seat 0 fires again.
                let idle = (0..100).find(|&c| !opp.fired[1][c]).unwrap();
                assert!(opp.apply(&Salvo::format_shot(idle)));
            }
            // Honest derived bound: 17 unavoidable ship-cell hits + a parity-limited
            // search of the ≤50 same-parity water cells (l_min = 2 ⇒ checkerboard)
            // + a small margin for hunt overshoot ≈ 70. Measured here: 57.
            assert!(shots <= 72, "Hard took {shots} shots (layout {li}, expected ≤72)");
        }
    }

    #[test]
    fn wrong_seat_never_sees_fleet() {
        // The load-bearing board-string secrecy test. Mid-game, neither seat's view
        // may expose the OTHER seat's un-sunk ship cells (dedicated field empty),
        // and a sunk ship IS revealed, and game-over reveals both fleets.
        let mut g = Salvo::new(8, [0, 0, 1, 0, 0], true, false); // one len3 each
        place_all(&mut g, &[(0, 0, 'h')]); // seat 0: 0,1,2
        place_all(&mut g, &[(0, 40, 'h')]); // seat 1: 40,41,42
        // Seat 0 half-hits seat 1's ship (not sunk).
        g.apply("s:40");
        g.apply("s:60"); // seat 1 wastes a shot
        g.apply("s:41");
        // Field layout: index 12 = oppsunk, 13 = oppfull.
        let v0: Vec<&str> = {
            let s = g.render(0);
            s.split('|').map(|x| x.to_string()).collect::<Vec<_>>().leak().iter().map(|s| s.as_str()).collect()
        };
        assert_eq!(v0[12], "", "seat 0 must not see seat 1's un-sunk ship (oppsunk should be empty)");
        assert_eq!(v0[13], "", "oppfull must be empty mid-game");
        // Seat 1's view must not expose seat 0's intact ship either.
        let s1 = g.render(1);
        let v1: Vec<&str> = s1.split('|').collect();
        assert_eq!(v1[12], "", "seat 1 must not see seat 0's un-sunk ship");
        assert_eq!(v1[13], "");
        // Now finish sinking seat 1's ship → it becomes revealed to seat 0.
        g.apply("s:63"); // seat 1 idle
        g.apply("s:42"); // sinks seat 1's len3
        assert!(g.is_terminal());
        let over = g.render(0);
        let vo: Vec<&str> = over.split('|').collect();
        assert!(vo[12].contains("40.41.42.3"), "sunk ship not revealed in oppsunk: {}", vo[12]);
        assert!(!vo[13].is_empty(), "game over must reveal the opponent's full fleet");
    }

    #[test]
    fn legal_sample_moves_are_all_applyable() {
        let mut g = Salvo::new(8, [0, 0, 1, 0, 1], true, false);
        // placement phase for seat 0
        for m in g.legal_sample() {
            let mut c = g.clone();
            assert!(c.apply(&m), "placement legal move rejected: {m}");
        }
        // Drive to fire phase and re-check.
        for m in g.random_placement() {
            g.apply(&m);
        }
        for m in g.random_placement() {
            g.apply(&m);
        }
        assert!(g.both_placed());
        for m in g.legal_sample() {
            let mut c = g.clone();
            assert!(c.apply(&m), "fire legal move rejected: {m}");
        }
    }

    #[test]
    fn ai_plays_a_full_self_play_game_to_terminal() {
        for salvo in [false, true] {
            let mut g = Salvo::new(8, [0, 0, 1, 1, 0], true, salvo); // lengths {4,3}
            let mut rng = SmallRng::seed_from_u64(if salvo { 7 } else { 3 });
            let mut plies = 0;
            while !g.is_terminal() {
                assert!(plies < 400, "self-play did not terminate (salvo={salvo})");
                let mv = g.pick_move(40, 2, 0.4, rng.gen()).unwrap();
                assert!(g.apply(&mv), "AI produced an illegal move: {mv} (salvo={salvo})");
                plies += 1;
            }
            assert!(!g.result().is_empty());
        }
    }

    #[test]
    fn cannot_fire_twice_at_the_same_cell() {
        let mut g = Salvo::new(8, [0, 0, 1, 0, 0], true, false);
        place_all(&mut g, &[(0, 0, 'h')]);
        place_all(&mut g, &[(0, 40, 'h')]);
        assert!(g.apply("s:20"));
        g.apply("s:60"); // seat 1
        assert!(!g.apply("s:20"), "refiring the same cell must be rejected");
    }
}
