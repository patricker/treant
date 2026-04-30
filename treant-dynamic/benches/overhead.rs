//! Binding overhead benchmarks: native (monomorphized) vs dynamic (trait-object) Mancala.
//!
//! Both implementations share the rules core from `treant_wasm::mancala`. The
//! native side uses the `Mancala` `GameState` impl directly; the dynamic side
//! wraps the same struct in a `GameCallbacks` adapter (string-based moves) to
//! isolate the cost of the dyn dispatch and `String` move encoding.

use criterion::{criterion_group, criterion_main, Criterion};
use treant::tree_policy::AlphaGoPolicy;
use treant::*;
use treant_dynamic::*;
use treant_wasm::mancala::{Mancala, MancalaMove};

const PITS: usize = 6;
const STONES: u8 = 4;
const NUM_PLAYERS: usize = 2;

// ===========================================================================
// Native Mancala — bench-specific evaluator (uniform priors, value 0)
// ===========================================================================

struct BenchMancalaEval;

impl Evaluator<NativeMancalaMCTS> for BenchMancalaEval {
    type StateEvaluation = i64;

    fn evaluate_new_state(
        &self,
        _state: &Mancala,
        moves: &Vec<MancalaMove>,
        _handle: Option<SearchHandle<NativeMancalaMCTS>>,
    ) -> (Vec<f64>, i64) {
        let n = moves.len();
        let priors = if n > 0 {
            vec![1.0 / n as f64; n]
        } else {
            vec![]
        };
        (priors, 0)
    }

    fn evaluate_existing_state(
        &self,
        _state: &Mancala,
        evaln: &i64,
        _handle: SearchHandle<NativeMancalaMCTS>,
    ) -> i64 {
        *evaln
    }

    fn interpret_evaluation_for_player(&self, evaln: &i64, _player: &u8) -> i64 {
        *evaln
    }
}

#[derive(Default)]
struct NativeMancalaMCTS;

impl MCTS for NativeMancalaMCTS {
    type State = Mancala;
    type Eval = BenchMancalaEval;
    type TreePolicy = AlphaGoPolicy;
    type NodeData = ();
    type TranspositionTable = ();
    type ExtraThreadData = ();

    fn cycle_behaviour(&self) -> CycleBehaviour<Self> {
        CycleBehaviour::Ignore
    }

    fn fpu_value(&self) -> f64 {
        0.0
    }
}

// ===========================================================================
// Dynamic Mancala — wraps the same Mancala struct via GameCallbacks
// ===========================================================================

#[derive(Clone)]
struct DynMancala(Mancala);

impl DynMancala {
    fn new() -> Self {
        Self(Mancala::new(PITS, STONES, NUM_PLAYERS))
    }
}

impl GameCallbacks for DynMancala {
    fn clone_box(&self) -> Box<dyn GameCallbacks> {
        Box::new(self.clone())
    }

    fn current_player(&self) -> i32 {
        <Mancala as GameState>::current_player(&self.0) as i32
    }

    fn available_moves(&self) -> Vec<String> {
        <Mancala as GameState>::available_moves(&self.0)
            .into_iter()
            .map(|m| m.0.to_string())
            .collect()
    }

    fn make_move(&mut self, mov: &str) {
        let pit: u8 = mov.parse().unwrap();
        <Mancala as GameState>::make_move(&mut self.0, &MancalaMove(pit));
    }
}

struct DynMancalaEval;

impl EvalCallbacks for DynMancalaEval {
    fn evaluate(&self, _state: &dyn GameCallbacks, moves: &[String]) -> (Vec<f64>, f64) {
        let n = moves.len();
        let priors = if n > 0 {
            vec![1.0 / n as f64; n]
        } else {
            vec![]
        };
        (priors, 0.0)
    }
}

// ===========================================================================
// Native CountingGame — trivial-game baseline
// ===========================================================================

#[derive(Clone)]
struct CountingGame(i64);

#[derive(Clone, Debug, PartialEq)]
enum CountingMove {
    Add,
    Sub,
}

impl std::fmt::Display for CountingMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CountingMove::Add => write!(f, "Add"),
            CountingMove::Sub => write!(f, "Sub"),
        }
    }
}

impl GameState for CountingGame {
    type Move = CountingMove;
    type Player = ();
    type MoveList = Vec<CountingMove>;

    fn current_player(&self) {}

    fn available_moves(&self) -> Vec<CountingMove> {
        if self.0 == 20 {
            vec![]
        } else {
            vec![CountingMove::Add, CountingMove::Sub]
        }
    }

    fn make_move(&mut self, mov: &CountingMove) {
        match *mov {
            CountingMove::Add => self.0 += 1,
            CountingMove::Sub => self.0 -= 1,
        }
    }
}

struct CountingEval;

impl Evaluator<NativeCountingMCTS> for CountingEval {
    type StateEvaluation = i64;

    fn evaluate_new_state(
        &self,
        _state: &CountingGame,
        moves: &Vec<CountingMove>,
        _handle: Option<SearchHandle<NativeCountingMCTS>>,
    ) -> (Vec<f64>, i64) {
        let n = moves.len();
        let priors = if n > 0 {
            vec![1.0 / n as f64; n]
        } else {
            vec![]
        };
        (priors, 0)
    }

