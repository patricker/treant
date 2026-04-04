---
sidebar_position: 7
id: architecture
---

# Library Architecture

The mcts crate separates concerns into three independent traits: what the game does, how positions are evaluated, and how children are selected. This separation means you can swap any component without touching the others. The same game works with random rollouts or neural networks. The same evaluator works with UCT or PUCT. The same tree policy works with chess or Go.

This page explains the structural decisions behind the library and the tradeoffs that shaped them.

## The three-trait design

### GameState: the rules

`GameState` defines the game mechanics: available moves, state transitions, player turns. It is the only trait that knows what the game is. Everything else is generic over it.

The trait is deliberately minimal. `available_moves()`, `make_move()`, `current_player()`. Optional methods add support for solver classification (`terminal_value`, `terminal_score`), stochastic transitions (`chance_outcomes`), and progressive widening (`max_children`). A game that needs none of these features implements three methods and ignores the rest.

`GameState` requires `Clone` because each playout needs its own copy of the state. The search clones the root state at the start of each playout and mutates the clone as it descends. This is simpler and often faster than implementing undo operations.

### Evaluator: the heuristics

`Evaluator` produces two outputs: a per-move evaluation (used by the tree policy) and a state evaluation (backpropagated as the reward signal).

For UCT, the per-move evaluation is `()` -- all moves start equal. For PUCT, it is `f64` -- a prior probability for each move. The type is controlled by the tree policy's associated type `MoveEvaluation`, ensuring that the evaluator produces what the policy expects.

`evaluate_new_state` is called once when a node is created. `evaluate_existing_state` is called on every subsequent visit. In open-loop chance games, these visits may have different underlying states (see [Open-Loop vs Closed-Loop](./chance-nodes)), so the method receives both the stored evaluation and the current state.

`interpret_evaluation_for_player` converts a state evaluation into a numeric reward for a specific player. In two-player zero-sum games, this typically negates the evaluation for the opponent. In cooperative or single-player games, it may return the same value for all players.

### TreePolicy: the selection rule

`TreePolicy` implements the UCB formula (or any alternative). It receives an iterator over children and a `SearchHandle`, and returns the child to explore. The `SearchHandle` provides access to the MCTS configuration (for reading `fpu_value`, `solver_enabled`, etc.) and thread-local data (for RNG access).

Two implementations are built in:

- `UCTPolicy`: classic UCB1 with `MoveEvaluation = ()`.
- `AlphaGoPolicy`: PUCT with `MoveEvaluation = f64` prior probabilities.

Custom tree policies implement the same trait. The policy sees move evaluations, visit counts, and reward sums. It does not see the game state or the evaluator -- those concerns are separated.

## The MCTS configuration trait

The `MCTS` trait wires everything together with associated types:

```rust
pub trait MCTS: Sized + Send + Sync + 'static {
    type State: GameState + Send + Sync;
    type Eval: Evaluator<Self> + Send;
    type TreePolicy: TreePolicy<Self> + Send;
    type NodeData: Default + Sync + Send;
    type TranspositionTable: TranspositionTable<Self> + Send;
    type ExtraThreadData: 'static;
    // ... methods with defaults ...
}
```

A typical implementation is a zero-sized struct that exists only to specify the type relationships:

```rust
#[derive(Default)]
struct MyMCTS;

impl MCTS for MyMCTS {
    type State = MyGame;
    type Eval = MyEvaluator;
    type TreePolicy = UCTPolicy;
    type NodeData = ();
    type ExtraThreadData = ();
    type TranspositionTable = ();
    // All methods use defaults
}
```

The struct carries no data. It is a compile-time wiring diagram. The associated types ensure that `MyEvaluator` produces evaluations compatible with `UCTPolicy`, that `MyGame` implements the `GameState` trait, and that all types satisfy the thread-safety bounds required by parallel search.

The method defaults on the `MCTS` trait control feature flags (`solver_enabled`, `score_bounded_enabled`, `closed_loop_chance`) and tuning parameters (`virtual_loss`, `fpu_value`, `selection_temperature`). Override only what you need.

## SearchTree internals

`SearchTree` owns the root node, the game state, the evaluator, the tree policy, and the transposition table. It provides the `playout` method that drives each iteration.

### Node structure

`SearchNode` contains:
- A `Vec<MoveInfo>` of child edges. Each `MoveInfo` holds the move, its evaluation, visit statistics (`AtomicUsize` visits, `AtomicI64` reward sum), and an `AtomicPtr` to the child `SearchNode`.
- The state evaluation from the `Evaluator`.
- User-defined `NodeData`.
- Solver state: `AtomicU8` proven value, `AtomicI32` lower/upper score bounds.
- Chance node metadata: `is_chance` flag and probability vector.

### Expansion and ownership

Each `MoveInfo` has an `owned` flag. When a thread creates a new child node (via compare-and-swap on the `AtomicPtr`), it sets `owned = true`. When the tree is dropped, only owned children are freed. Non-owned children are aliases from the transposition table -- freeing them would be a double-free.

