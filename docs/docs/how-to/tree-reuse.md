---
sidebar_position: 2
id: tree-reuse
---

# Preserve Search Across Turns

Reuse the search tree between moves instead of rebuilding it from scratch each turn.

**You will learn to:**
- Use `advance()` to re-root the tree after a move
- Handle the three `AdvanceError` variants

**Prerequisites:** Complete [Your First Search](../tutorials/02-first-search).

## The search-advance loop

After choosing a move, call `advance()` to commit to it. The subtree below that move becomes the new root, and all other branches are discarded.

```rust reference="examples/tree_reuse.rs#reuse_loop"
```

Each call to `advance()` preserves nodes already explored below the chosen move. On the next search, those nodes do not need to be re-expanded.

## Handle errors

`advance()` returns `Result<(), AdvanceError>` with three failure modes:

| Variant | Cause | Fix |
|---|---|---|
| `MoveNotFound` | The move is not among root children | Verify the move is legal in the current state |
| `ChildNotExpanded` | The child was never visited during search | Run more playouts before advancing |
| `ChildNotOwned` | The child is a transposition table alias | Use `()` for the transposition table, or call `reset()` and search from scratch |

In practice, `MoveNotFound` is a logic error. `ChildNotExpanded` means you advanced to a move the search never explored -- increase your playout budget. `ChildNotOwned` only occurs with transposition tables.

## Pondering

Search during the opponent's turn, then advance with their actual move:

```rust
// Your turn: search and play
mcts.playout_n_parallel(50_000, 4);
let my_move = mcts.best_move().unwrap();
mcts.advance(&my_move).unwrap();
send_move(my_move);

// Opponent's turn: keep searching (pondering)
let search = mcts.playout_parallel_async(4);

// When opponent moves, stop and advance
let opponent_move = receive_move();
search.halt();
mcts.advance(&opponent_move).unwrap();
```

All work done during pondering is preserved if the opponent plays into a subtree you already explored.

## Transposition table interaction

`advance()` clears the transposition table because entries may reference nodes in the discarded portion of the tree. If you rely on transposition hits, the table rebuilds naturally during subsequent search.

To avoid transposition issues entirely, use `()` as your `TranspositionTable` type.

## Expected result

Tree reuse typically saves 20-50% of search time per turn in the mid-game, when the opponent plays a move you already explored deeply.

## See also

- [Architecture](../concepts/architecture) -- how the search tree is structured
- [Run Parallel Search](./parallel-search) -- async search for pondering
