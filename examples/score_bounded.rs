//! Score-Bounded MCTS: tracking minimax score intervals.
//!
//! A fixed depth-2 two-player game where terminal nodes have known scores.
//! Score bounds tighten during search until they converge to the exact
//! minimax value. The solver derives proven win/loss/draw from those bounds.
//!
//! Run: cargo run --example score_bounded
//! Output: cargo run --example score_bounded > examples/output/score_bounded.txt

use treant::tree_policy::*;
use treant::*;

// region: score_game

/// A two-player game with a fixed depth-2 tree.
///
/// Root (P1, maximizer) picks branch A, B, or C.
/// Then P2 (minimizer) picks a response. Each leaf has a known score
/// (from P1's perspective):
///
/// ```text
///              Root (P1)
///           /      |      \
///         A        B        C
///       (P2)     (P2)     (P2)
///       / \      / \      / \
///      2   5    1   3    8   6
/// ```
///
/// Minimax: P2 minimizes, P1 maximizes.
///   A -> P2 picks min(2,5) = 2
///   B -> P2 picks min(1,3) = 1
///   C -> P2 picks min(8,6) = 6
///   Root -> P1 picks max(2,1,6) = 6 → best move is C
#[derive(Clone, Debug)]
struct ScoreGame {
    depth: u8,
    /// Which branch P1 chose (set after depth 0).
    branch: Option<Branch>,
    /// Terminal score from P1's perspective (set at depth 2).
    score: Option<i32>,
    current: Player,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Player {
    P1,
    P2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Branch {
    A,
    B,
    C,
}

/// Moves encode both P1's branch choice and P2's response index.
#[derive(Clone, Debug, PartialEq)]
enum ScoreMove {
    /// P1 picks a branch.
    Pick(Branch),
    /// P2 picks the i-th response within the chosen branch.
    Respond(u8),
}

impl std::fmt::Display for ScoreMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ScoreMove::Pick(b) => write!(f, "{b:?}"),
            ScoreMove::Respond(i) => write!(f, "R{i}"),
        }
    }
}

/// Terminal scores for each (branch, response) pair, from P1's perspective.
fn terminal_scores(branch: Branch, response: u8) -> i32 {
    match (branch, response) {
        (Branch::A, 0) => 2,
        (Branch::A, 1) => 5,
        (Branch::B, 0) => 1,
        (Branch::B, 1) => 3,
        (Branch::C, 0) => 8,
        (Branch::C, 1) => 6,
        _ => unreachable!(),
    }
}

impl GameState for ScoreGame {
    type Move = ScoreMove;
    type Player = Player;
    type MoveList = Vec<ScoreMove>;

    fn current_player(&self) -> Player {
        self.current
    }

    fn available_moves(&self) -> Vec<ScoreMove> {
        match self.depth {
            0 => vec![
                ScoreMove::Pick(Branch::A),
                ScoreMove::Pick(Branch::B),
                ScoreMove::Pick(Branch::C),
            ],
            1 => {
                // Each branch has exactly 2 responses.
                vec![ScoreMove::Respond(0), ScoreMove::Respond(1)]
            }
            _ => vec![],
        }
    }

    fn make_move(&mut self, mov: &ScoreMove) {
        match mov {
            ScoreMove::Pick(b) => {
                self.branch = Some(*b);
                self.depth = 1;
                self.current = Player::P2;
            }
            ScoreMove::Respond(i) => {
                let branch = self.branch.unwrap();
                self.score = Some(terminal_scores(branch, *i));
                self.depth = 2;
                self.current = Player::P1;
            }
        }
    }

    // Return the exact minimax score from the current player's perspective.
    // At depth 2 the current player is P1, and scores are already from P1's view.
    fn terminal_score(&self) -> Option<i32> {
        if self.depth == 2 {
            self.score
        } else {
            None
        }
    }

    // Note: terminal_value() is NOT implemented. The library cross-derives
    // proven values (Win/Loss/Draw) from terminal_score() automatically.
}

// endregion: score_game

// region: score_evaluator

struct ScoreEval;

impl Evaluator<ScoreMCTS> for ScoreEval {
    type StateEvaluation = i32;

