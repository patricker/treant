use mcts::{ProvenValue, ScoreBounds};
use std::fmt;

/// A type-erased move represented as a string.
/// Every host language can produce and consume strings natively.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DynMove(pub String);

impl fmt::Display for DynMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Runtime MCTS configuration. All fields map to methods on the core `MCTS` trait.
#[derive(Clone, Debug)]
pub struct DynConfig {
    /// PUCT exploration constant C (default 1.41).
    pub exploration_constant: f64,
    /// Enable MCTS-Solver (proven win/loss/draw).
    pub solver_enabled: bool,
    /// Enable Score-Bounded MCTS (minimax bounds).
    pub score_bounded_enabled: bool,
    /// Virtual loss for parallel search.
    pub virtual_loss: i64,
    /// Maximum tree nodes.
    pub node_limit: usize,
    /// First-play urgency value for unvisited children.
    pub fpu_value: f64,
    /// Temperature for post-search move selection.
    pub selection_temperature: f64,
    /// Dirichlet noise (epsilon, alpha) for root exploration.
    pub dirichlet_noise: Option<(f64, f64)>,
    /// RNG seed for deterministic search.
    pub rng_seed: Option<u64>,
    /// Safety cap on playout path length.
    pub max_playout_length: usize,
    /// Quality knob: forces leaf eval if exceeded.
    pub max_playout_depth: usize,
    /// Closed-loop chance nodes (vs open-loop).
    pub closed_loop_chance: bool,
}

impl Default for DynConfig {
    fn default() -> Self {
        Self {
            exploration_constant: 1.41,
            solver_enabled: false,
            score_bounded_enabled: false,
            virtual_loss: 0,
            node_limit: usize::MAX,
            fpu_value: f64::MAX,
            selection_temperature: 0.0,
            dirichlet_noise: None,
            rng_seed: None,
            max_playout_length: 1_000_000,
            max_playout_depth: usize::MAX,
            closed_loop_chance: false,
        }
    }
}

/// Summary statistics for a root child.
#[derive(Clone, Debug)]
pub struct DynChildStats {
    pub mov: String,
    pub visits: u64,
    pub avg_reward: f64,
    pub prior: f64,
    pub proven_value: ProvenValue,
    pub score_bounds: ScoreBounds,
}

/// Snapshot of a tree node for visualization.
#[derive(Clone, Debug)]
pub struct DynTreeNode {
    pub visits: u64,
    pub avg_reward: f64,
    pub proven: ProvenValue,
    pub children: Vec<DynTreeEdge>,
}

/// Snapshot of a tree edge (move + child).
#[derive(Clone, Debug)]
pub struct DynTreeEdge {
    pub mov: String,
    pub visits: u64,
    pub avg_reward: f64,
    pub prior: f64,
    pub child: Option<DynTreeNode>,
}
