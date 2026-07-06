//! Bulls & Cows — the public-domain code-breaking duel (Mastermind's ancestor;
//! never surface the trademarked name in user strings). Each player sets a
//! secret code; players alternate guesses; a guess earns **bulls** (right digit,
//! right place) and **cows** (right digit, wrong place). First to fully crack the
//! opponent's code wins.
//!
//! ## Rules provenance & the equalization question
//! Rules are the Wikipedia "Bulls and Cows" article: codes are `length` digits,
//! digits all different by default (repeats optional here), bulls/cows feedback,
//! "the first player to reveal the other's secret number wins the game." The
//! article documents **no** equalization rule (no final balancing guess for the
//! second player). Per the plan's documented fallback, we therefore take
//! **first-to-crack wins outright** — strict alternation, seat 0 guesses first.
//! (This hands seat 0 a small first-mover edge; that is the honest consequence of
//! the source not attesting equalization, and it is why difficulty is hand-set
//! rather than win-rate-calibrated — see the tile.)
//!
//! ## Where the AI lives (and why it CANNOT cheat)
//! This game deliberately does **not** wrap a treant tree search around its true
//! state. A tree search over the real position would see the opponent's hidden
//! code in every rollout — i.e. it would cheat. Doing it honestly needs
//! determinization (sampling hidden states), which is deferred to Salvo/Ambush.
//! Instead the whole intelligence is a **closed-form consistent-set deducer**
//! (Knuth-style): from ONLY the feedback the guessing seat has received, it
//! enumerates every code consistent with that history and picks the guess that
//! minimises the worst-case remaining candidate count. Because the deducer reads
//! *only* `guesses[seat]` (the seat's own guess+feedback log) and never the
//! opponent's stored code, it is structurally incapable of cheating — proven by
//! `ai_is_a_pure_function_of_feedback` below (two games with different opponent
//! codes but identical feedback logs yield identical candidate sets and moves).
//! `playout_n` is a documented no-op: this engine has nothing to search.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

const MAX_ROUNDS: usize = 20; // per-player guess cap before a bulls-progress tiebreak
const SCORE_CAP: usize = 1500; // candidates the Knuth scorer partitions against (cost guard)
const LEGAL_SAMPLE: usize = 12; // representative sample size for legal_moves()

/// Feedback for `guess` against `code`: `(bulls, cows)`. Position-independent
/// matches use the min-count-per-symbol rule, so repeated digits are handled
/// correctly (cows = total symbol matches − bulls).
fn feedback(guess: &[u8], code: &[u8]) -> (u8, u8) {
    let mut bulls = 0u8;
    for (g, c) in guess.iter().zip(code.iter()) {
        if g == c {
            bulls += 1;
        }
    }
    let mut total = 0u32;
    for sym in 0..10u8 {
        let gc = guess.iter().filter(|&&d| d == sym).count();
        let cc = code.iter().filter(|&&d| d == sym).count();
        total += gc.min(cc) as u32;
    }
    (bulls, total as u8 - bulls)
}

fn all_distinct(code: &[u8]) -> bool {
    let mut seen = [false; 10];
    for &d in code {
        if seen[d as usize] {
            return false;
        }
        seen[d as usize] = true;
    }
    true
}

#[derive(Clone)]
struct BullsCows {
    len: usize,
    sym: usize,
    repeats: bool,
    /// Secret codes, one per seat; `None` until that seat has set it.
    codes: [Option<Vec<u8>>; 2],
    /// Per-seat guess log: `(guess, bulls, cows)`, in order. Public information.
    guesses: [Vec<(Vec<u8>, u8, u8)>; 2],
    current: u8,
    winner: Option<u8>,
}

impl BullsCows {
    fn new(len: usize, sym: usize, repeats: bool) -> Self {
        let len = len.clamp(3, 5);
        let sym = sym.clamp(6, 10);
        Self {
            len,
            sym,
            repeats,
            codes: [None, None],
            guesses: [Vec::new(), Vec::new()],
            current: 0,
            winner: None,
        }
    }

    fn both_set(&self) -> bool {
        self.codes[0].is_some() && self.codes[1].is_some()
    }

    fn cap_reached(&self) -> bool {
        self.guesses[0].len() >= MAX_ROUNDS && self.guesses[1].len() >= MAX_ROUNDS
    }

    fn is_terminal(&self) -> bool {
        self.winner.is_some() || self.cap_reached()
    }