    fn evaluate_existing_state(
        &self,
        _state: &CountingGame,
        evaln: &i64,
        _handle: SearchHandle<NativeCountingMCTS>,
    ) -> i64 {
        *evaln
    }

    fn interpret_evaluation_for_player(&self, evaln: &i64, _player: &()) -> i64 {
        *evaln
    }
}

#[derive(Default)]
struct NativeCountingMCTS;

impl MCTS for NativeCountingMCTS {
    type State = CountingGame;
    type Eval = CountingEval;
    type TreePolicy = AlphaGoPolicy;
    type NodeData = ();
    type TranspositionTable = ();
    type ExtraThreadData = ();

    fn cycle_behaviour(&self) -> CycleBehaviour<Self> {
        CycleBehaviour::Ignore
    }

    fn fpu_value(&self) -> f64 {
        0.0
    }
}

// ===========================================================================
// Dynamic CountingGame
// ===========================================================================

#[derive(Clone)]
struct DynCountingGame(i64);

impl GameCallbacks for DynCountingGame {
    fn clone_box(&self) -> Box<dyn GameCallbacks> {
        Box::new(self.clone())
    }

    fn current_player(&self) -> i32 {
        0
    }

    fn available_moves(&self) -> Vec<String> {
        if self.0 == 20 {
            vec![]
        } else {
            vec!["Add".to_string(), "Sub".to_string()]
        }
    }

    fn make_move(&mut self, mov: &str) {
        match mov {
            "Add" => self.0 += 1,
            "Sub" => self.0 -= 1,
            _ => panic!("Unknown move: {}", mov),
        }
    }
}

struct DynCountingEval;

impl EvalCallbacks for DynCountingEval {
    fn evaluate(&self, _state: &dyn GameCallbacks, moves: &[String]) -> (Vec<f64>, f64) {
        let n = moves.len();
        let priors = if n > 0 {
            vec![1.0 / n as f64; n]
        } else {
            vec![]
        };
        (priors, 0.0)
    }
}

// ===========================================================================
// Benchmarks
// ===========================================================================

fn bench_counting_native(c: &mut Criterion) {
    c.bench_function("counting native 10k playouts", |b| {
        b.iter(|| {
            let mut mcts = MCTSManager::new(
                CountingGame(0),
                NativeCountingMCTS,
                CountingEval,
                AlphaGoPolicy::new(1.5),
                (),
            );
            mcts.playout_n(10_000);
        });
    });
}

fn bench_counting_dynamic(c: &mut Criterion) {
    c.bench_function("counting dynamic 10k playouts", |b| {
        b.iter(|| {
            let config = DynConfig {
                exploration_constant: 1.5,
                fpu_value: 0.0,
                ..DynConfig::default()
            };
            let mut mgr = DynMCTSManager::new(
                Box::new(DynCountingGame(0)),
                Box::new(DynCountingEval),
                config,
            );
            mgr.playout_n(10_000);
        });
    });
}

fn bench_mancala_native(c: &mut Criterion) {
    c.bench_function("mancala native 10k playouts", |b| {
        b.iter(|| {
            let mut mcts = MCTSManager::new(
                Mancala::new(PITS, STONES, NUM_PLAYERS),
                NativeMancalaMCTS,
                BenchMancalaEval,
                AlphaGoPolicy::new(1.5),
                (),
            );
            mcts.playout_n(10_000);
        });
    });
}

fn bench_mancala_dynamic(c: &mut Criterion) {
    c.bench_function("mancala dynamic 10k playouts", |b| {
        b.iter(|| {
            let config = DynConfig {
                exploration_constant: 1.5,
                fpu_value: 0.0,
                ..DynConfig::default()
            };
            let mut mgr = DynMCTSManager::new(
                Box::new(DynMancala::new()),
                Box::new(DynMancalaEval),
                config,
            );
            mgr.playout_n(10_000);
        });
    });
}

fn bench_mancala_native_parallel(c: &mut Criterion) {
    c.bench_function("mancala native 10k 4-thread", |b| {
        b.iter(|| {
            let mut mcts = MCTSManager::new(
                Mancala::new(PITS, STONES, NUM_PLAYERS),
                NativeMancalaMCTS,
                BenchMancalaEval,
                AlphaGoPolicy::new(1.5),
                (),
            );
            mcts.playout_n_parallel(10_000, 4);
        });
    });
}

fn bench_mancala_dynamic_parallel(c: &mut Criterion) {
    c.bench_function("mancala dynamic 10k 4-thread", |b| {
        b.iter(|| {
            let config = DynConfig {
                exploration_constant: 1.5,
                fpu_value: 0.0,
                ..DynConfig::default()
            };
            let mut mgr = DynMCTSManager::new(
                Box::new(DynMancala::new()),
                Box::new(DynMancalaEval),
                config,
            );
            mgr.playout_n_parallel(10_000, 4);
        });
    });
}

criterion_group!(
    benches,
    bench_counting_native,
    bench_counting_dynamic,
    bench_mancala_native,
    bench_mancala_dynamic,
    bench_mancala_native_parallel,
    bench_mancala_dynamic_parallel,
);
criterion_main!(benches);
