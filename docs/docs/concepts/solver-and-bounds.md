---
sidebar_position: 4
id: solver-and-bounds
---

# Game-Theoretic Proving

Standard MCTS produces statistical estimates: the best move is probably the one with the most visits. MCTS-Solver and Score-Bounded MCTS go further. They propagate exact game-theoretic results through the tree, proving that positions are won, lost, drawn, or bounded within a score interval.

These features transform MCTS from a sampling algorithm into a hybrid: statistical search for unexplored regions, exact proof for resolved ones. The search automatically stops exploring proven subtrees and focuses remaining budget on unresolved positions.

## MCTS-Solver

Winands, Bjornsson, and Samark (2008) observed that when MCTS reaches a terminal node, the result is not a statistical estimate -- it is a fact. A checkmate is a checkmate regardless of how many playouts you run. MCTS-Solver propagates these facts upward through the tree using minimax logic.

The propagation rules follow directly from the definition of optimal play:

**Any child is a Loss (from child's perspective) implies parent is a Win.** The parent's current player can choose this move, and the opponent will lose. One winning option is sufficient.

**All children are Wins (from child's perspective) implies parent is a Loss.** Every move available to the parent leads to the opponent winning. There is no escape.

**All children proven, none are Loss, at least one is Draw implies parent is a Draw.** The best the parent can achieve is a draw.

These rules are minimax in disguise. "Any child Loss = parent Win" is the maximizing player selecting the best outcome. "All children Win = parent Loss" is the minimizing player having no losing option available (from their opponent's perspective, every line wins).

### The ProvenValue enum

```
Unknown  -- not yet proven, use statistical estimates
Win      -- the current player (at this node) wins with optimal play
Loss     -- the current player loses with optimal play
Draw     -- optimal play leads to a draw
```

Values are stored from the perspective of the player who moved to reach this node. This convention matches the minimax framework: when backing up, a child's Loss means the parent can Win by choosing that child.

### Terminal classification

Terminal nodes (no legal moves) are classified by `GameState::terminal_value()`. If this method returns `None`, the solver falls back to `terminal_score()`: positive scores become Win, negative become Loss, zero becomes Draw. If neither method is implemented, terminal nodes remain Unknown and the solver has no facts to propagate.

When both `terminal_value()` and `terminal_score()` are provided, the library performs a debug-mode consistency check: a Win with a negative score, or a Loss with a positive score, triggers a debug assertion failure. This catches implementation bugs early. In release mode, `terminal_value()` takes precedence.

### When the solver helps

MCTS-Solver shines in endgames and small game trees where the search can fully resolve subtrees. In Tic-Tac-Toe, the solver proves the entire game tree within a few thousand playouts. In chess endgames, it can prove forced checkmates that statistical MCTS would only estimate.

The solver adds no overhead to unresolved nodes. Proven values are checked during selection (proven-Loss children get `f64::INFINITY` score, proven-Win children get `f64::NEG_INFINITY`), but the check is a single atomic load. When `solver_enabled()` returns false, the code path still compiles but the branch is never taken.

Proven-Draw children are handled differently: they receive their empirical average reward rather than a forced extreme value. This allows the search to prefer a draw over a loss but still explore for a possible win.

### Propagation in practice

Propagation walks bottom-up along the playout path after backpropagation. At each level, the library checks whether the child's proven value allows proving the parent. If the child is proven Loss (parent can Win), the parent is immediately set to Win via compare-and-swap. If all children are proven Win (no escape), the parent is set to Loss. If all children are proven but the best is a Draw, the parent is set to Draw.

If the parent cannot be proven (some children are still Unknown), propagation stops. It will be attempted again on future playouts that pass through this node. Proven values are set atomically and never reverted -- once a node is proven, it stays proven.

## Score-Bounded MCTS

MCTS-Solver deals in absolutes: Win, Loss, Draw. Score-Bounded MCTS (Cazenave and Saffidine, 2010) tracks finer-grained information: a proven interval `[lower, upper]` on each node's minimax value from the current player's perspective.

A node starts with bounds `[i32::MIN, i32::MAX]` -- completely unknown. As terminal nodes are discovered and their exact scores propagated, the interval tightens monotonically: lower bounds can only increase, upper bounds can only decrease. When `lower == upper`, the node's exact minimax value is proven.

### Negamax bound propagation

Bounds propagate using negamax -- the same principle as minimax, but alternating the sign at each level. The parent's perspective is the negation of the child's.

From a child with bounds `[child_lower, child_upper]`, the parent sees a score in `[-child_upper, -child_lower]`. The parent takes the best case across all children:

```
parent_lower = max over all children of negate(child_upper)
parent_upper = max over all children of negate(child_lower)
```

"Max of negate(child_upper)" is the parent's guaranteed minimum: even in the worst case for the chosen child, the parent can achieve at least this much by picking the best child's worst case.

"Max of negate(child_lower)" is the parent's ceiling: the best possible outcome, achieved by picking the child with the best optimistic bound.

### Sentinel-safe negation

Score bounds use `i32::MIN` and `i32::MAX` as sentinels meaning "unbounded." Negating these values would overflow, so `negate_bound()` maps them explicitly:

```
negate_bound(i32::MIN) = i32::MAX
negate_bound(i32::MAX) = i32::MIN
negate_bound(x)        = -x
```

An unbounded lower bound (no information about the minimum) becomes an unbounded upper bound when negated, and vice versa.

### Monotonic tightening

Bounds only move inward. The library enforces this with compare-and-swap: a new lower bound is written only if it exceeds the current lower bound. A new upper bound is written only if it is below the current upper bound. Concurrent threads may race to tighten bounds, but the result is always correct -- bounds can never widen.

### Bounds-based pruning

During selection, the tree policy uses bounds to skip children that cannot improve on the best guaranteed score. If any child gives the parent a guaranteed lower bound of X, then children whose upper bound (from the parent's perspective) is less than X cannot be the best move. They receive `f64::NEG_INFINITY` during selection and are effectively pruned.

This is analogous to alpha-beta pruning but operates within the MCTS framework. The pruning is conservative: it only skips children that are provably dominated, never children that might be better.

### Chance nodes and bounds

Chance nodes use different propagation rules because nature chooses randomly, not adversarially. Bounds are combined by weighted average rather than max/min, and no negation is applied (same player on both sides of the chance event). If any child has unbounded scores, the parent stays unbounded -- the weighted average is only meaningful when all components are known. See [Open-Loop vs Closed-Loop](./chance-nodes) for the full treatment.

## Cross-derivation

MCTS-Solver and Score-Bounded MCTS are independent features, but they reinforce each other when both are active.

**Terminal nodes.** `terminal_score()` auto-derives `ProvenValue` (positive score maps to Win, negative to Loss, zero to Draw). `terminal_value()` auto-derives bounds when no score is provided (Win maps to +1, Loss to -1, Draw to 0). This ensures both systems get initialized from whichever terminal classification the game provides.

**Converged bounds imply proven values.** When a node's bounds converge (`lower == upper`), the exact score is known. If the solver is also active, the library derives the proven value from the converged score: positive becomes Win, negative becomes Loss, zero becomes Draw.

**One-directional.** Proven values do not set bounds, because the Win/Loss/Draw classification does not carry scoring scale information. A Win might correspond to a score of 1 or 1,000 -- the solver does not know. The bounds system must discover the exact value independently.

This asymmetry is intentional. The solver operates in a coarse three-valued logic (Win/Loss/Draw). The bounds system operates in a fine-grained integer arithmetic. The coarse system can absorb information from the fine system (converged score implies proven value), but the fine system cannot extract information from the coarse system (proven Win does not imply a specific score). Keeping the information flow one-directional avoids incorrect assumptions about scoring scales.

## When to use what

**Solver only (`solver_enabled = true, score_bounded = false`).** Best for games with binary outcomes (win/loss/draw). Chess, Go, Tic-Tac-Toe, Hex. The solver proves positions completely in small enough trees. No overhead for the bounds system.

**Bounds only (`solver_enabled = false, score_bounded = true`).** Best for scoring games where the magnitude matters, not just the sign. Point-based games, auction games, optimization problems. Bounds tighten toward the exact minimax value.

**Both (`solver_enabled = true, score_bounded = true`).** Maximum proving power. Converged bounds feed the solver. Proven values feed selection (skipping proven-loss children). Use when the game has both tactical depth (benefiting from solver) and score sensitivity (benefiting from bounds).

**Neither (defaults).** Standard statistical MCTS. No proving overhead. Appropriate when the tree is too large for any subtrees to be fully resolved within the playout budget, or when the game has no natural terminal classification.

## Interaction with search termination

When the root node is proven, the search stops automatically. `playout()` returns `false`, signaling that no further playouts are useful. Similarly, when score bounds converge at the root, the search halts.

The `select_child_after_search` method is solver-aware. It prefers proven-Win children (child's Loss = parent's Win) over unproven children, and proven-Draw over proven-Loss. When score bounds are available, it selects the child with the best guaranteed score from the parent's perspective. This ensures the final move selection respects proven results even when visit counts might suggest otherwise -- a proven win with 10 visits beats an unproven move with 10,000 visits.

## Further reading

- Winands, M., Bjornsson, Y., and Samark, J.-T. (2008). "Monte-Carlo Tree Search Solver." *Computers and Games.*
- Cazenave, T. and Saffidine, A. (2010). "Score Bounded Monte-Carlo Tree Search." *Computers and Games.*

## Performance impact

The memory overhead is fixed: 9 bytes per node (1 byte for proven value, 4 bytes each for lower and upper bounds). On a typical node of 100+ bytes, this is less than 10%.

The runtime overhead when features are disabled is zero. The `if self.manager.solver_enabled()` and `if self.manager.score_bounded_enabled()` checks are constant `false` and eliminated by the compiler.

When enabled, the per-playout overhead is proportional to the playout path length. Propagation walks the path once bottom-up, performing one compare-and-swap per node (for proven values) or two compare-and-swaps per node (for score bounds). On a typical path of 10-30 nodes, this adds microseconds per playout. The benefit -- skipping proven subtrees and pruning dominated children -- typically saves far more time than the propagation costs.

The interaction between solver and bounds adds one additional check: when bounds converge and the solver is active, a compare-and-swap sets the proven value. This is at most one extra atomic operation per node per propagation pass, and it only triggers when bounds actually converge -- a rare event that signals the endgame of proving.

For large game trees (Go, chess middlegames), the solver rarely proves anything and the bounds rarely tighten. The overhead is negligible. For small or endgame trees (Tic-Tac-Toe, simple puzzles, forced mating sequences), the solver aggressively prunes resolved subtrees and the search converges dramatically faster than statistical MCTS alone. The feature pays for itself exactly when it has something to prove.
