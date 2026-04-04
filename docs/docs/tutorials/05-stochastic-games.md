---
sidebar_position: 5
id: 05-stochastic-games
---

import ChanceNodesDemo from '@site/src/components/demos/ChanceNodesDemo';

# Games with Chance

Not all games are deterministic. Dice rolls, card draws, random events -- these are stochastic transitions where nature, not a player, determines the next state. MCTS handles them through chance nodes.

**You will learn to:**
- Implement `chance_outcomes()` to define stochastic transitions with probabilities
- Implement `evaluate_existing_state` correctly for open-loop stochastic games
- Distinguish when to use open-loop vs closed-loop chance node modes

**Prerequisites:** [Proving Wins and Losses](./04-solving-games.md).

## The dice game

A player repeatedly chooses Roll or Stop. Rolling adds a d6 to the score. The game ends when the score reaches 20 or the player stops. MCTS must learn that rolling always beats stopping (the expected value of a d6 is 3.5, which is always positive).

```rust reference="examples/dice_game.rs#dice_game"
```

The critical method is `chance_outcomes()`. When a roll is pending, it returns `Some(Vec<(Move, f64)>)` -- a list of possible outcomes with their probabilities. Each die face has probability 1/6. When no random event is pending, it returns `None`, and MCTS treats the node normally.

MCTS samples from these probabilities during playouts. Over many playouts, the statistics converge to the true expected value.

## Open-loop MCTS

By default, chance outcomes use open-loop mode. This means:

- Chance outcomes are sampled during each playout but **not** stored as separate tree nodes.
- Multiple playouts through the same "Roll" edge experience different dice values.
- The node's statistics converge to the **expected** value across all outcomes.
- Memory-efficient: no per-outcome subtrees.

The tradeoff is that per-outcome information is lost. The tree cannot distinguish "I rolled a 6" from "I rolled a 1" at the same node. For many games, this is fine -- the expected value is what matters for decision-making.

## Evaluating under randomness

Open-loop MCTS visits the same tree node with different game states, because different dice rolls land on the same node. The evaluator must handle this correctly:

```rust reference="examples/dice_game.rs#dice_evaluator"
```

`evaluate_existing_state` re-evaluates from the **current** state, not the cached evaluation. This is the key difference from deterministic games. In deterministic MCTS, a node always represents the same state. In open-loop stochastic MCTS, the same node is reached via different chance outcomes, so the state varies between visits. Returning the cached value would ignore the actual dice roll.

## Running the search

The MCTS config for the dice game requires no special flags -- chance nodes work out of the box when `chance_outcomes()` returns `Some`:

```rust reference="examples/dice_game.rs#dice_config"
```

MCTS learns that Roll dominates Stop at every score. The expected payoff of rolling (current score + 3.5) always exceeds stopping (current score + 0). After enough playouts, Roll receives the overwhelming majority of visits.

## Closed-loop mode

For games where per-outcome information matters, enable closed-loop chance with `closed_loop_chance() -> true` in your MCTS config. This stores each chance outcome as a separate child node in the tree. The result is more accurate per-outcome statistics at the cost of a larger tree.

See the [chance nodes concept page](/docs/concepts/chance-nodes) for a detailed comparison of open-loop and closed-loop strategies.

## Interactive demo

Watch MCTS learn that rolling is always better than stopping.

<ChanceNodesDemo />

## What's next

[Tutorial 6](./06-neural-network-priors) adds neural network priors to guide search with learned policy and value estimates.
