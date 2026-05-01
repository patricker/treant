<p align="center">
  <img src="docs/static/img/wordmark.svg" alt="treant" width="280" />
</p>

**A high-performance, lock-free Monte Carlo Tree Search library for Rust.**

[![Crates.io](https://img.shields.io/crates/v/treant.svg)](https://crates.io/crates/treant)
[![Docs site](https://img.shields.io/badge/docs-mcts.dev-brightgreen)](https://mcts.dev)
[![API Docs](https://img.shields.io/badge/api-docs.rs-blue)](https://docs.rs/treant)
[![License](https://img.shields.io/crates/l/treant.svg)](./LICENSE)

> ### Full documentation & interactive playground → **[mcts.dev](https://mcts.dev)**
>
> Tutorials, how-to guides, concept explainers, API reference, and browser-based demos of every feature. New here? **Start with [What is MCTS?](https://mcts.dev/docs/tutorials/01-what-is-mcts)**.

*Fork and significant modernization of [zxqfl/mcts](https://github.com/zxqfl/mcts) by Jacob Jackson.*

## Features

- **Lock-free parallel search** -- uses `std::thread::scope` with an atomic pointer tree for safe, scalable concurrency
- **UCT and PUCT tree policies** -- classic UCT and AlphaGo/AlphaZero-style PUCT with learned prior probabilities
- **Batched neural network evaluation** -- queue leaf states for GPU inference in bulk
- **MCTS-Solver** -- propagate proven win/loss/draw values up the tree, pruning solved subtrees
- **Score-Bounded MCTS** -- track proven minimax score intervals that tighten during search
- **Chance nodes** -- stochastic transitions (dice rolls, card draws) via open-loop sampling
- **Dirichlet noise** -- root exploration noise for self-play training
- **First Play Urgency (FPU)** -- configurable default value for unvisited children
- **Temperature-based selection** -- soft move selection proportional to visit counts
- **Tree re-rooting** -- preserve search across turns with `advance()`
- **Transposition tables** -- lock-free approximate hash table for DAG-structured games
- **Progressive widening** -- expand children gradually based on visit count
- **Seeded RNG** -- deterministic, reproducible search for debugging and testing

## Quick Start

```rust
use treant::tree_policy::*;
use treant::*;

// A single-player game: count from 0 to 100. Best strategy is always Add.
#[derive(Clone, Debug, PartialEq)]
struct CountingGame(i64);

#[derive(Clone, Debug, PartialEq)]
enum Move { Add, Sub }

impl GameState for CountingGame {
    type Move = Move;
    type Player = ();
    type MoveList = Vec<Move>;

    fn current_player(&self) -> Self::Player { }
    fn available_moves(&self) -> Vec<Move> {
        if self.0 == 100 { vec![] } else { vec![Move::Add, Move::Sub] }
    }
    fn make_move(&mut self, mov: &Self::Move) {
        match *mov {
            Move::Add => self.0 += 1,
            Move::Sub => self.0 -= 1,
        }
    }
}

struct MyEvaluator;

impl Evaluator<MyMCTS> for MyEvaluator {
    type StateEvaluation = i64;

    fn evaluate_new_state(&self, state: &CountingGame, moves: &Vec<Move>,
        _: Option<SearchHandle<MyMCTS>>) -> (Vec<()>, i64) {
        (vec![(); moves.len()], state.0)
    }
    fn interpret_evaluation_for_player(&self, evaln: &i64, _: &()) -> i64 { *evaln }
    fn evaluate_existing_state(&self, _: &CountingGame, evaln: &i64,
        _: SearchHandle<MyMCTS>) -> i64 { *evaln }
}

#[derive(Default)]
struct MyMCTS;

impl MCTS for MyMCTS {
    type State = CountingGame;
    type Eval = MyEvaluator;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();

    fn cycle_behaviour(&self) -> CycleBehaviour<Self> {
        CycleBehaviour::Ignore
    }
}

let game = CountingGame(0);
let mut mcts = MCTSManager::new(game, MyMCTS, MyEvaluator, UCTPolicy::new(0.5), ());
mcts.playout_n_parallel(10000, 4);
assert_eq!(mcts.best_move().unwrap(), Move::Add);
```

## Examples

| Example | Description | Command |
|---------|-------------|---------|
| `counting_game` | Basic MCTS with a transposition table | `cargo run --example counting_game` |
| `nim_solver` | MCTS-Solver proving game-theoretic values | `cargo run --example nim_solver` |
| `dice_game` | Chance nodes with stochastic transitions | `cargo run --example dice_game` |
| `alphazero_basics` | PUCT, Dirichlet noise, temperature selection | `cargo run --example alphazero_basics` |
| `tree_reuse` | Tree re-rooting to preserve search across turns | `cargo run --example tree_reuse` |

## Key Traits

### `GameState`

Define your game: moves, players, and state transitions. Implement `available_moves()` to generate legal moves, `make_move()` to apply them, and `current_player()` to identify whose turn it is. Optional methods support chance nodes (`chance_outcomes`), solver integration (`terminal_value`, `terminal_score`), and progressive widening (`max_children`).

### `Evaluator`

Score leaf nodes during search. `evaluate_new_state` is called when a node is first expanded and returns per-move evaluations (priors for PUCT, `()` for UCT) along with a state evaluation. `interpret_evaluation_for_player` converts the evaluation to a reward from a given player's perspective. Implementations can range from simple heuristics to neural network inference.

### `MCTS`

Wire everything together. Associate your `GameState`, `Evaluator`, `TreePolicy`, and `TranspositionTable` types, and configure search behavior: virtual loss, FPU, solver/score-bounded modes, Dirichlet noise, temperature, depth limits, and RNG seeding.

## Learn more

The material in this README is a taste. Everything else lives at **[mcts.dev](https://mcts.dev)**:

- **[Tutorials](https://mcts.dev/docs/tutorials/01-what-is-mcts)** — from "what is MCTS" through AlphaZero-style PUCT and Gumbel search
- **[How-To Guides](https://mcts.dev/docs/how-to/parallel-search)** — parallel search, tree reuse, custom policies, WASM integration
- **[Concepts](https://mcts.dev/docs/concepts/algorithm)** — algorithm deep dives, exploration-exploitation, lock-free parallelism
- **[Reference](https://mcts.dev/docs/reference/traits)** — full trait signatures, configuration options, glossary
- **[Playground](https://mcts.dev/playground)** — play Tic-Tac-Toe, Connect Four, Nim, and 2048 against the search, all in-browser

## Credits

`treant` is maintained by Peter Wicks. The original MCTS engine was written by Jacob Jackson and published as [zxqfl/mcts](https://github.com/zxqfl/mcts). `treant` continues and extends that work with lock-free parallelism, PUCT/AlphaGo policies, batched neural-net evaluation, MCTS-Solver, Score-Bounded MCTS, chance nodes, Gumbel search, and WASM/dynamic language bindings.

## License

MIT

