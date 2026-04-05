use mcts::ProvenValue;

/// Game state interface that host languages implement.
///
/// This is the dynamic equivalent of the core `GameState` trait.
/// All methods use strings for moves and integers for players,
/// making them easy to implement from any language.
pub trait GameCallbacks: Send + Sync {
    /// Create a deep clone of this game state.
    /// Called once per playout (the root state is cloned before each playout).
    fn clone_box(&self) -> Box<dyn GameCallbacks>;

    /// The player whose turn it is. Use distinct integers for each player
    /// (e.g., 0 for single-player, 1/2 for two-player).
    fn current_player(&self) -> i32;

    /// Legal moves from this state. Return an empty vec for terminal states.
    /// For deterministic results, always return moves in the same order.
    fn available_moves(&self) -> Vec<String>;

    /// Apply a move, mutating the state in place.
    fn make_move(&mut self, mov: &str);

    /// Classify a terminal state for MCTS-Solver.
    /// Return `Some(ProvenValue::Win)` if the current player has won,
    /// `Some(ProvenValue::Loss)` if lost, `Some(ProvenValue::Draw)` for draw.
    /// Default: `None` (solver treats terminals as Unknown).
    fn terminal_value(&self) -> Option<ProvenValue> {
        None
    }

    /// Exact minimax score at a terminal state (from current player's perspective).
    /// Used by Score-Bounded MCTS.
    fn terminal_score(&self) -> Option<i32> {
        None
    }

    /// Chance outcomes with probabilities (must sum to 1.0).
    /// Return `None` for deterministic transitions (default).
    fn chance_outcomes(&self) -> Option<Vec<(String, f64)>> {
        None
    }

    /// Maximum children to expand at this visit count (for progressive widening).
    fn max_children(&self, _visits: u64) -> usize {
        usize::MAX
    }
}

/// Evaluator interface that host languages implement.
///
/// The evaluator scores game states and optionally provides per-move priors.
pub trait EvalCallbacks: Send + Sync {
    /// Evaluate a game state. Returns `(per-move priors, state value)`.
    ///
    /// - `moves` is the list of available moves (same order as `available_moves()`).
    /// - Priors should be non-negative and sum to approximately 1.0.
    /// - Return an empty priors vec to use uniform priors (1.0/N).
    /// - State value should be from the current player's perspective
    ///   (positive = good for current player).
    fn evaluate(&self, state: &dyn GameCallbacks, moves: &[String]) -> (Vec<f64>, f64);

    /// Convert a state value to a reward for a given player.
    /// Default: negate for opponent (zero-sum two-player games).
    fn interpret_for_player(
        &self,
        value: f64,
        evaluating_player: i32,
        requesting_player: i32,
    ) -> f64 {
        if evaluating_player == requesting_player {
            value
        } else {
            -value
        }
    }
}
