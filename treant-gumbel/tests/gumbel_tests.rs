use treant::{GameState, ProvenValue};
use treant_gumbel::{GumbelConfig, GumbelEvaluator, GumbelSearch};

// ============================================================
// Test game: Nim (take 1 or 2, last to take wins)
// ============================================================

#[derive(Clone, Debug)]
struct NimGame {
    stones: u32,
    current: u8,
}

#[derive(Clone, Debug, PartialEq)]
struct NimMove(u32);

impl GameState for NimGame {
    type Move = NimMove;
    type Player = u8;
    type MoveList = Vec<NimMove>;

    fn current_player(&self) -> u8 {
        self.current
    }

    fn available_moves(&self) -> Vec<NimMove> {
        if self.stones == 0 {
            return vec![];
        }
        let mut moves = vec![NimMove(1)];
        if self.stones >= 2 {
            moves.push(NimMove(2));
        }
        moves
    }

    fn make_move(&mut self, m: &NimMove) {
        self.stones -= m.0;
        self.current = 1 - self.current;
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.stones == 0 {
            // Previous player took last stone and won; current player lost
            Some(ProvenValue::Loss)
        } else {
            None
        }
    }
}

/// Nim heuristic: stones % 3 == 0 is losing for current player.
struct NimEvaluator;

impl GumbelEvaluator<NimGame> for NimEvaluator {
    fn evaluate(&self, state: &NimGame, moves: &[NimMove]) -> (Vec<f64>, f64) {
        let logits = vec![0.0; moves.len()]; // uniform policy
        let value = if state.stones.is_multiple_of(3) {
            -0.5
        } else {
            0.5
        };
        (logits, value)
    }
}

// ============================================================
// Test game: WideGame (N legal moves, exercises Sequential Halving)
// ============================================================

/// A game with configurable width. Move 0 is best (leads to opponent losing).
#[derive(Clone, Debug)]
struct WideGame {
    width: u32,
    depth: u32,
    current: u8,
}

impl GameState for WideGame {
    type Move = u32;
    type Player = u8;
    type MoveList = Vec<u32>;

    fn current_player(&self) -> u8 {
        self.current
    }

    fn available_moves(&self) -> Vec<u32> {
        if self.depth == 0 {
            vec![]
        } else {
            (0..self.width).collect()
        }
    }

    fn make_move(&mut self, _m: &u32) {
        self.depth -= 1;
        self.current = 1 - self.current;
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.depth == 0 {
            Some(ProvenValue::Loss) // current player lost
        } else {
            None
        }
    }
}

/// Evaluator that gives move 0 a slight advantage via value estimation.
struct WideEvaluator;

impl GumbelEvaluator<WideGame> for WideEvaluator {
    fn evaluate(&self, state: &WideGame, moves: &[u32]) -> (Vec<f64>, f64) {
        let logits = vec![0.0; moves.len()]; // uniform priors
                                             // Odd depth = current player wins (depth reduces by 1 each move)
        let value = if state.depth % 2 == 1 { 0.3 } else { -0.3 };
        (logits, value)
    }
}

// ============================================================
// Nim tests
// ============================================================

