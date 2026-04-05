use criterion::{criterion_group, criterion_main, Criterion};
use mcts::transposition_table::*;
use mcts::tree_policy::*;
use mcts::*;

#[derive(Clone)]
struct CountingGame(i64);

#[derive(Clone, Debug)]
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

impl Evaluator<MyMCTS> for MyEvaluator {
    type StateEvaluation = i64;

    fn evaluate_new_state(
        &self,
        state: &CountingGame,
        moves: &Vec<Move>,
        _: Option<SearchHandle<MyMCTS>>,
    ) -> (Vec<()>, i64) {
        (vec![(); moves.len()], state.0)
    }
    fn interpret_evaluation_for_player(&self, evaln: &i64, _: &()) -> i64 {
        *evaln
    }
    fn evaluate_existing_state(
        &self,
        _: &CountingGame,
        evaln: &i64,
        _: SearchHandle<MyMCTS>,
    ) -> i64 {
        *evaln
    }
}

#[derive(Default)]
struct MyMCTS;

impl MCTS for MyMCTS {
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

fn bench_single_threaded(c: &mut Criterion) {
    c.bench_function("playout_n 100k single-threaded", |b| {
        b.iter(|| {
            let mut mcts = MCTSManager::new(
                CountingGame(0),
                MyMCTS,
                MyEvaluator,
                UCTPolicy::new(5.0),
                ApproxTable::new(1024),
            );
            mcts.playout_n(100_000);
        });
    });
}

fn bench_parallel(c: &mut Criterion) {
    c.bench_function("playout_n 100k 4-thread", |b| {
        b.iter(|| {
            let mut mcts = MCTSManager::new(
                CountingGame(0),
                MyMCTS,
                MyEvaluator,
                UCTPolicy::new(5.0),
                ApproxTable::new(1024),
            );
            mcts.playout_n_parallel(100_000, 4);
        });
    });
}

criterion_group!(benches, bench_single_threaded, bench_parallel);
criterion_main!(benches);
