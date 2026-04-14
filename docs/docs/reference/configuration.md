---
sidebar_position: 2
id: configuration
---

# Configuration Options

Every configurable method in `GameState` and `MCTS`, in one place.

**Conventions:** "Required" means no default -- you must implement it. All other methods have defaults you can override. Types shown are return types.

---

## GameState methods

| Method | Type | Default | Purpose | Tutorial |
|---|---|---|---|---|
| `current_player()` | `Self::Player` | required | Identifies whose turn it is | [2](../tutorials/02-first-search.md) |
| `available_moves()` | `Self::MoveList` | required | Legal moves from this state; empty = terminal | [2](../tutorials/02-first-search.md) |
| `make_move(mov)` | `()` | required | Apply a move, mutating state in place | [2](../tutorials/02-first-search.md) |
| `max_children(visits)` | `usize` | `usize::MAX` | Max children to expand at this visit count; override for progressive widening | [Progressive widening](../how-to/progressive-widening.md) |
| `terminal_value()` | `Option<ProvenValue>` | `None` | Classify terminal state for MCTS-Solver (current player's perspective) | [4](../tutorials/04-solving-games.md) |
| `terminal_score()` | `Option<i32>` | `None` | Exact minimax score of terminal state for Score-Bounded MCTS | [4](../tutorials/04-solving-games.md) |
| `chance_outcomes()` | `Option<Vec<(Move, f64)>>` | `None` | Chance event outcomes with probabilities (must sum to 1.0) | [5](../tutorials/05-stochastic-games.md) |

---

## MCTS associated types

| Type | Bounds | Purpose | Tutorial |
|---|---|---|---|
| `State` | `GameState + Send + Sync + 'static` | The game state type | [2](../tutorials/02-first-search.md) |
| `Eval` | `Evaluator<Self> + Send + 'static` | State and move evaluator | [2](../tutorials/02-first-search.md) |
| `TreePolicy` | `TreePolicy<Self> + Send + 'static` | Child selection policy | [2](../tutorials/02-first-search.md) |
| `NodeData` | `Default + Sync + Send + 'static` | Custom per-node data (accessible via `SearchHandle`) | [7](../tutorials/07-advanced-search.md) |
| `TranspositionTable` | `TranspositionTable<Self> + Send + 'static` | Transposition table (use `()` for none) | [7](../tutorials/07-advanced-search.md) |
| `ExtraThreadData` | `'static` | Custom per-thread data (accessible via `SearchHandle`) | [7](../tutorials/07-advanced-search.md) |

---

## MCTS methods

### Search control

| Method | Type | Default | Purpose | Tutorial |
|---|---|---|---|---|
| `virtual_loss()` | `i64` | `0` | Bias subtracted during descent, added back during backprop; discourages thread collision | [Parallel MCTS](../concepts/parallel-mcts.md) |
| `fpu_value()` | `f64` | `f64::INFINITY` | First Play Urgency: value for unvisited children; `INFINITY` = try all first; finite = let prior guide | [6](../tutorials/06-neural-network-priors.md) |
| `visits_before_expansion()` | `u64` | `1` | Visits to a leaf before creating a tree node | [7](../tutorials/07-advanced-search.md) |
| `node_limit()` | `usize` | `usize::MAX` | Maximum tree nodes; search halts when reached | [7](../tutorials/07-advanced-search.md) |
| `max_playout_length()` | `usize` | `1_000_000` | Safety cap: panics if a single playout exceeds this depth | [7](../tutorials/07-advanced-search.md) |
| `max_playout_depth()` | `usize` | `usize::MAX` | Quality knob: forces leaf evaluation when exceeded | [7](../tutorials/07-advanced-search.md) |

### Selection and output

| Method | Type | Default | Purpose | Tutorial |
|---|---|---|---|---|
| `select_child_after_search(children)` | `&MoveInfo<Self>` | Most-visited (solver/bounds-aware) | Post-search child selection; override for custom behavior | [7](../tutorials/07-advanced-search.md) |
| `selection_temperature()` | `f64` | `0.0` | Temperature for `best_move()`; 0 = argmax, 1 = proportional to visits | [6](../tutorials/06-neural-network-priors.md) |

### Solver and bounds

| Method | Type | Default | Purpose | Tutorial |
|---|---|---|---|---|
| `solver_enabled()` | `bool` | `false` | Enable MCTS-Solver: propagate proven win/loss/draw values | [4](../tutorials/04-solving-games.md) |
| `score_bounded_enabled()` | `bool` | `false` | Enable Score-Bounded MCTS: track `[lower, upper]` minimax bounds | [4](../tutorials/04-solving-games.md) |

### Stochastic games

| Method | Type | Default | Purpose | Tutorial |
|---|---|---|---|---|
| `closed_loop_chance()` | `bool` | `false` | Give each chance outcome its own tree node (larger tree, better per-outcome stats) | [5](../tutorials/05-stochastic-games.md) |

### Determinism and noise

| Method | Type | Default | Purpose | Tutorial |
|---|---|---|---|---|
| `rng_seed()` | `Option<u64>` | `None` | Seed for deterministic search; each thread gets `seed + thread_id` | [7](../tutorials/07-advanced-search.md) |
| `dirichlet_noise()` | `Option<(f64, f64)>` | `None` | `(epsilon, alpha)` for root exploration noise: `(1-eps)*prior + eps*Dir(alpha)` | [6](../tutorials/06-neural-network-priors.md) |

### Callbacks

| Method | Type | Default | Purpose | Tutorial |
|---|---|---|---|---|
| `on_backpropagation(evaln, handle)` | `()` | no-op | Called for each node on the playout path during backpropagation | [7](../tutorials/07-advanced-search.md) |
| `cycle_behaviour()` | `CycleBehaviour<Self>` | `PanicWhenCycleDetected` (with TT) / `Ignore` (without TT) | Strategy for graph cycles from transposition tables | [7](../tutorials/07-advanced-search.md) |

---

## Interaction matrix

Some options interact with or require others.

| Option | Requires | Interacts with |
|---|---|---|
| `solver_enabled()` | `GameState::terminal_value()` or `terminal_score()` | `select_child_after_search()` prefers proven wins |
| `score_bounded_enabled()` | `GameState::terminal_score()` or `terminal_value()` | Converged bounds set proven values when solver is also active |
| `closed_loop_chance()` | `GameState::chance_outcomes()` | Increases tree size; interacts with `node_limit()` |
| `dirichlet_noise()` | `TreePolicy::apply_dirichlet_noise()` (non-trivial impl) | Only effective with `AlphaGoPolicy` or custom policy with `f64` priors |
| `fpu_value()` (finite) | Nothing | Best combined with `AlphaGoPolicy` and neural network priors |
| `virtual_loss()` | Nothing | Only meaningful with parallel search (`playout_n_parallel`, etc.) |
| `selection_temperature()` | Nothing | Only affects `best_move()`; `principal_variation()` always uses argmax |
| `rng_seed()` | Nothing | Seeds policy thread data via `TreePolicy::seed_thread_data()` |
| `max_children()` | Moves returned in priority order | Works with `TreePolicy::compare_move_evaluations()` for ordering |

---

## `treant-gumbel` configuration

The `treant-gumbel` crate uses `GumbelConfig` instead of the `MCTS` trait. See [Gumbel Search tutorial](../tutorials/08-gumbel-search.md) and [Traits Reference](./traits.md).

| Field | Type | Default | Purpose |
|---|---|---|---|
| `m_actions` | `usize` | `16` | Actions to consider after Gumbel-Top-k sampling; higher = broader search |
| `c_puct` | `f64` | `1.25` | PUCT exploration constant for below-root tree traversal |
| `max_depth` | `usize` | `200` | Maximum depth per simulation before forcing leaf evaluation |
| `value_scale` | `f64` | `50.0` | Scale factor mapping Q-values to logit scale; controls Q vs prior balance in the improved policy |
| `seed` | `u64` | `42` | RNG seed for reproducible Gumbel noise sampling |

**Tuning guidance:**
- `m_actions`: set to the typical number of legal moves or smaller. For games with 200+ moves, 16-32 is typical. For games with fewer than 10 moves, use the move count.
- `value_scale`: higher values make the improved policy sharper (more exploitation). Lower values let the prior dominate. The paper uses 50.0 with Q-values in \[-1, 1\].
- `c_puct`: same role as in `AlphaGoPolicy`. Typical range: 1.0-2.5.