    /// `""` in progress, `"Draw"`, or a 1-indexed winner-seat digit.
    fn result(&self) -> String {
        if let Some(w) = self.winner {
            return format!("{}", w + 1);
        }
        if self.cap_reached() {
            let p0 = self.guesses[0].iter().map(|g| g.1).max().unwrap_or(0);
            let p1 = self.guesses[1].iter().map(|g| g.1).max().unwrap_or(0);
            return match p0.cmp(&p1) {
                std::cmp::Ordering::Greater => "1".into(),
                std::cmp::Ordering::Less => "2".into(),
                std::cmp::Ordering::Equal => "Draw".into(),
            };
        }
        String::new()
    }

    /// Validate a code string: exactly `len` digits, each `< sym`, distinct
    /// unless `repeats`. Returns the parsed digits.
    fn parse_code(&self, s: &str) -> Option<Vec<u8>> {
        if s.len() != self.len {
            return None;
        }
        let mut out = Vec::with_capacity(self.len);
        for ch in s.chars() {
            let d = ch.to_digit(10)? as u8;
            if d as usize >= self.sym {
                return None;
            }
            out.push(d);
        }
        if !self.repeats && !all_distinct(&out) {
            return None;
        }
        Some(out)
    }

    fn code_str(code: &[u8]) -> String {
        code.iter().map(|d| char::from(b'0' + d)).collect()
    }

    /// Apply a move string (`"set:0123"` or `"g:4567"`). Returns false if illegal
    /// or out of phase.
    fn apply(&mut self, mov: &str) -> bool {
        if self.is_terminal() {
            return false;
        }
        let seat = self.current as usize;
        if let Some(rest) = mov.strip_prefix("set:") {
            if self.both_set() || self.codes[seat].is_some() {
                return false; // setup phase only, once per seat
            }
            let code = match self.parse_code(rest) {
                Some(c) => c,
                None => return false,
            };
            self.codes[seat] = Some(code);
            self.current = if self.both_set() { 0 } else { 1 - self.current };
            true
        } else if let Some(rest) = mov.strip_prefix("g:") {
            if !self.both_set() {
                return false; // no guessing before both codes are set
            }
            let guess = match self.parse_code(rest) {
                Some(c) => c,
                None => return false,
            };
            let target = self.codes[1 - seat].as_ref().unwrap();
            let (b, c) = feedback(&guess, target);
            self.guesses[seat].push((guess, b, c));
            if b as usize == self.len {
                self.winner = Some(self.current); // first to crack wins outright
            } else if !self.cap_reached() {
                self.current = 1 - self.current;
            }
            true
        } else {
            false
        }
    }

