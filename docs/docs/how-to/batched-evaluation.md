---
sidebar_position: 4
id: batched-evaluation
---

# Batch Neural Network Evaluation

Evaluate multiple leaf nodes in a single forward pass for 10-100x faster GPU inference.

**You will learn to:**
- Implement the `BatchEvaluator` trait
- Configure `BatchConfig` for your latency/throughput trade-off

**Prerequisites:** Complete [Neural Network Priors](../tutorials/06-neural-network-priors).

## The `BatchEvaluator` trait

Instead of evaluating one leaf at a time, `BatchEvaluator` receives a slice of states and returns all evaluations at once:

```rust
use treant::batch::*;
use treant::*;

impl BatchEvaluator<MyMCTS> for MyNNEvaluator {
    type StateEvaluation = NNOutput;

    fn evaluate_batch(
        &self,
        states: &[(MyGameState, Vec<MyMove>)],
    ) -> Vec<(Vec<f64>, NNOutput)> {
        // Build a batch tensor from all states
        let inputs = states.iter()
            .map(|(state, _)| state.to_tensor())
            .collect::<Vec<_>>();

        // Single forward pass through the network
        let outputs = self.network.forward_batch(&inputs);

        // Return (move_priors, state_value) for each state
        outputs.into_iter()
            .zip(states.iter())
            .map(|(output, (_, moves))| {
                let priors = extract_priors(&output, moves);
                let value = NNOutput { value: output.value };
                (priors, value)
            })
            .collect()
    }

    fn interpret_evaluation_for_player(
        &self,
        evaluation: &NNOutput,
        player: &Player,
    ) -> i64 {
        // Convert [-1.0, 1.0] value to integer
        (evaluation.value * 1000.0) as i64
    }
}
```

The return vector must have the same length as the input slice. Each element is `(move_evaluations, state_evaluation)` -- the same shape as `Evaluator::evaluate_new_state`.

## Configure `BatchConfig`

`BatchConfig` controls how long the collector thread waits before dispatching an incomplete batch:

```rust
use std::time::Duration;

let config = BatchConfig {
    max_batch_size: 32,                 // fire when 32 leaves are queued
    max_wait: Duration::from_millis(5), // or after 5ms, whichever comes first
};
```

| Parameter | Typical range | Trade-off |
|---|---|---|
| `max_batch_size` | 8--64 | Larger = better GPU utilization, higher latency |
| `max_wait` | 1--10ms | Longer = fuller batches, but threads block longer |

Start with `max_batch_size: 8` and `max_wait: 1ms` (the defaults). Increase batch size until GPU utilization plateaus. Increase wait time only if batches are consistently underfilled.

## Wire it up

Wrap the `BatchEvaluator` in a `BatchedEvaluatorBridge` and use it as a normal `Evaluator`:

```rust
let evaluator = BatchedEvaluatorBridge::new(
    MyNNEvaluator::new("model.pt"),
    BatchConfig {
        max_batch_size: 32,
        max_wait: Duration::from_millis(5),
    },
);

let mut mcts = MCTSManager::new(
    initial_state,
    MyMCTS,
    evaluator,
    AlphaGoPolicy::new(1.5),
    (),
);

// Search threads automatically batch their evaluations
mcts.playout_n_parallel(10_000, 4);
```

The bridge spawns a collector thread that accumulates evaluation requests from search threads and dispatches them to `evaluate_batch` in groups. Search threads block until their batch completes.

## How it works

1. Search threads reach leaf nodes and call `evaluate_new_state` on the bridge.
2. The bridge enqueues each request and blocks the calling thread.
3. A collector thread waits until `max_batch_size` requests accumulate or `max_wait` elapses.
4. The collector calls `evaluate_batch` with all queued states.
5. Results are distributed back to the waiting search threads.

Use at least as many search threads as `max_batch_size` to keep the batch pipeline full.

## Expected result

With a GPU-based evaluator and `max_batch_size: 32`, expect 10-30x throughput improvement over evaluating one state at a time. The exact speedup depends on your model size and GPU.

## See also

- [Architecture](../concepts/architecture) -- how evaluation fits into the search loop
- [Traits reference](../reference/traits) -- full `BatchEvaluator` and `Evaluator` signatures
