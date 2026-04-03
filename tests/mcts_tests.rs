use mcts::transposition_table::*;
use mcts::tree_policy::*;
use mcts::*;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Game definitions
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
struct CountingGame(i64);

#[derive(Clone, Debug, PartialEq)]
enum Move {
	Add,
	Sub,
}

impl GameState for CountingGame {
	type Move = Move;
	type Player = ();
	type MoveList = Vec<Move>;

	fn current_player(&self) -> Self::Player {}
	fn available_moves(&self) -> Vec<Move> {
		if self.0 == 100 {
			vec![]
		} else {
			vec![Move::Add, Move::Sub]
		}
	}
	fn make_move(&mut self, mov: &Self::Move) {
		match *mov {
			Move::Add => self.0 += 1,
			Move::Sub => self.0 -= 1,
		}
	}
}

impl TranspositionHash for CountingGame {
	fn hash(&self) -> u64 {
		self.0 as u64
	}
}

struct MyEvaluator;

impl<Spec: MCTS<State = CountingGame, TreePolicy = UCTPolicy>> Evaluator<Spec> for MyEvaluator {
	type StateEvaluation = i64;

	fn evaluate_new_state(
		&self,
		state: &CountingGame,
		moves: &Vec<Move>,
		_: Option<SearchHandle<Spec>>,
	) -> (Vec<()>, i64) {
		(vec![(); moves.len()], state.0)
	}
	fn interpret_evaluation_for_player(&self, evaln: &i64, _player: &()) -> i64 {
		*evaln
	}
	fn evaluate_existing_state(&self, _: &CountingGame, evaln: &i64, _: SearchHandle<Spec>) -> i64 {
		*evaln
	}
}

// --- MCTS configurations ---

#[derive(Default)]
struct CountingMCTS;

impl MCTS for CountingMCTS {
	type State = CountingGame;
	type Eval = MyEvaluator;
	type NodeData = ();
	type ExtraThreadData = ();
	type TreePolicy = UCTPolicy;
	type TranspositionTable = ApproxTable<Self>;

	fn cycle_behaviour(&self) -> CycleBehaviour<Self> {
		CycleBehaviour::UseCurrentEvalWhenCycleDetected
	}
}

#[derive(Default)]
struct VirtualLossMCTS;

impl MCTS for VirtualLossMCTS {
	type State = CountingGame;
	type Eval = MyEvaluator;
	type NodeData = ();
	type ExtraThreadData = ();
	type TreePolicy = UCTPolicy;
	type TranspositionTable = ApproxTable<Self>;

	fn virtual_loss(&self) -> i64 {
		500
	}
	fn cycle_behaviour(&self) -> CycleBehaviour<Self> {
		CycleBehaviour::UseCurrentEvalWhenCycleDetected
	}
}

#[derive(Default)]
struct NodeLimitMCTS;

impl MCTS for NodeLimitMCTS {
	type State = CountingGame;
	type Eval = MyEvaluator;
	type NodeData = ();
	type ExtraThreadData = ();
	type TreePolicy = UCTPolicy;
	type TranspositionTable = ApproxTable<Self>;

	fn node_limit(&self) -> usize {
		50
	}
	fn cycle_behaviour(&self) -> CycleBehaviour<Self> {
		CycleBehaviour::UseCurrentEvalWhenCycleDetected
	}
}

#[derive(Default)]
struct NoTranspositionMCTS;

impl MCTS for NoTranspositionMCTS {
	type State = CountingGame;
	type Eval = MyEvaluator;
	type NodeData = ();
	type ExtraThreadData = ();
	type TreePolicy = UCTPolicy;
	type TranspositionTable = ();
}

// AlphaGo policy configuration

struct AlphaGoEvaluator;

#[derive(Default)]
struct AlphaGoMCTS;

impl MCTS for AlphaGoMCTS {
	type State = CountingGame;
	type Eval = AlphaGoEvaluator;
	type NodeData = ();
	type ExtraThreadData = ();
	type TreePolicy = AlphaGoPolicy;
	type TranspositionTable = ();
}

impl Evaluator<AlphaGoMCTS> for AlphaGoEvaluator {
	type StateEvaluation = i64;

