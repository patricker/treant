# MCTS Crate Modernization Plan

**Date**: 2026-04-03
**Source**: Fork of zxqfl/mcts v0.3.0 (partially modernized: rand 0.8, vec_storage_reuse replacing smallvec)
**Goal**: A modern, safe, narrative-ready MCTS library that works with fabula + any DataSource
**Approach**: Modernize the fork in place. The architecture (Evaluator + TreePolicy + GameState separation) is sound and maps well to narrative MCTS. We clean up safety, add missing features, and incorporate design ideas from recon_mcts where they fit.

---

## Current State

- **Edition 2018**, compiles clean on stable Rust 1.94
- **Deps**: `rand = "0.8"`, `vec_storage_reuse = "0.1"` (already modernized from the original's crossbeam 0.3 / rand 0.4 / smallvec 0.6)
- **Internal crossbeam_mini**: 407 lines of custom scoped threading with `mem::transmute` for lifetime evasion -- the original crossbeam dep was removed but the unsafe reimplementation remains
- **32 `unsafe` occurrences** across the crate (see breakdown below)
- **No tests** (only a counting_game example and a benchmark)
- **i64-only evaluations** throughout (AtomicI64 for sum_evaluations, `interpret_evaluation_for_player` returns `i64`)
- **No tree re-rooting** (`reset()` wipes everything)
- **No progressive widening** (`available_moves()` called once, returns all moves)

### Unsafe Breakdown (verified)

| Location | Count | Nature |
|----------|-------|--------|
| crossbeam_mini/ | 7 | mem::transmute, unsafe fn decls, Send/Sync impls, Box::from_raw |
| lib.rs | 4 | unsafe fn spawn_worker_thread + 3 call sites |
| search_tree.rs | 11 | Raw pointer derefs for lock-free tree (AtomicPtr ops) |
| transposition_table.rs | 9 | unsafe trait decl + impls, get_unchecked, raw ptr derefs |
| tree_policy.rs | 1 | get_unchecked with redundant bounds guard |
| **Total** | **32** | |

> **Review note**: Phase 1 eliminates crossbeam_mini (7) + lib.rs spawn (4) + tree_policy (1) = **12 unsafe items**. The remaining 20 in search_tree.rs and transposition_table.rs are structural -- they implement the lock-free concurrent tree and are not targeted by this plan. This is reasonable: replacing them would require a fundamentally different tree architecture.

### Design ideas borrowed from recon_mcts

[recon_mcts](https://github.com/trtsl/recon_mcts) (0.1.0, all-safe Rust, DAG-based) has a different architecture that we won't adopt wholesale (mandatory transposition DAG is wrong for narrative -- states reached via different event sequences carry different narrative meaning, and DAG backprop is O(n^2)). But several of its design ideas are worth incorporating into our fork:

- **Generic `Score` type with no bounds** -- recon_mcts makes Score a fully generic associated type. We adopt this via the `ScoreType` trait in Phase 2.
- **`Option<Score>` backprop short-circuit** -- `backprop_scores` returning `None` stops upward propagation when the score hasn't meaningfully changed. We adopt this in Phase 3 via an `on_backpropagation` enhancement.
- **`Explore` vs `Exploit` flag in selection** -- recon_mcts passes a `SelectNodeState` enum to its selection function, cleanly separating exploration (during search) from exploitation (choosing final action). We adopt this in Phase 4 as a `TreePolicy` enhancement.
- **`Arc`/`Weak` cascade pruning for root advancement** -- dropping the old root cascades cleanup through unreachable subtrees via refcounting. Elegant and correct. Informs our Phase 3 design, though we use the existing `orphaned` Vec mechanism instead of Arc/Weak.

---

## Phase 1 -- Safety & Modernization (no API changes)

Replace internal unsafe threading with `std::thread::scope` (stable since Rust 1.63). This eliminates the most dangerous unsafe code (transmute for lifetime evasion) without changing any public API.

### 1.1 Replace crossbeam_mini with std::thread::scope

**What**: Delete `src/crossbeam_mini/` entirely (407 lines: mod.rs, scoped.rs, atomic_option.rs). Replace `spawn_worker_thread` in lib.rs with `std::thread::scope`.

**Why**: The crossbeam_mini module exists solely because `std::thread::spawn` requires `'static` closures. `std::thread::scope` (stabilized Rust 1.63) solves this natively -- scoped threads can borrow from the parent stack. This removes the `mem::transmute` that extends closure lifetimes, which is the single most dangerous unsafe block in the crate.

**How**: The current parallel playout methods (`playout_parallel_for`, `playout_n_parallel`, `playout_parallel_async`) call `spawn_worker_thread` which calls `crossbeam_mini::spawn_unsafe`. Replace with:

```rust
pub fn playout_n_parallel(&mut self, n: u32, num_threads: usize) {
    let search_tree = &self.search_tree;
    let stop = AtomicBool::new(false);
    let count = AtomicU32::new(0);
    std::thread::scope(|s| {
        for _ in 0..num_threads {
            s.spawn(|| {
                let mut tld = ThreadDataFull::default();
                while count.fetch_add(1, Ordering::Relaxed) < n {
                    if !search_tree.playout(&mut tld) { break; }
                }
            });
        }
    });
}
```

The `AsyncSearch` and `AsyncSearchOwned` types need rethinking -- they currently hold `JoinHandle`s from unsafe spawns. With `std::thread::scope`, threads can't outlive the scope. Options:
- **Option A**: Keep async API using `std::thread::spawn` with `Arc` for shared state (search tree behind Arc). Requires `SearchTree: Send + Sync` (it already is).
- **Option B**: Remove async API, only offer `playout_parallel_for` and `playout_n_parallel` (blocking). Simpler but less flexible.

**Recommendation**: Option A for async, scoped threads for blocking. The GM needs async search (search while simulation continues).

> **Review note**: `playout_n_parallel` (lib.rs:356-377) already uses `crossbeam_mini::scope` internally -- it's the closest to a direct port. The async methods (lib.rs:320-350) use `spawn_worker_thread` which calls `crossbeam_mini::spawn_unsafe` directly. Option A is correct: `AsyncSearch`/`AsyncSearchOwned` need `std::thread::spawn` + `Arc` since scoped threads can't outlive their scope. The current `into_playout_parallel_async` already boxes the manager -- wrapping in `Arc` instead of `Box` is a natural fit.

**Files**: `src/lib.rs` (spawn_worker_thread, playout_parallel_*, AsyncSearch), delete `src/crossbeam_mini/`
**Tests**: Parallel counting_game example should produce same results
**Effort**: 3-4 hours

### 1.2 Edition bump to 2021

**What**: Change `edition = "2018"` to `edition = "2021"` in Cargo.toml.

**Why**: Enables modern Rust idioms (closure capture rules, IntoIterator for arrays). No code changes expected -- 2021 is backward compatible for this crate's patterns.

**Files**: `Cargo.toml`
**Effort**: 5 minutes + verify `cargo test`

### 1.3 Simplify atomics.rs

**What**: Drop the 32-bit nightly workaround. Use `AtomicI64`/`AtomicU64` directly (stable since Rust 1.34). Remove the `FakeI64`/`FakeU64` aliases and the `compile_error!` for non-64-bit without nightly.

**Current** (src/atomics.rs):
```rust
#[cfg(target_pointer_width = "64")]
pub type AtomicI64 = std::sync::atomic::AtomicIsize;  // wrong type!

#[cfg(target_pointer_width = "64")]
pub type FakeI64 = isize;  // cast bridge for fetch_add
```

**After**:
```rust
pub use std::sync::atomic::{AtomicI64, AtomicU64, AtomicBool, AtomicPtr, AtomicUsize, Ordering};
```

**Why**: The current code uses `AtomicIsize` as a stand-in for `AtomicI64` on 64-bit platforms. This is technically correct (same size) but confusing and unnecessary since `AtomicI64` is stable. It also blocks the Phase 2 f64 conversion since we need to understand what's actually atomic.

> **Review note**: The `FakeI64` type is used in search_tree.rs for casting i64 values before `fetch_add`/`fetch_sub` on the mistyped atomics. Eliminating `FakeI64` will clean up those cast sites (e.g., `delta as FakeI64` becomes just `delta`). This doesn't remove unsafe blocks but improves clarity.

**Files**: `src/atomics.rs`, update all `FakeI64`/`FakeU64` usage in `search_tree.rs`
**Effort**: 30 minutes

### 1.4 Remove unsafe in tree_policy.rs

**What**: Replace `unsafe { *self.reciprocals.get_unchecked(x) }` with a safe bounds check.

**Current** (tree_policy.rs:66-72):
```rust
fn reciprocal(&self, x: usize) -> f64 {
    if x < RECIPROCAL_TABLE_LEN {
        unsafe { *self.reciprocals.get_unchecked(x) }
    } else {
        1.0 / x as f64
    }
}
```

The bounds check is already done by the `if` -- the `unsafe` saves nothing. Replace with `self.reciprocals[x]`.

**Files**: `src/tree_policy.rs`
**Effort**: 5 minutes

### 1.5 Add basic tests

**What**: Add `#[cfg(test)]` modules covering core functionality. Convert the counting_game example into a test. Add tests for:
- Single-threaded playout produces correct result
- Parallel playout produces correct result
- UCT policy selects better moves
- Reset clears state
- Node limit stops search
- Virtual loss affects scores

> **Review note**: The existing benchmark in `benches/bench.rs` uses the counting_game example types. Tests should reuse the same game definition. Consider putting the shared game definition in a `tests/common/` module or using the example directly.

**Files**: `src/lib.rs` (integration tests), `tests/counting_game.rs`
**Effort**: 2-3 hours

---

## Phase 2 -- Generic Score Type

Make the score type generic so evaluators can return i64 or f64 (or any numeric type) without configuration boilerplate. The score type is inferred from what the evaluator returns -- users just write their evaluation function naturally and the compiler does the rest.

### 2.1 Add ScoreType trait and make evaluation generic

**What**: Add a `ScoreType` trait that covers the atomic accumulation operations MCTS needs, implement it for i64 and f64, and thread the type through the evaluator and tree.

```rust
/// A numeric type that can be accumulated atomically during MCTS backpropagation.
pub trait ScoreType: Copy + Send + Sync + Default + PartialOrd + 'static {
    type Atomic: Send + Sync;

    fn atomic_new(val: Self) -> Self::Atomic;
    fn atomic_add(atomic: &Self::Atomic, val: Self);
    fn atomic_sub(atomic: &Self::Atomic, val: Self);
    fn atomic_load(atomic: &Self::Atomic) -> Self;
    fn atomic_store(atomic: &Self::Atomic, val: Self);
    fn to_f64(self) -> f64;  // UCB formula always works in f64
}
```

**i64 impl** -- zero-cost, uses native `AtomicI64::fetch_add`:
```rust
impl ScoreType for i64 {
    type Atomic = AtomicI64;
    fn atomic_add(atomic: &AtomicI64, val: i64) {
        atomic.fetch_add(val, Ordering::Relaxed);
    }
    fn to_f64(self) -> f64 { self as f64 }
    // ...
}
```

**f64 impl** -- bit-cast CAS loop on AtomicU64 (standard practice for atomic floats):
```rust
impl ScoreType for f64 {
    type Atomic = AtomicU64;
    fn atomic_add(atomic: &AtomicU64, val: f64) {
        let mut current = atomic.load(Ordering::Relaxed);
        loop {
            let new = (f64::from_bits(current) + val).to_bits();
            match atomic.compare_exchange_weak(current, new, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(x) => current = x,
            }
        }
    }
    fn to_f64(self) -> f64 { self }
    // ...
}
```

**Evaluator trait change**:
```rust
pub trait Evaluator<Spec: MCTS>: Sync {
    type StateEvaluation: Sync + Send;
    type Score: ScoreType;  // NEW -- inferred from what the user returns

    fn evaluate_new_state(&self, ...) -> (Vec<MoveEvaluation<Spec>>, Self::StateEvaluation);
    fn evaluate_existing_state(&self, ...) -> Self::StateEvaluation;
    fn interpret_evaluation_for_player(&self, evaln: &Self::StateEvaluation, player: &...) -> Self::Score;
}
```

**User experience** -- the score type is just whatever `interpret_evaluation_for_player` returns:
```rust
// f64 user -- just returns f64, everything flows
impl Evaluator<MyMCTS> for NarrativeEvaluator {
    type StateEvaluation = NarrativeScore;
    type Score = f64;

    fn interpret_evaluation_for_player(&self, evaln: &NarrativeScore, _: &()) -> f64 {
        evaln.total  // done. no scaling, no conversion.
    }
}

// i64 user -- same pattern, native atomic performance
impl Evaluator<MyMCTS> for GameEvaluator {
    type StateEvaluation = GameResult;
    type Score = i64;

    fn interpret_evaluation_for_player(&self, evaln: &GameResult, player: &Player) -> i64 {
        evaln.score_for(player)
    }
}
```

No `type Score = ...` on the MCTS trait. No configuration. The Evaluator knows what it returns, the compiler infers the atomic storage type.

**Cascading changes**:
- `NodeStats` becomes generic: `NodeStats<S: ScoreType>` with `sum_evaluations: S::Atomic`
- `MoveInfo::sum_rewards()` returns `EvalScore<Spec>` (type alias for the evaluator's score)
- `MCTS::virtual_loss()` returns `EvalScore<Spec>` (or just f64 -- virtual loss is always small)
- UCT formula: `mov.sum_rewards().to_f64() / child_visits as f64` -- works for both types
- `select_child_after_search` default: sort by visits (unchanged, score type irrelevant)

> **Review note on cascading changes**: The `virtual_loss` signature change needs care. Currently `virtual_loss() -> i64` is used in `NodeStats::down()` and `up()` (search_tree.rs). If virtual_loss returns `EvalScore<Spec>`, the ScoreType trait needs `atomic_sub` for the down path. The plan includes this in the trait but should note that `virtual_loss` default must change from `0i64` to `S::default()` or similar. Alternatively, keep virtual_loss as f64 and convert -- simpler.

**i64 path stays zero-cost**: `AtomicI64::fetch_add` is a single instruction. No CAS loop, no overhead. Existing game-oriented users get identical performance.

**Files**: New `src/score.rs` (ScoreType trait + impls, ~60 LOC), `src/search_tree.rs` (NodeStats generic), `src/lib.rs` (Evaluator trait), `src/tree_policy.rs` (UCB formula uses .to_f64()), example
**Tests**: Both i64 and f64 evaluators produce correct results; parallel f64 search; verify i64 path has no CAS overhead
**Effort**: 3-4 hours

---

## Phase 3 -- Tree Re-rooting

Add the ability to commit to a chosen move and advance the root, preserving the subtree for continued search. This is critical for the GM use case where search happens across multiple simulation ticks.

### 3.1 Add `advance_root` to SearchTree

**What**: New method that takes the chosen move, detaches its subtree, makes it the new root, and orphans all sibling subtrees for cleanup.

```rust
impl<Spec: MCTS> SearchTree<Spec> {
    /// Advance the root to the child reached by `mov`.
    /// The chosen child's subtree becomes the new root.
    /// All sibling subtrees are orphaned for deallocation.
    /// Returns the new root state.
    pub fn advance_root(&mut self, mov: &Move<Spec>) -> Option<&Spec::State>
    where
        Move<Spec>: PartialEq,
    {
        // 1. Find the MoveInfo matching `mov`
        // 2. Take ownership of its child SearchNode (swap AtomicPtr to null)
        // 3. Orphan all other children (move to orphaned Vec for deferred dealloc)
        // 4. Replace self.root_node with the chosen child
        // 5. Apply the move to self.root_state
        // 6. Update num_nodes count
    }
}
```

**Why**: Currently `reset()` destroys the entire tree. After the GM picks an intervention, we want to keep the subtree below that choice -- it contains valid search results for the next tick. Without re-rooting, every tick starts from zero playouts.

**Subtlety -- orphan cleanup**: Sibling subtrees may be large. The existing `orphaned: Mutex<Vec<Box<SearchNode>>>` mechanism handles deferred deallocation. We push sibling root nodes there and they get dropped when the Mutex is next accessed.

> **Review note**: The `SearchTree` stores `root_node` as `Box<SearchNode<Spec>>` and `root_state` as `Spec::State`. The children are stored as `AtomicPtr<SearchNode<Spec>>` inside each `MoveInfo`. To advance:
> 1. Load the chosen child's `AtomicPtr`, swap it to null
> 2. Reconstruct the `Box<SearchNode>` from the raw pointer (unsafe -- but this is existing pattern, see search_tree.rs:162)
> 3. Push the old root's other children into `orphaned`
> 4. Replace `self.root_node` with the chosen child box
> 5. `self.root_state.make_move(mov)` to advance game state
>
> The `num_nodes` counter is approximate (AtomicUsize) and used for node_limit checks. After re-rooting, it should ideally reflect only the surviving subtree size, but an exact count requires traversal. Subtracting the orphaned count is approximate but sufficient for limit checks.

**MCTSManager wrapper**:
```rust
impl<Spec: MCTS> MCTSManager<Spec> {
    /// Commit to a move: advance the root and preserve the subtree.
    pub fn advance(&mut self, mov: &Move<Spec>) -> bool
    where
        Move<Spec>: PartialEq,
    {
        self.search_tree.advance_root(mov).is_some()
    }
}
```

**Files**: `src/search_tree.rs`, `src/lib.rs`
**Tests**: Advance root, verify subtree stats preserved, verify sibling cleanup, verify continued search after advance
**Effort**: 3-4 hours

---

## Phase 4 -- Search Quality & Narrative Ergonomics

Features informed by recent MCTS research (2024-2026). These are additive -- they don't change existing API.

### 4.1 Depth-limited search (high priority)

**What**: Add `max_playout_depth` to the MCTS trait (separate from `max_playout_length`). Limit how deep a single playout descends before evaluating the leaf with the `Evaluator`.

```rust
trait MCTS {
    // ... existing ...

    /// Maximum depth per playout before forcing leaf evaluation.
    /// Default: usize::MAX (descend until terminal state).
    fn max_playout_depth(&self) -> usize { usize::MAX }
}
```

In `playout()`, when depth exceeds this limit, stop descending and evaluate the current node as a leaf.

> **Review note**: Currently `max_playout_length` (lib.rs:167, default 1,000,000) is checked in the playout loop at search_tree.rs:270. The new `max_playout_depth` serves a different purpose: `max_playout_length` caps total moves in a playout (safety limit), while `max_playout_depth` caps tree descent depth (quality knob). Both checks go in the same loop but trigger different behaviors -- length limit aborts with no eval, depth limit triggers leaf evaluation.

**Why**: This is the single most important search quality knob for narrative MCTS. Lookahead Pathology research (Brockman & Saffidine, ICAPS 2024) demonstrates that **more lookahead can make UCT worse in some settings** -- deeper search doesn't always improve decision quality. For narrative domains:
- Each playout step is expensive (apply action, run sifting engine, build TickDelta)
- Deep rollouts in narrative domains produce increasingly incoherent states (the simulation drifts from plausible trajectories)
- A shallow search (depth 3-5) with a good leaf evaluator (fabula-narratives' composite scorer) produces better decisions than a deep search (depth 20+) with noisy terminal evaluation

This is not a minor optimization -- it's a fundamental design choice. The GM should search *wide* (many candidate interventions) and *shallow* (few steps ahead), not narrow and deep.

**Research**: Brockman & Saffidine (ICAPS 2024) "Lookahead Pathology in MCTS."

**Files**: `src/lib.rs` (MCTS trait), `src/search_tree.rs` (playout loop)
**Effort**: 1-2 hours

### 4.2 Move prioritization and progressive widening

**What**: Instead of generating all moves at node creation, support prioritized/lazy move generation. Two mechanisms:

**A. Progressive widening** -- limit children expanded per visit count:
```rust
trait GameState: Clone {
    // ... existing methods ...

    /// Maximum children to expand at this node, given the current visit count.
    /// Default: usize::MAX (expand all moves immediately).
    /// Override for progressive widening: e.g., `(visits as f64).sqrt() as usize`
    fn max_children(&self, visits: u64) -> usize { usize::MAX }
}
```

**B. Move ordering** -- the `Evaluator` already returns `Vec<MoveEvaluation>` alongside the state evaluation. When progressive widening is active, expand moves in evaluation order (best-first). This gives the evaluator control over *which* moves get search budget.

**Why**: Narrative action spaces can be enormous (every character x every possible action x every target). Two research directions support this:

- **Progressive widening** (Coulom 2007) is the standard approach -- expand `O(n^alpha)` children at visit count `n`. Avoids allocating and evaluating all children upfront.
- **State-conditioned action abstraction** (Kwak et al., UAI 2024 oral) goes further: learn which sub-actions matter in the current state. We won't implement learned abstraction, but by combining progressive widening with evaluator-ranked move ordering, we get the same effect manually: the evaluator (or the GM's action generator) can rank "political interventions" above "trade route changes" when the world state has a succession crisis, and progressive widening ensures only the top-ranked actions get search budget.

The key design choice: `available_moves()` returns ALL legal moves (sorted by evaluator preference), but `max_children()` controls how many the tree actually expands. This keeps the API simple while supporting both full expansion (board games) and filtered expansion (narrative).

> **Review note**: Currently moves are stored in `SearchNode.moves: Vec<MoveInfo<Spec>>` and populated at node creation time (search_tree.rs `create_node`). Progressive widening means we need to split this: store ALL moves but only create `MoveInfo` entries (with child pointers) for the first N. The `descend` method (search_tree.rs:278-350) iterates over `node.moves()` to find children -- it would need to check `max_children(visits)` and potentially expand additional moves as visits increase.

**Research**: Coulom (2007) "Computing Elo Ratings of Move Patterns in the Game of Go." Kwak et al. (UAI 2024) "State-Conditioned Action Abstraction."

**Files**: `src/lib.rs` (GameState trait), `src/search_tree.rs` (descend method -- expand only up to max_children, in MoveEvaluation order)
**Effort**: 2-3 hours

### 4.3 Statistics export

**What**: Add methods to extract visit distribution and score statistics from the tree for analysis and debugging.

```rust
impl<Spec: MCTS> MCTSManager<Spec> {
    /// Visit counts and average rewards for all root children.
    pub fn root_child_stats(&self) -> Vec<ChildStats<Spec>> { ... }
}

pub struct ChildStats<Spec: MCTS> {
    pub mov: Move<Spec>,
    pub visits: u64,
    pub avg_reward: f64,
    pub move_evaluation: MoveEvaluation<Spec>,
}
```

> **Review note**: Some of this data is already accessible via `principal_variation_info()` and `MoveInfo`'s public methods (`visits()`, `sum_rewards()`, `get_move()`, `move_evaluation()`). The main gap is a convenient method that returns stats for ALL root children (not just the PV). `tree().root_node().moves()` gives access but returns `MoveInfoHandle` which lacks `avg_reward` convenience. This is a small ergonomic addition.

**Why**: The GM needs to explain its decisions ("I chose intervention X because it scores 8.3 vs Y's 5.1 across 200 playouts"). Also useful for debugging pathological search behavior -- if one action dominates visits despite low reward, the exploration constant needs tuning.

**Files**: `src/lib.rs`, `src/search_tree.rs`
**Effort**: 1 hour

---

## Phase 5 -- Packaging & Documentation

### 5.1 Rename and publish

**What**: Rename the crate (e.g., `mcts-narrative`, `narrative-mcts`, or keep `mcts` if we're comfortable claiming the name). Update Cargo.toml metadata (authors, repository, description, keywords).

### 5.2 Documentation

**What**: Module-level docs on every public type. Research citations where relevant:
- UCT: Kocsis & Szepesv\'ari (2006) "Bandit based Monte-Carlo Planning"
- PUCT: Silver et al. (2016) "Mastering the game of Go" (AlphaGo)
- Progressive widening: Coulom (2007) "Computing Elo Ratings of Move Patterns in Go"
- Depth limiting rationale: Brockman & Saffidine (ICAPS 2024) "Lookahead Pathology in MCTS"
- Action abstraction: Kwak et al. (UAI 2024) "State-Conditioned Action Abstraction"
- Narrative MCTS: Nelson & Mateas (2005) "Search-Based Drama Management", Kartal et al. (2014) "User-Driven Narrative Variation"

Usage example showing narrative MCTS with fabula.

### 5.3 README with narrative examples

**What**: README showing the GM use case:
```rust
struct NarrativeState { engine: SiftEngineFor<MyDS>, ds: MyDS }
struct GmIntervention { ... }
impl GameState for NarrativeState { ... }
impl Evaluator<MyMCTS> for NarrativeEvaluator { ... }
```

### 5.4 Design rationale document

**What**: A DESIGN.md explaining key design decisions and their research basis:
- **Why shallow search + good evaluation**: Lookahead pathology (ICAPS 2024) -- deeper != better. Narrative domains drift from plausibility with depth. Our quality function (fabula-narratives) is more informative than random rollouts.
- **Why progressive widening over full expansion**: Action spaces in narrative are combinatorial. State-conditioned prioritization (UAI 2024) focuses budget where it matters.
- **Why tree re-rooting matters**: GM operates across simulation ticks. Discarding search work between ticks wastes the most expensive computation.
- **Why generic score types**: Different domains need different precision. Games want i64 (fast atomics). Narrative scorers produce f64. The library shouldn't force a choice.
- **Relationship to LLM-MCTS trend**: The library supports but does not require LLM integration. The `Evaluator` can call an LLM, or `available_moves` can be LLM-generated. But the core search is deterministic and testable without LLM dependencies. This aligns with Farrell & Ware (2024): "LLMs as search guides, not search engines."

**Effort**: 4-5 hours total for Phase 5

---

## Research Context

Recent MCTS research (2024-2026) that informed this plan:

| Paper | Venue | Key insight for us |
|-------|-------|-------------------|
| Brockman & Saffidine (2024) "Lookahead Pathology in MCTS" | ICAPS 2024 | Deeper search can make UCT *worse*. Validates shallow search + good evaluation. |
| Kwak et al. (2024) "State-Conditioned Action Abstraction" | UAI 2024 (oral) | Reduce branching by learning which sub-actions matter per state. Informs move prioritization design. |
| Ghaffari (2025) "Narrative Studio: LLMs + MCTS" | ACL WNU 2025 | Directly relevant: MCTS over LLM-generated narrative branches. Open source reference. |
| Farrell & Ware (2024) "LLMs as Narrative Planning Search Guides" | IEEE ToG | LLM as heuristic within symbolic planner. "LLMs as search guides, not search engines." |
| Speculative MCTS (2024) | NeurIPS 2024 | Parallelizes future search work speculatively. Validates our async/lock-free architecture. |
| UniZero / ScaleZero (2025-2026) | TMLR / ICLR 2026 | Learned world models for MCTS. We have the opposite advantage (perfect simulator), but the trait design should not preclude learned models. |
| BiT-MCTS (2026) | arXiv | Bidirectional MCTS with Freytag's Pyramid. Climax-first narrative search. |

The strongest directly relevant paper is Narrative Studio (2025). The most transferable technical insights come from Lookahead Pathology (depth limiting), State-Conditioned Action Abstraction (move prioritization), and Speculative MCTS (parallelism).

The overall trend: MCTS is increasingly used as a **controller over LLM-evaluated branches**, not just a game tree search. Our architecture naturally supports this -- the `Evaluator` can wrap an LLM, `available_moves` can be LLM-generated -- but the core library stays deterministic and testable without LLM dependencies.

---

## Summary

| Phase | What | Unsafe removed | API change | Effort |
|-------|------|---------------|------------|--------|
| **1** | Safety & modernization | ~12 items (crossbeam_mini, spawn_worker, reciprocal) | None | 6-8 hours |
| **3** | Tree re-rooting | 0 | Additive | 3-4 hours |
| **4** | Search quality & narrative ergonomics | 0 | Additive (depth limit, progressive widening, stats) | 4-5 hours |
| **2** | Generic score type | 0 | Additive (ScoreType trait, Evaluator gains Score assoc type) | 3-4 hours |
| **5** | Packaging & docs | 0 | None | 4-5 hours |
| | **Total** | **~12** | | **~22-26 hours** |

**Priority order**: Phase 1 -> Phase 3 -> Phase 4 -> Phase 2 -> Phase 5.

- **Phase 1** (safety) is prerequisite -- removes ~12 unsafe items, replaces crossbeam_mini with std::thread::scope.
- **Phase 3** (tree re-rooting) is the most load-bearing feature for GM use -- without it, every tick discards all search work.
- **Phase 4** (depth limiting, move prioritization, stats) directly affects search quality. Depth limiting is research-backed (Lookahead Pathology) and critical for narrative domains where deep rollouts produce incoherent states.
- **Phase 2** (generic score type) is nice-to-have -- i64 with `(score * 1000.0) as i64` scaling works fine in practice, but the generic approach means users never have to think about it. Inferred from the evaluator's return type, zero-cost for i64 users.
- **Phase 5** (packaging) whenever we're ready to publish. Includes DESIGN.md with research rationale.

After Phase 1 + 3, the crate is usable for narrative MCTS with fabula-narratives. After Phase 4, it's *good* at it.

> **Review note -- remaining unsafe (20 items, not targeted):**
> The search_tree.rs (11) and transposition_table.rs (9) unsafe blocks implement the lock-free concurrent tree using raw pointers and AtomicPtr. These are fundamental to the architecture -- `SearchNode` children are heap-allocated and shared across threads via atomic pointer swaps. Eliminating these would require either:
> - Arc-based tree (recon_mcts approach) -- adds refcount overhead on every node visit
> - Arena allocator with indices -- eliminates raw pointers but adds complexity
>
> Neither is worth the tradeoff for this crate's use case. The existing raw-pointer patterns are correct (nodes are never freed during search, only via orphaned cleanup after search stops) and follow established lock-free tree patterns. The `unsafe trait TranspositionTable` is also reasonable -- implementors must guarantee thread-safe reference semantics, which can't be expressed in safe Rust.