    fn evaluate_new_state(
        &self,
        state: &ScoreGame,
        moves: &Vec<ScoreMove>,
        _: Option<SearchHandle<ScoreMCTS>>,
    ) -> (Vec<()>, i32) {
        // At terminals, use the exact score. Non-terminals get 0 (doesn't
        // matter much — score bounds do the heavy lifting).
        let eval = state.score.unwrap_or(0);
        (vec![(); moves.len()], eval)
    }

    fn interpret_evaluation_for_player(&self, evaln: &i32, player: &Player) -> i64 {
        // Scores are from P1's perspective. Negate for P2.
        match player {
            Player::P1 => *evaln as i64,
            Player::P2 => -(*evaln as i64),
        }
    }

    fn evaluate_existing_state(
        &self,
        _: &ScoreGame,
        evaln: &i32,
        _: SearchHandle<ScoreMCTS>,
    ) -> i32 {
        *evaln
    }
}

// endregion: score_evaluator

// region: score_config

#[derive(Default)]
struct ScoreMCTS;

impl MCTS for ScoreMCTS {
    type State = ScoreGame;
    type Eval = ScoreEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();

    fn visits_before_expansion(&self) -> u64 {
        0
    }

    fn score_bounded_enabled(&self) -> bool {
        true
    }

    fn solver_enabled(&self) -> bool {
        true
    }
}

// endregion: score_config

// region: run_score_bounded

fn main() {
    println!("=== Score-Bounded MCTS ===\n");
    println!("A depth-2 two-player game with known terminal scores.");
    println!("P1 picks branch A/B/C, then P2 responds.\n");
    println!("Tree (scores from P1's perspective):");
    println!("            Root (P1)");
    println!("         /      |      \\");
    println!("       A        B        C");
    println!("     (P2)     (P2)     (P2)");
    println!("     / \\      / \\      / \\");
    println!("    2   5    1   3    8   6");
    println!();
    println!("Minimax: A=min(2,5)=2, B=min(1,3)=1, C=min(8,6)=6");
    println!("Root = max(2,1,6) = 6 via branch C\n");

    let mut mcts = MCTSManager::new(
        ScoreGame {
            depth: 0,
            branch: None,
            score: None,
            current: Player::P1,
        },
        ScoreMCTS,
        ScoreEval,
        UCTPolicy::new(1.0),
        (),
    );

    mcts.playout_n(100);

    // Root score bounds — should converge to [6, 6]
    let bounds = mcts.root_score_bounds();
    println!("Root score bounds: [{}, {}]", bounds.lower, bounds.upper);
    println!("Bounds converged:  {}", bounds.is_proven());

    // Proven value — cross-derived from converged score bounds
    let proven = mcts.root_proven_value();
    println!("Root proven value: {proven:?}");

    // Best move — should be C (minimax value 6)
    let best = mcts
        .best_move()
        .map(|m| format!("{m}"))
        .unwrap_or_else(|| "-".into());
    println!("Best move: {best}");

    let nodes = mcts.tree().num_nodes();
    println!("Nodes: {nodes}\n");

    // Child stats with score bounds
    println!("Child stats (bounds from P1's perspective):");
    let stats = mcts.root_child_stats();
    for s in &stats {
        // Score bounds are from the child's perspective (P2).
        // Negate to show from P1's perspective.
        let p1_lower = negate(s.score_bounds.upper);
        let p1_upper = negate(s.score_bounds.lower);
        println!(
            "  {:<3} — visits: {:4}, avg_reward: {:6.1}, \
             value: [{:>4}, {:>4}], proven: {:?}",
            s.mov,
            s.visits,
            s.avg_reward,
            format_bound(p1_lower),
            format_bound(p1_upper),
            s.proven_value,
        );
    }

    // Verify correctness
    println!();
    assert!(
        bounds.is_proven() && bounds.lower == 6,
        "Expected proven minimax value 6, got [{}, {}]",
        bounds.lower,
        bounds.upper
    );
    println!("Verified: minimax value = {} (exact)", bounds.lower);
}

/// Negate a score bound, mapping sentinels correctly.
fn negate(v: i32) -> i32 {
    match v {
        i32::MIN => i32::MAX,
        i32::MAX => i32::MIN,
        _ => -v,
    }
}

/// Format a bound for display, showing "inf" for sentinels.
fn format_bound(v: i32) -> String {
    match v {
        i32::MIN => "-inf".to_string(),
        i32::MAX => "+inf".to_string(),
        _ => v.to_string(),
    }
}

// endregion: run_score_bounded
