//! Gumbel MuZero search: policy improvement by planning with Gumbel.
//!
//! Implements the algorithm from
//! [Danihelka et al., "Policy improvement by planning with Gumbel" (ICLR 2022)](https://openreview.net/forum?id=bERaNdoegnO).
//!
//! Key features:
//! - **Gumbel-Top-k** sampling at the root for action selection
//! - **Sequential Halving** for optimal simulation budget allocation
//! - **PUCT** selection for tree traversal below the root
//! - **Improved policy** output — a better training target than visit counts
//!
//! # Design
//!
//! Gumbel search is fundamentally different from standard MCTS at the root level:
//! instead of UCT/PUCT selection, it samples Gumbel noise, selects top-m actions,
//! then uses Sequential Halving to allocate simulations. Below the root, standard
//! PUCT guides tree traversal. This produces monotonically improving policies —
//! more simulations always help.
//!
//! The crate reuses [`mcts::GameState`] so any game implemented for the core MCTS
//! crate works with Gumbel search.
//!
//! # Example
//!
//! ```
//! use mcts::GameState;
//! use mcts_gumbel::{GumbelSearch, GumbelConfig, GumbelEvaluator};
//!
//! # #[derive(Clone, Debug)] struct MyGame;
//! # #[derive(Clone, Debug, PartialEq)] struct MyMove;
//! # impl GameState for MyGame {
//! #     type Move = MyMove; type Player = (); type MoveList = Vec<MyMove>;
//! #     fn current_player(&self) {}
//! #     fn available_moves(&self) -> Vec<MyMove> { vec![MyMove] }
//! #     fn make_move(&mut self, _: &MyMove) {}
//! #     fn terminal_value(&self) -> Option<mcts::ProvenValue> { Some(mcts::ProvenValue::Draw) }
//! # }
//! # struct Eval;
//! # impl GumbelEvaluator<MyGame> for Eval {
//! #     fn evaluate(&self, _: &MyGame, m: &[MyMove]) -> (Vec<f64>, f64) { (vec![0.0; m.len()], 0.0) }
//! # }
//! let mut search = GumbelSearch::new(Eval, GumbelConfig::default());
//! let result = search.search(&MyGame, 100);
//! println!("Best move: {:?}", result.best_move);
//! ```

use mcts::{GameState, ProvenValue};
use rand::Rng;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;

// ============================================================
// Public API
// ============================================================

/// Evaluator providing policy logits and value estimates.
///
/// Implement this for your game to provide Gumbel search with action
/// priors and state values. For neural networks, wrap the policy/value
/// heads. For heuristic evaluators, return uniform logits with a
/// heuristic value.
pub trait GumbelEvaluator<G: GameState>: Send {
    /// Evaluate a game state.
    ///
    /// Returns `(logits, value)` where:
    /// - `logits`: one `f64` per move (unnormalized log-probabilities)
    /// - `value`: state value for the current player, in `[-1.0, 1.0]`
    fn evaluate(&self, state: &G, moves: &[G::Move]) -> (Vec<f64>, f64);
}

/// Configuration for Gumbel search.
#[derive(Clone, Copy, Debug)]
pub struct GumbelConfig {
    /// Number of actions to consider after Gumbel-Top-k sampling.
    /// Larger values explore more broadly but use more simulation budget.
    /// Default: 16.
    pub m_actions: usize,

    /// PUCT exploration constant for below-root tree traversal.
    /// Default: 1.25.
    pub c_puct: f64,

    /// Maximum search depth per simulation. Default: 200.
    pub max_depth: usize,

    /// Scale factor mapping Q-values to the logit scale (`c_visit` in the paper).
    /// Higher values make the search more exploitation-focused; the improved policy
    /// becomes sharper. Tune relative to your logit scale. Default: 50.0.
    pub value_scale: f64,

    /// RNG seed for Gumbel noise sampling. Default: 42.
    pub seed: u64,
}

impl Default for GumbelConfig {
    fn default() -> Self {
        Self {
            m_actions: 16,
            c_puct: 1.25,
            max_depth: 200,
            value_scale: 50.0,
            seed: 42,
        }
    }
}

/// Per-move statistics from Gumbel search.
pub struct MoveStats<M: Clone> {
    /// The move.
    pub mov: M,
    /// Number of simulations allocated to this move.
    pub visits: u32,
    /// Completed Q-value (empirical mean if visited, root value estimate otherwise).
    pub completed_q: f64,
    /// Improved policy probability from Gumbel search (sums to 1.0 across all moves).
    pub improved_policy: f64,
}