    /// Board string for `seat`'s eyes only. Fields joined by `|`:
    /// `phase|len|sym|rep|cur|winner|mycode|oppcode|myrows|opprows`.
    /// `mycode` is ALWAYS only `seat`'s own code; `oppcode` is the opponent's and
    /// is populated **only** once the game is over (nothing secret remains). Rows
    /// are `digits:bulls:cows` entries joined by `;`. This is the single source of
    /// truth for the board encoding — the React board parses exactly this shape.
    fn render(&self, seat: usize) -> String {
        let seat = seat.min(1);
        let over = self.is_terminal();
        let phase = if over {
            "over"
        } else if self.both_set() {
            "guess"
        } else {
            "set"
        };
        let winner = match self.winner {
            Some(w) => format!("{w}"),
            None if over => "draw".into(),
            None => String::new(),
        };
        let mycode = self.codes[seat].as_ref().map(|c| Self::code_str(c)).unwrap_or_default();
        // The opponent's code is revealed ONLY at game over — never mid-game.
        let oppcode = if over {
            self.codes[1 - seat].as_ref().map(|c| Self::code_str(c)).unwrap_or_default()
        } else {
            String::new()
        };
        let rows = |g: &Vec<(Vec<u8>, u8, u8)>| {
            g.iter()
                .map(|(guess, b, c)| format!("{}:{}:{}", Self::code_str(guess), b, c))
                .collect::<Vec<_>>()
                .join(";")
        };
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            phase,
            self.len,
            self.sym,
            if self.repeats { 1 } else { 0 },
            self.current,
            winner,
            mycode,
            oppcode,
            rows(&self.guesses[seat]),
            rows(&self.guesses[1 - seat]),
        )
    }

    /// Every code consistent with `seat`'s OWN guess+feedback log. Reads only
    /// `guesses[seat]` — never any stored code — which is what makes the AI
    /// provably non-cheating.
    fn consistent_codes(&self, seat: usize) -> Vec<Vec<u8>> {
        let hist = &self.guesses[seat];
        let mut out = Vec::new();
        let mut cur = vec![0u8; self.len];
        loop {
            let distinct_ok = self.repeats || all_distinct(&cur);
            if distinct_ok && hist.iter().all(|(g, b, c)| feedback(g, &cur) == (*b, *c)) {
                out.push(cur.clone());
            }
            // odometer increment over base-`sym` digits
            let mut i = 0;
            loop {
                if i == self.len {
                    return out;
                }
                cur[i] += 1;
                if (cur[i] as usize) < self.sym {
                    break;
                }
                cur[i] = 0;
                i += 1;
            }
        }
    }

    fn canonical_first(&self) -> Vec<u8> {
        if self.repeats {
            // A mild spread beats all-same for information: 0,1,2,… wrapping sym.
            (0..self.len).map(|i| (i % self.sym) as u8).collect()
        } else {
            (0..self.len).map(|i| i as u8).collect()
        }
    }

    fn random_code(&self, rng: &mut SmallRng) -> Vec<u8> {
        if self.repeats {
            (0..self.len).map(|_| rng.gen_range(0..self.sym) as u8).collect()
        } else {
            let mut pool: Vec<u8> = (0..self.sym as u8).collect();
            // partial Fisher–Yates
            for i in 0..self.len {
                let j = i + rng.gen_range(0..(pool.len() - i));
                pool.swap(i, j);
            }
            pool[..self.len].to_vec()
        }
    }

    /// The deducer's next guess for `seat`. `playouts` caps how many candidate
    /// guesses get scored (the difficulty budget); `top_k`/`temp` sample among the
    /// best-scoring candidates (temp→0, top_k=1 ⇒ the information-optimal guess).
    fn ai_guess(&self, seat: usize, playouts: u64, top_k: usize, temp: f64, seed: u32) -> Vec<u8> {
        let mut rng = SmallRng::seed_from_u64(seed as u64 ^ 0xB0115C0);
        if self.guesses[seat].is_empty() {
            return self.canonical_first(); // no information yet — all guesses equivalent
        }
        let cs = self.consistent_codes(seat);
        if cs.len() <= 1 {
            return cs.into_iter().next().unwrap_or_else(|| self.random_code(&mut rng));
        }
        // Candidate guesses: drawn from the consistent set. Cap by the playout budget.
        let cand_cap = (playouts as usize).max(1);
        let cand_idx: Vec<usize> = if cs.len() <= cand_cap {
            (0..cs.len()).collect()
        } else {
            let mut idx: Vec<usize> = (0..cs.len()).collect();
            for i in 0..cand_cap {
                let j = i + rng.gen_range(0..(idx.len() - i));
                idx.swap(i, j);
            }
            idx.truncate(cand_cap);
            idx
        };
        // Set the candidates are scored against (cost-guarded sample of cs).
        let against: Vec<&Vec<u8>> = if cs.len() <= SCORE_CAP {
            cs.iter().collect()
        } else {
            let mut idx: Vec<usize> = (0..cs.len()).collect();
            for i in 0..SCORE_CAP {
                let j = i + rng.gen_range(0..(idx.len() - i));
                idx.swap(i, j);
            }
            idx[..SCORE_CAP].iter().map(|&i| &cs[i]).collect()
        };
        // Knuth minimax: worst-case surviving-candidate count (smaller is better).
        let mut scored: Vec<(usize, usize)> = cand_idx
            .iter()
            .map(|&ci| {
                let mut buckets: HashMap<(u8, u8), usize> = HashMap::new();
                for c in &against {
                    *buckets.entry(feedback(&cs[ci], c)).or_insert(0) += 1;
                }
                (buckets.values().copied().max().unwrap_or(0), ci)
            })
            .collect();
        scored.sort_by_key(|&(worst, _)| worst);
        let k = top_k.clamp(1, scored.len());
        let cand = &scored[..k];
        if temp <= 0.0001 || cand.len() == 1 {
            return cs[cand[0].1].clone();
        }
        // Softmax over the top-K by inverse worst-bucket (lower worst = higher weight).
        let best = cand[0].0.max(1) as f64;
        let weights: Vec<f64> = cand.iter().map(|&(w, _)| (best / w.max(1) as f64).powf(1.0 / temp)).collect();
        let sum: f64 = weights.iter().sum();
        if sum <= 0.0 {
            return cs[cand[0].1].clone();
        }
        let mut r = rng.gen::<f64>() * sum;
        for (i, w) in weights.iter().enumerate() {
            r -= w;
            if r <= 0.0 {
                return cs[cand[i].1].clone();
            }
        }
        cs[cand[0].1].clone()
    }

    /// The current seat's move (setup: a random code — chance; guess: the deducer).
    fn pick_move(&self, playouts: u64, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        if self.is_terminal() {
            return None;
        }
        let seat = self.current as usize;
        if !self.both_set() {
            let mut rng = SmallRng::seed_from_u64(seed as u64 ^ 0x5E7C0DE);
            return Some(format!("set:{}", Self::code_str(&self.random_code(&mut rng))));
        }
        Some(format!("g:{}", Self::code_str(&self.ai_guess(seat, playouts, top_k, temp, seed))))
    }

    /// A small, cheap, representative sample of legal moves (never the exhaustive
    /// space — the human board uses a keypad and the AI uses `weak_move`; this
    /// only needs to be non-empty and applyable for the random-move fallback).
    fn legal_sample(&self) -> Vec<String> {
        if self.is_terminal() {
            return Vec::new();
        }
        let prefix = if self.both_set() { "g" } else { "set" };
        let mut rng = SmallRng::seed_from_u64(0x1E6A1 ^ self.guesses[0].len() as u64 ^ ((self.guesses[1].len() as u64) << 8));
        let mut out = Vec::new();
        // Deterministic canonical entry first, then random spread — dedup-friendly.
        out.push(format!("{prefix}:{}", Self::code_str(&self.canonical_first())));
        while out.len() < LEGAL_SAMPLE {
            let m = format!("{prefix}:{}", Self::code_str(&self.random_code(&mut rng)));
            if !out.contains(&m) {
                out.push(m);
            }
        }
        out
    }
}

