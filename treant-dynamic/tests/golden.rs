use treant::ProvenValue;
use treant_dynamic::*;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Canonical test games — these must be implemented identically in every language
// ---------------------------------------------------------------------------

/// CountingGame: single-player, counter from 0 to target.
/// Moves: "Add" (+1), "Sub" (counter = max(counter-1, 0)).
/// Terminal when counter == target. Terminal is a Win.
/// The floor at 0 ensures random rollouts always terminate
/// (reflecting barrier at 0, absorbing barrier at target).
#[derive(Clone)]
struct CountingGame {
    counter: i64,
    target: i64,
}

impl GameCallbacks for CountingGame {
    fn clone_box(&self) -> Box<dyn GameCallbacks> {
        Box::new(self.clone())
    }

    fn current_player(&self) -> i32 {
        0 // single player
    }

    fn available_moves(&self) -> Vec<String> {
        if self.counter == self.target {
            vec![]
        } else {
            vec!["Add".to_string(), "Sub".to_string()]
        }
    }

    fn make_move(&mut self, mov: &str) {
        match mov {
            "Add" => self.counter += 1,
            "Sub" => self.counter = (self.counter - 1).max(0),
            _ => panic!("Unknown move: {}", mov),
        }
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.counter == self.target {
            Some(ProvenValue::Win)
        } else {
            None
        }
    }
}

/// Evaluator for CountingGame: biases priors toward "Add" (closer to target).
/// Returns prior 0.9 for Add, 0.1 for Sub.
struct CountingPriorEval;

impl EvalCallbacks for CountingPriorEval {
    fn evaluate(&self, _state: &dyn GameCallbacks, moves: &[String]) -> (Vec<f64>, f64) {
        let priors: Vec<f64> = moves
            .iter()
            .map(|m| if m == "Add" { 0.9 } else { 0.1 })
            .collect();
        (priors, 0.0)
    }

    fn interpret_for_player(
        &self,
        value: f64,
        _evaluating_player: i32,
        _requesting_player: i32,
    ) -> f64 {
        value // single player
    }
}

/// TinyNim: two-player, pile of stones.
/// Players alternate removing 1 or 2 stones.
/// The player who takes the last stone(s) wins.
/// Game-theoretic solution: position is losing iff stones % 3 == 0.
#[derive(Clone)]
struct TinyNim {
    stones: u8,
    current_player: i32, // 1 = P1, 2 = P2
}

impl GameCallbacks for TinyNim {
    fn clone_box(&self) -> Box<dyn GameCallbacks> {
        Box::new(self.clone())
    }

    fn current_player(&self) -> i32 {
        self.current_player
    }

    fn available_moves(&self) -> Vec<String> {
        if self.stones == 0 {
            vec![]
        } else if self.stones == 1 {
            vec!["Take1".to_string()]
        } else {
            vec!["Take1".to_string(), "Take2".to_string()]
        }
    }

