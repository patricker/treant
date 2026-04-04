---
sidebar_position: 3
id: progressive-widening
---

# Handle Large Action Spaces

Limit how many children are expanded at each node so that MCTS focuses on promising moves in games with hundreds or thousands of legal options.

**You will learn to:**
- Implement `max_children()` to control progressive widening
- Order moves by priority for best results

**Prerequisites:** Complete [Your First Search](../tutorials/02-first-search).

## Override `max_children`

By default, every legal move gets a child node. Override `max_children` on your `GameState` to cap expansion based on the parent's visit count:

```rust
impl GameState for MyGame {
    type Move = MyMove;
    type Player = Player;
    type MoveList = Vec<MyMove>;

    fn max_children(&self, visits: u64) -> usize {
        // Start with 5 children, grow with sqrt(visits)
        5 + (visits as f64).sqrt() as usize
    }

    fn current_player(&self) -> Player { /* ... */ }
    fn available_moves(&self) -> Vec<MyMove> { /* ... */ }
    fn make_move(&mut self, mov: &MyMove) { /* ... */ }
}
```

Only the first `max_children(visits)` moves from `available_moves()` are expanded. As the node accumulates visits, more children become eligible.

## Common schedules

| Schedule | Formula | Use case |
|---|---|---|
| Square root | `(visits as f64).sqrt() as usize` | General purpose, most common |
| Logarithmic | `(visits as f64).ln().ceil() as usize` | Very large action spaces (1000+) |
| Linear | `visits as usize / 10` | Aggressive widening |
| Constant + sqrt | `k + (visits as f64).sqrt() as usize` | Guarantee a minimum width `k` |

Start with square root. Switch to logarithmic only if you observe too many children being expanded in high-visit nodes.

## Move ordering

Moves are expanded in the order returned by `available_moves()`. The first moves returned are expanded first, so return high-priority moves first:

```rust
fn available_moves(&self) -> Vec<MyMove> {
    let mut moves = self.all_legal_moves();
    // Sort captures and checks first
    moves.sort_by_key(|m| match m {
        MyMove::Capture(_) => 0,
        MyMove::Check(_) => 1,
        _ => 2,
    });
    moves
}
```

With progressive widening, move ordering is critical. A move that is never expanded is never explored.

## Reordering with PUCT priors

When using `AlphaGoPolicy`, moves are automatically sorted by their prior probability (highest first) via `compare_move_evaluations`. The neural network's priors determine which moves are expanded first, and progressive widening limits expansion to the top-prior moves until enough visits accumulate.

This combination -- neural network priors + progressive widening -- is how AlphaZero-style systems handle large action spaces efficiently.

## Expected result

A game with 500 legal moves per position and `max_children = sqrt(visits)` expands only ~32 children after 1000 visits to a node, focusing search on the most promising 6% of moves.

## See also

- [Tree Policies](../concepts/tree-policies) -- how selection interacts with widening
- [Neural Network Priors](../tutorials/06-neural-network-priors) -- using PUCT with move priors