#[wasm_bindgen]
pub struct BullsCowsWasm {
    g: BullsCows,
    len: usize,
    sym: usize,
    repeats: bool,
}

#[wasm_bindgen]
impl BullsCowsWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(length: usize, symbols: usize, repeats: u32) -> Self {
        let g = BullsCows::new(length, symbols, repeats != 0);
        Self { len: g.len, sym: g.sym, repeats: g.repeats, g }
    }

    /// No-op by design: Bulls & Cows' AI is a closed-form deducer, not a tree
    /// search (see the module header). Present only for surface uniformity.
    pub fn playout_n(&mut self, _n: u32) {}

    /// Board for the CURRENT player's eyes. Prefer `get_board_for` in hidden-info
    /// flows; this fallback still only ever reveals the current seat's secret.
    pub fn get_board(&self) -> String {
        self.g.render(self.g.current as usize)
    }

    /// Board for a specific seat — the per-seat secret view. Reveals only that
    /// seat's own code (plus all public guesses/feedback); never the opponent's
    /// code until the game is over.
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

    pub fn apply_move(&mut self, mov: &str) -> bool {
        self.g.apply(mov)
    }

    pub fn reset(&mut self) {
        self.g = BullsCows::new(self.len, self.sym, self.repeats);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_distinct_digits() {
        // Wikipedia's worked example: secret 4271, guess 1234 → 1 bull, 2 cows.
        let (b, c) = feedback(&[1, 2, 3, 4], &[4, 2, 7, 1]);
        assert_eq!((b, c), (1, 2));
    }

    #[test]
    fn feedback_all_bulls() {
        assert_eq!(feedback(&[0, 1, 2, 3], &[0, 1, 2, 3]), (4, 0));
    }

    #[test]
    fn feedback_handles_repeated_digits() {
        // guess 1122 vs code 1213: pos0 '1'=='1' bull; pos1 '1' vs '2'; pos2 '2'
        // vs '1'; pos3 '2' vs '3'. Symbol counts: '1' g2/c2→2, '2' g2/c1→1,
        // '3' g0/c1→0 → total matches 3, bulls 1 → cows 2.
        assert_eq!(feedback(&[1, 1, 2, 2], &[1, 2, 1, 3]), (1, 2));
        // guess 2222 vs code 1213: one '2' in code → 1 match, 0 bulls → 1 cow.
        assert_eq!(feedback(&[2, 2, 2, 2], &[1, 2, 1, 3]), (1, 0));
    }

    #[test]
    fn code_setting_then_guessing_phases() {
        let mut g = BullsCows::new(4, 10, false);
        assert_eq!(g.current, 0);
        assert!(g.apply("set:0123")); // seat 0
        assert_eq!(g.current, 1);
        assert!(!g.apply("g:0123")); // can't guess before both set
        assert!(g.apply("set:4567")); // seat 1
        assert!(g.both_set());
        assert_eq!(g.current, 0); // seat 0 guesses first
    }

    #[test]
    fn rejects_malformed_codes() {
        let mut g = BullsCows::new(4, 10, false);
        assert!(!g.apply("set:012")); // too short
        assert!(!g.apply("set:0122")); // repeat when repeats disabled
        assert!(g.apply("set:0123"));
    }

    #[test]
    fn first_to_crack_wins_and_result_is_seat_digit() {
        let mut g = BullsCows::new(4, 10, false);
        g.apply("set:0123"); // seat 0 code
        g.apply("set:4567"); // seat 1 code
        assert!(g.apply("g:4567")); // seat 0 cracks seat 1's code
        assert!(g.is_terminal());
        assert_eq!(g.winner, Some(0));
        assert_eq!(g.result(), "1");
    }

    #[test]
    fn round_cap_falls_back_to_bulls_progress() {
        let mut g = BullsCows::new(4, 10, false);
        g.codes[0] = Some(vec![0, 1, 2, 3]);
        g.codes[1] = Some(vec![4, 5, 6, 7]);
        // Hand-fill MAX_ROUNDS guesses each, never cracking, seat 0 with more bulls.
        for _ in 0..MAX_ROUNDS {
            g.guesses[0].push((vec![4, 5, 6, 8], 3, 0)); // 3 bulls vs 4567
            g.guesses[1].push((vec![0, 1, 8, 9], 2, 0)); // 2 bulls vs 0123
        }
        assert!(g.cap_reached());
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1"); // seat 0 has more bulls-progress
    }

    #[test]
    fn round_cap_equal_progress_is_a_draw() {
        let mut g = BullsCows::new(4, 10, false);
        g.codes[0] = Some(vec![0, 1, 2, 3]);
        g.codes[1] = Some(vec![4, 5, 6, 7]);
        for _ in 0..MAX_ROUNDS {
            g.guesses[0].push((vec![4, 5, 8, 9], 2, 0));
            g.guesses[1].push((vec![0, 1, 8, 9], 2, 0));
        }
        assert_eq!(g.result(), "Draw");
    }

    #[test]
    fn move_encoding_round_trips() {
        let g = BullsCows::new(4, 10, false);
        let code = vec![3, 1, 4, 0];
        let s = BullsCows::code_str(&code);
        assert_eq!(s, "3140");
        assert_eq!(g.parse_code(&s), Some(code));
    }

    #[test]
    fn board_never_leaks_opponent_code_midgame() {
        let mut g = BullsCows::new(4, 10, false);
        g.apply("set:0123"); // seat 0's secret
        g.apply("set:4567"); // seat 1's secret
        // Seat 0 makes a guess; now it's seat 1's turn.
        g.apply("g:8901");
        assert_eq!(g.current, 1);
        let view = g.render(1); // seat 1's view (the wrong seat for seat 0's code)
        assert!(!view.contains("0123"), "seat 1 must not see seat 0's code: {view}");
        // Seat 0's own view shows seat 0's code but NOT seat 1's code.
        let v0 = g.render(0);
        assert!(v0.contains("0123"));
        assert!(!v0.contains("4567"), "seat 0 must not see seat 1's code mid-game: {v0}");
    }

    #[test]
    fn board_reveals_both_codes_only_at_game_over() {
        let mut g = BullsCows::new(4, 10, false);
        g.apply("set:0123");
        g.apply("set:4567");
        g.apply("g:4567"); // seat 0 cracks it — game over
        let v = g.render(0);
        assert!(v.contains("0123") && v.contains("4567"), "game over reveals both: {v}");
    }

    #[test]
    fn consistent_set_filters_by_feedback() {
        let mut g = BullsCows::new(4, 10, false);
        g.codes[0] = Some(vec![0, 1, 2, 3]);
        g.codes[1] = Some(vec![4, 5, 6, 7]);
        // Seat 0 guessed 4567 and (hypothetically) got 4 bulls → only 4567 fits.
        g.guesses[0].push((vec![4, 5, 6, 7], 4, 0));
        let cs = g.consistent_codes(0);
        assert_eq!(cs, vec![vec![4, 5, 6, 7]]);
    }

    #[test]
    fn ai_is_a_pure_function_of_feedback() {
        // THE never-cheats property. Two games whose opponent codes DIFFER but
        // whose guessing seat has an IDENTICAL feedback log must yield identical
        // candidate sets AND identical AI guesses — proving the AI reads only the
        // feedback history, never the hidden code.
        let mut a = BullsCows::new(4, 10, false);
        let mut b = BullsCows::new(4, 10, false);
        a.codes[0] = Some(vec![9, 8, 7, 6]); // seat 0 (the guesser) — irrelevant
        b.codes[0] = Some(vec![1, 2, 3, 4]);
        a.codes[1] = Some(vec![4, 5, 6, 7]); // DIFFERENT opponent codes
        b.codes[1] = Some(vec![0, 3, 8, 9]);
        // Inject the SAME feedback log for the guessing seat (0) in both.
        let log = vec![(vec![0, 1, 2, 3], 0u8, 1u8), (vec![4, 5, 8, 9], 1u8, 1u8)];
        a.guesses[0] = log.clone();
        b.guesses[0] = log;
        assert_eq!(a.consistent_codes(0), b.consistent_codes(0));
        for seed in [1u32, 7, 42, 1000] {
            assert_eq!(
                a.ai_guess(0, 5000, 3, 0.7, seed),
                b.ai_guess(0, 5000, 3, 0.7, seed),
                "AI guess diverged for identical feedback (seed {seed}) — it peeked at the code",
            );
        }
    }

    #[test]
    fn hard_ai_cracks_a_four_digit_code_quickly() {
        // Play the deducer (Hard settings) against a fixed secret, feeding real
        // feedback, and assert it cracks in ≤ 6 guesses (the plan's target).
        let secret = vec![7, 1, 8, 3];
        let mut g = BullsCows::new(4, 10, false);
        g.codes[0] = Some(vec![0, 1, 2, 3]); // guesser's own code (unused by AI)
        g.codes[1] = Some(secret.clone());
        let mut guesses = 0;
        loop {
            assert!(guesses < 8, "Hard AI failed to crack within 8 guesses");
            let guess = g.ai_guess(0, u64::MAX, 1, 0.0, 0);
            let (b, c) = feedback(&guess, &secret);
            g.guesses[0].push((guess.clone(), b, c));
            guesses += 1;
            if b as usize == g.len {
                break;
            }
        }
        assert!(guesses <= 6, "Hard AI took {guesses} guesses (expected ≤6)");
    }

    #[test]
    fn ai_sets_a_valid_random_code_in_setup() {
        let g = BullsCows::new(4, 10, false);
        let mv = g.pick_move(100, 1, 0.0, 3).unwrap();
        let code = mv.strip_prefix("set:").unwrap();
        assert!(g.parse_code(code).is_some(), "AI setup code invalid: {mv}");
    }

    #[test]
    fn legal_sample_moves_are_all_applyable() {
        let mut g = BullsCows::new(4, 10, false);
        // setup phase
        for m in g.legal_sample() {
            let mut c = g.clone();
            assert!(c.apply(&m), "setup legal move rejected: {m}");
        }
        g.apply("set:0123");
        g.apply("set:4567");
        // guess phase
        for m in g.legal_sample() {
            let mut c = g.clone();
            assert!(c.apply(&m), "guess legal move rejected: {m}");
        }
    }

    #[test]
    fn ai_plays_a_full_self_play_game_to_terminal() {
        let mut g = BullsCows::new(4, 10, false);
        let mut rng = SmallRng::seed_from_u64(99);
        let mut plies = 0;
        while !g.is_terminal() {
            assert!(plies < 100, "self-play did not terminate");
            let mv = g.pick_move(2000, 2, 0.4, rng.gen()).unwrap();
            assert!(g.apply(&mv), "AI produced an illegal move: {mv}");
            plies += 1;
        }
        assert!(!g.result().is_empty());
    }

    #[test]
    fn repeats_variant_allows_and_scores_repeats() {
        let mut g = BullsCows::new(4, 10, true);
        assert!(g.apply("set:1122")); // repeats allowed
        assert!(g.apply("set:3344"));
        assert!(g.apply("g:1122")); // seat 0 cracks its target? target is 3344
        assert!(!g.is_terminal());
    }
}
