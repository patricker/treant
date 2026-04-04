---
sidebar_position: 6
id: custom-tree-policy
---

# Write a Custom Tree Policy

Implement a selection algorithm beyond the built-in UCT and PUCT policies.

**You will learn to:**
- Implement the `TreePolicy` trait
- Use `PolicyRng::select_by_key` for deterministic tie-breaking

**Prerequisites:** Complete [Your First Search](../tutorials/02-first-search). Read [Tree Policies](../concepts/tree-policies).

## The `TreePolicy` trait

A tree policy decides which child to explore during each playout's descent through the tree.

```rust
pub trait TreePolicy<Spec: MCTS<TreePolicy = Self>>: Sync + Sized {
    /// Per-move data from the evaluator (e.g., `()` for UCT, `f64` prior for PUCT).
    type MoveEvaluation: Sync + Send + Default;

    /// Thread-local data (e.g., an RNG for tie-breaking).
    type ThreadLocalData: Default;

    /// Select the most promising child to explore.
    fn choose_child<'a, MoveIter>(
        &self,
        moves: MoveIter,
        handle: SearchHandle<Spec>,
    ) -> &'a MoveInfo<Spec>
    where
        MoveIter: Iterator<Item = &'a MoveInfo<Spec>> + Clone;

    // Optional overrides:
    fn validate_evaluations(&self, _evalns: &[Self::MoveEvaluation]) {}
    fn seed_thread_data(&self, _tld: &mut Self::ThreadLocalData, _seed: u64) {}
    fn compare_move_evaluations(
        &self, _a: &Self::MoveEvaluation, _b: &Self::MoveEvaluation,
    ) -> std::cmp::Ordering { std::cmp::Ordering::Equal }
    fn apply_dirichlet_noise(
        &self, _moves: &mut [MoveInfo<Spec>],
        _epsilon: f64, _alpha: f64, _rng: &mut SmallRng,
    ) {}
}
```

`choose_child` is called once per node during descent. It receives an iterator over all expanded children and a `SearchHandle` that provides access to the MCTS config and thread-local data. Return the child to explore next.

## Use `PolicyRng` for tie-breaking

`PolicyRng::select_by_key` scores every child and returns the one with the highest score. When multiple children tie, one is chosen uniformly at random.

```rust
use mcts::tree_policy::PolicyRng;

handle
    .thread_data()
    .policy_data  // your ThreadLocalData
    .select_by_key(moves, |mov| {
        // Return a score for this child. Higher = more likely to be selected.
        score_child(mov)
    })
    .unwrap()
```

This is the recommended way to implement `choose_child`. It handles tie-breaking correctly and supports deterministic replay when `rng_seed()` is set.

## Example: Thompson Sampling

Thompson Sampling selects children by sampling from each child's posterior distribution rather than using an upper confidence bound. Here is a skeleton implementation using a Beta distribution:

```rust
use mcts::*;
use mcts::tree_policy::*;

#[derive(Clone, Debug)]
struct ThompsonPolicy;

impl<Spec: MCTS<TreePolicy = Self>> TreePolicy<Spec> for ThompsonPolicy {
    type MoveEvaluation = ();
    type ThreadLocalData = PolicyRng;

    fn choose_child<'a, MoveIter>(
        &self,
        moves: MoveIter,
        mut handle: SearchHandle<Spec>,
    ) -> &'a MoveInfo<Spec>
    where
        MoveIter: Iterator<Item = &'a MoveInfo<Spec>> + Clone,
    {
        let fpu = handle.mcts().fpu_value();
        handle
            .thread_data()
            .policy_data
            .select_by_key(moves, |mov| {
                let visits = mov.visits();
                if visits == 0 {
                    return fpu;
                }
                // Model reward as Beta(successes + 1, failures + 1)
                let sum = mov.sum_rewards() as f64;
                let alpha = sum.max(0.0) + 1.0;
                let beta = (visits as f64 - sum).max(0.0) + 1.0;
                // Approximate sample: use mean + noise scaled by variance
                // Replace with a real Beta sample for production use
                let mean = alpha / (alpha + beta);
                let variance = (alpha * beta)
                    / ((alpha + beta).powi(2) * (alpha + beta + 1.0));
                mean + variance.sqrt()
            })
            .unwrap()
    }

    fn seed_thread_data(&self, tld: &mut PolicyRng, seed: u64) {
        *tld = PolicyRng::seeded(seed);
    }
}
```

Wire it into your MCTS config:

```rust
impl MCTS for MyMCTS {
    type TreePolicy = ThompsonPolicy;
    // ...
}

let mcts = MCTSManager::new(state, MyMCTS, eval, ThompsonPolicy, ());
```

## Optional methods

| Method | Purpose |
|---|---|
| `validate_evaluations` | Assert invariants on move evaluations after node creation (e.g., priors sum to 1) |
| `compare_move_evaluations` | Sort moves by priority for progressive widening. Return `Greater` for higher priority. |
| `apply_dirichlet_noise` | Add exploration noise to root move evaluations during self-play |
| `seed_thread_data` | Initialize thread-local data from a seed for deterministic replay |

## Expected result

A custom tree policy plugs into the search with zero overhead beyond what `choose_child` itself costs. The policy is called once per node per playout, so keep it fast -- avoid allocations and complex math when possible.

## See also

- [Tree Policies](../concepts/tree-policies) -- theory behind UCT, PUCT, and selection formulas
- [Traits reference](../reference/traits) -- full trait signatures for `TreePolicy`, `MoveInfo`, `SearchHandle`