If two threads try to expand the same child simultaneously, compare-and-swap ensures exactly one succeeds. The loser drops its node and uses the winner's. If a transposition table hit occurs after expansion, the newly-created node is moved to an orphan list and the transposition table's existing node is used instead. The orphan list is protected by a `Mutex`, but this is off the hot path (it happens only on delayed transposition hits, which are rare).

### Cascade drop

Deep trees can have millions of nodes. Recursive drop would overflow the stack. Instead, `MoveInfo::drop` checks the `owned` flag and drops the child via `Box::from_raw`. Rust's default recursive drop of the `Vec<MoveInfo>` inside each `SearchNode` naturally cascades. For trees that are very deep, the iterative traversal in `advance_root` (which clears the transposition table and drops the old root) ensures the stack stays bounded.

## Memory model

Nodes are heap-allocated via `Box` and never moved. Once created, a node's address is stable for the lifetime of the tree. `AtomicPtr` references to these nodes are always valid. No garbage collection, no reference counting, no epoch-based reclamation.

This works because MCTS trees only grow during search. Nodes are never deleted until the entire tree is dropped (or the root is advanced, which drops sibling subtrees). The monotonically-growing tree is a natural fit for the "allocate, never move, free all at once" pattern.

The downside is that memory usage only increases during search. A long search accumulates nodes until the node limit is reached or the tree is reset. For applications that reuse the tree across moves (`advance`), only the selected subtree is retained -- siblings are freed.

## Feature zero-cost

The solver and score-bounded features add fields to every node (`AtomicU8`, two `AtomicI32`s) regardless of whether they are enabled. This costs 9 bytes per node even when unused.

The runtime cost, however, is zero when disabled. All solver and bounds code is guarded by `if self.manager.solver_enabled()` or `if self.manager.score_bounded_enabled()`. Since these methods return constant `false` by default, the compiler eliminates the dead branches entirely. The check compiles to nothing.

This is a deliberate tradeoff: a small per-node memory cost (9 bytes on a node that is typically 100+ bytes) in exchange for zero runtime overhead and zero API complexity. Users who do not need the solver never interact with it.

## TranspositionTable

The transposition table maps game states to existing tree nodes. When the search reaches a position that was already expanded elsewhere in the tree, the table provides a pointer to the existing node instead of creating a duplicate.

The `TranspositionTable` trait is `unsafe` because implementors must follow a strict invariant: **if `insert` stores the value, it must return `None`.** Violating this causes a double-free. The table controls ownership transfer: returning `None` tells the caller "I took it, do not free it." Returning `Some(existing)` tells the caller "I did not take it, use this existing node instead."

The built-in `ApproxTable` uses quadratic probing with 16-byte entries: an `AtomicU64` key (the state hash) and an `AtomicPtr<SearchNode>` value. The table is approximate -- hash collisions are not resolved precisely. Two states with the same 64-bit hash share a tree node. This is by design: in MCTS, occasional incorrect sharing between similar positions is acceptable because the statistics converge regardless. Precise collision resolution would require storing the full game state in each entry, which is far more expensive.

The unit type `()` implements `TranspositionTable` as a no-op: no storage, no lookups. This is the default when you do not need transpositions.

## BatchedEvaluatorBridge

Neural network inference is dramatically faster when batched. A single forward pass evaluating 32 positions takes about the same GPU time as evaluating 1. The `BatchedEvaluatorBridge` bridges the gap between MCTS's one-leaf-at-a-time playout model and the batch evaluator's many-at-once model.

The bridge runs a dedicated collector thread. Search threads encountering a new leaf send their state to the collector via a channel and block. The collector accumulates requests until the batch is full (`max_batch_size`) or a timeout expires (`max_wait`). It then calls `BatchEvaluator::evaluate_batch` with all accumulated states and distributes the results back to the waiting threads.

This architecture means search threads spend most of their time blocked on the channel, not traversing the tree. Virtual loss keeps the tree exploring diverse paths while threads wait. The batch evaluator processes positions in parallel on the GPU. The overall throughput is determined by the GPU's batch evaluation speed, not the CPU's tree traversal speed.

When the `MCTSManager` is dropped, the sender is closed, which signals the collector thread to exit. The destructor joins the collector thread to ensure clean shutdown.

## Tree re-rooting

`MCTSManager::advance()` commits to a move by promoting the selected child to the new root. The old root and all sibling subtrees are dropped. The transposition table is cleared (its pointers into the old subtrees are now dangling). The selected subtree is preserved, so accumulated statistics carry over to the next search.

This operation is O(old tree size) for the drop but O(1) for the re-rooting itself. It saves significant search effort in sequential games where positions recur (e.g., chess opening preparation, self-play training). The key constraint is that the child must be owned (not a transposition table alias) and expanded. If either condition fails, `advance` returns an error rather than risking memory unsafety.
