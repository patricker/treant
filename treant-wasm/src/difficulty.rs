//! Difficulty: a value-aware MCTS weakening dial shared by every game.
//!
//! `pick_weak` adds `playouts` MCTS iterations to the current search, then selects a move by a
//! temperature softmax over visit counts **restricted to the top-K most-visited
//! children** (so it never samples a 1-visit blunder off the tail). temp→0 = the
//! engine's best move; larger temp = flatter choice among the K strongest moves;
//! more playouts = stronger search.
//!
//! **Win protection is solver-only.** The two safety nets below — "never throw a
//! proven win" and "take a proven-Loss child as an immediate win" — read
//! `ProvenValue`s, and proven values are stored **only** when the game's
//! `MCTS::solver_enabled()` returns `true` (see `search_tree`; the trait default
//! is `false`). For a game that does not enable the solver — e.g. Connect6,
//! Mancala, ConnectFour, Reversi, Pig, Dice — both guards are inert: every
//! child's `proven_value` stays `Unknown`, so at high temperature `pick_weak`
//! can and will sample a losing move off a winning position. The guards only
//! function for the solver-enabled games (the small perfect-information ones:
//! TicTacToe, Squava, Notakto, SquareUp, Order&Chaos, Nim, …).
//!
//! **`seed` is not full reproducibility.** It seeds only the final softmax
//! tie-break draw. The visit counts that draw samples over come from
//! `manager.playout_n(playouts)`, whose selection RNG is seeded from
//! `MCTS::rng_seed()` — `None` (the default) for every arcade config, so the
//! search itself draws from `thread_rng()`. The same `seed` therefore yields
//! different visit distributions (and often different moves) run to run. Full
//! reproducibility would additionally require a seeded search config.
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::fmt::Display;
use treant::{MCTSManager, Move, MoveEvaluation, ProvenValue, MCTS};

/// Pick a difficulty-weakened move. Parameter domains:
///
/// - `playouts == 0` → returns `None` (the caller falls back to a random legal
///   move); otherwise this many MCTS iterations are added to the search first.
/// - `top_k` is clamped to `[1, #root moves]`, so `0` behaves as `1` (greedy).
/// - `temp <= 1e-4` (which includes any non-positive temp) means greedy: return
///   the most-visited move. Larger `temp` flattens the softmax over the top-K.
/// - `seed` only makes the softmax tie-break draw reproducible; see the module
///   docs — the underlying search RNG is unseeded.
///
/// The proven-win / proven-Loss guards only fire for solver-enabled games (see
/// the module docs).
pub fn pick_weak<Spec>(
    manager: &mut MCTSManager<Spec>,
    playouts: u64,
    top_k: usize,
    temp: f64,
    seed: u32,
) -> Option<String>
where
    Spec: MCTS,
    Spec::ExtraThreadData: Default,
    MoveEvaluation<Spec>: Clone,
    Move<Spec>: Display + Clone,
{
    if playouts == 0 {
        return None; // caller falls back to a random legal move
    }
    manager.playout_n(playouts);

    // Protect a forced win: never throw a game the search has proven won.
    if matches!(manager.root_proven_value(), ProvenValue::Win) {
        return manager.best_move().map(|m| format!("{m}"));
    }

    let mut stats = manager.root_child_stats();
    if stats.is_empty() {
        return None;
    }
    stats.sort_by(|a, b| b.visits.cmp(&a.visits));
    let k = top_k.clamp(1, stats.len());
    let cand = &stats[..k];

    // A child proven Loss (for the opponent) is an immediate winning move — take it.
    if let Some(win) = cand.iter().find(|s| s.proven_value == ProvenValue::Loss) {
        return Some(format!("{}", win.mov));
    }

    if temp <= 0.0001 || cand.len() == 1 {
        return Some(format!("{}", cand[0].mov));
    }

    let maxv = cand[0].visits.max(1) as f64;
    let weights: Vec<f64> = cand
        .iter()
        .map(|s| (s.visits as f64 / maxv).powf(1.0 / temp))
        .collect();
    let sum: f64 = weights.iter().sum();
    if sum == 0.0 {
        return Some(format!("{}", cand[0].mov));
    }
    let mut rng = SmallRng::seed_from_u64(seed as u64);
    let mut r = rng.gen::<f64>() * sum;
    for (i, w) in weights.iter().enumerate() {
        r -= w;
        if r <= 0.0 {
            return Some(format!("{}", cand[i].mov));
        }
    }
    Some(format!("{}", cand[0].mov))
}
