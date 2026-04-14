---
sidebar_position: 1
id: traits
---

# Core Traits Reference

Complete API reference for every public trait, struct, and enum in the `treant` crate.

---

## `GameState`

Defines the game rules: legal moves, state transitions, and players. See [Tutorial 2](../tutorials/02-first-search.md) for implementation walkthrough.

```rust
pub trait GameState: Clone {
    type Move: Sync + Send + Clone;
    type Player: Sync;
    type MoveList: IntoIterator<Item = Self::Move>;
    // ...
}
```

### Associated types

| Type | Bounds | Purpose |
|---|---|---|
| `Move` | `Sync + Send + Clone` | A game action |
| `Player` | `Sync` | Identifies whose turn it is |
| `MoveList` | `IntoIterator<Item = Move>` | Collection returned by `available_moves()` |

### Required methods

#### `current_player`

```rust
fn current_player(&self) -> Self::Player;
```

Returns the player whose turn it is. Called once per node expansion.

#### `available_moves`

```rust
fn available_moves(&self) -> Self::MoveList;
```

Returns all legal moves from this state. An empty result signals a terminal node. Moves are expanded in the order returned -- put high-priority moves first when using [progressive widening](../how-to/progressive-widening.md).

#### `make_move`

```rust
fn make_move(&mut self, mov: &Self::Move);
```

Apply a move, mutating the state in place. Called during every playout step.

### Provided methods

#### `max_children`

```rust
fn max_children(&self, _visits: u64) -> usize  // default: usize::MAX
```

Maximum children to expand at this node given the current visit count. Override for [progressive widening](../how-to/progressive-widening.md), e.g. `(visits as f64).sqrt() as usize`.

**Parameters:**
- `visits` (`u64`) -- total visits to the parent node

**Returns:** `usize` -- maximum number of children to consider during selection

#### `terminal_value`

```rust
fn terminal_value(&self) -> Option<ProvenValue>  // default: None
```

Classify the outcome of a terminal state for [MCTS-Solver](../concepts/solver-and-bounds.md). The value is from the perspective of the current player (the player who would move next but cannot).

**Returns:** `Some(ProvenValue::Loss)` if the current player has lost, `Some(ProvenValue::Win)` if won, `Some(ProvenValue::Draw)` for draws, `None` to skip solver classification.

#### `terminal_score`

```rust
fn terminal_score(&self) -> Option<i32>  // default: None
```

Exact minimax score of a terminal state for [Score-Bounded MCTS](../concepts/solver-and-bounds.md). From the current player's perspective.

**Returns:** `Some(score)` to set exact bounds on this terminal, `None` to skip.

#### `chance_outcomes`

```rust
fn chance_outcomes(&self) -> Option<Vec<(Self::Move, f64)>>  // default: None
```

If the current state requires a chance event (dice roll, card draw) before the next player decision, return the possible outcomes with their probabilities. Probabilities must be positive and sum to 1.0. Outcomes are applied via `make_move()`. See [Tutorial 5](../tutorials/05-stochastic-games.md) and [Chance Nodes](../concepts/chance-nodes.md).

**Returns:** `Some(vec)` of `(move, probability)` pairs for chance events, `None` for deterministic transitions.

---

## `Evaluator<Spec>`

Evaluates game states during search. Produces state evaluations and per-move evaluations. See [Tutorial 2](../tutorials/02-first-search.md).

```rust
pub trait Evaluator<Spec: MCTS>: Sync {
    type StateEvaluation: Sync + Send;
    // ...
}
```

### Associated types

| Type | Bounds | Purpose |
|---|---|---|
| `StateEvaluation` | `Sync + Send` | The evaluation produced for a game state (e.g., `i64` score, neural network value head output) |

### Required methods

#### `evaluate_new_state`

```rust
fn evaluate_new_state(
    &self,
    state: &Spec::State,
    moves: &MoveList<Spec>,
    handle: Option<SearchHandle<Spec>>,
) -> (Vec<MoveEvaluation<Spec>>, Self::StateEvaluation);
```

Evaluate a newly expanded leaf node. Returns per-move evaluations (one per move, in order) and a state evaluation. The `handle` is `None` for the root node.

**Parameters:**
- `state` -- the game state to evaluate
- `moves` -- legal moves from this state (same order as `available_moves()`)
- `handle` -- search context (node data, thread-local data); `None` at root

**Returns:** `(move_evals, state_eval)` -- move evaluations must match the length of `moves`

#### `evaluate_existing_state`

```rust
fn evaluate_existing_state(
    &self,
    state: &Spec::State,
    existing_evaln: &Self::StateEvaluation,
    handle: SearchHandle<Spec>,
) -> Self::StateEvaluation;
```

