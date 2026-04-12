---
sidebar_position: 8
id: 08-gumbel-search
---

# Gumbel Search

Standard MCTS suffers from a surprising flaw: more simulations don't always produce a better policy. The visit distribution can oscillate as search deepens, making it an unreliable training signal. Gumbel MuZero search fixes this by replacing UCT/PUCT root selection with Gumbel noise and Sequential Halving, guaranteeing monotonic policy improvement. The `mcts-gumbel` crate implements this algorithm as a standalone search engine that reuses the core crate's `GameState` trait.

**You will learn to:**
- Explain why Gumbel search improves on standard MCTS for training targets
- Implement the `GumbelEvaluator` trait for a game
- Run Gumbel search and read the `SearchResult`
- Compare Gumbel's improved policy with standard MCTS visit counts
- Configure `GumbelConfig` for different use cases

**Prerequisites:** [Neural Network Priors](./06-neural-network-priors.md).

## Why Gumbel?

PUCT allocates simulations by repeatedly selecting the most promising child at the root. This produces visit counts that are non-monotonic -- adding more simulations can shift the distribution away from the best move before shifting back. For competitive play this is tolerable, but for self-play training (where the visit distribution *is* the training target), non-monotonicity introduces noise into the learning signal.

Gumbel search (Danihelka et al., "Policy improvement by planning with Gumbel", ICLR 2022) replaces root selection entirely:

1. Sample Gumbel(0,1) noise for each legal action.
2. Select the top-m actions by `gumbel(a) + logit(a)`.
3. Allocate simulations via **Sequential Halving** -- repeatedly halve the candidate set, giving each survivor equal budget.
4. Compute an **improved policy** from `softmax(logit + value_scale * Q_completed)`.

Below the root, standard PUCT guides tree traversal. The result is a policy that provably improves with more simulations -- every additional simulation makes the output at least as good.

## The `GumbelEvaluator` trait

Gumbel search needs policy logits and a value estimate from your evaluator. The trait is simpler than the core crate's `Evaluator` -- one method, no search handles.

```rust,ignore
pub trait GumbelEvaluator<G: GameState>: Send {
    /// Returns (logits, value) where:
    /// - logits: one f64 per move (unnormalized log-probabilities)
    /// - value: state value for the current player, in [-1.0, 1.0]
    fn evaluate(&self, state: &G, moves: &[G::Move]) -> (Vec<f64>, f64);
}
```

Logits are unnormalized log-probabilities -- the search applies softmax internally. The value is from the current player's perspective, bounded to `[-1.0, 1.0]`. For a neural network, these map directly to the policy and value heads. For a heuristic, return uniform logits (`vec![0.0; moves.len()]`) and a heuristic score.

## Build a Nim evaluator

Nim from [tutorial 03](./03-two-player-games.md): a pile of stones, take 1 or 2 per turn, last stone wins. The theory is simple -- a position is losing if and only if `stones % 3 == 0`. This gives us a clean heuristic evaluator to test with.

The `GameState` implementation is identical to tutorial 03, so here it is in condensed form:

```rust,ignore
use mcts::{GameState, ProvenValue};
use mcts_gumbel::{GumbelSearch, GumbelConfig, GumbelEvaluator};

#[derive(Clone, Debug)]
struct Nim { stones: u8, current: Player }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Player { P1, P2 }

#[derive(Clone, Debug, PartialEq)]
enum NimMove { Take1, Take2 }

impl std::fmt::Display for NimMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NimMove::Take1 => write!(f, "Take1"),
            NimMove::Take2 => write!(f, "Take2"),
        }
    }
}

impl GameState for Nim {
    type Move = NimMove;
    type Player = Player;
    type MoveList = Vec<NimMove>;

    fn current_player(&self) -> Player { self.current }

    fn available_moves(&self) -> Vec<NimMove> {
        match self.stones {
            0 => vec![],
            1 => vec![NimMove::Take1],
            _ => vec![NimMove::Take1, NimMove::Take2],
        }
    }

    fn make_move(&mut self, mov: &NimMove) {
        match mov {
            NimMove::Take1 => self.stones -= 1,
            NimMove::Take2 => self.stones -= 2,
        }
        self.current = match self.current {
            Player::P1 => Player::P2,
            Player::P2 => Player::P1,
        };
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.stones == 0 { Some(ProvenValue::Loss) } else { None }
    }
}
```

Now the `GumbelEvaluator`. The heuristic uses the `stones % 3` theory: if the opponent faces a multiple of 3, we are winning.

```rust,ignore
struct NimEval;

impl GumbelEvaluator<Nim> for NimEval {
    fn evaluate(&self, state: &Nim, moves: &[NimMove]) -> (Vec<f64>, f64) {
        // Logits: prefer the move that leaves opponent on a multiple of 3
        let logits: Vec<f64> = moves.iter().map(|m| {
            let remaining = match m {
                NimMove::Take1 => state.stones - 1,
                NimMove::Take2 => state.stones - 2,
            };
            if remaining % 3 == 0 { 2.0 } else { 0.0 }
        }).collect();

        // Value: +1.0 if opponent faces multiple of 3, -1.0 if we do
        let value = if state.stones % 3 == 0 { -1.0 } else { 1.0 };

        (logits, value)
    }
}
```