    fn make_move(&mut self, mov: &str) {
        match mov {
            "Take1" => self.stones -= 1,
            "Take2" => self.stones -= 2,
            _ => panic!("Unknown move: {}", mov),
        }
        self.current_player = if self.current_player == 1 { 2 } else { 1 };
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.stones == 0 {
            // Previous player took the last stone and won.
            // Current player (who would move next) has lost.
            Some(ProvenValue::Loss)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Golden test runner
// ---------------------------------------------------------------------------

fn load_golden_tests() -> Vec<Value> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/golden/golden_tests.json"
    );
    let content = std::fs::read_to_string(path).expect("Failed to read golden_tests.json");
    serde_json::from_str(&content).expect("Failed to parse golden_tests.json")
}

fn make_game(test: &Value) -> Box<dyn GameCallbacks> {
    let game_type = test["game"].as_str().unwrap();
    match game_type {
        "counting" => {
            let target = test["state"]["target"].as_i64().unwrap_or(100);
            Box::new(CountingGame { counter: 0, target })
        }
        "nim" => {
            let stones = test["state"]["stones"].as_u64().unwrap() as u8;
            Box::new(TinyNim {
                stones,
                current_player: 1,
            })
        }
        _ => panic!("Unknown game type: {}", game_type),
    }
}

fn make_eval(test: &Value) -> Box<dyn EvalCallbacks> {
    let game_type = test["game"].as_str().unwrap();
    let seed = test["config"]
        .get("seed")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    match game_type {
        "counting" => Box::new(CountingPriorEval),
        "nim" => Box::new(RandomRollout::with_seed(seed + 1000)),
        _ => panic!("Unknown game type: {}", game_type),
    }
}

fn make_config(test: &Value) -> DynConfig {
    let cfg = &test["config"];
    let mut config = DynConfig::default();

    if let Some(c) = cfg.get("exploration_constant").and_then(|v| v.as_f64()) {
        config.exploration_constant = c;
    }
    if let Some(seed) = cfg.get("seed").and_then(|v| v.as_u64()) {
        config.rng_seed = Some(seed);
    }
    if let Some(solver) = cfg.get("solver").and_then(|v| v.as_bool()) {
        config.solver_enabled = solver;
    }

    config
}

fn check_expectations(test: &Value, mgr: &DynMCTSManager) {
    let name = test["name"].as_str().unwrap();
    let expect = &test["expect"];

    if let Some(expected_move) = expect.get("best_move").and_then(|v| v.as_str()) {
        let actual = mgr.best_move();
        assert_eq!(
            actual.as_deref(),
            Some(expected_move),
            "[{}] best_move mismatch",
            name
        );
    }

    if let Some(expected_pv) = expect.get("proven_value").and_then(|v| v.as_str()) {
        let actual = mgr.root_proven_value();
        let actual_str = match actual {
            ProvenValue::Win => "Win",
            ProvenValue::Loss => "Loss",
            ProvenValue::Draw => "Draw",
            ProvenValue::Unknown => "Unknown",
        };
        assert_eq!(actual_str, expected_pv, "[{}] proven_value mismatch", name);
    }

    if let Some(expected_stats) = expect.get("child_stats").and_then(|v| v.as_array()) {
        let stats = mgr.root_child_stats();
        for es in expected_stats {
            let mov = es["mov"].as_str().unwrap();
            let cs = stats
                .iter()
                .find(|s| s.mov == mov)
                .unwrap_or_else(|| panic!("[{}] move '{}' not found in child_stats", name, mov));

            if let Some(gte) = es.get("visits_gte").and_then(|v| v.as_u64()) {
                assert!(
                    cs.visits >= gte,
                    "[{}] move '{}': visits {} < expected >= {}",
                    name,
                    mov,
                    cs.visits,
                    gte
                );
            }
            if let Some(lte) = es.get("visits_lte").and_then(|v| v.as_u64()) {
                assert!(
                    cs.visits <= lte,
                    "[{}] move '{}': visits {} > expected <= {}",
                    name,
                    mov,
                    cs.visits,
                    lte
                );
            }
            if let Some(proven) = es.get("proven").and_then(|v| v.as_str()) {
                let actual_str = match cs.proven_value {
                    ProvenValue::Win => "Win",
                    ProvenValue::Loss => "Loss",
                    ProvenValue::Draw => "Draw",
                    ProvenValue::Unknown => "Unknown",
                };
                assert_eq!(
                    actual_str, proven,
                    "[{}] move '{}': proven_value mismatch",
                    name, mov
                );
            }
        }
    }

    if let Some(gte) = expect.get("num_nodes_gte").and_then(|v| v.as_u64()) {
        let actual = mgr.num_nodes() as u64;
        assert!(
            actual >= gte,
            "[{}] num_nodes {} < expected >= {}",
            name,
            actual,
            gte
        );
    }
}

#[test]
fn golden_tests() {
    let tests = load_golden_tests();
    for test in &tests {
        let name = test["name"].as_str().unwrap();
        let playouts = test["config"]["playouts"].as_u64().unwrap();

        let game = make_game(test);
        let eval = make_eval(test);
        let config = make_config(test);

        let mut mgr = DynMCTSManager::new(game, eval, config);
        mgr.playout_n(playouts);

        check_expectations(test, &mgr);
        eprintln!("  PASS: {}", name);
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[test]
fn counting_game_basic() {
    let game = Box::new(CountingGame {
        counter: 0,
        target: 10,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(CountingPriorEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(500);

    assert_eq!(mgr.best_move(), Some("Add".to_string()));
    assert!(mgr.num_nodes() > 0);

    let stats = mgr.root_child_stats();
    assert_eq!(stats.len(), 2);
    let add_stats = stats.iter().find(|s| s.mov == "Add").unwrap();
    let sub_stats = stats.iter().find(|s| s.mov == "Sub").unwrap();
    assert!(
        add_stats.visits > sub_stats.visits,
        "Add visits ({}) should exceed Sub visits ({})",
        add_stats.visits,
        sub_stats.visits
    );
}

#[test]
fn nim_solver() {
    let game = Box::new(TinyNim {
        stones: 5,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(RandomRollout::with_seed(123));
    let config = DynConfig {
        exploration_constant: 0.5,
        solver_enabled: true,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(10000);

    assert_eq!(mgr.root_proven_value(), ProvenValue::Win);
    assert_eq!(mgr.best_move(), Some("Take2".to_string()));
}

#[test]
fn nim_losing_position() {
    let game = Box::new(TinyNim {
        stones: 3,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(RandomRollout::with_seed(123));
    let config = DynConfig {
        exploration_constant: 0.5,
        solver_enabled: true,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(10000);

    assert_eq!(mgr.root_proven_value(), ProvenValue::Loss);
}

#[test]
fn tree_advance() {
    let game = Box::new(TinyNim {
        stones: 5,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(RandomRollout::with_seed(123));
    let config = DynConfig {
        exploration_constant: 1.41,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(1000);

    let nodes_before = mgr.num_nodes();
    assert!(nodes_before > 0);

    mgr.advance("Take2").unwrap();
    let nodes_after = mgr.num_nodes();
    assert!(nodes_after <= nodes_before);

    mgr.playout_n(100);
    assert!(mgr.num_nodes() > 0);
}

#[test]
fn tree_snapshot() {
    let game = Box::new(CountingGame {
        counter: 0,
        target: 10,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(CountingPriorEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(200);

    let snapshot = mgr.tree_snapshot(2);
    assert_eq!(snapshot.children.len(), 2);
    assert!(snapshot.children.iter().any(|e| e.mov == "Add"));
    assert!(snapshot.children.iter().any(|e| e.mov == "Sub"));

    let add_edge = snapshot.children.iter().find(|e| e.mov == "Add").unwrap();
    assert!(add_edge.child.is_some());
}

#[test]
fn principal_variation() {
    let game = Box::new(CountingGame {
        counter: 0,
        target: 10,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(CountingPriorEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(500);

    let pv = mgr.principal_variation(5);
    assert!(!pv.is_empty());
    assert_eq!(pv[0], "Add");
}

#[test]
fn reset_manager() {
    let game = Box::new(CountingGame {
        counter: 0,
        target: 10,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(CountingPriorEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(100);
    assert!(mgr.num_nodes() > 1);

    let mut mgr = mgr.reset();
    assert_eq!(mgr.num_nodes(), 1);

    mgr.playout_n(100);
    assert!(mgr.num_nodes() > 1);
}

// ===========================================================================
// Additional test games
// ===========================================================================

/// DiceGame: single-player, stochastic.
/// Moves: "Roll" (triggers chance outcome), "Stop" (freeze score).
/// Chance outcomes: Die(1), Die(2), Die(3) each with probability 1/3.
/// Terminal when stopped or score >= 10.
#[derive(Clone)]
struct DiceGame {
    score: i64,
    pending_roll: bool,
    stopped: bool,
}

impl GameCallbacks for DiceGame {
    fn clone_box(&self) -> Box<dyn GameCallbacks> {
        Box::new(self.clone())
    }

    fn current_player(&self) -> i32 {
        0
    }

    fn available_moves(&self) -> Vec<String> {
        if self.pending_roll || self.stopped || self.score >= 10 {
            vec![]
        } else {
            vec!["Roll".to_string(), "Stop".to_string()]
        }
    }

    fn make_move(&mut self, mov: &str) {
        match mov {
            "Roll" => self.pending_roll = true,
            "Stop" => self.stopped = true,
            "Die1" => {
                self.score += 1;
                self.pending_roll = false;
            }
            "Die2" => {
                self.score += 2;
                self.pending_roll = false;
            }
            "Die3" => {
                self.score += 3;
                self.pending_roll = false;
            }
            _ => panic!("Unknown move: {}", mov),
        }
    }

    fn chance_outcomes(&self) -> Option<Vec<(String, f64)>> {
        if self.pending_roll {
            Some(vec![
                ("Die1".to_string(), 1.0 / 3.0),
                ("Die2".to_string(), 1.0 / 3.0),
                ("Die3".to_string(), 1.0 / 3.0),
            ])
        } else {
            None
        }
    }
}

/// DiceGame evaluator: returns score as value.
struct DiceEval;

impl EvalCallbacks for DiceEval {
    fn evaluate(&self, _state: &dyn GameCallbacks, _moves: &[String]) -> (Vec<f64>, f64) {
        // Can't access score through GameCallbacks, return neutral.
        // The signal comes from terminal states (score >= 10).
        (vec![], 0.0)
    }

    fn interpret_for_player(
        &self,
        value: f64,
        _evaluating_player: i32,
        _requesting_player: i32,
    ) -> f64 {
        value
    }
}

/// ScoreGame: two-player, depth-2 minimax tree with known terminal scores.
/// State machine: state 0 → moves to 1 or 2 (terminal with scores).
///                state 10 → moves to 11 or 12 (terminal).
///                state 20 → moves to 21 or 22 (terminal or branch).
///                state 21 → moves to 23 or 24 (terminal).
/// Terminal scores (from current player's perspective):
///   1→10, 2→-5, 11→3, 12→7, 22→0, 23→8, 24→-3
#[derive(Clone)]
struct ScoreGame {
    state: u8,
    current_player: i32,
}

impl GameCallbacks for ScoreGame {
    fn clone_box(&self) -> Box<dyn GameCallbacks> {
        Box::new(self.clone())
    }

    fn current_player(&self) -> i32 {
        self.current_player
    }

    fn available_moves(&self) -> Vec<String> {
        match self.state {
            0 => vec!["M1".to_string(), "M2".to_string()],
            10 => vec!["M11".to_string(), "M12".to_string()],
            20 => vec!["M21".to_string(), "M22".to_string()],
            21 => vec!["M23".to_string(), "M24".to_string()],
            _ => vec![],
        }
    }

    fn make_move(&mut self, mov: &str) {
        self.state = match mov {
            "M1" => 1,
            "M2" => 2,
            "M11" => 11,
            "M12" => 12,
            "M21" => 21,
            "M22" => 22,
            "M23" => 23,
            "M24" => 24,
            _ => panic!("Unknown move: {}", mov),
        };
        self.current_player = if self.current_player == 1 { 2 } else { 1 };
    }

    fn terminal_score(&self) -> Option<i32> {
        match self.state {
            1 => Some(10),
            2 => Some(-5),
            11 => Some(3),
            12 => Some(7),
            22 => Some(0),
            23 => Some(8),
            24 => Some(-3),
            _ => None,
        }
    }
}

/// ScoreGame evaluator: returns 0 (score bounds do the work).
struct ScoreEval;

impl EvalCallbacks for ScoreEval {
    fn evaluate(&self, _state: &dyn GameCallbacks, _moves: &[String]) -> (Vec<f64>, f64) {
        (vec![], 0.0)
    }
}

/// PriorGame: single-player, 3 moves at each level, depth 3.
/// Moves: "A" (+10), "B" (+5), "C" (+1).
/// Evaluator returns misleading priors [0.1, 0.2, 0.7] (C has highest prior).
/// MCTS should overcome wrong priors and find A is best.
#[derive(Clone)]
struct PriorGame {
    depth: u8,
    score: i64,
}

impl GameCallbacks for PriorGame {
    fn clone_box(&self) -> Box<dyn GameCallbacks> {
        Box::new(self.clone())
    }

    fn current_player(&self) -> i32 {
        0
    }

    fn available_moves(&self) -> Vec<String> {
        if self.depth >= 3 {
            vec![]
        } else {
            vec!["A".to_string(), "B".to_string(), "C".to_string()]
        }
    }

    fn make_move(&mut self, mov: &str) {
        self.depth += 1;
        match mov {
            "A" => self.score += 10,
            "B" => self.score += 5,
            "C" => self.score += 1,
            _ => panic!("Unknown move: {}", mov),
        }
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.depth >= 3 {
            Some(ProvenValue::Win)
        } else {
            None
        }
    }

    fn terminal_score(&self) -> Option<i32> {
        if self.depth >= 3 {
            Some(self.score as i32)
        } else {
            None
        }
    }
}

/// PriorGame evaluator: misleading priors [0.1, 0.2, 0.7] + greedy rollout value.
/// Does a greedy rollout (always picks first move = "A"), then uses
/// terminal_score to differentiate paths (A-path scores 30, C-path scores 3).
struct MisleadingPriorEval;

impl EvalCallbacks for MisleadingPriorEval {
    fn evaluate(&self, state: &dyn GameCallbacks, moves: &[String]) -> (Vec<f64>, f64) {
        let priors = if moves.len() == 3 {
            vec![0.1, 0.2, 0.7] // Wrong: C gets highest prior
        } else {
            vec![]
        };

        // Greedy rollout: always pick first move ("A"), read terminal_score
        let mut sim = state.clone_box();
        loop {
            let avail = sim.available_moves();
            if avail.is_empty() {
                break;
            }
            sim.make_move(&avail[0]);
        }
        // terminal_score returns accumulated score (max 30 for all-A path).
        // Return raw score — REWARD_SCALE in adapter handles i64 conversion.
        let value = sim.terminal_score().unwrap_or(0) as f64;
        (priors, value)
    }

    fn interpret_for_player(
        &self,
        value: f64,
        _evaluating_player: i32,
        _requesting_player: i32,
    ) -> f64 {
        value
    }
}

/// MicroGame: two-player state machine for testing Draw propagation.
/// Multiple scenarios depending on starting state:
/// - State 10: both children are Draw → parent is Draw
/// - State 20: child 21=Loss, 22=Draw → parent is Win (picks 21)
/// - State 30: child 31=Win, 32=Draw → parent is Draw (best is Draw)
/// - State 40: depth-2, all leaves are Win or Draw → parent is Draw
#[derive(Clone)]
struct MicroGame {
    state: u8,
    current_player: i32,
}

impl GameCallbacks for MicroGame {
    fn clone_box(&self) -> Box<dyn GameCallbacks> {
        Box::new(self.clone())
    }

    fn current_player(&self) -> i32 {
        self.current_player
    }

    fn available_moves(&self) -> Vec<String> {
        match self.state {
            10 => vec!["M11".to_string(), "M12".to_string()],
            20 => vec!["M21".to_string(), "M22".to_string()],
            30 => vec!["M31".to_string(), "M32".to_string()],
            40 => vec!["M41".to_string(), "M42".to_string()],
            41 => vec!["M43".to_string(), "M44".to_string()],
            42 => vec!["M45".to_string(), "M46".to_string()],
            _ => vec![],
        }
    }

    fn make_move(&mut self, mov: &str) {
        self.state = match mov {
            "M11" => 11,
            "M12" => 12,
            "M21" => 21,
            "M22" => 22,
            "M31" => 31,
            "M32" => 32,
            "M41" => 41,
            "M42" => 42,
            "M43" => 43,
            "M44" => 44,
            "M45" => 45,
            "M46" => 46,
            _ => panic!("Unknown move: {}", mov),
        };
        self.current_player = if self.current_player == 1 { 2 } else { 1 };
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        match self.state {
            11 | 12 => Some(ProvenValue::Draw),
            21 => Some(ProvenValue::Loss),
            22 => Some(ProvenValue::Draw),
            31 => Some(ProvenValue::Win),
            32 => Some(ProvenValue::Draw),
            43 | 45 => Some(ProvenValue::Win),
            44 | 46 => Some(ProvenValue::Draw),
            _ => None,
        }
    }
}

/// Neutral evaluator for solver-driven games.
struct NeutralEval;

impl EvalCallbacks for NeutralEval {
    fn evaluate(&self, _state: &dyn GameCallbacks, _moves: &[String]) -> (Vec<f64>, f64) {
        (vec![], 0.0)
    }
}

// ===========================================================================
// Tests for new games
// ===========================================================================

#[test]
fn dice_game_open_loop() {
    let game = Box::new(DiceGame {
        score: 0,
        pending_roll: false,
        stopped: false,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(DiceEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(2000);

    // Should have explored the tree
    assert!(mgr.num_nodes() > 1);
    // Should have a best move (Roll or Stop)
    let best = mgr.best_move();
    assert!(best.is_some());
    assert!(best.as_deref() == Some("Roll") || best.as_deref() == Some("Stop"));
}

#[test]
fn dice_game_closed_loop() {
    let game = Box::new(DiceGame {
        score: 0,
        pending_roll: false,
        stopped: false,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(DiceEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        rng_seed: Some(42),
        closed_loop_chance: true,
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(2000);

    // Closed-loop creates more nodes (one per chance outcome)
    assert!(mgr.num_nodes() > 1);
    let best = mgr.best_move();
    assert!(best.is_some());
}

#[test]
fn score_bounded_game() {
    let game = Box::new(ScoreGame {
        state: 0,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(ScoreEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        score_bounded_enabled: true,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(5000);

    // State 0 → P1 chooses between M1 (→state 1, score 10) and M2 (→state 2, score -5).
    // Score 10 from P2's perspective (terminal after P1 moves). From P1's perspective: -10.
    // Score -5 from P2's perspective. From P1's perspective: 5.
    // So P1 should prefer M2 (gets score 5 from their perspective).
    let bounds = mgr.root_score_bounds();
    // Bounds should have tightened
    assert!(
        bounds.lower > i32::MIN || bounds.upper < i32::MAX,
        "Score bounds should tighten: {:?}",
        bounds
    );
}

#[test]
fn prior_game_overcomes_wrong_priors() {
    let game = Box::new(PriorGame { depth: 0, score: 0 });
    let eval: Box<dyn EvalCallbacks> = Box::new(MisleadingPriorEval);
    let config = DynConfig {
        exploration_constant: 2.5, // Higher C to overcome wrong priors (matches original test)
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(5000);

    // Despite C having highest prior (0.7), A is the best move (+10 per step)
    assert_eq!(mgr.best_move(), Some("A".to_string()));

    // A should have the most visits
    let stats = mgr.root_child_stats();
    let a = stats.iter().find(|s| s.mov == "A").unwrap();
    let c = stats.iter().find(|s| s.mov == "C").unwrap();
    assert!(
        a.visits > c.visits,
        "A visits ({}) should exceed C visits ({})",
        a.visits,
        c.visits
    );
}

#[test]
fn micro_game_draw_both_children() {
    // State 10: both children (11, 12) are Draw → parent should be Draw
    let game = Box::new(MicroGame {
        state: 10,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(NeutralEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        solver_enabled: true,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(100);

    assert_eq!(mgr.root_proven_value(), ProvenValue::Draw);
}

#[test]
fn micro_game_win_via_loss_child() {
    // State 20: child 21=Loss (current player lost), 22=Draw
    // Parent can Win by choosing 21 (opponent's loss = parent's win)
    let game = Box::new(MicroGame {
        state: 20,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(NeutralEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        solver_enabled: true,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(100);

    assert_eq!(mgr.root_proven_value(), ProvenValue::Win);
}

#[test]
fn micro_game_draw_best_outcome() {
    // State 30: child 31=Win (bad for parent), 32=Draw → best is Draw
    let game = Box::new(MicroGame {
        state: 30,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(NeutralEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        solver_enabled: true,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(100);

    assert_eq!(mgr.root_proven_value(), ProvenValue::Draw);
}

#[test]
fn micro_game_deep_draw() {
    // State 40: depth-2 tree. All leaves are Win or Draw.
    // 41 → (43=Win, 44=Draw) → Draw for 41
    // 42 → (45=Win, 46=Draw) → Draw for 42
    // Both children are Draw → parent is Draw
    let game = Box::new(MicroGame {
        state: 40,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(NeutralEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        solver_enabled: true,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(1000);

    assert_eq!(mgr.root_proven_value(), ProvenValue::Draw);
}

// ===========================================================================
// Config variation tests
// ===========================================================================

#[test]
fn config_node_limit() {
    let game = Box::new(CountingGame {
        counter: 0,
        target: 100,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(CountingPriorEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        node_limit: 20,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(10000); // Request many, but node limit should cap tree

    assert!(
        mgr.num_nodes() <= 20,
        "Node limit should cap at 20, got {}",
        mgr.num_nodes()
    );
}

#[test]
fn config_fpu_and_max_depth() {
    let game = Box::new(CountingGame {
        counter: 0,
        target: 100,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(CountingPriorEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        fpu_value: 0.0,
        max_playout_depth: 20,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(500);

    // With max_playout_depth=20, tree depth is limited but search still runs
    assert!(mgr.num_nodes() > 1);
    // With fpu=0.0, first visits are random (all unvisited children score 0.0).
    // Don't assert best_move — just verify search completes and tree grows.
    assert!(mgr.best_move().is_some());
}

#[test]
fn config_temperature() {
    let game = Box::new(CountingGame {
        counter: 0,
        target: 10,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(CountingPriorEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        selection_temperature: 1.0, // proportional to visits
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(500);

    // With temperature=1.0, best_move is sampled proportional to visits.
    // With seeded RNG, result is deterministic but may not be argmax.
    let best = mgr.best_move();
    assert!(best.is_some()); // Just verify it doesn't crash
}

#[test]
fn config_dirichlet_noise() {
    let game = Box::new(PriorGame { depth: 0, score: 0 });
    let eval: Box<dyn EvalCallbacks> = Box::new(MisleadingPriorEval);
    let config = DynConfig {
        exploration_constant: 2.5,
        dirichlet_noise: Some((0.25, 0.3)),
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(5000);

    // With Dirichlet noise perturbing priors, MCTS should still find A
    // (noise adds randomness but doesn't change the value landscape)
    assert_eq!(mgr.best_move(), Some("A".to_string()));
}

#[test]
fn config_virtual_loss_parallel() {
    let game = Box::new(CountingGame {
        counter: 0,
        target: 10,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(CountingPriorEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        virtual_loss: 500,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n_parallel(500, 2);

    // Parallel search with virtual loss should still work
    assert!(mgr.num_nodes() > 1);
    assert_eq!(mgr.best_move(), Some("Add".to_string()));
}

// ===========================================================================
// Edge case tests
// ===========================================================================

#[test]
fn edge_terminal_at_root() {
    // Game is already terminal (counter == target)
    let game = Box::new(CountingGame {
        counter: 5,
        target: 5,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(CountingPriorEval);
    let config = DynConfig::default();

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(10);

    assert_eq!(mgr.best_move(), None); // No moves available
    assert_eq!(mgr.root_child_stats().len(), 0);
}

#[test]
fn edge_single_move() {
    // Nim(1): only one legal move (Take1)
    let game = Box::new(TinyNim {
        stones: 1,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(RandomRollout::with_seed(42));
    let config = DynConfig {
        solver_enabled: true,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(100);

    assert_eq!(mgr.best_move(), Some("Take1".to_string()));
    assert_eq!(mgr.root_proven_value(), ProvenValue::Win);
    assert_eq!(mgr.root_child_stats().len(), 1);
}

#[test]
fn edge_interpret_for_player_negation() {
    // Nim with solver: verify that two-player evaluation works correctly
    // through the interpret_for_player callback path.
    let game = Box::new(TinyNim {
        stones: 4,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(RandomRollout::with_seed(42));
    let config = DynConfig {
        exploration_constant: 0.5,
        solver_enabled: true,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(10000);

    // Nim(4) is winning for P1 (4 % 3 != 0)
    assert_eq!(mgr.root_proven_value(), ProvenValue::Win);
    // Best move is Take1 → leaves 3 (losing for P2)
    assert_eq!(mgr.best_move(), Some("Take1".to_string()));

    // After advancing Take1, P2 is in a losing position (3 stones)
    mgr.advance("Take1").unwrap();
    mgr.playout_n(10000);
    assert_eq!(mgr.root_proven_value(), ProvenValue::Loss);
}

#[test]
fn edge_custom_eval_with_priors() {
    // Verify that custom evaluator priors are actually used by PUCT
    let game = Box::new(PriorGame { depth: 0, score: 0 });

    // Custom evaluator that gives A=0.8, B=0.15, C=0.05 (correct priors)
    struct CorrectPriorEval;
    impl EvalCallbacks for CorrectPriorEval {
        fn evaluate(&self, _state: &dyn GameCallbacks, moves: &[String]) -> (Vec<f64>, f64) {
            if moves.len() == 3 {
                (vec![0.8, 0.15, 0.05], 0.0) // A gets highest prior (correct!)
            } else {
                (vec![], 0.0)
            }
        }
        fn interpret_for_player(&self, value: f64, _ep: i32, _rp: i32) -> f64 {
            value
        }
    }

    let eval: Box<dyn EvalCallbacks> = Box::new(CorrectPriorEval);
    let config = DynConfig {
        exploration_constant: 1.41,
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(500);

    // With correct priors, A should dominate even with few playouts
    assert_eq!(mgr.best_move(), Some("A".to_string()));
    let stats = mgr.root_child_stats();
    let a = stats.iter().find(|s| s.mov == "A").unwrap();
    // A's prior should be 0.8
    assert!(
        (a.prior - 0.8).abs() < 0.01,
        "A's prior should be ~0.8, got {}",
        a.prior
    );
}

#[test]
fn edge_advance_invalid_move() {
    let game = Box::new(TinyNim {
        stones: 5,
        current_player: 1,
    });
    let eval: Box<dyn EvalCallbacks> = Box::new(RandomRollout::with_seed(42));
    let config = DynConfig {
        rng_seed: Some(42),
        ..Default::default()
    };

    let mut mgr = DynMCTSManager::new(game, eval, config);
    mgr.playout_n(100);

    // Advance with a move that doesn't exist
    let result = mgr.advance("Take99");
    assert!(result.is_err());
}