Re-evaluate a previously seen state. Called on revisits (e.g., open-loop chance nodes where the same node may be reached through different random outcomes).

#### `interpret_evaluation_for_player`

```rust
fn interpret_evaluation_for_player(
    &self,
    evaluation: &Self::StateEvaluation,
    player: &Player<Spec>,
) -> i64;
```

Convert a state evaluation to a reward from the given player's perspective. Called during backpropagation for each ancestor node.

---

## `MCTS`

Configuration trait for the search. Defines all associated types and tuning parameters. See [Tutorial 2](../tutorials/02-first-search.md) and [Configuration Options](./configuration.md).

```rust
pub trait MCTS: Sized + Send + Sync + 'static {
    type State: GameState + Send + Sync + 'static;
    type Eval: Evaluator<Self> + Send + 'static;
    type TreePolicy: TreePolicy<Self> + Send + 'static;
    type NodeData: Default + Sync + Send + 'static;
    type TranspositionTable: TranspositionTable<Self> + Send + 'static;
    type ExtraThreadData: 'static;
    // ...
}
```

### Associated types

| Type | Bounds | Purpose |
|---|---|---|
| `State` | `GameState + Send + Sync + 'static` | The game state type |
| `Eval` | `Evaluator<Self> + Send + 'static` | The state/move evaluator |
| `TreePolicy` | `TreePolicy<Self> + Send + 'static` | Child selection policy (e.g., `UCTPolicy`, `AlphaGoPolicy`) |
| `NodeData` | `Default + Sync + Send + 'static` | Custom per-node data accessible via `SearchHandle` |
| `TranspositionTable` | `TranspositionTable<Self> + Send + 'static` | Transposition table implementation (use `()` for none) |
| `ExtraThreadData` | `'static` | Custom per-thread data accessible via `SearchHandle` |

### Provided methods

All methods have defaults. See [Configuration Options](./configuration.md) for a consolidated table.

#### `virtual_loss`

```rust
fn virtual_loss(&self) -> i64  // default: 0
```

Virtual loss for [parallel search](../concepts/parallel-mcts.md). Subtracted during descent, added back during backpropagation. Discourages multiple threads from exploring the same path.

#### `fpu_value`

```rust
fn fpu_value(&self) -> f64  // default: f64::INFINITY
```

First Play Urgency. Value assigned to unvisited children during selection. `f64::INFINITY` (default) forces all children to be tried before any revisit. Set to a finite value (e.g., `0.0`) for [neural-network-guided search](../tutorials/06-neural-network-priors.md).

#### `visits_before_expansion`

```rust
fn visits_before_expansion(&self) -> u64  // default: 1
```

Minimum visits to a leaf before expanding it into a tree node.

#### `node_limit`

```rust
fn node_limit(&self) -> usize  // default: usize::MAX
```

Maximum number of tree nodes. Search stops when reached.

#### `select_child_after_search`

```rust
fn select_child_after_search<'a>(
    &self,
    children: &'a [MoveInfo<Self>],
) -> &'a MoveInfo<Self>
```

Select the best child after search completes. Default behavior: prefer proven wins, then proven draws, then highest visit count. Override for custom post-search selection.

#### `max_playout_length`

```rust
fn max_playout_length(&self) -> usize  // default: 1_000_000
```

Safety cap. `playout` panics if a single playout exceeds this depth.

#### `max_playout_depth`

```rust
fn max_playout_depth(&self) -> usize  // default: usize::MAX
```

Quality knob. When exceeded, the current node is evaluated as a leaf instead of descending further. See [Tutorial 7](../tutorials/07-advanced-search.md).

#### `rng_seed`

```rust
fn rng_seed(&self) -> Option<u64>  // default: None
```

Optional RNG seed for deterministic search. Each thread gets a reproducible RNG seeded from `seed + thread_id`.

#### `dirichlet_noise`

```rust
fn dirichlet_noise(&self) -> Option<(f64, f64)>  // default: None
```

Root exploration noise for self-play. Returns `Some((epsilon, alpha))` where the noisy prior = `(1 - epsilon) * prior + epsilon * Dir(alpha)`. Typical: `epsilon=0.25`, `alpha=0.03` (Go), `alpha=0.3` (Chess). Only effective when `TreePolicy::MoveEvaluation` supports noise (e.g., `f64`). See [Tutorial 6](../tutorials/06-neural-network-priors.md).

#### `selection_temperature`

```rust
fn selection_temperature(&self) -> f64  // default: 0.0
```

Temperature for post-search move selection in `best_move()`. `0.0` = argmax by visits. `1.0` = proportional to visits. `principal_variation()` always uses argmax regardless.

#### `solver_enabled`

```rust
fn solver_enabled(&self) -> bool  // default: false
```