	fn evaluate_new_state(
		&self,
		state: &CountingGame,
		moves: &Vec<Move>,
		_: Option<SearchHandle<AlphaGoMCTS>>,
	) -> (Vec<f64>, i64) {
		let n = moves.len();
		let prior = if n > 0 { 1.0 / n as f64 } else { 0.0 };
		(vec![prior; n], state.0)
	}
	fn interpret_evaluation_for_player(&self, evaln: &i64, _player: &()) -> i64 {
		*evaln
	}
	fn evaluate_existing_state(
		&self,
		_: &CountingGame,
		evaln: &i64,
		_: SearchHandle<AlphaGoMCTS>,
	) -> i64 {
		*evaln
	}
}

// AlphaGo with asymmetric priors

struct AlphaGoAsymmetricEvaluator;

#[derive(Default)]
struct AlphaGoAsymmetricMCTS;

impl MCTS for AlphaGoAsymmetricMCTS {
	type State = CountingGame;
	type Eval = AlphaGoAsymmetricEvaluator;
	type NodeData = ();
	type ExtraThreadData = ();
	type TreePolicy = AlphaGoPolicy;
	type TranspositionTable = ();
}

impl Evaluator<AlphaGoAsymmetricMCTS> for AlphaGoAsymmetricEvaluator {
	type StateEvaluation = i64;