impl<M: Clone + std::fmt::Debug> std::fmt::Debug for MoveStats<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoveStats")
            .field("mov", &self.mov)
            .field("visits", &self.visits)
            .field("completed_q", &self.completed_q)
            .field("improved_policy", &self.improved_policy)
            .finish()
    }
}

/// Result of a Gumbel search.
#[must_use]
pub struct SearchResult<M: Clone> {
    /// The best move found by search.
    pub best_move: M,

    /// Value estimate for the root state's current player.
    pub root_value: f64,

    /// Per-move statistics: visits, completed Q, and improved policy.
    pub move_stats: Vec<MoveStats<M>>,

    /// Total simulations used.
    pub simulations_used: u32,
}

impl<M: Clone + std::fmt::Debug> std::fmt::Debug for SearchResult<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchResult")
            .field("best_move", &self.best_move)
            .field("root_value", &self.root_value)
            .field("simulations_used", &self.simulations_used)
            .field("move_stats", &self.move_stats)
            .finish()
    }
}

/// Gumbel MCTS search engine.
///
/// Implements Gumbel-Top-k root selection with Sequential Halving,
/// providing monotonic policy improvement and better simulation
/// efficiency compared to standard MCTS.
///
/// Two-player zero-sum games (negamax). Single-threaded.
pub struct GumbelSearch<G: GameState, E: GumbelEvaluator<G>> {
    config: GumbelConfig,
    evaluator: E,
    rng: Xoshiro256PlusPlus,
    _phantom: std::marker::PhantomData<G>,
}

impl<G: GameState, E: GumbelEvaluator<G>> std::fmt::Debug for GumbelSearch<G, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GumbelSearch")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

// ============================================================
// Internal tree structures
// ============================================================

struct Node<M: Clone> {
    edges: Vec<Edge<M>>,
    visits: u32,
}

struct Edge<M: Clone> {
    mov: M,
    prior: f64,
    visits: u32,
    value_sum: f64,
    child: Option<Box<Node<M>>>,
}

// ============================================================
// Implementation
// ============================================================