Enable [MCTS-Solver](../concepts/solver-and-bounds.md). Proven game-theoretic values (win/loss/draw) propagate up the tree. Requires `GameState::terminal_value()`. See [Tutorial 4](../tutorials/04-solving-games.md).

#### `score_bounded_enabled`

```rust
fn score_bounded_enabled(&self) -> bool  // default: false
```

Enable [Score-Bounded MCTS](../concepts/solver-and-bounds.md). Each node tracks `[lower, upper]` bounds on its minimax value. Requires `GameState::terminal_score()`. Independent of `solver_enabled()`.

#### `closed_loop_chance`

```rust
fn closed_loop_chance(&self) -> bool  // default: false
```

Enable [closed-loop chance nodes](../concepts/chance-nodes.md). Each chance outcome gets its own child in the tree. More accurate per-outcome statistics but larger trees. Requires `GameState::chance_outcomes()`.

#### `on_backpropagation`

```rust
fn on_backpropagation(
    &self,
    _evaln: &StateEvaluation<Self>,
    _handle: SearchHandle<Self>,
)  // default: no-op
```

Called during backpropagation for each node on the playout path. Use for custom statistics accumulation.

#### `cycle_behaviour`

```rust
fn cycle_behaviour(&self) -> CycleBehaviour<Self>
```

How to handle graph cycles caused by transposition tables. Default: `PanicWhenCycleDetected` if a transposition table is configured, `Ignore` otherwise.

---

## `TreePolicy<Spec>`

Selects which child to explore during tree traversal. See [Tree Policies](../concepts/tree-policies.md) and [Custom Tree Policy](../how-to/custom-tree-policy.md).

```rust
pub trait TreePolicy<Spec: MCTS<TreePolicy = Self>>: Sync + Sized {
    type MoveEvaluation: Sync + Send + Default;
    type ThreadLocalData: Default;
    // ...
}
```

### Associated types

| Type | Bounds | Purpose |
|---|---|---|
| `MoveEvaluation` | `Sync + Send + Default` | Per-move evaluation (e.g., `()` for UCT, `f64` prior for PUCT) |
| `ThreadLocalData` | `Default` | Thread-local policy data (e.g., RNG for tie-breaking) |

### Required methods

#### `choose_child`

```rust
fn choose_child<'a, MoveIter>(
    &self,
    moves: MoveIter,
    handle: SearchHandle<Spec>,
) -> &'a MoveInfo<Spec>
where
    MoveIter: Iterator<Item = &'a MoveInfo<Spec>> + Clone;
```

Select the most promising child to explore during a playout. Called at every internal node during selection.

### Provided methods

#### `validate_evaluations`

```rust
fn validate_evaluations(&self, _evalns: &[Self::MoveEvaluation])  // default: no-op
```

Validate move evaluations after node creation (e.g., check priors sum to 1). Called once per node expansion.

#### `seed_thread_data`

```rust
fn seed_thread_data(&self, _tld: &mut Self::ThreadLocalData, _seed: u64)  // default: no-op
```

Seed the thread-local data for deterministic search. Called when `MCTS::rng_seed()` is set.

#### `compare_move_evaluations`

```rust
fn compare_move_evaluations(
    &self,
    _a: &Self::MoveEvaluation,
    _b: &Self::MoveEvaluation,
) -> std::cmp::Ordering  // default: Equal
```

Compare two move evaluations for ordering during [progressive widening](../how-to/progressive-widening.md). Higher-priority moves should sort first (return `Greater` for higher priority `a`).

#### `apply_dirichlet_noise`

```rust
fn apply_dirichlet_noise(
    &self,
    _moves: &mut [MoveInfo<Spec>],
    _epsilon: f64,
    _alpha: f64,
    _rng: &mut SmallRng,
)  // default: no-op
```

Apply Dirichlet noise to root move evaluations for self-play exploration. Only meaningful when `MoveEvaluation` is numeric.

---

## `TranspositionTable<Spec>` (unsafe trait)

Maps game states to search nodes for graph-structured search. See [Transposition Tables](../concepts/architecture.md). Use `()` for no transposition table.

```rust
pub unsafe trait TranspositionTable<Spec: MCTS>: Sync + Sized { ... }
```

**Safety:** If `insert` inserts a value, it **must** return `None`. Violating this causes memory unsafety (double-free).

### Required methods

#### `insert`

```rust
fn insert<'a>(
    &'a self,
    key: &Spec::State,
    value: &'a SearchNode<Spec>,
    handle: SearchHandle<Spec>,
) -> Option<&'a SearchNode<Spec>>;
```

Attempt to insert a key/value pair. If the key is inserted, **must return `None`**. If the key already exists, may return `Some(existing_node)` or `None`. The table may silently drop entries or return approximate matches.