The logits assign higher weight to moves that leave the opponent in a losing position. The value head gives `+1.0` when the current player is winning, `-1.0` when losing.

## Run Gumbel search

Create a `GumbelSearch`, call `search()`, and read the result.

```rust,ignore
fn main() {
    let state = Nim { stones: 7, current: Player::P1 };
    let mut search = GumbelSearch::new(NimEval, GumbelConfig::default());

    let result = search.search(&state, 64);

    println!("Best move: {:?}", result.best_move);
    println!("Root value: {:.3}", result.root_value);
    println!("Simulations used: {}", result.simulations_used);
    println!();

    for stat in &result.move_stats {
        println!(
            "  {:5}  visits={:<3}  Q={:+.3}  improved_policy={:.3}",
            stat.mov, stat.visits, stat.completed_q, stat.improved_policy
        );
    }
}
```

`search()` takes the root state and a simulation budget. The `SearchResult` contains:

- **`best_move`** -- the action selected by Gumbel + Sequential Halving.
- **`root_value`** -- the evaluator's estimate at the root (before search).
- **`move_stats`** -- per-move visits, completed Q-values, and the improved policy.
- **`simulations_used`** -- actual simulations run (may be less than the budget for trivial positions).

The improved policy in `move_stats` is the key output. It combines the prior logits with search-backed Q-values via `softmax(logit + value_scale * Q_completed)`, producing a distribution that is strictly better than the raw prior.

## Compare with standard MCTS

The same position with standard MCTS using `UCTPolicy`:

```rust,ignore
use mcts::tree_policy::UCTPolicy;
use mcts::*;

#[derive(Default)]
struct NimMCTS;

impl MCTS for NimMCTS {
    type State = Nim;
    type Eval = NimEvalClassic;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
}

// Standard evaluator (returns move priors as () and reward as i64)
struct NimEvalClassic;

impl Evaluator<NimMCTS> for NimEvalClassic {
    type StateEvaluation = Option<Player>;

    fn evaluate_new_state(
        &self, state: &Nim, moves: &Vec<NimMove>, _: Option<SearchHandle<NimMCTS>>,
    ) -> (Vec<()>, Option<Player>) {
        let winner = if state.stones == 0 {
            Some(match state.current { Player::P1 => Player::P2, Player::P2 => Player::P1 })
        } else { None };
        (vec![(); moves.len()], winner)
    }

    fn interpret_evaluation_for_player(&self, w: &Option<Player>, p: &Player) -> i64 {
        match w { Some(w) if w == p => 100, Some(_) => -100, None => 0 }
    }

    fn evaluate_existing_state(
        &self, _: &Nim, e: &Option<Player>, _: SearchHandle<NimMCTS>,
    ) -> Option<Player> { *e }
}
```

The key difference is in the output. Standard MCTS gives you raw visit counts -- you derive a policy by normalizing them. Gumbel search gives you the improved policy directly, which is a better training target because it incorporates both the prior and the search-backed Q-values in a principled way. For self-play training loops, this means less noise in the policy targets and faster learning convergence.

## Configure the search

`GumbelConfig` controls the search behavior:

| Field | Default | Effect |
|---|---|---|
| `m_actions` | 16 | Number of actions after Gumbel-Top-k sampling. Wider initial sampling explores more broadly but spreads the budget thinner. |
| `c_puct` | 1.25 | PUCT exploration constant for below-root traversal. Same role as in AlphaGo/AlphaZero. |
| `max_depth` | 200 | Maximum tree depth per simulation. Deeper trees capture longer-horizon play. |
| `value_scale` | 50.0 | Weight of Q-values relative to logits in the improved policy (`c_visit` in the paper). Higher values make the policy sharper. |
| `seed` | 42 | RNG seed for Gumbel noise. Change for diversity across searches. |

Adjust `value_scale` when the improved policy is too flat (increase it) or too sharp (decrease it). The right value depends on the magnitude of your logits -- if your network outputs logits in `[-5, 5]`, a `value_scale` of 50 means Q-values dominate; if logits are in `[-50, 50]`, you may need to increase it.

Adjust `m_actions` based on the branching factor. For games with fewer legal moves than `m_actions`, all moves are considered. For large action spaces (Go, continuous control), keep `m_actions` moderate (16--32) so the budget is not spread too thin.

```rust,ignore
let config = GumbelConfig {
    m_actions: 8,
    value_scale: 25.0,
    seed: 123,
    ..GumbelConfig::default()
};
let mut search = GumbelSearch::new(NimEval, config);
let result = search.search(&state, 128);
```

## What's next

See [Tree Policies](../concepts/tree-policies.md) for a deeper comparison of UCT, PUCT, and Gumbel selection. For integrating a neural network evaluator with batching, see [Batched Evaluation](../how-to/batched-evaluation.md). Gumbel search is particularly valuable in self-play training loops, where the improved policy serves as a higher-quality target than raw visit counts.
