---
sidebar_position: 6
id: parallel-mcts
---

# Lock-Free Parallel Search

MCTS is naturally parallel. Each playout is a semi-independent descent through the tree, and multiple playouts can proceed simultaneously. The challenge is making this concurrent access safe without destroying performance.

This library uses lock-free data structures for all hot-path operations. No mutex is held during selection, expansion, or backpropagation. The result is near-linear scaling on typical hardware, limited by memory bandwidth and tree depth rather than contention.

## The challenge

Multiple threads traverse the same tree simultaneously. Each thread selects a path from root to leaf, expands a new node, evaluates it, and backpropagates the result. Every step involves reading or writing shared state:

- **Selection** reads visit counts and reward sums to compute UCB scores.
- **Expansion** writes a new child node pointer.
- **Backpropagation** updates visit counts and reward sums along the entire path.

The naive solution -- a global mutex around the tree -- serializes all operations. Throughput collapses to single-threaded speed because threads spend most of their time waiting for the lock. Per-node mutexes reduce contention but add memory overhead and still create bottlenecks at frequently-visited nodes (the root and its immediate children).

The lock-free approach eliminates waiting entirely. Threads never block on each other. Concurrent writes are resolved with atomic operations. Occasionally a thread's work is discarded (e.g., when two threads try to expand the same node), but this is cheaper than the alternative.

## Tree structure: AtomicPtr expansion

Each `MoveInfo` holds an `AtomicPtr<SearchNode>` pointing to its child. Initially null (child not yet expanded).

When a thread wants to expand a child, it creates the new node, then attempts a compare-and-swap:

```
compare_exchange(null, new_node, Relaxed, Relaxed)
```

If the pointer was still null, the swap succeeds and this thread's node becomes the child. If another thread already expanded the same child, the swap fails. The losing thread drops its newly-created node and uses the winner's node instead.

This is the only point where work is wasted, and it is rare. Two threads must reach the same unexpanded child at nearly the same instant. The `expansion_contention_events` counter in `SearchTree::diagnose()` tracks how often this happens. In practice, it is a fraction of a percent of total expansions.

Nodes are never moved after creation. `Box::into_raw` allocates each node on the heap, and the pointer stored in `AtomicPtr` remains valid for the lifetime of the tree. No garbage collection is needed during search. Nodes are freed only when the tree is dropped, using an iterative stack-based traversal to avoid stack overflow from deep recursive drops.

## Statistics: relaxed atomics

Visit counts and reward sums are stored as `AtomicUsize` and `AtomicI64`, accessed with `Ordering::Relaxed`.

Relaxed ordering is the weakest memory ordering. It guarantees atomicity (no torn reads/writes) but provides no guarantees about when other threads see the update. Thread A might increment a visit count, but thread B might not see the new value for several microseconds.

This is safe for MCTS because the algorithm is statistically robust. Selection uses visit counts and reward sums to compute UCB scores. A stale visit count of 99 instead of 100 changes the UCB score by a negligible amount. Over thousands of playouts, these tiny errors average out. The convergence proof does not depend on every thread having a perfectly synchronized view of the tree.

Using relaxed ordering avoids the performance cost of cache coherence protocols. `SeqCst` or `AcqRel` ordering would force cache line invalidations across cores on every update. With relaxed ordering, each core updates its local cache and the value propagates naturally through the cache hierarchy. On x86 (which has strong memory ordering by default), this distinction matters less. On ARM, it matters a great deal.

## Virtual loss: parallel diversification

Without virtual loss, parallel threads tend to follow the same path. They all read the same UCB scores and select the same child. The threads produce correlated playouts, reducing the effective sample diversity.

Virtual loss solves this by making each thread's path look temporarily worse. When a thread descends through a node, it atomically subtracts `virtual_loss` from the reward sum and increments the visit count:

```rust
fn down(&self, manager: &Spec) {
    self.sum_evaluations.fetch_sub(manager.virtual_loss(), Relaxed);
    self.visits.fetch_add(1, Relaxed);
}
```

This depresses the node's average reward, pushing subsequent threads toward different children. After the playout completes and the actual reward is determined, backpropagation adds back the virtual loss plus the real reward:

```rust
fn up(&self, manager: &Spec, evaln: i64) {
    let delta = evaln + manager.virtual_loss();
    self.sum_evaluations.fetch_add(delta, Relaxed);
}
```

The visit count increment during descent is permanent. The reward correction during backpropagation is additive. The net effect: the node ends up with one additional visit and the real reward, as if virtual loss had never been applied. The pessimism was temporary.

The magnitude of the virtual loss controls how aggressively threads diversify. A value of 0 (the default) disables diversification entirely. Values of 1-10 are typical. Larger values force threads apart more aggressively but introduce a temporary bias that persists until backpropagation corrects it. For neural-network evaluators with slow leaf evaluation, this bias can persist for hundreds of microseconds -- long enough to distort selection at heavily-visited nodes.

## Thread scaling

Parallel MCTS shows diminishing returns. The theoretical speedup from N threads is N, but several factors reduce the practical speedup:

**Tree depth.** Shallow trees (Tic-Tac-Toe, simple games) have few nodes and high contention at the root. Most threads are examining the same few children. Deep trees (Go, chess) have more nodes and less contention.

**Memory bandwidth.** Each thread reads node statistics from shared memory. On modern hardware with 4-8 cores sharing an L3 cache, this is not a bottleneck. Beyond 8 cores, cross-socket memory access becomes expensive.

**Virtual loss bias.** With many concurrent threads, the virtual loss bias is larger and persists longer. The search makes slightly worse decisions per playout, partially offsetting the higher playout rate.

