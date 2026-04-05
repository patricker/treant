use std::sync::Arc;
use std::time::Duration;

use mcts::tree_policy::AlphaGoPolicy;
use mcts::{MCTSManager, ProvenValue, ScoreBounds};

use crate::adapter::{DynEvaluator, DynGameState, DynSpec};
use crate::callbacks::{EvalCallbacks, GameCallbacks};
use crate::types::{DynChildStats, DynConfig, DynTreeEdge, DynTreeNode};

/// Runtime-polymorphic MCTS manager.
///
/// Wraps the core `MCTSManager<DynSpec>` with a string-based API
/// that can be driven from any language via the `GameCallbacks` and
/// `EvalCallbacks` traits.
pub struct DynMCTSManager {
    inner: MCTSManager<DynSpec>,
}

impl DynMCTSManager {
    /// Create a new MCTS manager.
    ///
    /// - `state`: initial game state (will be cloned each playout)
    /// - `eval`: evaluator for scoring states
    /// - `config`: search configuration
    pub fn new(
        state: Box<dyn GameCallbacks>,
        eval: Box<dyn EvalCallbacks>,
        config: DynConfig,
    ) -> Self {
        let policy = AlphaGoPolicy::new(config.exploration_constant);
        let spec = DynSpec { config };
        let game_state = DynGameState(state);
        let evaluator = DynEvaluator(Arc::from(eval));
        let inner = MCTSManager::new(game_state, spec, evaluator, policy, ());
        Self { inner }
    }

    /// Run a single playout.
    pub fn playout(&mut self) {
        self.inner.playout();
    }

    /// Run `n` playouts sequentially.
    pub fn playout_n(&mut self, n: u64) {
        self.inner.playout_n(n);
    }

    /// Run `n` playouts across `num_threads` threads.
    pub fn playout_n_parallel(&mut self, n: u32, num_threads: usize) {
        self.inner.playout_n_parallel(n, num_threads);
    }

    /// Run parallel search for the given duration.
    pub fn playout_parallel_for(&mut self, duration: Duration, num_threads: usize) {
        self.inner.playout_parallel_for(duration, num_threads);
    }

    /// The best move found by search.
    pub fn best_move(&self) -> Option<String> {
        self.inner.best_move().map(|m| m.0)
    }

    /// The best sequence of moves found by search.
    pub fn principal_variation(&self, depth: usize) -> Vec<String> {
        self.inner
            .principal_variation(depth)
            .into_iter()
            .map(|m| m.0)
            .collect()
    }

    /// Statistics for all root children.
    pub fn root_child_stats(&self) -> Vec<DynChildStats> {
        self.inner
            .root_child_stats()
            .into_iter()
            .map(|cs| DynChildStats {
                mov: cs.mov.0,
                visits: cs.visits,
                avg_reward: cs.avg_reward,
                prior: cs.move_evaluation,
                proven_value: cs.proven_value,
                score_bounds: cs.score_bounds,
            })
            .collect()
    }

    /// Proven value of the root (for MCTS-Solver).
    pub fn root_proven_value(&self) -> ProvenValue {
        self.inner.root_proven_value()
    }

    /// Score bounds of the root (for Score-Bounded MCTS).
    pub fn root_score_bounds(&self) -> ScoreBounds {
        self.inner.root_score_bounds()
    }

    /// Number of nodes in the search tree.
    pub fn num_nodes(&self) -> usize {
        self.inner.tree().num_nodes()
    }

    /// Advance the root to a child, preserving the subtree.
    pub fn advance(&mut self, mov: &str) -> Result<(), mcts::AdvanceError> {
        use crate::types::DynMove;
        self.inner.advance(&DynMove(mov.to_string()))
    }

    /// Reset the search tree, keeping the game state and config.
    pub fn reset(self) -> Self {
        Self {
            inner: self.inner.reset(),
        }
    }

    /// Snapshot the search tree for visualization.
    pub fn tree_snapshot(&self, max_depth: u32) -> DynTreeNode {
        let root = self.inner.tree().root_node();
        export_node(&root, max_depth)
    }
}

fn export_node(node: &mcts::NodeHandle<'_, DynSpec>, max_depth: u32) -> DynTreeNode {
    let mut children = Vec::new();
    if max_depth > 0 {
        for mi in node.moves() {
            let child_node = if max_depth > 1 {
                mi.child().map(|c| export_node(&c, max_depth - 1))
            } else {
                None
            };
            let visits = mi.visits();
            children.push(DynTreeEdge {
                mov: mi.get_move().0.clone(),
                visits,
                avg_reward: if visits > 0 {
                    mi.sum_rewards() as f64 / visits as f64
                } else {
                    0.0
                },
                prior: *mi.move_evaluation(),
                child: child_node,
            });
        }
    }

    let root_visits = node.moves().map(|m| m.visits()).sum::<u64>();
    let root_rewards: i64 = node.moves().map(|m| m.sum_rewards()).sum();

    DynTreeNode {
        visits: root_visits,
        avg_reward: if root_visits > 0 {
            root_rewards as f64 / root_visits as f64
        } else {
            0.0
        },
        proven: node.proven_value(),
        children,
    }
}
