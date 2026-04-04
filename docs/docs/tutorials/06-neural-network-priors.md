---
sidebar_position: 6
id: 06-neural-network-priors
---

import UCTvsPUCTDemo from '@site/src/components/demos/UCTvsPUCTDemo';

# Neural Network Priors

AlphaGo and AlphaZero replaced UCT with PUCT (Predictor + UCT) -- a variant that uses neural network priors to guide exploration. Instead of treating all moves equally, PUCT explores moves the network thinks are good first, then corrects course as search accumulates evidence. This tutorial shows how to wire prior probabilities into your evaluator and configure all the AlphaZero knobs.

**You will learn to:**
- Understand how PUCT differs from UCT
- Build an evaluator that returns prior probabilities
- Configure `AlphaGoPolicy` with FPU, Dirichlet noise, and temperature

**Prerequisites:** [Games with Chance](./05-stochastic-games.md).

## UCT vs PUCT

UCT treats every untried move identically. PUCT adds a prior probability `P(a)` from an external source (typically a neural network):

```
UCT:  Q(a) + C * sqrt(ln(N) / n(a))
PUCT: Q(a) + C * P(a) * sqrt(N) / (1 + n(a))
```

The prior `P(a)` means high-probability moves are explored first. A move with `P(a) = 0.7` gets roughly seven times the initial exploration bonus of a move with `P(a) = 0.1`. But as visits accumulate, the `Q(a)` term dominates -- search overcomes a bad prior.

## Define a game with priors

A simple game: pick move A (+10), B (+5), or C (+1) at each step, three levels deep. A is objectively best, but the "neural network" will say C is best.

```rust reference="examples/alphazero_basics.rs#prior_game"
```

The structure is the same as any `GameState`: list available moves, apply them, and stop at the terminal depth. The reward values in `make_move()` give us a known ground truth to verify that search overcomes the misleading prior.

## Evaluator with misleading priors

The evaluator returns prior probabilities alongside the state evaluation. Here, the priors are intentionally wrong -- they assign 70% to the worst move.

```rust reference="examples/alphazero_basics.rs#prior_evaluator"
```

`evaluate_new_state()` now returns `(Vec<f64>, i64)` instead of `(Vec<()>, i64)`. The `Vec<f64>` contains one prior probability per move, in the same order as `available_moves()`. These represent what a neural network would output: a probability distribution over legal moves. Priors must be non-negative and should sum to approximately 1.0.

The state evaluation (`state.score`) serves as the value head -- the network's estimate of how good the current position is.

## Configure AlphaZero-style search

The MCTS config selects `AlphaGoPolicy` as the tree policy and enables the AlphaZero-specific features.

```rust reference="examples/alphazero_basics.rs#alphazero_config"
```

Each method controls a different aspect of the search:

- **`TreePolicy = AlphaGoPolicy`** -- use the PUCT formula instead of UCT. The exploration constant passed to `AlphaGoPolicy::new(1.5)` controls how much weight the prior gets relative to the value.
- **`dirichlet_noise() -> Some((0.25, 0.3))`** -- blend 25% Dirichlet noise into the root priors. Dirichlet noise is a random perturbation drawn from the Dirichlet distribution -- it adds randomness to the root priors so the search doesn't always follow the network's top pick. The noisy prior becomes `0.75 * prior + 0.25 * Dir(0.3)`. This prevents the search from fixating on the network's top choice during self-play (self-play: the system plays games against itself to generate training data), ensuring diverse training data. Alpha of 0.3 is typical for chess-scale games; use 0.03 for Go.
- **`selection_temperature() -> 1.0`** -- temperature is borrowed from statistical mechanics: at temperature 0 (frozen), only the best move is chosen; at temperature 1 (hot), moves are sampled proportionally to their visit counts, adding diversity. Concretely, `best_move()` samples proportional to visit counts instead of always picking the most-visited child. At temperature 0.0 (the default), it returns the argmax. At 1.0, a child with twice the visits is twice as likely to be selected. Use 1.0 early in self-play games for diversity, 0.0 for competitive play.
- **`rng_seed() -> Some(42)`** -- seed the internal RNG for reproducible search. Each thread gets `seed + thread_id`, so parallel search is also deterministic.

## First Play Urgency (FPU)

When `fpu_value()` returns infinity (the default), MCTS tries every child at least once before comparing them. This makes sense for UCT, where you have no information about untried moves.

With neural priors, you often do have information. Setting `fpu_value()` to a finite value (e.g., `-1.0` or `0.0`) assigns that score to unvisited children. The search then trusts the prior to decide which children to visit first, rather than wasting visits on moves the network considers terrible.

The example above uses the default (infinity) to keep things simple. In a production AlphaZero system, you would typically set `fpu_value()` to a pessimistic estimate -- slightly below the parent's average value.

## Run the search

```rust reference="examples/alphazero_basics.rs#run_alphazero"
```

`AlphaGoPolicy::new(1.5)` creates the PUCT policy with exploration constant C=1.5. The search runs 10,000 playouts, then prints child statistics and samples moves using the temperature setting.

The key result: despite C having a 0.7 prior, MCTS converges to A as the best move. The principal variation is `[A, A, A]` -- the objectively optimal path. Search overcomes prior bias.

## Interactive demo

Compare UCT (no priors) vs PUCT (with wrong priors). Run playouts and watch PUCT initially over-visit C, then correct as evidence accumulates. Adjust the exploration constants to see how they change convergence speed.

<UCTvsPUCTDemo />

## What's next

[Advanced Search Features](./07-advanced-search.md) covers transposition tables, tree reuse, progressive widening, and depth limiting -- the features that turn a prototype into a production system.