**Evaluation cost.** If leaf evaluation is cheap (random rollouts), the tree traversal and backpropagation dominate and parallelism helps less. If leaf evaluation is expensive (neural network), parallelism helps more because threads spend most of their time in evaluation, not in the tree.

In practice, 4-8 threads provide good scaling for most games. Beyond 8 threads, benchmark your specific application. The `perf_test` method runs a 10-second measurement, reporting nodes per second, which gives a direct measure of scaling efficiency.

An important distinction: scaling in nodes per second is not the same as scaling in playing strength. Adding threads increases throughput (more playouts per wall-clock second), but each additional playout provides diminishing marginal information due to the virtual loss bias and the tree's diminishing returns from additional samples in already-well-explored regions. The practical question is always "does doubling threads noticeably improve the move chosen?" rather than "does doubling threads double the playout count?"

## Thread-local data

Each thread maintains its own `ThreadDataFull` containing:

- **Policy thread-local data.** An RNG for tie-breaking during selection (when multiple children have equal UCB scores, one is chosen randomly).
- **Extra thread data.** User-defined per-thread state, available through `SearchHandle`.
- **Path tracking.** Vectors for the current playout's path, reused across playouts to avoid allocation. These are not shared between threads.
- **Chance RNG.** A per-thread RNG for sampling chance outcomes, seeded deterministically when `rng_seed()` is set.

No thread-local data is shared. Each thread reads from the shared tree and writes to its own path vectors, then atomically updates the shared tree during backpropagation. This separation eliminates false sharing and keeps the hot loop allocation-free after the first playout.

When `rng_seed()` is set on the MCTS config, each thread's RNG is seeded deterministically: `base_seed + thread_id`. This makes parallel search reproducible for testing and debugging, despite the inherent non-determinism of thread scheduling. The chance RNG uses a different offset (`seed + 0xCAFE_BABE`) to avoid correlation between selection tie-breaking and chance sampling.

## Comparison with other parallelization strategies

**Root parallelization.** Run N independent searches, each with its own tree. After all searches complete, vote on the best move (e.g., pick the move with the most total visits across all trees). No shared state, no contention, trivially parallel. But the trees cannot share information: each search independently discovers the same tactical patterns. Root parallelization wastes work proportional to the amount of shared structure in the game tree.

**Leaf parallelization.** A single tree with sequential selection and expansion, but leaf evaluation is batched and sent to a parallel evaluator (e.g., a GPU). This is what `BatchedEvaluatorBridge` provides. The tree traversal is still sequential (or lightly parallel with virtual loss), but the expensive evaluation step runs in parallel. Optimal when evaluation dominates the per-playout cost.

**Tree parallelization (this library).** The entire search is parallel: selection, expansion, evaluation, and backpropagation. This is the most general approach and the only one that scales well when leaf evaluation is cheap. The cost is complexity in the lock-free data structures and the virtual loss bias.

These strategies can be combined. Tree parallelization with batched leaf evaluation uses tree parallelism for the search structure and leaf parallelism for the neural network. This combination -- used by Leela Chess Zero and similar engines -- achieves high throughput on both CPU and GPU.

## Practical considerations

**Async vs scoped search.** The library provides both models. `playout_n_parallel` uses scoped threads that borrow the tree -- the call blocks until all playouts complete. `playout_parallel_async` spawns persistent threads that run until the `AsyncSearch` handle is dropped. The async model is useful for time-controlled search (run until the clock runs out), while scoped search is simpler for fixed playout budgets.

**Node limit.** The `node_limit()` method caps total tree nodes. When reached, all threads receive `false` from `playout()` and stop. This prevents memory exhaustion in long-running searches. The limit is checked with a relaxed atomic load, so threads may slightly overshoot due to concurrent expansion. In practice, the overshoot is negligible.

**Stop signal.** Async search threads check a shared `AtomicBool` stop signal between playouts. The check uses `SeqCst` ordering to ensure the signal is visible promptly across all cores. When the `AsyncSearch` handle is dropped, the signal is set and all threads join.

**Solver interaction.** When MCTS-Solver proves the root node (all moves resolved to Win, Loss, or Draw), `playout()` returns `false` and all threads stop. Similarly, when score bounds converge at the root, the search halts. These checks happen at the top of each playout, so at most one unnecessary playout per thread occurs after the root is proven.

**Reproducibility.** Despite using lock-free concurrency, the search is reproducible when `rng_seed()` is set and the thread count is fixed. Each thread's RNG is deterministically seeded, and the relaxed atomic ordering does not affect the final statistics in expectation. In practice, exact bit-for-bit reproducibility across runs depends on thread scheduling, which varies. For testing, single-threaded search with a fixed seed is fully deterministic.

**Memory usage.** Each search node is individually heap-allocated. With many threads expanding rapidly, allocation pressure can become a factor. The library does not use an arena allocator -- this keeps the code simpler and avoids the complexity of thread-safe arena management. For very high-throughput scenarios (millions of nodes per second), a custom allocator like jemalloc can improve performance by reducing contention in the system allocator.

**Backpropagation ordering.** Threads backpropagate in reverse order along their playout path. Since different threads have different paths, backpropagation updates are interleaved. Node statistics may temporarily reflect partial updates from multiple threads. This is safe because each individual atomic operation (fetch_add, fetch_sub) is correct in isolation. The cumulative effect converges to the same statistics regardless of interleaving order.

**Tree re-rooting.** The `advance()` method cannot be called while an async search is running. It requires exclusive access to the tree (`Arc::get_mut`), which is only possible when no search threads hold a reference. Always halt the async search before advancing the root.