	fn evaluate_new_state(
		&self,
		state: &CountingGame,
		moves: &Vec<Move>,
		_: Option<SearchHandle<AlphaGoAsymmetricMCTS>>,
	) -> (Vec<f64>, i64) {
		// Heavily favor Sub (0.9) over Add (0.1)
		let evals = if moves.len() == 2 {
			vec![0.1, 0.9]
		} else {
			vec![]
		};
		(evals, state.0)
	}
	fn interpret_evaluation_for_player(&self, evaln: &i64, _player: &()) -> i64 {
		*evaln
	}
	fn evaluate_existing_state(
		&self,
		_: &CountingGame,
		evaln: &i64,
		_: SearchHandle<AlphaGoAsymmetricMCTS>,
	) -> i64 {
		*evaln
	}
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn make_counting_mcts() -> MCTSManager<CountingMCTS> {
	MCTSManager::new(
		CountingGame(0),
		CountingMCTS,
		MyEvaluator,
		UCTPolicy::new(0.5),
		ApproxTable::new(1024),
	)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_single_threaded_best_move() {
	let mut mcts = make_counting_mcts();
	mcts.playout_n(10000);
	assert_eq!(mcts.best_move().unwrap(), Move::Add);
	assert!(mcts.tree().num_nodes() > 1);
}

#[test]
fn test_parallel_best_move() {
	let mut mcts = make_counting_mcts();
	mcts.playout_n_parallel(10000, 4);
	assert_eq!(mcts.best_move().unwrap(), Move::Add);
}

#[test]
fn test_uct_visit_distribution() {
	let mut mcts = make_counting_mcts();
	mcts.playout_n(5000);
	let mut add_visits = 0u64;
	let mut sub_visits = 0u64;
	for mov in mcts.tree().root_node().moves() {
		match mov.get_move() {
			Move::Add => add_visits = mov.visits(),
			Move::Sub => sub_visits = mov.visits(),
		}
	}
	assert!(
		add_visits > sub_visits,
		"Add should get more visits than Sub: {} vs {}",
		add_visits,
		sub_visits
	);
}

#[test]
fn test_reset_clears_state() {
	let mut mcts = MCTSManager::new(
		CountingGame(0),
		NoTranspositionMCTS,
		MyEvaluator,
		UCTPolicy::new(0.5),
		(),
	);
	mcts.playout_n(1000);
	let nodes_before = mcts.tree().num_nodes();
	assert!(nodes_before > 1);
	let mut mcts = mcts.reset();
	assert_eq!(mcts.tree().num_nodes(), 1);
	mcts.playout_n(1000);
	assert_eq!(mcts.best_move().unwrap(), Move::Add);
}

#[test]
fn test_node_limit_stops_search() {
	let mut mcts = MCTSManager::new(
		CountingGame(0),
		NodeLimitMCTS,
		MyEvaluator,
		UCTPolicy::new(0.5),
		ApproxTable::new(1024),
	);
	mcts.print_on_playout_error(false);
	mcts.playout_n(10000);
	assert!(mcts.tree().num_nodes() <= 50);
	assert!(mcts.best_move().is_some());
}

#[test]
fn test_virtual_loss_does_not_change_result() {
	let mut mcts = MCTSManager::new(
		CountingGame(0),
		VirtualLossMCTS,
		MyEvaluator,
		UCTPolicy::new(0.5),
		ApproxTable::new(1024),
	);
	mcts.playout_n(5000);
	assert_eq!(mcts.best_move().unwrap(), Move::Add);
}

#[test]
fn test_principal_variation() {
	let mut mcts = make_counting_mcts();
	mcts.playout_n(10000);
	let pv = mcts.principal_variation(10);
	assert_eq!(pv, vec![Move::Add; 10]);
}

#[test]
fn test_principal_variation_states() {
	let mut mcts = make_counting_mcts();
	mcts.playout_n(10000);
	let states = mcts.principal_variation_states(5);
	assert_eq!(
		states,
		vec![
			CountingGame(0),
			CountingGame(1),
			CountingGame(2),
			CountingGame(3),
			CountingGame(4),
			CountingGame(5),
		]
	);
}

#[test]
fn test_principal_variation_info() {
	let mut mcts = make_counting_mcts();
	mcts.playout_n(5000);
	let pv_info = mcts.principal_variation_info(5);
	assert!(!pv_info.is_empty());
	for info in &pv_info {
		assert!(info.visits() > 0);
		assert_eq!(info.get_move(), &Move::Add);
	}
}

#[test]
fn test_alphago_policy_basic() {
	let mut mcts = MCTSManager::new(
		CountingGame(0),
		AlphaGoMCTS,
		AlphaGoEvaluator,
		AlphaGoPolicy::new(0.5),
		(),
	);
	mcts.playout_n(5000);
	assert_eq!(mcts.best_move().unwrap(), Move::Add);
}

#[test]
fn test_alphago_policy_asymmetric_priors() {
	let mut mcts = MCTSManager::new(
		CountingGame(0),
		AlphaGoAsymmetricMCTS,
		AlphaGoAsymmetricEvaluator,
		AlphaGoPolicy::new(0.5),
		(),
	);
	mcts.playout_n(10000);
	// Reward signal should overcome prior bias
	assert_eq!(mcts.best_move().unwrap(), Move::Add);
}

#[test]
fn test_transposition_table_hits() {
	let mut mcts = make_counting_mcts();
	mcts.playout_n(10000);
	let diag = mcts.tree().diagnose();
	assert!(diag.contains("transposition table hits"));
	// With Add/Sub creating cycles, there should be transposition hits
	assert!(mcts.tree().num_nodes() < 10000);
}

#[test]
fn test_no_transposition_table() {
	let mut mcts = MCTSManager::new(
		CountingGame(0),
		NoTranspositionMCTS,
		MyEvaluator,
		UCTPolicy::new(0.5),
		(),
	);
	mcts.playout_n(5000);
	assert_eq!(mcts.best_move().unwrap(), Move::Add);
	let diag = mcts.tree().diagnose();
	assert!(diag.contains("0 transposition table hits"));
}

#[test]
fn test_terminal_state() {
	let mut mcts = MCTSManager::new(
		CountingGame(100),
		CountingMCTS,
		MyEvaluator,
		UCTPolicy::new(0.5),
		ApproxTable::new(1024),
	);
	mcts.playout_n(100);
	assert!(mcts.best_move().is_none());
	assert!(mcts.principal_variation(10).is_empty());
	assert_eq!(mcts.tree().num_nodes(), 1);
}

#[test]
fn test_near_terminal_state() {
	let mut mcts = MCTSManager::new(
		CountingGame(99),
		CountingMCTS,
		MyEvaluator,
		UCTPolicy::new(0.5),
		ApproxTable::new(1024),
	);
	mcts.playout_n(1000);
	assert_eq!(mcts.best_move().unwrap(), Move::Add);
}

#[test]
fn test_playout_parallel_async() {
	let mut mcts = make_counting_mcts();
	let search = mcts.playout_parallel_async(2);
	std::thread::sleep(Duration::from_millis(100));
	search.halt();
	assert!(mcts.tree().num_nodes() > 1);
	assert!(mcts.best_move().is_some());
}

#[test]
fn test_diagnose_output() {
	let mut mcts = make_counting_mcts();
	mcts.playout_n(1000);
	let diag = mcts.tree().diagnose();
	assert!(diag.contains("nodes"));
	assert!(diag.contains("transposition table hits"));
	assert!(diag.contains("expansion contention events"));
	assert!(diag.contains("orphaned nodes"));
}