#### `lookup`

```rust
fn lookup<'a>(
    &'a self,
    key: &Spec::State,
    handle: SearchHandle<Spec>,
) -> Option<&'a SearchNode<Spec>>;
```

Look up a key. Returns `Some(node)` if found, `None` otherwise. May return `None` even for present keys (approximate).

### Provided methods

#### `clear`

```rust
fn clear(&mut self)  // default: no-op
```

Clear all entries. Called during tree re-rooting (`advance()`) to prevent dangling pointers.

### Built-in implementations

- **`()`** -- no-op transposition table. `insert` and `lookup` always return `None`. Zero overhead.

---

## `TranspositionHash`

Hash trait for game states used with `ApproxTable`. See [Transposition Tables](../concepts/architecture.md).

```rust
pub trait TranspositionHash {
    fn hash(&self) -> u64;
}
```

#### `hash`

```rust
fn hash(&self) -> u64;
```

Compute a hash of this game state. Equal states must produce equal hashes. Hash `0` is reserved and will not be inserted into the table.

---

## `BatchEvaluator<Spec>`

Batch evaluation interface for GPU-accelerated neural networks. See [Batched Evaluation](../how-to/batched-evaluation.md).

```rust
pub trait BatchEvaluator<Spec: MCTS>: Send + Sync + 'static {
    type StateEvaluation: Sync + Send + Clone;
    // ...
}
```

### Associated types

| Type | Bounds | Purpose |
|---|---|---|
| `StateEvaluation` | `Sync + Send + Clone` | State evaluation type (must be `Clone` for batched path) |

### Required methods

#### `evaluate_batch`

```rust
fn evaluate_batch(
    &self,
    states: &[(Spec::State, MoveList<Spec>)],
) -> Vec<(Vec<MoveEvaluation<Spec>>, Self::StateEvaluation)>;
```

Evaluate a batch of newly expanded leaf nodes. Each input is a `(state, moves)` pair. Returns a `Vec` of the same length, each element being `(move_evaluations, state_evaluation)`.

#### `interpret_evaluation_for_player`

```rust
fn interpret_evaluation_for_player(
    &self,
    evaluation: &Self::StateEvaluation,
    player: &Player<Spec>,
) -> i64;
```

Convert a state evaluation to a score from a specific player's perspective.

### Provided methods

#### `evaluate_existing_state`

```rust
fn evaluate_existing_state(
    &self,
    _state: &Spec::State,
    existing_evaln: &Self::StateEvaluation,
) -> Self::StateEvaluation  // default: existing_evaln.clone()
```

Re-evaluate a node that has already been evaluated. Called synchronously (not batched). Default clones the existing evaluation.

---

## `MCTSManager<Spec>`

Main entry point for running MCTS search. Owns the search tree and provides methods for running playouts and extracting results. See [Tutorial 2](../tutorials/02-first-search.md).

### Constructor

#### `new`

```rust
pub fn new(
    state: Spec::State,
    manager: Spec,
    eval: Spec::Eval,
    tree_policy: Spec::TreePolicy,
    table: Spec::TranspositionTable,
) -> Self
```

Create a new search manager.

**Parameters:**
- `state` -- initial game state (becomes the root)
- `manager` -- MCTS configuration (the `Spec` impl)
- `eval` -- evaluator instance
- `tree_policy` -- tree policy instance
- `table` -- transposition table instance (use `()` for none)

### Search methods

#### `playout`

```rust
pub fn playout(&mut self)
```

Run a single playout (single-threaded). Descends from root, expands a leaf, evaluates, and backpropagates.

#### `playout_n`

```rust
pub fn playout_n(&mut self, n: u64)
```

Run `n` playouts sequentially on the calling thread.

#### `playout_n_parallel`

```rust
pub fn playout_n_parallel(&mut self, n: u32, num_threads: usize)
```

Run `n` playouts distributed across `num_threads` using scoped threads. Blocks until all playouts complete.

**Parameters:**
- `n` (`u32`) -- total number of playouts
- `num_threads` (`usize`) -- number of search threads (must be > 0)

#### `playout_parallel_for`

```rust
pub fn playout_parallel_for(&mut self, duration: Duration, num_threads: usize)
```

Run parallel search for the given duration. Blocks until time expires.

**Parameters:**
- `duration` (`Duration`) -- how long to search
- `num_threads` (`usize`) -- number of search threads (must be > 0)

#### `playout_until`

```rust
pub fn playout_until<Predicate: FnMut() -> bool>(&mut self, pred: Predicate)
```

Run single-threaded playouts until the predicate returns `true`. Checked between each playout.

#### `playout_parallel_async`

