---
sidebar_position: 5
id: hyperparameter-tuning
---

# Tune Hyperparameters

Choose good values for exploration constant, FPU, Dirichlet noise, and temperature.

**You will learn to:**
- Select appropriate values for each hyperparameter
- Diagnose search pathologies using built-in tools

**Prerequisites:** Complete [Your First Search](../tutorials/02-first-search) and [Neural Network Priors](../tutorials/06-neural-network-priors).

## Exploration constant (C)

Controls the exploration-exploitation balance during tree traversal.

| Policy | Typical range | Default guidance |
|---|---|---|
| `UCTPolicy` | 0.5 -- 2.0 | Start at `sqrt(2) = 1.41` |
| `AlphaGoPolicy` | 1.0 -- 2.5 | Start at `1.5` |

**Too low:** Search converges prematurely on suboptimal moves. The visit distribution is sharply peaked -- one child gets 90%+ of visits regardless of quality.

**Too high:** Search wastes visits on clearly bad moves. Visit distribution is nearly uniform across all children.

```rust
let policy = UCTPolicy::new(1.41);     // classic UCB1 value
let policy = AlphaGoPolicy::new(1.5);  // typical PUCT value
```

## First Play Urgency (FPU)

Controls the score assigned to children that have never been visited.

```rust
impl MCTS for MyMCTS {
    fn fpu_value(&self) -> f64 {
        f64::INFINITY  // default: try every child at least once
    }
}
```

| Value | Behavior | Use case |
|---|---|---|
| `f64::INFINITY` (default) | Every child is visited before any revisit | UCT without priors |
| `0.0` | Unvisited children score 0 | Trust priors to guide exploration |
| `-1.0` | Unvisited children score -1 | Strong priors (neural nets) |

With `AlphaGoPolicy`, set FPU to a finite value (e.g., `-1.0` to `0.0`). Infinite FPU forces all children to be tried once before the prior has any effect, defeating the purpose of neural network priors in large action spaces.

## Dirichlet noise

Adds randomness to root priors during self-play training to ensure diverse game trajectories.

```rust
impl MCTS for MyMCTS {
    fn dirichlet_noise(&self) -> Option<(f64, f64)> {
        Some((0.25, 0.3))  // (epsilon, alpha)
    }
}
```

**Epsilon** controls the noise weight: `noisy_prior = (1 - eps) * prior + eps * Dir(alpha)`. Typical value: `0.25`.

**Alpha** scales inversely with the branching factor of your game:

| Game | Branching factor | Alpha |
|---|---|---|
| Chess | ~30 | 0.3 |
| Shogi | ~80 | 0.15 |
| Go | ~250 | 0.03 |

Higher alpha produces more uniform noise. Lower alpha produces spikier noise that occasionally overrides the prior for a single move.

Disable Dirichlet noise during competitive play (return `None`). It is only useful during self-play training data generation.

## Temperature

Controls post-search move selection in `best_move()`.

```rust
impl MCTS for MyMCTS {
    fn selection_temperature(&self) -> f64 {
        0.0  // default: always pick the most-visited move
    }
}
```

| Value | Behavior | Use case |
|---|---|---|
| `0.0` | Argmax (most visits wins) | Competitive play |
| `1.0` | Proportional to visit count | Training diversity |
| `0.0 < t < 1.0` | Sharpened proportional | Mild diversity |

**Decay schedule for training:** Use temperature 1.0 for the first 30 moves to generate diverse openings, then switch to 0.0 for the remainder of the game. Implement this by changing the MCTS config between moves:

```rust
struct MyMCTS { temperature: f64 }

impl MCTS for MyMCTS {
    fn selection_temperature(&self) -> f64 {
        self.temperature
    }
    // ...
}

// During self-play:
let temp = if move_number < 30 { 1.0 } else { 0.0 };
```

## Diagnose search pathologies

### Visit distribution

Use `root_child_stats()` to inspect how visits are distributed:

```rust
let stats = mcts.root_child_stats();
for s in &stats {
    println!("{}: {} visits, {:.2} avg reward", s.mov, s.visits, s.avg_reward);
}
```

A healthy distribution shows the best move with a clear plurality but not a monopoly. If one move has 99% of visits with only 1000 total playouts, C is too low. If visits are nearly equal after 100,000 playouts, C is too high.

### Tree diagnostics

Call `diagnose()` on the search tree for structural statistics:

```rust
println!("{}", mcts.tree().diagnose());
// Output:
// 42,391 nodes
// 128 transposition table hits
// 0 delayed transposition table hits
// 3 expansion contention events
// 0 orphaned nodes
```

High **expansion contention events** indicate threads are colliding during node expansion. Increase virtual loss to spread threads apart.

import ExplorationDemo from '@site/src/components/demos/ExplorationDemo';

<ExplorationDemo />

## Expected result

Well-tuned hyperparameters produce a visit distribution where the best move receives 50-80% of visits, with remaining visits distributed among 2-5 alternatives. The principal variation stabilizes early and does not change in the last 20% of playouts.

## See also

- [Exploration vs. Exploitation](../concepts/exploration-exploitation) -- the theory behind C and UCB
- [Tree Policies](../concepts/tree-policies) -- how UCT and PUCT use these parameters
- [Configuration reference](../reference/configuration) -- all MCTS trait options
