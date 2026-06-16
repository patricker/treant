//! Difficulty: a value-aware MCTS weakening dial shared by every game.
//!
//! `pick_weak` runs `playouts` MCTS iterations, then selects a move by a
//! temperature softmax over visit counts **restricted to the top-K most-visited
//! children** (so it never samples a 1-visit blunder off the tail), while always
//! taking a proven win. temp→0 = the engine's best move; larger temp = flatter
//! choice among the K strongest moves; more playouts = stronger search.
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::fmt::Display;
use treant::{MCTSManager, Move, MoveEvaluation, ProvenValue, MCTS};

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

    if temp <= 0.0001 || cand.len() == 1 {
        return Some(format!("{}", cand[0].mov));
    }

    let maxv = cand[0].visits.max(1) as f64;
    let weights: Vec<f64> = cand
        .iter()
        .map(|s| (s.visits as f64 / maxv).powf(1.0 / temp))
        .collect();
    let sum: f64 = weights.iter().sum();
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