impl<G, E> GumbelSearch<G, E>
where
    G: GameState,
    E: GumbelEvaluator<G>,
{
    /// Create a new Gumbel search engine.
    #[must_use]
    pub fn new(evaluator: E, config: GumbelConfig) -> Self {
        let rng = Xoshiro256PlusPlus::seed_from_u64(config.seed);
        Self {
            config,
            evaluator,
            rng,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Access the evaluator.
    #[must_use]
    pub fn evaluator(&self) -> &E {
        &self.evaluator
    }

    /// Access the configuration.
    #[must_use]
    pub fn config(&self) -> &GumbelConfig {
        &self.config
    }

    /// Reset the RNG seed for reproducible searches.
    pub fn set_seed(&mut self, seed: u64) {
        self.rng = Xoshiro256PlusPlus::seed_from_u64(seed);
    }

    /// Run Gumbel search from the given state.
    ///
    /// # Panics
    ///
    /// Panics if the state is terminal (no available moves).
    pub fn search(&mut self, state: &G, n_simulations: u32) -> SearchResult<G::Move> {
        let moves: Vec<G::Move> = state.available_moves().into_iter().collect();
        assert!(!moves.is_empty(), "cannot search from terminal state");

        // Single move: evaluate for root_value but skip search
        if moves.len() == 1 {
            let (_, root_value) = self.evaluator.evaluate(state, &moves);
            return SearchResult {
                best_move: moves[0].clone(),
                root_value,
                move_stats: vec![MoveStats {
                    mov: moves[0].clone(),
                    visits: 0,
                    completed_q: root_value,
                    improved_policy: 1.0,
                }],
                simulations_used: 0,
            };
        }

        // Evaluate root
        let (logits, root_value) = self.evaluator.evaluate(state, &moves);
        let priors = softmax(&logits);

        // Sample Gumbel(0,1) noise for each action
        let gumbels: Vec<f64> = (0..moves.len())
            .map(|_| sample_gumbel(&mut self.rng))
            .collect();

        // Create root node
        let mut root = Node {
            edges: moves
                .iter()
                .enumerate()
                .map(|(i, m)| Edge {
                    mov: m.clone(),
                    prior: priors[i],
                    visits: 0,
                    value_sum: 0.0,
                    child: None,
                })
                .collect(),
            visits: 0,
        };

        // Top-m selection by g(a) + logit(a)
        let m = self.config.m_actions.min(moves.len());
        let mut alive: Vec<usize> = (0..moves.len()).collect();
        alive.sort_by(|&a, &b| {
            let sa = gumbels[a] + logits[a];
            let sb = gumbels[b] + logits[b];
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        alive.truncate(m);

        // Sequential Halving
        let n_seq = if m <= 1 {
            1
        } else {
            (m as f64).log2().ceil() as u32
        };
        let mut budget = n_simulations;
        let mut total_sims = 0u32;

        for phase in 0..n_seq {
            if alive.len() <= 1 || total_sims >= n_simulations {
                break;
            }

            // Remaining-budget allocation: each phase gets a fair share of what's left
            let phases_left = n_seq - phase;
            let n_a = budget / (alive.len() as u32 * phases_left);
            if n_a == 0 {
                break; // budget exhausted
            }

            for &action_idx in &alive {
                for _ in 0..n_a {
                    if total_sims >= n_simulations {
                        break;
                    }
                    let mut s = state.clone();
                    self.simulate(&mut root, &mut s, action_idx);
                    total_sims += 1;
                }
            }
            budget = budget.saturating_sub(alive.len() as u32 * n_a);

            // Score each surviving action and halve
            let mut scored: Vec<(usize, f64)> = alive
                .iter()
                .map(|&idx| {
                    let q = completed_q(&root.edges[idx], root_value);
                    let score = gumbels[idx] + logits[idx] + self.config.value_scale * q;
                    (idx, score)
                })
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let keep = alive.len().div_ceil(2);
            alive = scored[..keep].iter().map(|&(idx, _)| idx).collect();
        }

        // Spend remaining budget on the survivor(s), distributing remainder fairly
        if total_sims < n_simulations && !alive.is_empty() {
            let mut remaining = n_simulations - total_sims;
            for (i, &action_idx) in alive.iter().enumerate() {
                let actions_left = alive.len() as u32 - i as u32;
                let share = remaining / actions_left;
                for _ in 0..share {
                    let mut s = state.clone();
                    self.simulate(&mut root, &mut s, action_idx);
                    total_sims += 1;
                }
                remaining -= share;
            }
        }

        // Re-rank survivors after final simulations
        let best_idx = if alive.len() > 1 {
            *alive
                .iter()
                .max_by(|&&a, &&b| {
                    let sa = gumbels[a]
                        + logits[a]
                        + self.config.value_scale * completed_q(&root.edges[a], root_value);
                    let sb = gumbels[b]
                        + logits[b]
                        + self.config.value_scale * completed_q(&root.edges[b], root_value);
                    sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
        } else {
            alive[0]
        };

        // Improved policy: softmax(logit + value_scale * q_completed)
        let improved_scores: Vec<f64> = root
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| logits[i] + self.config.value_scale * completed_q(e, root_value))
            .collect();
        let improved_probs = softmax(&improved_scores);

        let move_stats: Vec<MoveStats<G::Move>> = root
            .edges
            .iter()
            .zip(improved_probs.iter())
            .map(|(e, &p)| MoveStats {
                mov: e.mov.clone(),
                visits: e.visits,
                completed_q: completed_q(e, root_value),
                improved_policy: p,
            })
            .collect();

        SearchResult {
            best_move: root.edges[best_idx].mov.clone(),
            root_value,
            move_stats,
            simulations_used: total_sims,
        }
    }

    /// Run a single simulation, forcing `forced_action` at the root.
    fn simulate(&self, root: &mut Node<G::Move>, state: &mut G, forced_action: usize) {
        let mov = root.edges[forced_action].mov.clone();
        state.make_move(&mov);

        let child_value = if root.edges[forced_action].child.is_some() {
            self.descend(
                root.edges[forced_action].child.as_mut().unwrap(),
                state,
                1,
            )
        } else {
            let (child_node, leaf_value) = self.expand(state);
            root.edges[forced_action].child = Some(Box::new(child_node));
            leaf_value
        };

        // Negamax: root player's value = -child's value
        root.edges[forced_action].value_sum += -child_value;
        root.edges[forced_action].visits += 1;
        root.visits += 1;
    }

    /// Descend the tree with PUCT selection. Returns value for the current player.
    fn descend(&self, node: &mut Node<G::Move>, state: &mut G, depth: usize) -> f64 {
        // Terminal node
        if node.edges.is_empty() {
            return terminal_value(state);
        }

        // Depth limit: evaluate in place
        if depth >= self.config.max_depth {
            let moves: Vec<G::Move> = state.available_moves().into_iter().collect();
            if moves.is_empty() {
                return terminal_value(state);
            }
            let (_, value) = self.evaluator.evaluate(state, &moves);
            return value;
        }

        // PUCT selection
        let action_idx = puct_select(node, self.config.c_puct);

        let mov = node.edges[action_idx].mov.clone();
        state.make_move(&mov);

        let child_value = if node.edges[action_idx].child.is_some() {
            self.descend(
                node.edges[action_idx].child.as_mut().unwrap(),
                state,
                depth + 1,
            )
        } else {
            let (child_node, leaf_value) = self.expand(state);
            node.edges[action_idx].child = Some(Box::new(child_node));
            leaf_value
        };

        // Negamax
        let my_value = -child_value;
        node.edges[action_idx].value_sum += my_value;
        node.edges[action_idx].visits += 1;
        node.visits += 1;

        my_value
    }

    /// Expand a leaf: evaluate the state and create a new node.
    fn expand(&self, state: &G) -> (Node<G::Move>, f64) {
        if let Some(pv) = state.terminal_value() {
            return (Node { edges: vec![], visits: 0 }, proven_to_value(pv));
        }

        let moves: Vec<G::Move> = state.available_moves().into_iter().collect();
        if moves.is_empty() {
            return (Node { edges: vec![], visits: 0 }, 0.0);
        }

        let (logits, value) = self.evaluator.evaluate(state, &moves);
        let priors = softmax(&logits);

        let node = Node {
            edges: moves
                .into_iter()
                .enumerate()
                .map(|(i, m)| Edge {
                    mov: m,
                    prior: priors[i],
                    visits: 0,
                    value_sum: 0.0,
                    child: None,
                })
                .collect(),
            visits: 0,
        };

        (node, value)
    }
}

// ============================================================
// Utility functions
// ============================================================

/// Sample from Gumbel(0, 1): -ln(-ln(U)), U ~ Uniform(0,1).
fn sample_gumbel(rng: &mut impl Rng) -> f64 {
    let u: f64 = rng.gen();
    let u = u.clamp(1e-20, 1.0 - 1e-20);
    -(-u.ln()).ln()
}

/// Numerically stable softmax. Returns uniform distribution for degenerate inputs.
fn softmax(logits: &[f64]) -> Vec<f64> {
    if logits.is_empty() {
        return vec![];
    }
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if !max.is_finite() {
        // All -inf or NaN: fall back to uniform
        let n = logits.len() as f64;
        return vec![1.0 / n; logits.len()];
    }
    let exps: Vec<f64> = logits.iter().map(|&x| (x - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    if sum == 0.0 {
        let n = logits.len() as f64;
        return vec![1.0 / n; logits.len()];
    }
    exps.iter().map(|&e| e / sum).collect()
}

/// PUCT action selection: argmax Q(a) + c * P(a) * sqrt(N) / (1 + n(a)).
fn puct_select<M: Clone>(node: &Node<M>, c: f64) -> usize {
    let sqrt_n = (node.visits as f64).sqrt();

    node.edges
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            let sa = puct_score(a, c, sqrt_n);
            let sb = puct_score(b, c, sqrt_n);
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn puct_score<M: Clone>(edge: &Edge<M>, c: f64, sqrt_parent_visits: f64) -> f64 {
    let q = if edge.visits > 0 {
        edge.value_sum / edge.visits as f64
    } else {
        0.0
    };
    let u = c * edge.prior * sqrt_parent_visits / (1.0 + edge.visits as f64);
    q + u
}

/// Completed Q-value: empirical mean if visited, otherwise falls back to the
/// parent node's value estimate (used as a surrogate for unvisited actions).
fn completed_q<M: Clone>(edge: &Edge<M>, default_value: f64) -> f64 {
    if edge.visits > 0 {
        edge.value_sum / edge.visits as f64
    } else {
        default_value
    }
}

/// Map a ProvenValue to a numeric value for the current player.
fn proven_to_value(pv: ProvenValue) -> f64 {
    match pv {
        ProvenValue::Win => 1.0,
        ProvenValue::Loss => -1.0,
        ProvenValue::Draw | ProvenValue::Unknown => 0.0,
    }
}

/// Terminal value for the current player.
fn terminal_value<G: GameState>(state: &G) -> f64 {
    state
        .terminal_value()
        .map(proven_to_value)
        .unwrap_or(0.0)
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_gumbel_mean() {
        // Gumbel(0,1) has mean = Euler-Mascheroni constant ~0.5772
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(123);
        let n = 50_000;
        let sum: f64 = (0..n).map(|_| sample_gumbel(&mut rng)).sum();
        let mean = sum / n as f64;
        assert!(
            (mean - 0.5772).abs() < 0.02,
            "Gumbel mean {mean} too far from 0.5772"
        );
    }

    #[test]
    fn test_softmax_sums_to_one() {
        let logits = vec![1.0, 2.0, 3.0, 4.0];
        let probs = softmax(&logits);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_softmax_ordering() {
        let logits = vec![1.0, 3.0, 2.0];
        let probs = softmax(&logits);
        assert!(probs[1] > probs[2]);
        assert!(probs[2] > probs[0]);
    }

    #[test]
    fn test_softmax_uniform() {
        let logits = vec![0.0, 0.0, 0.0];
        let probs = softmax(&logits);
        for &p in &probs {
            assert!((p - 1.0 / 3.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_softmax_empty() {
        assert!(softmax(&[]).is_empty());
    }

    #[test]
    fn test_softmax_single() {
        let probs = softmax(&[42.0]);
        assert_eq!(probs.len(), 1);
        assert!((probs[0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_softmax_extreme_large_logits() {
        let logits = vec![1000.0, 1001.0, 999.0];
        let probs = softmax(&logits);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10, "sum = {sum}");
        assert!(probs[1] > probs[0]);
    }

    #[test]
    fn test_softmax_extreme_negative_logits() {
        let logits = vec![-1000.0, -1001.0, -999.0];
        let probs = softmax(&logits);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10, "sum = {sum}");
        assert!(probs[2] > probs[0]);
    }

    #[test]
    fn test_softmax_all_neg_infinity_returns_uniform() {
        let logits = vec![f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        let probs = softmax(&logits);
        for &p in &probs {
            assert!((p - 1.0 / 3.0).abs() < 1e-10, "should be uniform, got {p}");
        }
    }

    #[test]
    fn test_softmax_nan_returns_uniform() {
        let logits = vec![f64::NAN, f64::NAN];
        let probs = softmax(&logits);
        for &p in &probs {
            assert!((p - 0.5).abs() < 1e-10, "NaN logits should produce uniform");
        }
    }

    #[test]
    fn test_puct_prefers_high_prior_initially() {
        let node = Node {
            edges: vec![
                Edge { mov: 0u32, prior: 0.1, visits: 0, value_sum: 0.0, child: None },
                Edge { mov: 1, prior: 0.9, visits: 0, value_sum: 0.0, child: None },
            ],
            visits: 1,
        };
        let selected = puct_select(&node, 1.25);
        assert_eq!(selected, 1);
    }

    #[test]
    fn test_puct_prefers_high_value_after_visits() {
        let node = Node {
            edges: vec![
                Edge { mov: 0u32, prior: 0.5, visits: 10, value_sum: 8.0, child: None },
                Edge { mov: 1, prior: 0.5, visits: 10, value_sum: 2.0, child: None },
            ],
            visits: 20,
        };
        let selected = puct_select(&node, 1.25);
        assert_eq!(selected, 0);
    }

    #[test]
    fn test_puct_zero_priors_degenerates_to_exploitation() {
        let node = Node {
            edges: vec![
                Edge { mov: 0u32, prior: 0.0, visits: 5, value_sum: 3.0, child: None },
                Edge { mov: 1, prior: 0.0, visits: 5, value_sum: 1.0, child: None },
            ],
            visits: 10,
        };
        // Zero priors: exploration term is 0, should pick highest Q (action 0: Q=0.6)
        let selected = puct_select(&node, 1.25);
        assert_eq!(selected, 0);
    }

    #[test]
    fn test_completed_q_visited() {
        let edge = Edge { mov: 0u32, prior: 0.5, visits: 4, value_sum: 2.0, child: None };
        assert!((completed_q(&edge, 0.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_completed_q_unvisited() {
        let edge = Edge { mov: 0u32, prior: 0.5, visits: 0, value_sum: 0.0, child: None };
        assert!((completed_q(&edge, 0.7) - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_completed_q_negative() {
        let edge = Edge { mov: 0u32, prior: 0.5, visits: 4, value_sum: -2.0, child: None };
        assert!((completed_q(&edge, 0.0) - (-0.5)).abs() < 1e-10);
    }

    #[test]
    fn test_proven_to_value() {
        assert!((proven_to_value(ProvenValue::Win) - 1.0).abs() < 1e-10);
        assert!((proven_to_value(ProvenValue::Loss) - (-1.0)).abs() < 1e-10);
        assert!((proven_to_value(ProvenValue::Draw) - 0.0).abs() < 1e-10);
        assert!((proven_to_value(ProvenValue::Unknown) - 0.0).abs() < 1e-10);
    }
}
