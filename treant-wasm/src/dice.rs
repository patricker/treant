use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

// --- Game (mirrors examples/dice_game.rs) ---

#[derive(Clone, Debug)]
struct DiceGame {
    score: i64,
    pending_roll: bool,
    stopped: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum DiceMove {
    Roll,
    Stop,
    Die(u8),
}

impl std::fmt::Display for DiceMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DiceMove::Roll => write!(f, "Roll"),
            DiceMove::Stop => write!(f, "Stop"),
            DiceMove::Die(n) => write!(f, "Die({n})"),
        }
    }
}

impl GameState for DiceGame {
    type Move = DiceMove;
    type Player = ();
    type MoveList = Vec<DiceMove>;

    fn current_player(&self) -> Self::Player {}

    fn available_moves(&self) -> Vec<DiceMove> {
        if self.pending_roll || self.stopped || self.score >= 20 {
            vec![]
        } else {
            vec![DiceMove::Roll, DiceMove::Stop]
        }
    }

    fn make_move(&mut self, mov: &DiceMove) {
        match mov {
            DiceMove::Roll => self.pending_roll = true,
            DiceMove::Stop => self.stopped = true,
            DiceMove::Die(v) => {
                self.score += *v as i64;
                self.pending_roll = false;
            }
        }
    }

    fn chance_outcomes(&self) -> Option<Vec<(DiceMove, f64)>> {
        if self.pending_roll {
            Some((1..=6).map(|i| (DiceMove::Die(i), 1.0 / 6.0)).collect())
        } else {
            None
        }
    }
}

struct DiceEval;

impl Evaluator<DiceConfig> for DiceEval {
    type StateEvaluation = i64;

    fn evaluate_new_state(
        &self,
        state: &DiceGame,
        moves: &Vec<DiceMove>,
        _: Option<SearchHandle<DiceConfig>>,
    ) -> (Vec<()>, i64) {
        (vec![(); moves.len()], state.score)
    }

    fn interpret_evaluation_for_player(&self, evaln: &i64, _: &()) -> i64 {
        *evaln
    }

    fn evaluate_existing_state(
        &self,
        state: &DiceGame,
        _: &i64,
        _: SearchHandle<DiceConfig>,
    ) -> i64 {
        state.score
    }
}

#[derive(Default)]
struct DiceConfig;

impl MCTS for DiceConfig {
    type State = DiceGame;
    type Eval = DiceEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
}

// --- WASM API ---

#[wasm_bindgen]
pub struct DiceGameWasm {
    manager: MCTSManager<DiceConfig>,
    start_score: i64,
}

#[wasm_bindgen]
impl DiceGameWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(start_score: i64) -> Self {
        Self {
            manager: MCTSManager::new(
                DiceGame {
                    score: start_score,
                    pending_roll: false,
                    stopped: false,
                },
                DiceConfig,
                DiceEval,
                UCTPolicy::new(0.5),
                (),
            ),
            start_score,
        }
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }

    pub fn get_stats(&self) -> JsValue {
        let stats = types::build_stats(&self.manager, |_| None);
        serde_wasm_bindgen::to_value(&stats).unwrap_or(JsValue::NULL)
    }

    pub fn get_tree(&self, max_depth: u32) -> JsValue {
        let tree =
            types::export_tree::<DiceConfig>(self.manager.tree().root_node(), max_depth, &|_| None);
        serde_wasm_bindgen::to_value(&tree).unwrap_or(JsValue::NULL)
    }

    pub fn current_score(&self) -> i64 {
        self.manager.tree().root_state().score
    }

    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(
            DiceGame {
                score: self.start_score,
                pending_roll: false,
                stopped: false,
            },
            DiceConfig,
            DiceEval,
            UCTPolicy::new(0.5),
            (),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roll_opens_a_chance_node_and_resolves() {
        let mut g = DiceGame { score: 0, pending_roll: false, stopped: false };
        // Rolling opens a chance node with six equiprobable die faces.
        g.make_move(&DiceMove::Roll);
        assert!(g.pending_roll);
        let outcomes = g.chance_outcomes().expect("a pending roll is a chance node");
        assert_eq!(outcomes.len(), 6);
        let total: f64 = outcomes.iter().map(|(_, p)| p).sum();
        assert!((total - 1.0).abs() < 1e-9, "chance probs sum to 1");
        // Resolving the chance outcome adds pips and closes the roll.
        g.make_move(&DiceMove::Die(6));
        assert_eq!(g.score, 6);
        assert!(!g.pending_roll);
        assert!(!g.available_moves().is_empty(), "still playable below 20");
    }

    #[test]
    fn terminal_when_capped_or_stopped() {
        // Terminal for this single-player game = no legal moves. Reaching the cap...
        let mut g = DiceGame { score: 18, pending_roll: false, stopped: false };
        g.make_move(&DiceMove::Roll);
        g.make_move(&DiceMove::Die(6)); // 24 >= 20
        assert_eq!(g.score, 24);
        assert!(g.available_moves().is_empty());
        // ...and choosing to stop both end the game.
        let mut h = DiceGame { score: 5, pending_roll: false, stopped: false };
        h.make_move(&DiceMove::Stop);
        assert!(h.available_moves().is_empty());
    }

    #[test]
    fn wasm_surface_searches_the_chance_tree() {
        // Drive the chance machinery end to end through the WASM class. (get_stats /
        // get_tree serialize via wasm-bindgen and can't run on a native test
        // target, so we exercise the search + decision path only.)
        let mut g = DiceGameWasm::new(0);
        g.playout_n(300);
        assert_eq!(g.current_score(), 0, "root score is unchanged by search");
        // A weakened decision is one of the two push-your-luck moves.
        let m = g.weak_move(300, 3, 0.5, 7);
        assert!(matches!(m.as_deref(), Some("Roll") | Some("Stop")));
    }
}