```rust
pub fn playout_parallel_async<'a>(
    &'a mut self,
    num_threads: usize,
) -> AsyncSearch<'a, Spec>
```

Start asynchronous parallel search. Returns an `AsyncSearch` handle; search stops when the handle is dropped or `.halt()` is called.

#### `into_playout_parallel_async`

```rust
pub fn into_playout_parallel_async(
    self,
    num_threads: usize,
) -> AsyncSearchOwned<Spec>
```

Like `playout_parallel_async`, but takes ownership. Call `.halt()` on the returned `AsyncSearchOwned` to recover the manager.

### Result methods

#### `best_move`

```rust
pub fn best_move(&self) -> Option<Move<Spec>>
```

The best move found by search. Uses temperature-based selection if `selection_temperature() > 0`. Returns `None` if no moves were explored.

#### `principal_variation`

```rust
pub fn principal_variation(&self, num_moves: usize) -> Vec<Move<Spec>>
```

The best sequence of moves found by search, up to `num_moves` deep. Always uses argmax (ignores temperature).

#### `principal_variation_states`

```rust
pub fn principal_variation_states(&self, num_moves: usize) -> Vec<Spec::State>
```

The principal variation as a sequence of game states. Returns `num_moves + 1` states (starting with the root state).

#### `principal_variation_info`

```rust
pub fn principal_variation_info(
    &self,
    num_moves: usize,
) -> Vec<MoveInfoHandle<'_, Spec>>
```

The principal variation with full `MoveInfo` handles for inspecting visit counts and rewards.

#### `root_child_stats`

```rust
pub fn root_child_stats(&self) -> Vec<ChildStats<Spec>>
where
    MoveEvaluation<Spec>: Clone
```

Visit counts, average rewards, move evaluations, proven values, and score bounds for all root children.

#### `root_proven_value`

```rust
pub fn root_proven_value(&self) -> ProvenValue
```

The proven game-theoretic value of the root node. Meaningful only when `solver_enabled()` is true.

#### `root_score_bounds`

```rust
pub fn root_score_bounds(&self) -> ScoreBounds
```

The score bounds of the root node. Meaningful only when `score_bounded_enabled()` is true.

#### `tree`

```rust
pub fn tree(&self) -> &SearchTree<Spec>
```

Access the underlying `SearchTree` for diagnostics (`num_nodes()`, `diagnose()`, `debug_moves()`).

### Lifecycle methods

#### `reset`

```rust
pub fn reset(self) -> Self
```

Reset the search tree, keeping the same game state and configuration. Consumes and returns self. Panics if async search is running.

#### `advance`

```rust
pub fn advance(&mut self, mov: &Move<Spec>) -> Result<(), AdvanceError>
where
    Move<Spec>: PartialEq
```

Commit to a move: advance the root and preserve the subtree below it. See [Tree Reuse](../how-to/tree-reuse.md). Panics if async search is running.

**Errors:**
- `AdvanceError::MoveNotFound` -- the move does not exist among root children
- `AdvanceError::ChildNotExpanded` -- the child node was never expanded during search
- `AdvanceError::ChildNotOwned` -- the child is a transposition table alias

#### `print_on_playout_error`

```rust
pub fn print_on_playout_error(&mut self, v: bool) -> &mut Self
```

Control whether node-limit messages are printed to stderr. Default: `true`.

### Benchmarking

#### `perf_test`

```rust
pub fn perf_test<F>(&mut self, num_threads: usize, f: F)
where
    F: FnMut(usize)
```

Run a 10-second performance benchmark, calling `f` with nodes/sec each second.

#### `perf_test_to_stderr`

```rust
pub fn perf_test_to_stderr(&mut self, num_threads: usize)
```

Run `perf_test` and print results to stderr.

---

## `ProvenValue`

Game-theoretic proven value for [MCTS-Solver](../concepts/solver-and-bounds.md). Stored from the perspective of the player who moved to reach this node.

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProvenValue {
    Unknown = 0,
    Win = 1,
    Loss = 2,
    Draw = 3,
}
```

| Variant | Meaning |
|---|---|
| `Unknown` | Not yet proven |
| `Win` | Current player wins with perfect play |
| `Loss` | Current player loses with perfect play |
| `Draw` | Game is drawn with perfect play |

### Methods

#### `from_u8`

```rust
pub fn from_u8(v: u8) -> Self
```

Convert from raw `u8`. Returns `Unknown` for unrecognized values.

---

## `ScoreBounds`

Proven score interval for [Score-Bounded MCTS](../concepts/solver-and-bounds.md). Tracks `[lower, upper]` bounds on the true minimax value.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScoreBounds {
    pub lower: i32,
    pub upper: i32,
}
```

