use std::sync::Arc;

use treant::tree_policy::AlphaGoPolicy;
use treant::{CycleBehaviour, Evaluator, GameState, ProvenValue, SearchHandle, MCTS};

use crate::callbacks::{EvalCallbacks, GameCallbacks};
use crate::types::{DynConfig, DynMove};

// ---------------------------------------------------------------------------
// DynGameState — wraps Box<dyn GameCallbacks>, implements core GameState
// ---------------------------------------------------------------------------

pub(crate) struct DynGameState(pub(crate) Box<dyn GameCallbacks>);

impl Clone for DynGameState {
    fn clone(&self) -> Self {
        DynGameState(self.0.clone_box())
    }
}

impl GameState for DynGameState {
    type Move = DynMove;
    type Player = i32;
    type MoveList = Vec<DynMove>;

    fn current_player(&self) -> i32 {
        self.0.current_player()
    }

    fn available_moves(&self) -> Vec<DynMove> {
        self.0.available_moves().into_iter().map(DynMove).collect()
    }

    fn make_move(&mut self, mov: &DynMove) {
        self.0.make_move(&mov.0);
    }

    fn max_children(&self, visits: u64) -> usize {
        self.0.max_children(visits)
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        self.0.terminal_value()
    }

    fn terminal_score(&self) -> Option<i32> {
        self.0.terminal_score()
    }

    fn chance_outcomes(&self) -> Option<Vec<(DynMove, f64)>> {
        self.0
            .chance_outcomes()
            .map(|v| v.into_iter().map(|(m, p)| (DynMove(m), p)).collect())
    }
}

// ---------------------------------------------------------------------------
// DynEvaluator — wraps Arc<dyn EvalCallbacks>, implements core Evaluator
// ---------------------------------------------------------------------------

/// Scale factor for converting f64 state values to i64 rewards.
/// The core library uses `i64` for atomic accumulation.
/// Keep this small enough that the PUCT explore term (C * sqrt(N) * prior)
/// can compete with the exploit term (mean reward). Too large a scale
/// makes prior-guided exploration ineffective.
const REWARD_SCALE: f64 = 10.0;

pub(crate) struct DynEvaluator(pub(crate) Arc<dyn EvalCallbacks>);

impl Evaluator<DynSpec> for DynEvaluator {
    type StateEvaluation = DynStateEval;

    fn evaluate_new_state(
        &self,
        state: &DynGameState,
        moves: &Vec<DynMove>,
        _handle: Option<SearchHandle<DynSpec>>,
    ) -> (Vec<f64>, DynStateEval) {
        let move_strings: Vec<String> = moves.iter().map(|m| m.0.clone()).collect();
        let (mut priors, value) = self.0.evaluate(&*state.0, &move_strings);

        // If no priors returned, generate uniform
        if priors.is_empty() && !moves.is_empty() {
            let uniform = 1.0 / moves.len() as f64;
            priors = vec![uniform; moves.len()];
        }

        let eval = DynStateEval {
            value,
            player: state.0.current_player(),
        };
        (priors, eval)
    }

    fn evaluate_existing_state(
        &self,
        state: &DynGameState,
        existing_evaln: &DynStateEval,
        _handle: SearchHandle<DynSpec>,
    ) -> DynStateEval {
        // For open-loop chance nodes: re-evaluate the state
        let move_strings: Vec<String> = state.0.available_moves().into_iter().collect();
        let (_, value) = self.0.evaluate(&*state.0, &move_strings);
        DynStateEval {
            value,
            player: existing_evaln.player,
        }
    }

    fn interpret_evaluation_for_player(&self, evaluation: &DynStateEval, player: &i32) -> i64 {
        let raw = self
            .0
            .interpret_for_player(evaluation.value, evaluation.player, *player);
        (raw * REWARD_SCALE) as i64
    }
}

/// State evaluation: the f64 value and which player it was evaluated for.
#[derive(Clone, Debug)]
pub struct DynStateEval {
    pub value: f64,
    pub player: i32,
}

// ---------------------------------------------------------------------------
// DynSpec — the concrete MCTS config type
// ---------------------------------------------------------------------------

pub(crate) struct DynSpec {
    pub(crate) config: DynConfig,
}

impl MCTS for DynSpec {
    type State = DynGameState;
    type Eval = DynEvaluator;
    type TreePolicy = AlphaGoPolicy;
    type NodeData = ();
    type TranspositionTable = ();
    type ExtraThreadData = ();

    fn virtual_loss(&self) -> i64 {
        self.config.virtual_loss
    }

    fn fpu_value(&self) -> f64 {
        self.config.fpu_value
    }

    fn node_limit(&self) -> usize {
        self.config.node_limit
    }

    fn max_playout_length(&self) -> usize {
        self.config.max_playout_length
    }

    fn max_playout_depth(&self) -> usize {
        self.config.max_playout_depth
    }

    fn rng_seed(&self) -> Option<u64> {
        self.config.rng_seed
    }

    fn dirichlet_noise(&self) -> Option<(f64, f64)> {
        self.config.dirichlet_noise
    }

    fn selection_temperature(&self) -> f64 {
        self.config.selection_temperature
    }

    fn solver_enabled(&self) -> bool {
        self.config.solver_enabled
    }

    fn score_bounded_enabled(&self) -> bool {
        self.config.score_bounded_enabled
    }

    fn closed_loop_chance(&self) -> bool {
        self.config.closed_loop_chance
    }

    fn cycle_behaviour(&self) -> CycleBehaviour<Self> {
        CycleBehaviour::Ignore
    }
}
