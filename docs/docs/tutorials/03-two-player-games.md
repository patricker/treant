---
sidebar_position: 3
id: 03-two-player-games
---

import NimSolverDemo from '@site/src/components/demos/NimSolverDemo';

# Two-Player Games

MCTS handles two-player games by tracking whose turn it is and flipping the evaluation at each level. You'll implement Nim: a pile of stones, take 1 or 2 per turn, last stone wins.

**You will learn to:**
- Implement `GameState` with alternating players
- Implement adversarial evaluation via `interpret_evaluation_for_player`
- Explain how negamax perspective eliminates separate min/max logic

**Prerequisites:** [Your First Search](./02-first-search.md).

## Define the game

```rust reference="examples/nim_solver.rs#nim_game"
```

The key differences from the single-player counting game:

- **`Player` enum** -- `P1` and `P2` alternate turns.
- **`current_player()`** returns whichever player is active. MCTS uses this to route evaluations to the correct perspective.
- **`make_move()`** subtracts stones and switches the current player.
- **`terminal_value()`** -- when `stones == 0`, the current player has no move. The previous player took the last stone and won, so the position is a `Loss` for the player to move. This method returns `Option<ProvenValue>`, used by the MCTS solver (covered in the next tutorial). Return `None` for non-terminal positions.

`available_moves()` returns an empty vec at zero stones, a single `Take1` at one stone, and both options otherwise.

## Evaluate positions

```rust reference="examples/nim_solver.rs#nim_evaluator"
```

The evaluator identifies the winner (if any) and scores from each player's perspective.

**`evaluate_new_state()`** checks whether the game has ended. If `stones == 0`, the player who just moved won. The returned `Option<Player>` records the winner, or `None` if the game is still in progress. Move priors are uniform (`()` for each move).

**`interpret_evaluation_for_player()`** is the critical method for adversarial games. It takes the state evaluation and a player, and returns a signed reward:

- `+100` if that player won
- `-100` if the opponent won
- `0` if the game is still going

MCTS calls this method with the perspective of the player who made the move leading to this node. A child node's `+100` for Player 2 is automatically a `-100` for Player 1. No separate minimax pass is needed.

**`evaluate_existing_state()`** returns the same winner on revisit -- the outcome of a terminal position never changes.

## The negamax perspective

In a minimax tree, you alternate between maximizing and minimizing. MCTS avoids this asymmetry entirely. Every node maximizes from the perspective of the player who moved there. The adversarial structure comes from `interpret_evaluation_for_player` returning opposite signs for opposite players.

This means:

- The tree policy (UCT) always picks the child with the highest score.
- The highest-scored child is the best move for the player at that node.
- Because evaluations are negated across players, selecting the best move for the current player automatically selects the worst move for the opponent.

One tree, one selection rule, two players.

## Interactive demo

Play Nim against MCTS below. The solver proves whether each position is winning or losing.

<NimSolverDemo />

## What's next

In [Solving Games](./04-solving-games.md), you'll enable the MCTS-Solver extension to prove game-theoretic wins and losses -- turning MCTS from an approximation into an exact solver.
