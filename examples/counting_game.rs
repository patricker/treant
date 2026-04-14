// Regenerate output: cargo run --example counting_game > examples/output/counting_game.txt
use treant::transposition_table::*;
use treant::tree_policy::*;
use treant::*;

// region: game_definition
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
    type MoveList = Vec<Self::Move>;

    fn current_player(&self) -> Self::Player {}

    fn available_moves(&self) -> Vec<Self::Move> {
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
// endregion: game_definition

// region: transposition_hash
impl TranspositionHash for CountingGame {
    fn hash(&self) -> u64 {
        self.0 as u64
    }
}
// endregion: transposition_hash

// region: evaluator
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

    fn interpret_evaluation_for_player(&self, evaln: &i64, _player: &()) -> i64 {
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
// endregion: evaluator

// region: mcts_config
#[derive(Default)]
struct MyMCTS;

impl MCTS for MyMCTS {
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
// endregion: mcts_config

// region: run_search
fn main() {
    let game = CountingGame(0);
    let mut mcts = MCTSManager::new(
        game,
        MyMCTS,
        MyEvaluator,
        UCTPolicy::new(5.0),
        ApproxTable::new(1024),
    );
    mcts.playout_n(100000);
    let pv: Vec<_> = mcts
        .principal_variation_states(10)
        .into_iter()
        .map(|x| x.0)
        .collect();
    println!("Principal variation: {:?}", pv);
    println!("Evaluation of moves:");
    mcts.tree().debug_moves();
}
// endregion: run_search
