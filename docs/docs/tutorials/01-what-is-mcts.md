---
sidebar_position: 1
id: 01-what-is-mcts
---

import StepThroughDemo from '@site/src/components/demos/StepThroughDemo';

# What is MCTS?

Monte Carlo Tree Search finds good decisions in games and planning problems where the search space is too large for exhaustive methods. Chess has roughly 10^44 legal positions. Go has 10^170. Brute force is out. MCTS works by sampling: play random games from the current position, track which moves lead to wins, and gradually focus on the most promising lines.

**You will learn to:**
- Explain the four phases of MCTS (selection, expansion, simulation, backpropagation)
- Describe how the UCT (Upper Confidence bound for Trees) formula balances exploration and exploitation
- Predict how the search tree grows asymmetrically based on move quality

**Prerequisites:** None. No code required.

## The key insight

Play a random game from the current position. Record who won. Repeat thousands of times. Moves that lead to more wins get higher scores. The more games you sample, the better your estimates become. This is the Monte Carlo method applied to game trees. (The name comes from the Monte Carlo casino — using randomness to estimate something, like throwing darts to estimate pi.)

The challenge is efficiency. Pure random sampling wastes time on clearly bad moves. MCTS solves this with a tree that grows asymmetrically -- spending most samples on promising branches while still occasionally exploring alternatives.

## The four phases

Every MCTS iteration runs four phases in sequence: selection, expansion, simulation, and backpropagation.

### Selection

Start at the root. At each node, choose the child that maximizes the **UCT** (Upper Confidence bound for Trees) score — a formula that balances exploiting known-good moves with exploring less-visited ones:

```
UCT(a) = Q(a) + C * sqrt(ln(N) / n(a))
```

- `Q(a)` -- average reward for move `a` (exploit: prefer moves that have scored well)
- `n(a)` -- visit count for move `a`
- `N` -- total visits to the parent node
- `C` -- exploration constant (higher = more exploration)

The first term favors known-good moves. The second term grows when a move has been tried relatively few times, pulling the search toward unexplored territory.

```
         [Root: 100 visits]
        /                   \
  [Move A: 70 visits]   [Move B: 30 visits]
   Q=0.6, high exploit    Q=0.5, high explore
        |
   [selected -- Q + explore term is highest]
```

Walk down the tree, selecting at each level, until you reach a node with untried moves.

### Expansion

At the selected node, one or more moves have never been tried. Create a new child node for one of them. The tree grows by one node per iteration.

```
         [Root]
        /      \
     [A]       [B]
    /    \
  [A1]   [A2]
          |
        [NEW]  <-- expansion: first visit to this move
```

### Simulation

From the new node, play out the game to completion. The simplest approach uses random moves. More sophisticated implementations use heuristics or neural networks to guide the rollout. The game ends at a **terminal state** — a position where no more moves are possible (someone won, or the game is drawn). The result is a terminal outcome: win, loss, or draw.

```
  [NEW node]
      |
   random move
      |
   random move
      |
   random move
      |
  terminal state --> result: Win (+1)
```

### Backpropagation

Walk back up the path from the new node to the root. At each node along the way, increment the visit count and add the simulation result to the cumulative reward.

```
  [Root: 101 visits, reward += 1]   ^
        |                           |
     [A: 71 visits, reward += 1]    | backpropagate
        |                           |
      [A2: reward += 1]             |
        |                           |
      [NEW: 1 visit, reward = 1]    | start here
```

After thousands of iterations, the visit counts and average rewards at the root's children reflect their relative strength. The most-visited child is typically the best move.

## Why it works

UCT is derived from UCB1 (Upper Confidence Bound), an algorithm for the [multi-armed bandit problem](https://en.wikipedia.org/wiki/Multi-armed_bandit) that guarantees sublinear regret. In practice, this means:

- Moves that win often get sampled more, converging on the best play.
- Moves that lose often get sampled less, but never zero -- an unexplored move always has an inflated explore term.
- With enough playouts, the values converge toward the true game-theoretic value.

The tree grows asymmetrically. Strong lines of play become deep and well-explored. Weak lines stay shallow. The algorithm allocates its compute budget where it matters most.

## Interactive demo

The demo below runs a simple game: a counter starts at 0 and the player can Add (+1) or Sub (-1) each turn, trying to reach 100. MCTS must figure out that Add is always better.

Each time you press **Step**, one full playout runs — selection down the tree using UCT, expansion of a new node, simulation to a terminal state, and backpropagation of the result. Watch the visit counts climb on Add and the tree grow deeper along promising lines.

Try adjusting **C (exploration)** — lower values concentrate visits on the current best move, higher values spread visits more evenly across alternatives.

<StepThroughDemo />

## What's next

In [Your First Search](./02-first-search.md), you'll implement this algorithm in Rust using the `treant` crate.