| Field | Type | Meaning |
|---|---|---|
| `lower` | `i32` | Lower bound on minimax value (current player's perspective) |
| `upper` | `i32` | Upper bound on minimax value (current player's perspective) |

### Constants

#### `UNBOUNDED`

```rust
pub const UNBOUNDED: Self = Self { lower: i32::MIN, upper: i32::MAX };
```

No bounds known.

### Methods

#### `exact`

```rust
pub fn exact(v: i32) -> Self
```

Create bounds where `lower == upper == v`.

#### `is_proven`

```rust
pub fn is_proven(&self) -> bool
```

Returns `true` when bounds have converged (`lower == upper`).

---

## `CycleBehaviour<Spec>`

Strategy for handling graph cycles caused by transposition tables.

```rust
pub enum CycleBehaviour<Spec: MCTS> {
    Ignore,
    UseCurrentEvalWhenCycleDetected,
    PanicWhenCycleDetected,
    UseThisEvalWhenCycleDetected(StateEvaluation<Spec>),
}
```

| Variant | Behavior |
|---|---|
| `Ignore` | No cycle detection. May cause infinite loops without depth limits. |
| `UseCurrentEvalWhenCycleDetected` | Break the cycle and evaluate the current node as a leaf. |
| `PanicWhenCycleDetected` | Panic on cycle detection. Useful for debugging. |
| `UseThisEvalWhenCycleDetected(eval)` | Break the cycle and use the provided evaluation. |

---

## `ChildStats<Spec>`

Summary statistics for a root child, returned by `MCTSManager::root_child_stats()`.

```rust
pub struct ChildStats<Spec: MCTS> {
    pub mov: Move<Spec>,
    pub visits: u64,
    pub avg_reward: f64,
    pub move_evaluation: MoveEvaluation<Spec>,
    pub proven_value: ProvenValue,
    pub score_bounds: ScoreBounds,
}
```

| Field | Type | Meaning |
|---|---|---|
| `mov` | `Move<Spec>` | The move |
| `visits` | `u64` | Number of times this move was selected |
| `avg_reward` | `f64` | Average backpropagated reward |
| `move_evaluation` | `MoveEvaluation<Spec>` | Tree policy's evaluation (e.g., prior probability) |
| `proven_value` | `ProvenValue` | Proven value of the child node |
| `score_bounds` | `ScoreBounds` | Score bounds of the child node |

---

## `UCTPolicy`

Classic UCB1 tree policy. Balances exploitation and exploration using `Q(a) + C * sqrt(2 * ln(N) / n(a))`. Move evaluations are `()` -- all moves start equal. See [Tree Policies](../concepts/tree-policies.md).

### Constructor

#### `new`

```rust
pub fn new(exploration_constant: f64) -> Self
```

Create a UCT policy with the given exploration constant `C`. Typical values: 0.5--2.0. Higher values explore more. Panics if `C <= 0`.

### Methods

#### `exploration_constant`

```rust
pub fn exploration_constant(&self) -> f64
```

Returns the exploration constant `C`.

### TreePolicy impl

| Associated type | Value |
|---|---|
| `MoveEvaluation` | `()` |
| `ThreadLocalData` | `PolicyRng` |

---

## `AlphaGoPolicy`

PUCT tree policy used by AlphaGo/AlphaZero. Selects children using `(Q(a) + C * P(a) * sqrt(N)) / (1 + n(a))`, where `P(a)` is the prior probability from a neural network. See [Tree Policies](../concepts/tree-policies.md) and [Tutorial 6](../tutorials/06-neural-network-priors.md).

### Constructor

#### `new`

```rust
pub fn new(exploration_constant: f64) -> Self
```

Create a PUCT policy with the given exploration constant `C`. Typical values: 1.0--2.5. Panics if `C <= 0`.

### Methods

#### `exploration_constant`

```rust
pub fn exploration_constant(&self) -> f64
```

Returns the exploration constant `C`.

### TreePolicy impl

| Associated type | Value |
|---|---|
| `MoveEvaluation` | `f64` (prior probability, must be non-negative, should sum to ~1.0) |
| `ThreadLocalData` | `PolicyRng` |

`validate_evaluations` asserts all priors are non-negative and sum to approximately 1.0. `compare_move_evaluations` sorts higher priors first (for progressive widening). `apply_dirichlet_noise` blends priors with Dirichlet noise: `(1 - epsilon) * prior + epsilon * Dir(alpha)`.

---

## `ApproxTable<Spec>`

Lock-free approximate transposition table using quadratic probing. Type alias for `ApproxQuadraticProbingHashTable<Spec::State, SearchNode<Spec>>`.

Requires `Spec::State: TranspositionHash`.

### Constructor

#### `new`

```rust
pub fn new(capacity: usize) -> Self
```

Create a table with the given capacity. **Capacity must be a power of 2.** Panics otherwise.

#### `enough_to_hold`

```rust
pub fn enough_to_hold(num: usize) -> Self
```

Create a table large enough to hold `num` entries with room to spare (approximately `1.5x` capacity).

---

## `BatchedEvaluatorBridge<Spec, B>`

Adapter that wraps a `BatchEvaluator` into an `Evaluator`. Search threads enqueue leaf states and block until the batch collector processes them. See [Batched Evaluation](../how-to/batched-evaluation.md).

### Constructor

#### `new`

```rust
pub fn new(batch_eval: B, config: BatchConfig) -> Self
```

Create a bridge with the given batch evaluator and configuration. Spawns a dedicated collector thread.

**Parameters:**
- `batch_eval` (`B`) -- the batch evaluator implementation
- `config` (`BatchConfig`) -- batch size and timing configuration

---

## `BatchConfig`

Configuration for batched evaluation.

```rust
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub max_wait: Duration,
}
```

| Field | Type | Default | Purpose |
|---|---|---|---|
| `max_batch_size` | `usize` | `8` | Maximum leaves per batch |
| `max_wait` | `Duration` | `1ms` | Maximum time to wait for a full batch after the first request |

---

## `MoveInfo<Spec>`

Information about a single move edge in the search tree. Provides access to visit statistics, rewards, and the child node.

### Methods

#### `get_move`

```rust
pub fn get_move(&self) -> &Move<Spec>
```

The move this edge represents.

#### `move_evaluation`

```rust
pub fn move_evaluation(&self) -> &MoveEvaluation<Spec>
```

The tree policy's evaluation of this move (e.g., prior probability for PUCT, `()` for UCT).

#### `visits`

```rust
pub fn visits(&self) -> u64
```

Number of times this move has been selected during search.

#### `sum_rewards`

```rust
pub fn sum_rewards(&self) -> i64
```

Sum of backpropagated rewards through this move.

#### `child`

```rust
pub fn child(&self) -> Option<NodeHandle<'_, Spec>>
```

The child node reached by this move, if expanded.

#### `child_proven_value`

```rust
pub fn child_proven_value(&self) -> ProvenValue
```

Proven value of the child node. Returns `ProvenValue::Unknown` if unexpanded.

#### `child_score_bounds`

```rust
pub fn child_score_bounds(&self) -> ScoreBounds
```

Score bounds of the child node. Returns `ScoreBounds::UNBOUNDED` if unexpanded.

---

## `SearchHandle<'a, Spec>`

Handle passed to evaluators and callbacks during search. Provides access to the current node, thread-local data, and MCTS configuration.

### Methods

#### `node`

```rust
pub fn node(&self) -> NodeHandle<'a, Spec>
```

The current search node.

#### `thread_data`

```rust
pub fn thread_data(&mut self) -> &mut ThreadData<Spec>
```

Mutable access to thread-local data (`policy_data` + `extra_data`).

#### `mcts`

```rust
pub fn mcts(&self) -> &'a Spec
```

The MCTS configuration.

---

## `NodeHandle<'a, Spec>`

Immutable handle to a search node.

### Methods

#### `data`

```rust
pub fn data(&self) -> &'a Spec::NodeData
```

User-defined node data.

#### `moves`

```rust
pub fn moves(&self) -> Moves<'_, Spec>
```

Iterator over this node's `MoveInfo` entries.

#### `proven_value`

```rust
pub fn proven_value(&self) -> ProvenValue
```

The proven game-theoretic value of this node.

#### `score_bounds`

```rust
pub fn score_bounds(&self) -> ScoreBounds
```

The proven score bounds of this node.

---

## `AdvanceError`

Error returned by `MCTSManager::advance()`.

```rust
pub enum AdvanceError {
    MoveNotFound,
    ChildNotExpanded,
    ChildNotOwned,
}
```

| Variant | Meaning |
|---|---|
| `MoveNotFound` | The move does not exist among root children |
| `ChildNotExpanded` | The child node was never expanded during search |
| `ChildNotOwned` | The child is a transposition table alias and cannot be detached |

---

## Type aliases

| Alias | Expands to | Purpose |
|---|---|---|
| `MoveEvaluation<Spec>` | `<Spec::TreePolicy as TreePolicy<Spec>>::MoveEvaluation` | Per-move evaluation type |
| `StateEvaluation<Spec>` | `<Spec::Eval as Evaluator<Spec>>::StateEvaluation` | State evaluation type |
| `Move<Spec>` | `<Spec::State as GameState>::Move` | Move type |
| `MoveList<Spec>` | `<Spec::State as GameState>::MoveList` | Move list type |
| `Player<Spec>` | `<Spec::State as GameState>::Player` | Player type |
| `TreePolicyThreadData<Spec>` | `<Spec::TreePolicy as TreePolicy<Spec>>::ThreadLocalData` | Policy thread-local data type |
| `MoveInfoHandle<'a, Spec>` | `&'a MoveInfo<Spec>` | Borrowed reference to a `MoveInfo` |

---

# `treant-gumbel` Crate Reference

The `treant-gumbel` crate provides Gumbel MuZero search as a standalone search engine. It reuses `GameState` from the core crate but has its own evaluator trait, search manager, and result types. See [Gumbel Search tutorial](../tutorials/08-gumbel-search.md) and [Tree Policies](../concepts/tree-policies.md).

---

## `GumbelEvaluator<G>`

Evaluator providing policy logits and value estimates. Simpler than the core crate's `Evaluator` -- returns a `(Vec<f64>, f64)` tuple.

```rust
pub trait GumbelEvaluator<G: GameState>: Send {
    fn evaluate(&self, state: &G, moves: &[G::Move]) -> (Vec<f64>, f64);
}
```

### Required methods

#### `evaluate`

```rust
fn evaluate(&self, state: &G, moves: &[G::Move]) -> (Vec<f64>, f64);
```

Evaluate a game state. Returns `(logits, value)`.

**Parameters:**
- `state` -- the game state to evaluate
- `moves` -- available moves (same order as `GameState::available_moves()`)

**Returns:** `(logits, value)` where:
- `logits`: one `f64` per move, unnormalized log-probabilities (policy head output)
- `value`: state value for the current player, in `[-1.0, 1.0]` (value head output)

---

## `GumbelConfig`

Configuration for Gumbel search. Derives `Clone`, `Copy`, `Debug`.

```rust
pub struct GumbelConfig {
    pub m_actions: usize,
    pub c_puct: f64,
    pub max_depth: usize,
    pub value_scale: f64,
    pub seed: u64,
}
```

| Field | Type | Default | Purpose |
|---|---|---|---|
| `m_actions` | `usize` | `16` | Number of actions after Gumbel-Top-k sampling |
| `c_puct` | `f64` | `1.25` | PUCT exploration constant for below-root traversal |
| `max_depth` | `usize` | `200` | Maximum search depth per simulation |
| `value_scale` | `f64` | `50.0` | Scale factor mapping Q-values to logit scale (`c_visit` in the paper) |
| `seed` | `u64` | `42` | RNG seed for Gumbel noise sampling |

---

## `GumbelSearch<G, E>`

Gumbel MCTS search engine. Single-threaded, two-player zero-sum (negamax).

### Constructor

#### `new`

```rust
pub fn new(evaluator: E, config: GumbelConfig) -> Self
```

Create a new search engine.

### Methods

#### `search`

```rust
pub fn search(&mut self, state: &G, n_simulations: u32) -> SearchResult<G::Move>
```

Run Gumbel search from the given state. Panics if the state is terminal.

**Parameters:**
- `state` -- root game state (not modified)
- `n_simulations` -- total simulation budget

**Returns:** `SearchResult` with best move, value, and per-move statistics.

#### `set_seed`

```rust
pub fn set_seed(&mut self, seed: u64)
```

Reset the RNG for reproducible searches.

#### `evaluator`

```rust
pub fn evaluator(&self) -> &E
```

Access the evaluator.

#### `config`

```rust
pub fn config(&self) -> &GumbelConfig
```

Access the configuration.

---

## `SearchResult<M>`

Result of a Gumbel search. `#[must_use]`.

```rust
pub struct SearchResult<M: Clone> {
    pub best_move: M,
    pub root_value: f64,
    pub move_stats: Vec<MoveStats<M>>,
    pub simulations_used: u32,
}
```

| Field | Type | Purpose |
|---|---|---|
| `best_move` | `M` | Best move found by search |
| `root_value` | `f64` | Value estimate for root state's current player |
| `move_stats` | `Vec<MoveStats<M>>` | Per-move statistics (one per legal move) |
| `simulations_used` | `u32` | Total simulations actually used |

---

## `MoveStats<M>`

Per-move statistics from Gumbel search.

```rust
pub struct MoveStats<M: Clone> {
    pub mov: M,
    pub visits: u32,
    pub completed_q: f64,
    pub improved_policy: f64,
}
```

| Field | Type | Purpose |
|---|---|---|
| `mov` | `M` | The move |
| `visits` | `u32` | Simulations allocated to this move |
| `completed_q` | `f64` | Completed Q-value (empirical mean if visited, root value estimate otherwise) |
| `improved_policy` | `f64` | Gumbel-improved policy probability (sums to 1.0 across all moves) |