/// From N=4, taking 1 leaves opponent in N=3 (losing). Optimal move is Take(1).
#[test]
fn nim_finds_optimal_move_from_4() {
    let state = NimGame {
        stones: 4,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 4,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let result = search.search(&state, 200);
    assert_eq!(result.best_move, NimMove(1));
    assert!(
        result.root_value > 0.0,
        "winning position should have positive value"
    );
}

/// From N=3, both moves lose. Search should still return a valid move.
#[test]
fn nim_returns_move_from_losing_position() {
    let state = NimGame {
        stones: 3,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 4,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let result = search.search(&state, 100);
    assert!(result.best_move == NimMove(1) || result.best_move == NimMove(2));
}

/// From N=7, take 1 → N=6 (6%3=0, losing for opponent). Optimal.
#[test]
fn nim_finds_optimal_move_from_7() {
    let state = NimGame {
        stones: 7,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 4,
        seed: 99,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let result = search.search(&state, 300);
    assert_eq!(result.best_move, NimMove(1));
}

/// Nim with N=10, deeper tree exercises PUCT below root.
#[test]
fn nim_deeper_game() {
    let state = NimGame {
        stones: 10,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 4,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let result = search.search(&state, 500);
    assert_eq!(result.best_move, NimMove(1));
}

// ============================================================
// WideGame tests (Sequential Halving with 8 moves)
// ============================================================

/// With 8 legal moves, Sequential Halving runs 3 phases (8→4→2→1).
#[test]
fn wide_game_sequential_halving() {
    let state = WideGame {
        width: 8,
        depth: 4,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 8,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(WideEvaluator, config);
    let result = search.search(&state, 200);

    // Should return a valid move
    assert!(result.best_move < 8);
    assert!(result.simulations_used > 0);
    assert_eq!(result.move_stats.len(), 8);

    // Total visits should equal simulations used
    let total_visits: u32 = result.move_stats.iter().map(|s| s.visits).sum();
    assert_eq!(total_visits, result.simulations_used);
}

/// With m_actions=16 and only 4 moves, clamps to 4 and halves correctly.
#[test]
fn wide_game_m_larger_than_moves() {
    let state = WideGame {
        width: 4,
        depth: 3,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 16,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(WideEvaluator, config);
    let result = search.search(&state, 100);
    assert!(result.best_move < 4);
    assert_eq!(result.move_stats.len(), 4);
}

// ============================================================
// Edge cases
// ============================================================

#[test]
fn single_move_returns_immediately_with_value() {
    let state = NimGame {
        stones: 1,
        current: 0,
    };
    let mut search = GumbelSearch::new(NimEvaluator, GumbelConfig::default());
    let result = search.search(&state, 1000);
    assert_eq!(result.best_move, NimMove(1));
    assert_eq!(result.simulations_used, 0);
    assert_eq!(result.move_stats.len(), 1);
    assert!((result.move_stats[0].improved_policy - 1.0).abs() < 1e-10);
}

#[test]
fn zero_simulations() {
    let state = NimGame {
        stones: 4,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 2,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let result = search.search(&state, 0);
    assert_eq!(result.simulations_used, 0);
    assert!(result.best_move == NimMove(1) || result.best_move == NimMove(2));
}

#[test]
fn budget_not_exceeded() {
    let state = NimGame {
        stones: 4,
        current: 0,
    };
    for budget in [1, 2, 3, 5, 10, 50] {
        let config = GumbelConfig {
            m_actions: 4,
            seed: 42,
            ..Default::default()
        };
        let mut search = GumbelSearch::new(NimEvaluator, config);
        let result = search.search(&state, budget);
        assert!(
            result.simulations_used <= budget,
            "budget={budget}, used={}",
            result.simulations_used
        );
    }
}

/// Budget should not be exceeded even with many actions.
#[test]
fn budget_not_exceeded_wide() {
    let state = WideGame {
        width: 8,
        depth: 4,
        current: 0,
    };
    for budget in [1, 3, 7, 15, 31, 64] {
        let config = GumbelConfig {
            m_actions: 8,
            seed: 42,
            ..Default::default()
        };
        let mut search = GumbelSearch::new(WideEvaluator, config);
        let result = search.search(&state, budget);
        assert!(
            result.simulations_used <= budget,
            "budget={budget}, used={}",
            result.simulations_used
        );
    }
}

#[test]
#[should_panic(expected = "cannot search from terminal state")]
fn panics_on_terminal_state() {
    let state = NimGame {
        stones: 0,
        current: 0,
    };
    let mut search = GumbelSearch::new(NimEvaluator, GumbelConfig::default());
    let _ = search.search(&state, 100);
}

/// m_actions=1 degenerates to a single sampled action.
#[test]
fn m_actions_one() {
    let state = NimGame {
        stones: 4,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 1,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let result = search.search(&state, 50);
    assert!(result.best_move == NimMove(1) || result.best_move == NimMove(2));
    assert!(result.simulations_used > 0);
}

// ============================================================
// Policy and Q-value tests
// ============================================================

#[test]
fn improved_policy_is_valid_distribution() {
    let state = NimGame {
        stones: 5,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 4,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let result = search.search(&state, 200);

    let sum: f64 = result.move_stats.iter().map(|s| s.improved_policy).sum();
    assert!(
        (sum - 1.0).abs() < 1e-10,
        "improved policy should sum to 1.0, got {sum}"
    );
    for s in &result.move_stats {
        assert!(
            s.improved_policy >= 0.0,
            "policy probability should be non-negative"
        );
    }
}

#[test]
fn completed_q_values_populated() {
    let state = NimGame {
        stones: 4,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 4,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let result = search.search(&state, 200);

    assert_eq!(result.move_stats.len(), 2);
    let q_take1 = result
        .move_stats
        .iter()
        .find(|s| s.mov == NimMove(1))
        .unwrap()
        .completed_q;
    let q_take2 = result
        .move_stats
        .iter()
        .find(|s| s.mov == NimMove(2))
        .unwrap()
        .completed_q;
    assert!(
        q_take1 > q_take2,
        "Take(1) Q={q_take1} should be > Take(2) Q={q_take2}"
    );
}

#[test]
fn visit_counts_reflect_budget() {
    let state = NimGame {
        stones: 4,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 2,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let result = search.search(&state, 100);

    let total_visits: u32 = result.move_stats.iter().map(|s| s.visits).sum();
    assert_eq!(total_visits, result.simulations_used);
    assert!(result.simulations_used > 0);
}

// ============================================================
// Determinism and seed tests
// ============================================================

#[test]
fn deterministic_with_same_seed() {
    let state = NimGame {
        stones: 5,
        current: 0,
    };
    let config = GumbelConfig {
        seed: 777,
        m_actions: 4,
        ..Default::default()
    };

    let mut search1 = GumbelSearch::new(NimEvaluator, config);
    let mut search2 = GumbelSearch::new(NimEvaluator, config);

    let r1 = search1.search(&state, 100);
    let r2 = search2.search(&state, 100);

    assert_eq!(r1.best_move, r2.best_move);
    assert_eq!(r1.simulations_used, r2.simulations_used);
    assert!((r1.root_value - r2.root_value).abs() < 1e-10);
}

#[test]
fn set_seed_reproducible() {
    let state = NimGame {
        stones: 5,
        current: 0,
    };
    let config = GumbelConfig {
        seed: 42,
        m_actions: 2,
        value_scale: 1.0,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let r1 = search.search(&state, 30);

    search.set_seed(42);
    let r2 = search.search(&state, 30);

    assert_eq!(r1.best_move, r2.best_move);
    assert_eq!(r1.simulations_used, r2.simulations_used);
}

#[test]
fn different_seeds_produce_different_results() {
    // Symmetric game (N=3, losing position) with low value_scale so Gumbel dominates.
    let state = NimGame {
        stones: 3,
        current: 0,
    };
    let mut best_moves = Vec::new();
    for seed in 0..50 {
        let config = GumbelConfig {
            seed,
            m_actions: 2,
            value_scale: 1.0,
            ..Default::default()
        };
        let mut search = GumbelSearch::new(NimEvaluator, config);
        let result = search.search(&state, 20);
        best_moves.push(result.best_move.clone());
    }
    let take1_count = best_moves.iter().filter(|m| **m == NimMove(1)).count();
    let take2_count = best_moves.iter().filter(|m| **m == NimMove(2)).count();
    assert!(
        take1_count > 0 && take2_count > 0,
        "Different seeds should pick different moves. Take(1)={take1_count}, Take(2)={take2_count}"
    );
}

// ============================================================
// Non-uniform prior test
// ============================================================

/// Evaluator with strong (misleading) prior on the wrong move.
/// Search should still find the right move with enough simulations.
#[test]
fn search_overcomes_misleading_prior() {
    struct MisleadingEval;
    impl GumbelEvaluator<NimGame> for MisleadingEval {
        fn evaluate(&self, state: &NimGame, moves: &[NimMove]) -> (Vec<f64>, f64) {
            // Strong prior on Take(2), which is the wrong move from N=4
            let logits: Vec<f64> = moves
                .iter()
                .map(|m| if m.0 == 2 { 5.0 } else { -5.0 })
                .collect();
            let value = if state.stones.is_multiple_of(3) {
                -0.5
            } else {
                0.5
            };
            (logits, value)
        }
    }

    let state = NimGame {
        stones: 4,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 2,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(MisleadingEval, config);
    let result = search.search(&state, 500);
    assert_eq!(
        result.best_move,
        NimMove(1),
        "Search should overcome misleading prior with enough simulations"
    );
}

// ============================================================
// max_depth cutoff test
// ============================================================

#[test]
fn max_depth_triggers_evaluation() {
    let state = NimGame {
        stones: 10,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 2,
        max_depth: 2, // Very shallow — forces depth cutoff
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);
    let result = search.search(&state, 200);
    // Should still return a valid move without crashing
    assert!(result.best_move == NimMove(1) || result.best_move == NimMove(2));
    assert!(result.simulations_used > 0);
}

// ============================================================
// Debug trait test
// ============================================================

#[test]
fn debug_impls_work() {
    let state = NimGame {
        stones: 4,
        current: 0,
    };
    let config = GumbelConfig {
        m_actions: 2,
        seed: 42,
        ..Default::default()
    };
    let mut search = GumbelSearch::new(NimEvaluator, config);

    // GumbelSearch: Debug
    let debug = format!("{:?}", search);
    assert!(debug.contains("GumbelSearch"));

    // SearchResult: Debug
    let result = search.search(&state, 50);
    let debug = format!("{:?}", result);
    assert!(debug.contains("SearchResult"));
    assert!(debug.contains("best_move"));

    // GumbelConfig: Copy
    let config2 = config;
    assert_eq!(config2.seed, 42);
}
