---
sidebar_position: 7
id: 07-advanced-search
---

# Advanced Search Features

The features in this tutorial make the difference between a toy MCTS and a production system. Each one is a single method override or type alias.

**You will learn to:**
- Share search results across identical positions with transposition tables
- Preserve the search tree across turns with `advance()`
- Handle large action spaces with progressive widening
- Limit search depth for evaluator-driven search

## Transposition tables

Different move sequences can reach the same game state. A transposition table detects this and shares the search node, so positions that have already been explored are not re-searched.

First, implement `TranspositionHash` for your game state:

```rust reference="examples/counting_game.rs#transposition_hash"
```

`hash()` must return a nonzero `u64` for any state you want to store in the table. States that return 0 are never inserted. States that hash to the same value share a tree node -- so the hash must be a faithful fingerprint of the game-relevant state.

Then set the associated type in your MCTS config:

```rust
type TranspositionTable = ApproxTable<Self>;
```

And pass the table when constructing the search:

```rust
MCTSManager::new(state, config, eval, policy, ApproxTable::new(1024));
```

`ApproxTable::new(capacity)` creates a lock-free hash table (threads access it concurrently without waiting for each other). The capacity must be a power of 2. Larger tables reduce collisions at the cost of memory. For the counting game, different move sequences that reach the same counter value (e.g., +1+1-1 and +1) share a single node.

When transpositions create cycles, `cycle_behaviour()` controls what happens. The default panics on cycles when a transposition table is present. Set it to `CycleBehaviour::UseCurrentEvalWhenCycleDetected` to gracefully stop expansion and use the node's current evaluation.

## Tree reuse

In turn-based games, the search tree from the previous turn contains useful work. `advance()` commits to a move and re-roots the tree below that child, preserving the subtree.

```rust reference="examples/tree_reuse.rs#reuse_loop"
```

After `playout_n_parallel()` finishes, `advance(&best)` detaches the subtree rooted at the chosen move and promotes it to the new root. All nodes below that child -- positions already explored -- carry over to the next turn. The rest of the tree is discarded.

This is critical for real-time play. Search during the opponent's turn (pondering: searching during the opponent's thinking time), then call `advance()` with their move when it arrives. The work done while pondering is preserved.

`advance()` returns `Result<(), AdvanceError>` with three error cases:
- **`MoveNotFound`** -- the move does not exist among root children
- **`ChildNotExpanded`** -- the child node was never visited during search
- **`ChildNotOwned`** -- the child is a transposition table alias and cannot be detached as a standalone subtree

## Progressive widening

Progressive widening means starting with few children and gradually expanding more as the node accumulates visits.

In games with many legal moves (Go has ~250 per position, continuous action spaces can have infinitely many), expanding all children is wasteful. Most visits go to a handful of moves. Progressive widening limits how many children are expanded based on the node's visit count.

Override `max_children` on your `GameState`:

```rust
fn max_children(&self, visits: u64) -> usize {
    (visits as f64).sqrt() as usize + 1
}
```

At 1 visit, only 2 children are expanded. At 100 visits, 11. At 10,000, 101. The search starts narrow and widens as it gathers more evidence about which region of the action space is worth exploring.

Moves are expanded in the order returned by `available_moves()`. With progressive widening, this order matters -- return higher-priority moves first so they get expanded before lower-priority ones.

## Depth limiting

`max_playout_depth()` limits how deep the tree grows before forcing a leaf evaluation. When the depth limit is reached, the current node is evaluated as a leaf and the result is backpropagated.

Override it on the MCTS config:

```rust
fn max_playout_depth(&self) -> usize {
    20
}
```

This is most useful with a strong evaluator (like a neural network value head). Instead of playing random moves to a terminal state, the search evaluates positions at depth 20 and backs up those values. Shallower depth limits make each playout faster but rely more heavily on evaluator accuracy.

This is a quality knob, not a safety cap. `max_playout_length()` (default: 1,000,000) is the safety cap that panics if exceeded.

## Quick reference

All `MCTS` trait and `GameState` methods covered in tutorials 1-7:

| Method | Default | Tutorial |
|---|---|---|
| `GameState::current_player()` | (required) | [2](./02-first-search.md) |
| `GameState::available_moves()` | (required) | [2](./02-first-search.md) |
| `GameState::make_move()` | (required) | [2](./02-first-search.md) |
| `GameState::max_children()` | `usize::MAX` | 7 (this page) |
| `Evaluator::evaluate_new_state()` | (required) | [2](./02-first-search.md) |
| `MCTS::TreePolicy` | (required) | [2](./02-first-search.md) |
| `MCTS::TranspositionTable` | `()` for none | 7 (this page) |
| `MCTS::virtual_loss()` | `0` | [2](./02-first-search.md) |
| `MCTS::cycle_behaviour()` | panic on cycle | 7 (this page) |
| `MCTS::fpu_value()` | `f64::INFINITY` | [6](./06-neural-network-priors.md) |
| `MCTS::dirichlet_noise()` | `None` | [6](./06-neural-network-priors.md) |
| `MCTS::selection_temperature()` | `0.0` | [6](./06-neural-network-priors.md) |
| `MCTS::rng_seed()` | `None` | [6](./06-neural-network-priors.md) |
| `MCTS::max_playout_depth()` | `usize::MAX` | 7 (this page) |
| `MCTS::max_playout_length()` | `1_000_000` | 7 (this page) |
| `MCTS::solver_enabled()` | `false` | [4](./04-solving-games.md) |

## What's next

[Gumbel Search](./08-gumbel-search.md) introduces the `treant-gumbel` crate -- a search algorithm with monotonic policy improvement, designed for self-play training loops. Or, if you're done with tutorials, see the How-To guides for parallel search, batched neural network evaluation, and hyperparameter tuning, and the Concepts section for deeper understanding of the algorithms.
