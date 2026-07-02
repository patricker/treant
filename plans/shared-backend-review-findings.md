# Shared-backend review — comprehensive validated findings

_Adversarially-validated re-run. 104 raw → 93 deduped findings; **80 confirmed, 2 disputed, 4 uncertain, 7 refuted**. Each finding read by 2 independent skeptics (reachability lens + test/severity lens) against the source. 196 agents. Severities below are the **post-validation adjusted** values, not the reporters'._

Reachability legend: `arcade-path` = reachable from the shipped arcade/WASM; `library-only` = only via direct Rust API; `multithread-only` / `feature-gated` = needs parallel search or an opt-in feature (solver/score_bounded).


## CONFIRMED (80)

### F70 · [MEDIUM] · api · `treant-dynamic/src/adapter.rs:79-92` · arcade=no
**Host priors that don't sum to ~1.0 or are negative panic inside the search thread**

- **Why:** The priors are passed straight to the core. AlphaGoPolicy::validate_evaluations (src/tree_policy.rs:272) asserts every prior >= -1e-6 and that a non-empty set sums to within 0.1 of 1.0, panicking otherwise. The EvalCallbacks::evaluate doc only says priors 'should be non-negative and sum to approximately 1.0' — soft wording for a hard panic. A host language returning unnormalized logits/probabilities (a very common mistake at an FFI boundary) crashes the search. Under playout_n_parallel this panic occurs on a worker thread, which poisons shared mutexes / aborts rather than surfacing cleanly to the host.
- **Fix:** Either normalize host priors in the adapter (clamp negatives to 0 and divide by the sum when sum>0, falling back to uniform), or document explicitly in EvalCallbacks::evaluate that priors MUST be non-negative and sum to ~1.0 or the library panics. Normalizing is the more FFI-friendly choice.
- **Evidence:** let (mut priors, value) = self.0.evaluate(&*state.0, &move_strings);
// If no priors returned, generate uniform
if priors.is_empty() && !moves.is_empty() { ... }
...
(priors, eval)
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Core claim is correct and present in the code as cited.

adapter.rs:79-92 (evaluate_new_state): host priors from self.0.evaluate(...) are passed straight to the core; the only transformation is filling uniform priors when the vec is EMPTY (lines 83-86). No normalization, no clamping, no sum-check. Negative priors or unnormalized probabilities/logits flow through unchanged.

The priors are validate
- **Skeptic 2** (confirmed/MEDIUM/library-only, tested=False): Confirmed against source. Adapter (treant-dynamic/src/adapter.rs:79-92) passes host priors straight through, only synthesizing uniform priors when the vec is empty (lines 83-86); no normalization or clamping. Those priors flow into search_tree.rs:266 `policy.validate_evaluations(&move_eval)`, which in tree_policy.rs:272-288 hard-asserts each value >= -1e-6 and that a non-empty set sums to within 0

### F64 · [MEDIUM] · bug · `treant-wasm/src/mancala.rs:383-386` · arcade=yes
**Mancala numPlayers knob/preset allow 6 but the Rust engine silently coerces anything but 4 to 2 players**

- **Why:** mancala.tsx exposes a numPlayers knob with max 6 (knobs: min 2 max 6) and a '6-Player Sow' preset { numPlayers: 6 }, plus a '4-Player Ring' preset. Any selection of 3, 5, or 6 players is silently coerced to a 2-player game by the Rust constructor, while the React board (mancala.tsx MancalaBoard) renders `numPlayers` rows reading ring cells that the 2-player engine never populated (`counts[pl*stride+i] ?? 0`), so seats P3..P6 show all-zero pits and the game ends/behaves as 2-player. MAX_PLAYERS is 6 in the engine, so the coercion is an arbitrary WASM-layer restriction, not an engine limit. This is a user-visible correctness bug for the advertised 6-player mode.
- **Fix:** Either accept all of 2..=6 (Mancala::new already asserts only 2|4 — relax that and the sow/capture geometry for general N, since opposite_pit only defines 2 and 4) and clamp accordingly, or constrain the TS knob/preset to {2,4} so the UI cannot offer modes the engine refuses.
- **Evidence:** fn create(pits: usize, stones: u8, num_players: usize) -> Self {
        let pits = pits.clamp(2, MAX_PITS);
        let num_players = if num_players == 4 { 4 } else { 2 };
- **Skeptic 1** (confirmed/MEDIUM/arcade-path, tested=False): Verified against source. treant-wasm/src/mancala.rs:383-386 reads exactly as cited: `let num_players = if num_players == 4 { 4 } else { 2 };` — any value other than 4 (including 3, 5, 6) is silently coerced to 2. The underlying engine `Mancala::new` (line 37-39) asserts `num_players == 2 || num_players == 4`, so 6 is genuinely unsupported geometry; `opposite_pit` (line 89-95) only `match`es num_pl
- **Skeptic 2** (confirmed/MEDIUM/arcade-path, tested=False): Confirmed by reading source. treant-wasm/src/mancala.rs:385 coerces num_players to `if num_players == 4 { 4 } else { 2 }`. mancala.tsx:95 exposes a numPlayers knob with max 6 and mancala.tsx:90 ships a '6-Player Sow' preset {numPlayers:6}. The board (mancala.tsx:32-67) renders `np` rows reading `counts[pl*stride+i] ?? 0`, so selecting 6 players yields a 2-player engine game with phantom all-zero s

### F2 · [MEDIUM] · concurrency · `src/search_tree.rs:1189-1243` · arcade=no
**Cross-subtree transposition aliases can dangle after advance_root drops sibling subtrees (use-after-free)**

- **Why:** After old_root is dropped, the alias inside the preserved subtree points to freed memory; a subsequent playout that descends through it (`choice.child.load(Acquire)` at 752 returns the dangling pointer, deref at 754) is a use-after-free. Requires a transposition table and a cross-subtree transposition; the owned-check only guards the root edge, not the whole kept subtree.
- **Fix:** Either forbid advance_root when a transposition table is in use that can alias across siblings, or walk the kept subtree converting any alias whose target lies outside the kept subtree into an owned copy (or retain ownership of aliased-into nodes) before dropping old_root. At minimum document this as unsound with cross-subtree transpositions.
- **Evidence:** advance_root only validates the immediate kept child is owned: line 1204 `if !move_info.owned.load(Ordering::SeqCst) { return Err(AdvanceError::ChildNotOwned); }`. It then `drop(old_root)` at 1243, whose MoveInfo::Drop (215-228) frees every `owned==true` node in the discarded sibling subtrees. The kept `new_root` subtree may contain a MoveInfo with `owned==false` aliasing a node whose sole `owned==true` owner lives in a discarded sibling subtree (a transposition linking the two). Clearing the table at 1240 does not rewrite those already-stored alias pointers inside the kept subtree.
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Confirmed by reading src/search_tree.rs:1189-1256, 215-228, 745-816 and src/transposition_table.rs:174-241.

Mechanism (real as written): advance_root validates ONLY the kept root edge is owned (line 1204), then table.clear() (1240) and drop(old_root) (1243). MoveInfo::Drop (218-227) frees nodes only where owned==true, cascading through the discarded sibling subtrees. A transposition alias is crea
- **Skeptic 2** (confirmed/MEDIUM/library-only, tested=False): Confirmed by reading the cited code. MoveInfo::Drop (src/search_tree.rs:215-228) frees a child only when owned==true. Transposition aliases are created in descend (760-774 and 799-812): choice.child is set to a foreign node while owned stays false; the unique owner sets owned=true at line 814, which can be a MoveInfo in a different subtree. advance_root (1189-1256) only detaches/clears the single 

### F19 · [MEDIUM] · correctness · `src/search_tree.rs:435-468` · arcade=no
**Chance node with mixed proven Win/Loss children is incorrectly marked as proven Draw**

- **Why:** A chance node's value is the probability-weighted EXPECTATION of its outcomes, not a min/max. If outcome A is a proven Win and outcome B is a proven Loss, the node is a lottery whose true value is neither a proven Win, Loss, nor an exact Draw. Marking it `Draw` asserts the exact game-theoretic value is 0, which is false unless the weighted expectation happens to be 0. This wrong `Draw` then propagates: the decision-node parent consumes it via `try_prove_node` (a Draw child can make the parent conclude Draw), and `select_child_after_search` (src/lib.rs:183-188) prefers proven-Draw children. It also contradicts `try_tighten_bounds_chance`, which computes the real expectation — the two systems can disagree on the same node. Reachable only when solver_enabled() AND closed_loop_chance() are both on.
- **Fix:** A chance node can only be proven when the outcome class is unanimous (all Win / all Loss / all Draw, or — more correctly — when the weighted score bound has converged). Return ProvenValue::Unknown for any mixed set, OR derive the proven value from the converged score expectation (try_tighten_bounds_chance) instead of from a min/max-style vote. At minimum, only return Draw when every child is proven Draw.
- **Evidence:** fn try_prove_chance_node ... if all_win { ProvenValue::Win } else if all_loss { ProvenValue::Loss } else { ProvenValue::Draw }  // e.g. one child Win, one child Loss falls into the else branch -> Draw
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Defect is present exactly as cited. src/search_tree.rs:435-468 try_prove_chance_node uses a min/max-style vote: all_win->Win (461-462), all_loss->Loss (463-464), else->Draw (465-467). A chance node with mixed proven children (e.g. one Win + one Loss, or any mixed set) falls into the else branch and is marked proven Draw, asserting an exact game-theoretic value of 0. That is wrong for a probability
- **Skeptic 2** (confirmed/MEDIUM/feature-gated, tested=False): Source verified at src/search_tree.rs:435-468. try_prove_chance_node uses a min/max-style unanimity vote: all_win→Win, all_loss→Loss, else→Draw. The else branch is genuinely wrong: a chance node's game-theoretic value is the probability-weighted expectation Σ prob_i·value_i, not a vote. A mix of proven-Win and proven-Loss children does NOT have expectation 0; e.g. {Win@0.9, Loss@0.1} has expectati

### F69 · [MEDIUM] · correctness · `treant-dynamic/src/adapter.rs:95-108` · arcade=no
**evaluate_existing_state mislabels the re-evaluated value's player perspective**

- **Why:** The host's evaluate() returns a value from the perspective of state.0.current_player() (per the EvalCallbacks::evaluate doc: 'State value should be from the current player's perspective'). But the returned DynStateEval records player: existing_evaln.player, i.e. the player stored on the OLD evaluation, not the player of the state that just produced the new value. interpret_evaluation_for_player later uses this recorded player to decide whether to negate the value. For open-loop chance nodes (the only caller, search_tree.rs:734), chance resampling can land on a state whose current_player differs from the node's stored evaln player; the sign of the freshly-computed value is then interpreted against the wrong player and the reward is negated incorrectly. It happens to pass the dice-game tests because those resample without changing whose turn it is, so the contract is silently fragile rather than obviously broken.
- **Fix:** Record player: state.0.current_player() to match the value that was just computed, OR (cheaper and matching intent) keep the existing value entirely and only return existing_evaln unchanged. If re-evaluation is wanted, the player label must come from the state that produced the value.
- **Evidence:** let (_, value) = self.0.evaluate(&*state.0, &move_strings);
DynStateEval {
    value,
    player: existing_evaln.player,
}
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Read treant-dynamic/src/adapter.rs:95-108 — the code matches the finding verbatim: evaluate_existing_state recomputes `value` from the fresh resampled `state` via self.0.evaluate(&*state.0, ...) but returns DynStateEval { value, player: existing_evaln.player }, i.e. the value's perspective and its recorded player label are mismatched whenever state.0.current_player() != existing_evaln.player.

The
- **Skeptic 2** (confirmed/MEDIUM/library-only, tested=False): Read adapter.rs:95-116 directly. The code is exactly as cited: evaluate_existing_state computes a fresh `value` from the (open-loop resampled) `state` via `self.0.evaluate(...)`, then tags it `player: existing_evaln.player`. interpret_evaluation_for_player (line 110-114) passes `evaluation.player` into the host's interpret_for_player, which may negate based on (evaluating_player vs requesting_play

### F72 · [MEDIUM] · correctness · `treant-dynamic/src/evaluators.rs:54-93` · arcade=no
**RandomRollout reports value 0.0 (a false 'draw') when rollout hits max_depth or a non-terminal/Unknown state**

- **Why:** Three distinct outcomes all collapse to 0.0: (1) rollout exhausts the hardcoded 1000-step cap without reaching a terminal state, then terminal_value() is queried on a non-terminal state and returns None -> 0.0; (2) the state is terminal but the host did not override terminal_value() (default None, callbacks.rs:28) -> 0.0; (3) terminal_value returns Unknown -> 0.0. In all three the search receives a neutral 'draw' signal for what may be a win or loss, biasing search toward unfinished/unclassified lines. The doc says the evaluator 'Requires that games have terminal states' but the silent 0.0 on the failure path gives no diagnostic.
- **Fix:** Distinguish the cases: if moves remain after the loop (rollout truncated) consider returning a small/neutral but documented value, and document loudly that terminal_value() MUST be implemented for RandomRollout to produce signal. Consider a debug_assert when a terminal state yields None.
- **Evidence:** let max_depth = 1000;
for _ in 0..max_depth { ... }
// Try to get a terminal value
if let Some(pv) = sim.terminal_value() {
    ... ProvenValue::Unknown => 0.0, ...
} else {
    (priors, 0.0)
}
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Read treant-dynamic/src/evaluators.rs:42-94 and callbacks.rs:28. The code matches the finding's evidence exactly. RandomRollout::evaluate plays up to max_depth=1000 random moves (loop breaks only on empty available_moves, line 58), then calls sim.terminal_value(); ProvenValue::Unknown maps to 0.0 (line 80), and the else branch when terminal_value() returns None also maps to 0.0 (line 92). callback
- **Skeptic 2** (confirmed/MEDIUM/library-only, tested=False): Read treant-dynamic/src/evaluators.rs:42-94 and callbacks.rs:28. The cited code matches the finding exactly: the rollout loop runs `for _ in 0..max_depth` (max_depth=1000, line 54-72); afterward `if let Some(pv) = sim.terminal_value()` maps Win/Loss/Draw/Unknown, with both `ProvenValue::Unknown => 0.0` (line 80) AND the `else` branch (non-terminal / None) `(priors, 0.0)` (line 92). The default `te

### F21 · [MEDIUM] · test-gap · `tests/mcts_tests.rs:3160-3230` · arcade=no
**Solver / score-bounded propagation through chance nodes is completely untested**

- **Why:** `try_prove_chance_node` and `try_tighten_bounds_chance` (and the chance branches of propagate_proven/propagate_score_bounds) are never exercised with proven children. The only chance test (ClosedLoopDice) leaves both solver and score_bounded off. The chance-node Draw bug above would have been caught by a single test that enables the solver on a closed-loop chance node with mixed-outcome proven children. This is the highest-leverage coverage gap in the slice.
- **Fix:** Add tests: (1) closed-loop chance node where all outcomes are proven Win -> chance node proven Win; (2) all Loss -> Loss; (3) mixed Win/Loss outcomes -> assert NOT proven Draw (encodes the intended fix); (4) score_bounded chance node where outcomes have known scores -> assert weighted-expectation bounds.
- **Evidence:** ClosedLoopDiceMCTS impl MCTS { fn closed_loop_chance(&self) -> bool { true } ... } — no solver_enabled() or score_bounded_enabled() override; grep for chance+solver/bound tests returns nothing.
- **Skeptic 1** (confirmed/MEDIUM/feature-gated, tested=False): Verified by reading tests/mcts_tests.rs:3162-3247 and src/search_tree.rs:433-497, 856-913.

CLAIM 1 (untested): Confirmed. I cross-tabulated every `impl MCTS` spec in tests/mcts_tests.rs against its `type State` and its solver/score_bounded overrides. Only one game implements `chance_outcomes` — DiceGame (single impl at tests/mcts_tests.rs:2986). DiceGame is used by exactly three specs: DiceMCTS, 
- **Skeptic 2** (confirmed/MEDIUM/library-only, tested=False): Read tests/mcts_tests.rs:3158-3247 (ClosedLoopDiceMCTS impl, lines 3165-3179) and confirmed it overrides only closed_loop_chance() and rng_seed() — no solver_enabled() or score_bounded_enabled(). Grepped the whole test file: there is exactly ONE chance_outcomes implementation (DiceGame, line 2986). All three DiceGame-based specs (SeededDiceMCTS, ClosedLoopDiceMCTS, open-loop make_dice_mcts) leave 

### F11 · [LOW] · api · `src/batch.rs:154-163` · arcade=no
**evaluate_new_state panics (not graceful error) if the collector thread has died or dropped the response**

- **Why:** Three panic paths sit on the hot evaluation path of every search thread. (1) `.lock().unwrap()` poisons-and-panics if ANY thread panicked while holding the sender mutex. (2) `.send(...).expect("batch collector thread died")` panics on every search thread if the user's `evaluate_batch` panicked and unwound the collector (closing the receiver). (3) `recv().expect(...)` panics if the collector dropped the response sender. Because `evaluate_batch` is user code that may panic (e.g. a GPU/NN error), a single failure in the collector converts into a panic storm across all N search worker threads rather than a clean error. The `Mutex<Sender>` also serializes every enqueue across all search threads — a throughput foot-gun for highly parallel search, since the whole point of batching is parallel leaf collection.
- **Fix:** At minimum document that a panic in evaluate_batch will panic all search threads. Consider: use a lock-free/MPSC sender clone-per-thread instead of `Mutex<Sender>` (mpsc::Sender is already Clone+Send; wrap a clone in thread-local or hand each worker its own clone) to remove the global mutex; and propagate collector death as a recoverable error/sentinel rather than `expect`.
- **Evidence:** let sender = self.sender.as_ref().expect("bridge already shut down");
sender
    .lock()
    .unwrap()
    .send(request)
    .expect("batch collector thread died");
response_rx
    .recv()
    .expect("batch collector dropped response")
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Read src/batch.rs:142-163. The cited code matches exactly: line 154 `self.sender.as_ref().expect("bridge already shut down")`, lines 155-159 `.lock().unwrap().send(request).expect("batch collector thread died")`, lines 160-162 `response_rx.recv().expect("batch collector dropped response")`. All three panic paths are present as written and sit on the per-leaf evaluate_new_state hot path.

Cascade m
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read src/batch.rs:142-163 — the code matches the finding verbatim. All three panic paths are real: (1) line 157 `.lock().unwrap()` poison-panics if a thread panicked mid-send; (2) line 158-159 `.send(...).expect("batch collector thread died")`; (3) line 160-162 `recv().expect(...)`. The collector-death mechanism is genuine: `collector_loop` calls user code `eval.evaluate_batch(...)` at line 240 in

### F14 · [LOW] · api · `src/batch.rs:240-247` · arcade=no
**assert_eq! on evaluate_batch result length aborts the entire collector (and thus all search threads) on a user-impl bug**

- **Why:** If a user's BatchEvaluator returns the wrong number of results, this assert panics inside the collector thread, unwinding it and closing the receiver. Every search thread then hits the `.expect("batch collector thread died")` panic at batch.rs:159 (or `recv` panic at 162). One contract violation in user code cascades into a process-wide panic across the whole thread pool. The contract ('Returns a Vec of the same length') is stated only in a doc comment (lines 25-27) and enforced solely by this fatal assert — there is no graceful path. This is an API foot-gun for anyone implementing BatchEvaluator.
- **Fix:** On length mismatch, send an error/sentinel back to exactly the affected requests (or document clearly that a mismatch is a fatal contract violation). Prefer returning Err to each waiting thread over panicking the shared collector.
- **Evidence:** let results = eval.evaluate_batch(&batch_input);
assert_eq!(
    results.len(),
    batch_requests.len(),
    "evaluate_batch returned {} results for {} inputs",
    results.len(),
    batch_requests.len()
);
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Code matches the citation exactly. src/batch.rs:240-247 asserts results.len() == batch_requests.len() inside collector_loop; the contract ("Returns a Vec of the same length") is stated only in the doc comment at lines 21-23. The cascade is real: an assert panic unwinds the collector thread and drops the Receiver, so every search thread then panics at batch.rs:159 (.expect("batch collector thread d
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified src/batch.rs:240-247 — the assert_eq! matches the finding verbatim. The cascade is real: assert panics inside collector_loop (run in the spawned thread at batch.rs:119-120), unwinding the collector thread; this drops the mpsc receiver and any pending response channels, so search threads fail at sender.send().expect("batch collector thread died") (line 159) or response_rx.recv().expect("ba

### F32 · [LOW] · api · `src/tree_policy.rs:272-288` · arcade=no
**validate_evaluations uses release asserts: slightly-off priors panic the whole search**

- **Why:** Host-language bindings (treant-dynamic / wasm) that produce priors summing to 0.89 or 1.12 will hard-panic mid-search rather than degrade gracefully or normalize. The 0.1 tolerance is generous but the failure mode is a process panic on a plausibly caller-supplied runtime data condition, not a programmer invariant, and it fires on every expanded node.
- **Fix:** Normalize priors internally (divide by sum) instead of asserting, or downgrade to debug_assert! plus documented normalization, or return a Result from node creation. At minimum document that priors MUST sum to within 0.1 of 1.0 or the search panics, and surface this in the bindings' contract.
- **Evidence:** L274-278 `assert!(x >= -1e-6, ...)` and L282-286 `assert!((evaln_sum - 1.0).abs() < 0.1, ...)` — both run in release builds, called from create_node (search_tree.rs L266) on every node expansion.
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Code matches the citation exactly. src/tree_policy.rs:272-288 uses release-active `assert!` (not `debug_assert!`): L274-278 asserts each eval >= -1e-6, and L282-286 asserts |sum - 1.0| < 0.1. `validate_evaluations` is called unconditionally from create_node at src/search_tree.rs:266 on every node expansion. The asserting impl belongs to AlphaGoPolicy (impl block starts L207); UCTPolicy uses the no
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified src/tree_policy.rs:272-288 directly: AlphaGoPolicy::validate_evaluations uses plain assert! (release-active, NOT debug_assert!) — one checking x >= -1e-6 per element, one checking (sum - 1.0).abs() < 0.1. It is called from create_node at src/search_tree.rs:266 on every non-chance node expansion. Only AlphaGoPolicy (PUCT) implements it; the TreePolicy trait default at tree_policy.rs:27 is 

### F68 · [LOW] · api · `treant-wasm/src/gridpack.rs:468-470` · arcade=no
**weak_move's top_k parameter typed usize at the wasm-bindgen boundary; documented contract for 0 / huge values lives only in pick_weak**

- **Why:** The GameHandle TS contract (gameTypes.ts:40) types topK as `number` with no documented bounds. pick_weak clamps top_k to `1..=len` (difficulty.rs:41) and treats `temp <= 0.0001` (including any negative temp) as 'best move', and `playouts==0` as 'return None so caller does a random move' (difficulty.rs:26). None of these edge behaviors are documented on the WASM method or the TS GameHandle.weakMove signature, so a caller passing topK=0 (clamped to 1, i.e. greedy) or a negative temp (silently treated as 0) gets surprising-but-silent behavior. The pickAiMove fallback (gameTypes.ts:150) does handle the None return, so playouts==0 is safe in practice.
- **Fix:** Document the parameter domains on GameHandle.weakMove and on the WASM `weak_move` rustdoc: topK is clamped to [1, #moves], temp<=~1e-4 (and any non-positive temp) means greedy, playouts==0 returns undefined (caller plays random).
- **Evidence:** pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
                crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
            }
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read all cited code. Factual claims are accurate. gridpack.rs:468-470 matches verbatim: `pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String>` calling `crate::difficulty::pick_weak(...)`. In pick_weak: `top_k.clamp(1, stats.len())` (line 41), `temp <= 0.0001` greedy branch which subsumes negative temps (line 49), `playouts == 0 => return None` (lines 26-
- **Skeptic 2** (confirmed/LOW/library-only, tested=True): All cited code is accurate and verified: gridpack.rs:468-470 (weak_move sig with top_k: usize), difficulty.rs:41 (top_k.clamp(1,len)), difficulty.rs:49 (temp<=0.0001 greedy, includes negatives), difficulty.rs:26 (playouts==0 -> None), gameTypes.ts:40 (topK: number, undocumented bounds), gameTypes.ts:150 (pickAiMove ?? fallback handles None). The finding is a pure documentation-completeness nit, no

### F74 · [LOW] · api · `treant-dynamic/src/types.rs:68` · arcade=no
**Reported avg_reward is in scaled reward units (~[-10,10]), not the natural [-1,1], with no doc note**

- **Why:** interpret_evaluation_for_player multiplies the host's [-1,1] value by REWARD_SCALE=10 before storing it as the i64 reward (adapter.rs:110-115). Core computes avg_reward = sum_rewards/visits in that scaled space (src/search_tree.rs:1112). manager.rs:81/137/153 forward this scaled value straight to host languages as DynChildStats.avg_reward / DynTreeEdge.avg_reward / DynTreeNode.avg_reward. So a value a host expects in [-1,1] actually arrives roughly in [-10,10] (and rounded to integer sums). REWARD_SCALE is an internal PUCT-tuning detail leaking across the FFI boundary; the doc comments on these fields say nothing about scaling.
- **Fix:** Divide reported avg_reward by REWARD_SCALE in manager.rs before exposing it (or document the scale explicitly on every avg_reward field). Dividing is the least-surprising fix and makes the value match the host's [-1,1] convention.
- **Evidence:** pub avg_reward: f64,   // DynChildStats; also DynTreeNode/DynTreeEdge
// adapter.rs:114  (raw * REWARD_SCALE) as i64   with REWARD_SCALE = 10.0
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Verified the full chain in source. treant-dynamic/src/adapter.rs:66 defines REWARD_SCALE=10.0 and adapter.rs:110-114 stores the host's value as (raw * REWARD_SCALE) as i64 — so the host's natural [-1,1] state value is multiplied by 10 (and truncated to integer) before accumulation. src/search_tree.rs:1109-1112 computes avg_reward = mi.sum_rewards() as f64 / visits as f64 in that scaled i64 space. 
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified all cited code. adapter.rs:66 defines REWARD_SCALE=10.0 and adapter.rs:114 multiplies the host's interpret_for_player value by it before `as i64`. The EvalCallbacks contract (callbacks.rs:59-75) presents state value as a natural "positive=good", negate-for-opponent convention (effectively [-1,1] in the bundled RandomRollout/docs), so the scaling is an internal PUCT detail. Core computes a

### F82 · [LOW] · api · `treant-wasm/src/gridpack.rs:450, 521` · arcade=no
**`serde_wasm_bindgen::to_value(...).unwrap()` can trap the entire WASM module on a serialization error**

- **Why:** A panic across the wasm-bindgen boundary aborts/poisons the module for the rest of the page session, not just the call. `SearchStatsJS` only holds numbers/strings and NaN/Infinity f64 serialize to valid JsValues, so a real failure is not reachable today — but `.unwrap()` at an FFI boundary is a standing foot-gun: any future field that is not infallibly serializable would turn a recoverable error into a module-wide trap. The same pattern is used elsewhere in the wasm crate (build_stats consumers).
- **Fix:** Return `Result`/`JsValue::NULL` on error, or use `serde_wasm_bindgen::to_value(&stats).unwrap_or(JsValue::NULL)`, so a serialization failure degrades gracefully instead of trapping the module.
- **Evidence:** pub fn get_stats(&self) -> JsValue {
    let stats = types::build_stats(&self.manager, |_| None);
    serde_wasm_bindgen::to_value(&stats).unwrap()
}
...
serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap()
- **Skeptic 1** (uncertain/LOW/unreachable, tested=False): Code matches the citation exactly. treant-wasm/src/gridpack.rs:450 (`get_stats` in the `cell_game_wasm!` macro, expanded for Connect6/Squava/Notakto/SquareUp) and :521 (`OrderChaosWasm::get_stats`) both call `serde_wasm_bindgen::to_value(&types::build_stats(...)).unwrap()`. These methods are reachable from the arcade via the WASM bindings, so the call site executes on a real path.

However, the .u
- **Skeptic 2** (confirmed/LOW/unreachable, tested=False): Cited code is accurate: gridpack.rs:450 (`serde_wasm_bindgen::to_value(&stats).unwrap()`) and :521 (the OrderChaosWasm variant) match exactly. The pattern is pervasive — 40+ identical `to_value(...).unwrap()` call sites across the wasm crate (nogo, clobber, pig, hex, mancala, etc.), all serializing the output of `types::build_stats` / `export_tree`.

Reachability is unreachable today, which the fi

### F30 · [LOW] · bug · `src/lib.rs:684-705` · arcade=no
**select_move_by_temperature overflows to INFINITY for small temperatures and degenerates to 'always pick last move'**

- **Why:** best_move routes any temperature in (1e-8, small) here. For temperature near 1e-8, inv_temp ~1e8, and visits^1e8 = +INFINITY for any child with visits>=2. Multiple INFINITY weights make total=INFINITY and roll=INFINITY; `roll -= weight` stays INFINITY (or NaN for INF-INF), the `roll <= 0.0` test never trips, and the function deterministically returns the LAST child in iteration order — not the argmax-by-visits the user expects at low temperature, and not proportional sampling. Silent wrong-move selection in a numerically-reachable regime.
- **Fix:** Use max-normalization: compute weights as exp(inv_temp * (ln visits - ln max_visits)), or clamp inv_temp, or special-case very small temperatures to argmax. At minimum normalize by the max visit count before powf to avoid overflow.
- **Evidence:** L685 `let inv_temp = 1.0 / temperature;` L691 `(c.visits() as f64).powf(inv_temp)`. L696-704: `total` = sum of weights; `roll = rng.gen()*total`; loop subtracts weights; fallback `Some(weighted.last()...)`.
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/lib.rs:675-705. The defect is real. best_move() (L675-682) guards `if temperature < 1e-8` -> principal_variation argmax, else calls select_move_by_temperature. So the temperature-selection function is reached for any temperature >= 1e-8. select_move_by_temperature computes inv_temp = 1.0/temperature (L685) and weight = (visits as f64).powf(inv_temp) (L691). 

I reproduced the numeric beha
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): CONFIRMED as a real bug, but severity DEFLATED from MEDIUM to LOW due to limited reachability.

Source verified at src/lib.rs:684-705. best_move() (L676-681) routes temperature >= 1e-8 into select_move_by_temperature, where inv_temp = 1.0/temperature (L685) and weights = (visits as f64).powf(inv_temp) (L691). I reproduced the math standalone: for max_visits=100, 100^inv_temp overflows f64::MAX (~1

### F31 · [LOW] · bug · `src/tree_policy.rs:199, 269, 391-413` · arcade=no
**select_by_key panics (unwrap) if every candidate scores NaN; Dirichlet noise path can introduce NaN priors uncaught**

_also reported as: select_by_key().unwrap() panics if every child scores NaN (or non-finite comparisons collapse); `PolicyRng::select_by_key` can return `None` → `.unwrap()` panic in `choose_child` when all move scores are NaN_

- **Why:** validate_evaluations (L272-288) catches NaN priors at node creation (NaN fails `x >= -1e-6`). However apply_dirichlet_noise (L298-313) runs AFTER creation and does not re-validate; a non-finite epsilon/alpha could produce NaN priors. A NaN prior makes the PUCT key NaN for that move; if it were the only legal move (single-child root), select_by_key returns None and choose_child unwraps -> panic. Reachable only via the library API with Dirichlet noise mis-configured.
- **Fix:** Make select_by_key fall back to the first element when choice is None (treat NaN as -inf), or assert finiteness of priors after apply_dirichlet_noise. Replace the bare `.unwrap()` with `.expect("choose_child: no selectable child (all scores NaN?)")` for a diagnosable panic.
- **Evidence:** choose_child ends with `.unwrap()` (L199, L269). select_by_key (L396-413): NaN scores satisfy neither `score > best_so_far` nor `score == best_so_far`, so they never set `choice`; if all are NaN, `choice` stays None and the caller unwraps None.
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/tree_policy.rs:199, 269, and 391-413, plus the Dirichlet path and validation flow. The mechanism is real and as described: select_by_key (L396-413) initializes best_so_far=NEG_INFINITY; a NaN score satisfies neither `score > best_so_far` nor `score == best_so_far`, so it never sets `choice`. If every candidate's key is NaN, choice stays None and both choose_child impls (.unwrap() at L199/
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Mechanism verified against source and reproduced live. src/tree_policy.rs:396-412 (select_by_key): NaN scores satisfy neither `score > best_so_far` nor `score == best_so_far`, so `choice` stays None when every candidate is NaN; both choose_child impls unwrap it (L199 UCT, L269 AlphaGo). validate_evaluations (L272-288) only runs at node creation; apply_dirichlet_noise (L298-313) runs afterward (sea

### F77 · [LOW] · bug · `treant-dynamic/src/adapter.rs:110-115` · arcade=no
**Host value cast (raw * REWARD_SCALE) as i64 silently maps NaN->0 and Inf->saturated**

- **Why:** Rust's float-to-int 'as' cast is saturating: a host returning f64::NAN becomes 0 (a false 'draw' reward) and +/-inf or very large magnitudes saturate to i64::MAX/MIN. A buggy host evaluator returning NaN therefore corrupts search silently with no panic or warning. Verified empirically: (NaN*10) as i64 == 0, (inf*10) as i64 == i64::MAX.
- **Fix:** Guard against non-finite values: if !raw.is_finite() treat as 0 with a debug_assert, or clamp to a documented range before casting.
- **Evidence:** let raw = self.0.interpret_for_player(...);
(raw * REWARD_SCALE) as i64
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Code matches the citation exactly. treant-dynamic/src/adapter.rs:110-115 is `fn interpret_evaluation_for_player(...) -> i64 { let raw = self.0.interpret_for_player(evaluation.value, evaluation.player, *player); (raw * REWARD_SCALE) as i64 }` with REWARD_SCALE=10.0 (adapter.rs:66) and no finite-value guard. I verified the cast behavior by compiling/running: (NaN*10) as i64 == 0 and (inf*10) as i64 
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Code confirmed at /home/peter/code/treant/treant-dynamic/src/adapter.rs:110-115: `let raw = self.0.interpret_for_player(...); (raw * REWARD_SCALE) as i64` with REWARD_SCALE=10.0 (line 66). Empirically verified the saturating cast on this machine: (NaN*10) as i64 == 0, (inf*10) as i64 == i64::MAX, (-inf*10) as i64 == i64::MIN, (1e30*10) as i64 == i64::MAX. So a host returning NaN silently becomes a

### F1 · [LOW] · concurrency · `src/search_tree.rs:716-721, 827-832` · arcade=no
**finish_playout zips path/players/node_path of unequal length, dropping the last edge's virtual loss on UseThisEvalWhenCycleDetected**

- **Why:** `NodeStats::down` (line 1346) subtracts `virtual_loss` from the edge's `sum_evaluations` and is only ever reversed by a matching `up`/`replace` in finish_playout. The truncated last edge keeps a permanent virtual-loss debit, biasing the tree policy against that move forever. Only reachable with a transposition table that creates cycles AND `cycle_behaviour() == UseThisEvalWhenCycleDetected`.
- **Fix:** Push to node_path before the cycle check, or in the UseThisEval branch pop the last path/players entry (and undo its edge `down`) before calling finish_playout, so all three slices stay the same length.
- **Evidence:** line 687 `path.push(choice);` runs, then 685 `choice.stats.down(&self.manager);` already applied a virtual loss to the edge; if a cycle is detected at 717 `finish_playout(path, node_path, ...)` is called BEFORE 723 `node_path.push(node);`. So `path.len() == node_path.len() + 1`. In finish_playout: `for ((move_info, player), node) in path.iter().zip(players.iter()).zip(node_path.iter()).rev()` (827-828) truncates to the shorter `node_path`, so the last (cycle-causing) edge in `path` never receives its compensating `up`/`replace`.
- **Skeptic 1** (uncertain/LOW/library-only, tested=True): The mechanical defect is REAL and present as cited. In src/search_tree.rs the loop pushes to `path`/`players` (lines 686-687) before the cycle check, and only pushes to `node_path` at line 723 AFTER the early return at line 718. So when `UseThisEvalWhenCycleDetected` detects a cycle (line 717), it calls `finish_playout(path, node_path, players, ...)` with `path.len() == players.len() == node_path.
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): CONFIRMED as a real defect, with corrections to scope/severity. Verified in src/search_tree.rs: line 685 `choice.stats.down(&self.manager)` applies the edge virtual loss + visits+1; line 687 `path.push(choice)`; the cycle check at 717 calls `finish_playout(path, node_path, ...)` at 718 BEFORE `node_path.push(node)` at 723 and before the node-level `down` at 724. So in the cycle branch path.len() =

### F4 · [LOW] · concurrency · `src/search_tree.rs:919-936` · arcade=no
**propagate_score_bounds uses compare_exchange_weak and discards failures, so a tightening can be lost to spurious failure or contention in a given pass**

_also reported as: propagate_score_bounds derives proven value from locally computed bounds even when the bound CAS lost the race; `propagate_score_bounds` uses non-retrying `compare_exchange_weak` then breaks on stale local bounds, dropping tightenings under contention_

- **Why:** A legitimate bound tightening computed this pass is silently dropped on CAS failure. Because lower is monotone-increasing and upper monotone-decreasing, the correct lock-free primitive is `fetch_max`/`fetch_min`, which is lossless and needs no retry. Current code is only eventually-consistent (re-tightened on a later playout) so it converges, but it wastes propagation passes and is fragile.
- **Fix:** Replace the conditional compare_exchange_weak with `score_lower.fetch_max(new_bounds.lower, Relaxed)` and `score_upper.fetch_min(new_bounds.upper, Relaxed)`; derive whether anything changed from the returned previous value.
- **Evidence:** `if new_bounds.lower > old_lower { let _ = parent.score_lower.compare_exchange_weak(old_lower, new_bounds.lower, Relaxed, Relaxed); }` (919-925) and the symmetric upper update (928-936). `_weak` may fail spuriously even without contention, and any failure is dropped with no retry/loop.
- **Skeptic 1** (confirmed/LOW/multithread-only, tested=False): Code at src/search_tree.rs:919-936 matches the finding exactly: `if new_bounds.lower > old_lower { let _ = parent.score_lower.compare_exchange_weak(old_lower, new_bounds.lower, Relaxed, Relaxed); }` and the symmetric upper update, with the CAS result discarded (`let _ =`) and no retry loop. score_lower/score_upper are AtomicI32 (lines 58-59) and the bounds are monotone (lower only increases, upper
- **Skeptic 2** (confirmed/LOW/multithread-only, tested=True): Verified src/search_tree.rs:915-936. The code matches the finding verbatim: it loads old_lower/old_upper, then on a tightening does `let _ = parent.score_lower.compare_exchange_weak(old_lower, new, Relaxed, Relaxed)` (and symmetric upper). score_lower/score_upper are AtomicI32 (lines 58-59), so fetch_max/fetch_min are available and ARE the correct lossless primitive for monotone lower(↑)/upper(↓) 

### F5 · [LOW] · concurrency · `src/search_tree.rs:85-90, 633-638` · arcade=no
**score_bounds() reads lower/upper as two independent Relaxed loads (non-atomic snapshot) used by is_proven() to terminate search**

- **Why:** Under concurrent propagate_score_bounds the pair is not a consistent snapshot. Because lower only increases and upper only decreases, a torn read yields a WIDER interval (old-lower with new-upper, etc.), which is conservative for is_proven (won't falsely declare proven) — so today it is benign. But it is an undocumented invariant; if either bound's monotonicity direction ever changed, is_proven could fire on a transiently-equal torn pair and stop search prematurely.
- **Fix:** Document the monotonicity-makes-torn-reads-safe invariant at score_bounds(), or pack both bounds into a single AtomicU64 (lower<<32 | upper) for a true atomic snapshot.
- **Evidence:** `ScoreBounds { lower: self.score_lower.load(Relaxed), upper: self.score_upper.load(Relaxed) }` (86-89) is a split read of two atomics. playout() uses it: `let bounds = self.root_node.score_bounds(); if bounds.is_proven() { return false; }` (634-636), and try_tighten_bounds (413-416) reads children the same way.
- **Skeptic 1** (confirmed/LOW/multithread-only, tested=False): All citations verified against source. src/search_tree.rs:85-90 — score_bounds() performs two independent Relaxed loads of score_lower and score_upper (not a single atomic snapshot). src/lib.rs:377-378 — is_proven() returns lower == upper. src/search_tree.rs:633-637 — playout() calls root_node.score_bounds() and returns false (stops search) if is_proven(), gated on manager.score_bounded_enabled().
- **Skeptic 2** (confirmed/LOW/multithread-only, tested=False): Code matches citation exactly: score_bounds() (src/search_tree.rs:85-90) does two independent Relaxed loads; playout() (633-637) calls is_proven() (src/lib.rs:377-379, lower==upper) to terminate; try_tighten_bounds (412-415) reads children the same way. The monotonicity premise is verified and enforced in propagate_score_bounds (src/search_tree.rs:918-936): lower only rises (CAS guarded by new>old

### F9 · [LOW] · concurrency · `src/transposition_table.rs:198-207` · arcade=no
**size counter double-incremented when two threads insert the same hash into the same empty slot**

- **Why:** The CAS returns the prior key. If it succeeds (returned 0) this thread claimed the slot. If it fails because another thread concurrently wrote the SAME hash into the slot, the CAS returns `my_hash`, the `key_here == my_hash` branch is taken, and `size.fetch_add(1)` runs AGAIN for a slot the winning thread already counted. One occupied slot thus contributes up to N to `size` under N-way contention on the same key. `size` only gates the load-factor short-circuit at line 180 (`size*3 > capacity*2`), so the table will believe it is fuller than it is and start refusing inserts / degrading to lookup-only earlier than the true load warrants. Not a memory-safety issue (the double-free contract via get_or_write is still honored), but it is a genuine accounting defect reachable only under multithreaded insert contention.
- **Fix:** Only increment size on a genuine claim: `if key_here == 0 { self.size.fetch_add(1, Relaxed); return get_or_write(...); }` and let the `key_here == my_hash` (CAS-failed) case fall through to `get_or_write` WITHOUT touching size (the slot was already counted by whoever wrote my_hash).
- **Evidence:** if key_here == 0 {
    let key_here = entry
        .k
        .compare_exchange(0, my_hash, Ordering::Relaxed, Ordering::Relaxed)
        .unwrap_or_else(|x| x);
    if key_here == 0 || key_here == my_hash {
        self.size.fetch_add(1, Ordering::Relaxed);
        return get_or_write(&entry.v, value);
    }
}
- **Skeptic 1** (confirmed/LOW/multithread-only, tested=False): Read src/transposition_table.rs:188-211. Code matches the citation exactly. The defect is real: `entry.k.compare_exchange(0, my_hash, ...).unwrap_or_else(|x| x)` collapses both CAS outcomes to "the prior key value". On CAS success it returns Ok(0) -> key_here==0 -> legitimate first claim, fetch_add(1) correct. On CAS failure because a concurrent thread wrote the SAME hash into the slot, it returns
- **Skeptic 2** (confirmed/LOW/multithread-only, tested=False): Verified against src/transposition_table.rs:198-207. The cited code matches exactly. The bug is real: at line 199-202 the CAS uses unwrap_or_else(|x| x), so on failure it returns the prior key. If a concurrent thread already wrote the same my_hash into this empty slot, the CAS fails and returns my_hash; the condition at line 203 (key_here == 0 || key_here == my_hash) is then true via the second di

### F10 · [LOW] · concurrency · `src/transposition_table.rs:180-182` · arcade=no
**Load-factor short-circuit read is racy and unsynchronized, can briefly exceed target and start dropping inserts**

_also reported as: Null-value window in insert allows a concurrent lookup to return None for a key that is being inserted_

- **Why:** `size` is read with Relaxed ordering and is incremented by other threads without any happens-before relationship to this read, and (per the previous finding) can be overcounted. Combined with the 16-slot PROBE_LIMIT, once the table approaches ~0.66 load, inserts that fall through 16 occupied probe slots silently return `None` (line 211). For the caller in descend() (src/search_tree.rs:799), `insert` returning `None` means the freshly created node is treated as freshly owned and is NOT shared via transposition — so transposition sharing silently degrades near capacity. This is documented as 'approximate', but the interaction (overcounted size + low probe limit + relaxed read) means degradation can begin meaningfully before the nominal load target. Worth a doc note on the contract that `enough_to_hold(n)` only sizes for ~0.66 load and inserts are best-effort past that.
- **Fix:** Document the load-factor / probe-limit interaction on ApproxQuadraticProbingHashTable and enough_to_hold; consider deriving the load-factor threshold from PROBE_LIMIT, or at least note that capacity should comfortably exceed the working set.
- **Evidence:** if self.size.load(Ordering::Relaxed) * 3 > self.capacity * 2 {
    return self.lookup(key, handle);
}
- **Skeptic 1** (confirmed/LOW/multithread-only, tested=False): All code citations verified against source. src/transposition_table.rs:180-182 matches the evidence exactly: `if self.size.load(Ordering::Relaxed) * 3 > self.capacity * 2 { return self.lookup(...) }`. `size` is read Relaxed and incremented Relaxed by other threads at line 204 with no happens-before to the read — the race is real. `enough_to_hold` (lines 133-138) sizes capacity to ~1.5x requested, 
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified src/transposition_table.rs:180-182 matches the cited evidence exactly: `if self.size.load(Ordering::Relaxed) * 3 > self.capacity * 2 { return self.lookup(...) }`. The probe loop (188-211) returns `None` after PROBE_LIMIT=16 occupied slots. The caller in search_tree.rs:799-816 confirmed: a `None` from `insert` makes the freshly-created node owned (choice.owned=true, num_nodes incremented) 

### F75 · [LOW] · concurrency · `treant-dynamic/src/evaluators.rs:17-22, 61-70` · arcade=no
**Seeded RandomRollout serializes all parallel rollouts on one Mutex and is not actually deterministic under parallelism**

- **Why:** When constructed with with_seed(), every evaluate() in every search thread locks the same Mutex<Rng64> for each random move of each rollout. Under playout_n_parallel this fully serializes the rollout RNG, collapsing parallel speedup for rollout-heavy games. Worse, the doc claims with_seed gives 'deterministic single-threaded replay' — but nothing prevents a seeded evaluator from being used with playout_n_parallel, where thread-scheduling nondeterminism over the shared RNG makes results non-reproducible. The determinism guarantee holds only single-threaded, which is not enforced.
- **Fix:** Use per-thread RNGs seeded deterministically from the base seed (e.g. via core's ExtraThreadData/seed hooks) instead of a shared Mutex, or document that with_seed is single-threaded-only and is a perf trap under parallel search.
- **Evidence:** pub struct RandomRollout { rng: Option<Mutex<Rng64>> }
...
Some(mutex_rng) => { let mut rng = mutex_rng.lock().unwrap(); rng.gen_range(0..available.len()) }
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read treant-dynamic/src/evaluators.rs:17-22 and 61-70 directly; code matches the finding verbatim. Struct is `rng: Option<Mutex<Rng64>>` and the seeded branch (lines 62-64) locks the single shared Mutex for every random move of every rollout. Both claims are real: (1) under playout_n_parallel a seeded RandomRollout serializes the rollout RNG inner loop across all threads, collapsing parallel speed
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read treant-dynamic/src/evaluators.rs:17-22 and 61-70 — code matches the finding verbatim: `rng: Option<Mutex<Rng64>>`, and the seeded branch does `mutex_rng.lock().unwrap(); rng.gen_range(0..available.len())` inside the per-move rollout loop. EvalCallbacks is `Send + Sync` with `evaluate(&self, ...)` (callbacks.rs:53,61), and DynMCTSManager::playout_n_parallel exists (manager.rs:50), so under par

### F81 · [LOW] · concurrency · `src/search_tree.rs:808-811, 1045-1046, 1246` · arcade=no
**`.lock().unwrap()` on `orphaned` Mutex panics if poisoned (Mutex poisoning hazard)**

- **Why:** All accesses to the `orphaned` Mutex use `.lock().unwrap()`. If any thread panics while holding the lock, the Mutex is poisoned and every subsequent `lock().unwrap()` panics, cascading the failure across all search threads. In `descend` the lock is on the (transposition-table) hot path. Reachability is low because the critical sections only do `push`/`len`/`clear` (which don't panic on their own), so poisoning requires a panic from elsewhere while the guard is held — effectively only via an unwinding panic interleaving. Still, it is a fragile pattern in a library marketed as lock-free/parallel.
- **Fix:** Use `.lock().unwrap_or_else(|e| e.into_inner())` to recover from poisoning, or document that a panic in any search thread invalidates the tree. The orphaned-node bookkeeping is non-critical, so poison recovery is clearly safe here.
- **Evidence:** self.orphaned
    .lock()
    .unwrap()
    .push(unsafe { Box::from_raw(created) });  // descend()
...
self.orphaned.lock().unwrap().len()           // diagnose()
self.orphaned.lock().unwrap().clear();         // advance_root()
- **Skeptic 1** (confirmed/LOW/multithread-only, tested=False): Code matches the finding exactly. src/search_tree.rs:29 declares `orphaned: Mutex<Vec<Box<SearchNode<Spec>>>>`. The three cited `.lock().unwrap()` sites are present verbatim: descend() at 808-811 (`self.orphaned.lock().unwrap().push(unsafe { Box::from_raw(created) })`), diagnose() at 1045 (`self.orphaned.lock().unwrap().len()`), and advance_root() at 1246 (`self.orphaned.lock().unwrap().clear()`).
- **Skeptic 2** (confirmed/LOW/multithread-only, tested=False): Cited lines are accurate. src/search_tree.rs:29 declares `orphaned: Mutex<Vec<Box<SearchNode<Spec>>>>`, and all three accesses (descend push at 808-811, diagnose len at 1045, advance_root clear at 1246) use `.lock().unwrap()`. No test in tests/mcts_tests.rs exercises poisoning, catch_unwind, or thread panics against this Mutex; the only related tests are batch_sizes locks which are unrelated. So t

### F28 · [LOW] · contract · `src/lib.rs:154-160` · arcade=no
**fpu_value() doc claims INFINITY forces all children before revisit, but AlphaGoPolicy ignores that for the default**

- **Why:** In AlphaGoPolicy, when fpu is the default INFINITY, the `fpu.is_finite()` guard is false, so unvisited children fall through to the PUCT formula (finite score = 2*C*P*sqrt(N)) instead of getting +INFINITY. So the documented "try all children first" behavior only holds for UCTPolicy (L191-192 returns fpu unconditionally for n==0). For PUCT the default does NOT force exhaustive first-visiting; selection is prior-guided. A user reading the trait doc will mis-model PUCT's first-visit behavior.
- **Fix:** Clarify in fpu_value() docs that the INFINITY=try-all-first semantics apply to UCTPolicy; for AlphaGoPolicy the default INFINITY means prior-guided expansion (unvisited children scored by the PUCT formula), and a finite fpu overrides the first-visit value.
- **Evidence:** lib.rs L154-160 doc: "`f64::INFINITY` (default) forces all children to be tried before any revisit." tree_policy.rs L260: `if child_visits == 0 && fpu.is_finite() { fpu } else { ... PUCT formula }`
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Read all cited code. src/lib.rs:154-160: fpu_value() doc on the policy-agnostic MCTS trait states the default f64::INFINITY "forces all children to be tried before any revisit." UCTPolicy (tree_policy.rs:191-192) does honor this: `if child_visits == 0 { fpu }` returns fpu unconditionally, so default INFINITY => every unvisited child scores +INF => all tried first. But AlphaGoPolicy (tree_policy.rs
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified against source. lib.rs:154-160 documents fpu_value() generically: "f64::INFINITY (default) forces all children to be tried before any revisit." This holds for UCTPolicy (tree_policy.rs:191-192: `if child_visits == 0 { fpu }` returns fpu unconditionally, so INFINITY gives every unvisited child +INF). It does NOT hold for AlphaGoPolicy (tree_policy.rs:259-267: `if child_visits == 0 && fpu.i

### F36 · [LOW] · contract · `src/transposition_table.rs:45-47` · arcade=no
**unsafe trait TranspositionTable::clear has a no-op default but its doc says it prevents dangling pointers**

- **Why:** This is an `unsafe trait` whose safety contract (insert returns None on insert => no double-free) is paired with re-rooting that frees nodes. A custom `TranspositionTable` impl that does not override `clear` compiles fine and silently retains stale `AtomicPtr<SearchNode>` entries after `advance`/`reset` frees those nodes, yielding use-after-free on the next lookup. The default should not silently no-op for a table that actually stores pointers; the no-op is only correct for the `()` impl.
- **Fix:** Either make `clear` a required method (no default) for the unsafe trait, or document loudly in the safety section that any pointer-storing implementation MUST override clear, and have re-rooting assert/track that clear was implemented.
- **Evidence:** `/// Clear all entries from the table. /// Called during tree re-rooting to prevent dangling pointers. fn clear(&mut self) {}`
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/transposition_table.rs:5-48 directly. The cited code is accurate: it is an `unsafe trait TranspositionTable` whose safety doc (lines 5-7) covers ONLY the insert-returns-None double-free contract and says nothing about clear. `fn clear(&mut self) {}` (line 47) is a no-op default, and its doc (lines 45-46) claims it "prevents dangling pointers." I confirmed the re-rooting mechanic in src/se
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified src/transposition_table.rs:45-47 — clear() has a no-op default body on a public unsafe trait, exactly as quoted. The trait's safety doc (lines 5-7) only documents the insert-returns-None contract and does NOT mention the clear-on-rerooting requirement. clear() is genuinely load-bearing for memory safety: src/search_tree.rs:1239-1243 calls self.table.clear() immediately before drop(old_roo

### F53 · [LOW] · contract · `src/lib.rs:434-443` · arcade=no
**GameState::chance_outcomes documents a 'must sum to 1.0 / must be positive' contract enforced nowhere**

- **Why:** The doc states probabilities 'must be positive and sum to 1.0', but there is no validation: the chance-sampling path in search_tree.rs only has `debug_assert!(!outcomes.is_empty())` (search_tree.rs:523) and never checks positivity or normalization. Probabilities that don't sum to 1.0 (or include zeros/negatives) silently skew sampling rather than erroring, so a violated contract produces wrong search results with no diagnostic. Unlike AlphaGoPolicy::validate_evaluations (tree_policy.rs:272) which DOES assert priors sum to ~1, the analogous chance contract is unenforced even under debug.
- **Fix:** Either add a debug_assert validating positivity and approximate normalization at the chance-sampling site, or soften the rustdoc to say outcomes are normalized internally / weights are treated as relative — and state explicitly that the contract is unchecked.
- **Evidence:** /// Probabilities must be positive and sum to 1.0.
    /// Return `None` for deterministic transitions (the default).
...
    fn chance_outcomes(&self) -> Option<Vec<(Self::Move, f64)>> {
        None
    }
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Verified against source. lib.rs:434 documents "Probabilities must be positive and sum to 1.0." The chance node-creation path (search_tree.rs:241-260) collects probs via `outcomes.iter().map(|(_, p)| *p)` and stores them directly into `chance_probs` with NO positivity or normalization check, then returns early — before reaching the `policy.validate_evaluations(&move_eval)` call at search_tree.rs:26
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Factually accurate. src/lib.rs:434 documents "Probabilities must be positive and sum to 1.0" but no code enforces it. Both chance-sampling sites only assert non-emptiness: search_tree.rs:523 (open-loop, debug_assert !outcomes.is_empty()) and search_tree.rs:504-505 (closed-loop, debug_assert is_chance + non-empty). Node creation (search_tree.rs:241-260) stores probs verbatim (line 242 collects, lin

### F54 · [LOW] · contract · `src/transposition_table.rs:69-76` · arcade=no
**TranspositionHash hash==0 reservation is a silent foot-gun documented only on the trait, not enforced at insert**

- **Why:** The contract 'hash 0 is reserved' is real — ApproxTable::insert early-returns None when `my_hash == 0` (transposition_table.rs:184), using 0 as the empty-slot sentinel. But a GameState whose hash legitimately collides to 0 is silently never inserted, defeating transposition sharing for that state with zero diagnostic. The doc warns the implementer but nothing detects/repairs a 0 hash. This is an implicit correctness contract that fails silently (degraded search) rather than loudly. Worth at least a stronger doc note that a 0 hash silently disables sharing for that state, and ideally a recommendation to fold 0 to a nonzero constant in the hash impl.
- **Fix:** Strengthen the rustdoc: state that returning 0 silently disables transposition sharing for that state (it is the empty-slot sentinel), and recommend implementors map a computed 0 to a fixed nonzero value.
- **Evidence:** /// Hash `0` is reserved and will not be inserted into the table.
pub trait TranspositionHash {
    /// Compute a hash of this game state. Must return nonzero for insertable states.
    fn hash(&self) -> u64;
}
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/transposition_table.rs:69-76 and 174-240. The finding is factually accurate. The doc (lines 69-76) matches the quoted evidence verbatim: "Hash `0` is reserved and will not be inserted into the table" and "Must return nonzero for insertable states." ApproxTable::insert at lines 184-186 early-returns None when `my_hash == 0`. 0 is genuinely the empty-slot sentinel: in insert, `key_here == 0
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified against src/transposition_table.rs. Lines 184-186 confirm ApproxTable::insert early-returns None when `my_hash == 0`, and 0 is the empty-slot sentinel (key_here == 0 checks at lines 198, 203, and clear() zeroing at line 215). The contract is documented only on the trait (lines 72, 74: "Hash 0 is reserved and will not be inserted"; "Must return nonzero for insertable states") and is not de

### F63 · [LOW] · contract · `treant-wasm/src/gridpack.rs:26-33` · arcade=yes
**result() string contract is non-uniform across games and enforced only by per-game TS normalization**

- **Why:** gridpack (and most games) return a BARE 1-indexed digit ("1"/"2") for the winner, while mancala.rs:462 returns "P{N}" ("P1"/"P2"). The shared session layer consumes result() as `Number(result) - 1` to compute the winner seat (useGameSession.ts:61). A bare digit parses correctly; "P1" parses to NaN. The only reason mancala works is that mancala.tsx:14-17 manually strips the leading 'P' in its TS handle. This is an implicit contract enforced nowhere: a new game author copying mancala's Rust `result()` style but forgetting the TS normalization gets a silent winner-seat=NaN bug (wrong win/lose sound, broken winner highlighting). The GameHandle.result() doc in gameTypes.ts:33 does not state the required format at all.
- **Fix:** Pick one canonical format (bare 1-indexed digit + 'Draw' + '') and document it on GameHandle.result(); change mancala.rs to emit the bare digit and drop the TS strip, or add a shared Rust helper all games call so the format cannot drift.
- **Evidence:** fn result_str(term: Option<ProvenValue>, current: u8) -> String {
    match term {
        Some(ProvenValue::Win) => format!("{}", current + 1),
        Some(ProvenValue::Loss) => format!("{}", (1 - current) + 1),
        Some(ProvenValue::Draw) => "Draw".into(),
        _ => String::new(),
    }
}
- **Skeptic 1** (confirmed/MEDIUM/arcade-path, tested=False): All cited code verified verbatim. gridpack.rs:26-33 emits bare 1-indexed digits ("1"/"2") for Win/Loss, "Draw", or "" — matches the evidence exactly. Most wasm games follow this format (reversi.rs:215-217, dotsboxes.rs:180-182, chomp.rs:131, hex.rs:192, frontline.rs:200-201, etc.). mancala.rs:455-465 is the unique outlier: result() emits format!("P{}", p+1) → "P1"/"P2" (its own doc-comment even sa
- **Skeptic 2** (confirmed/LOW/unreachable, tested=False): All factual claims verified against source. gridpack.rs:26-33 result_str returns a bare 1-indexed digit ("1"/"2"), "Draw", or "" exactly as quoted. mancala.rs:456-465 is the SOLE outlier returning format!("P{}", p+1) ("P1"/"P2"); grep for format!("P confirms it is the only game using that style. Every other game (connectfour, pig, hex, frontline, shift, tictactoe, reversi, capturego, etc.) returns

### F76 · [LOW] · contract · `treant-dynamic/src/callbacks.rs:63-76` · arcade=no
**interpret_for_player default (-value for opponent) is wrong for 3+ player games but the foot-gun is undocumented at the dyn layer**

- **Why:** The default negates for any non-matching player, which is only correct for zero-sum two-player games. A 3-player game using the default gets every opponent's reward as -value, which is not the game's true reward structure and will mislead search. The doc says 'Default: negate for opponent (zero-sum two-player games)' but does not warn that using the default outside that regime is a silent correctness bug. current_player() doc (callbacks.rs:13-15) even hints at multi-player ('distinct integers for each player') without cross-referencing this limitation.
- **Fix:** Add a warning to interpret_for_player and current_player docs that the default reward interpretation is only valid for two-player zero-sum games and must be overridden otherwise.
- **Evidence:** fn interpret_for_player(&self, value: f64, evaluating_player: i32, requesting_player: i32) -> f64 {
    if evaluating_player == requesting_player { value } else { -value }
}
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read treant-dynamic/src/callbacks.rs:63-76 — the default interpret_for_player matches the quoted code exactly: `if evaluating_player == requesting_player { value } else { -value }`. This negation is only correct for two-player zero-sum games; for a 3+ player game it assigns every non-acting player -value, which is not the game's true reward structure. The doc comment (line 64) labels the regime ("
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read callbacks.rs:63-76 — the default interpret_for_player matches the finding verbatim: `if evaluating_player == requesting_player { value } else { -value }`, documented only as "Default: negate for opponent (zero-sum two-player games)". This is genuinely correct ONLY for two-player zero-sum; a 3+ player game using the default would treat every opponent's reward as -value, which is not the true r

### F86 · [LOW] · contract · `treant-dynamic/tests/golden.rs:187-267` · arcade=no
**root_score_bounds cross-language contract is unchecked: golden JSON has no score-bounded entry and check_expectations cannot assert bounds**

- **Why:** DynMCTSManager::root_score_bounds is a public adapter API (confirmed via `pub fn root_score_bounds`) and a documented opt-in feature, but its cross-language behavior is asserted nowhere in the shared JSON, and even the Rust smoke test accepts any tightening — it would pass even if the sign were flipped (e.g. exact(-5) instead of exact(5)). A binding that mis-derives bounds from terminal_score would not be caught.
- **Fix:** Add a `score_bounds` expectation branch to check_expectations (parse lower/upper, compare to mgr.root_score_bounds()), add a score-bounded ScoreGame entry to golden_tests.json with exact expected bounds, and tighten the score_bounded_game unit test to assert ScoreBounds::exact(5).
- **Evidence:** check_expectations() handles only best_move, proven_value, child_stats(visits_gte/lte/proven), and num_nodes_gte. There is no branch reading an expected `score_bounds`. The only score-bounded exercise is the Rust unit test score_bounded_game (golden.rs:836-864), which merely asserts `bounds.lower > i32::MIN || bounds.upper < i32::MAX` ("bounds should tighten") rather than the exact expected value (P1 picks score 5 per the comment).
- **Skeptic 1** (confirmed/LOW/feature-gated, tested=True): Verified against source. check_expectations (treant-dynamic/tests/golden.rs:187-267) only handles best_move, proven_value, child_stats (visits_gte/lte/proven), and num_nodes_gte — there is no score_bounds branch. The shared golden JSON (tests/golden/golden_tests.json, 7 tests) has expect keys limited to {best_move, child_stats, proven_value}; no score-bounded entry exists. The only score-bounds ex
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): All factual claims in F86 verified against source. check_expectations (treant-dynamic/tests/golden.rs:187-267) handles only best_move, proven_value, child_stats (visits_gte/lte/proven), and num_nodes_gte — there is no score_bounds branch. The shared golden JSON at tests/golden/golden_tests.json (path is workspace-root, not treant-dynamic/tests/ as the finding's File header implies) contains 7 test

### F88 · [LOW] · contract · `treant-wasm/src/gridpack.rs:461-464, 26-33, 491-494` · arcade=yes
**result() string contract ("1"/"2"/"Draw"/"") is never asserted for the cell_game_wasm! macro games' public types**

- **Why:** The arcade's React layer depends on result() returning exactly "" while in-progress and "1"/"2"/"Draw" at terminal (ARCADE.md result-format contract). The `(1 - current)` arithmetic silently assumes current is exactly 0 or 1 (u8); any game where current can be 2+ would underflow-panic. No test pins the empty-string-while-playing case for macro games, and Connect6's win/draw mapping (which has the unusual 2-stones-per-turn `current` toggling, gridpack.rs:72-81) is unverified.
- **Fix:** Add tests asserting result()=="" for a fresh/mid-game macro board and result()=="1"/"2"/"Draw" at the relevant terminals for Connect6Wasm specifically (its placed_turn quota logic makes the `current`-at-terminal mapping non-obvious).
- **Evidence:** result_str maps Win->`current+1`, Loss->`(1-current)+1`, Draw->"Draw", else "". The cell_game_wasm! macro generates result() for Connect6Wasm, SquavaWasm, NotaktoWasm, SquareUpWasm. The gridpack #[cfg(test)] tests assert result() only for the hand-written non-macro types (SquavaWasm/NotaktoWasm/OrderChaosWasm/SquareUpWasm via apply_move sequences) — but Connect6Wasm result() is never asserted, and the macro-generated result() for a *non-terminal* position (expected "") is never tested for any macro game.
- **Skeptic 1** (confirmed/LOW/arcade-path, tested=False): Read gridpack.rs:26-33 (result_str), 461-464 (macro result()), 491-494 (macro invocations), the Connect6 impl (36-85), and the full #[cfg(test)] block (566-627).

CONFIRMED factual core: result_str maps Win->current+1, Loss->(1-current)+1, Draw->"Draw", else "" (lines 27-32). The cell_game_wasm! macro generates result() for Connect6Wasm, SquavaWasm, NotaktoWasm, SquareUpWasm (491-494). The tests a
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read gridpack.rs at the cited lines and the full #[cfg(test)] module (566-627). The factual claims hold:

1. result_str (lines 26-33) maps Win->`current+1`, Loss->`(1-current)+1`, Draw->"Draw", else "". Confirmed verbatim.
2. The cell_game_wasm! macro (419-489) generates result() at lines 461-464 and is instantiated for Connect6Wasm/SquavaWasm/NotaktoWasm/SquareUpWasm (491-494).
3. Connect6Wasm.re

### F6 · [LOW] · correctness · `src/search_tree.rs:831-839` · arcade=no
**finish_playout backprop handle is built from a re-loaded move_info.child that may differ from the node actually accumulated into**

- **Why:** The user's on_backpropagation callback receives a handle to a node inconsistent with the node whose stats were just updated. Benign for built-in behavior (default on_backpropagation is a no-op) but a foot-gun for custom evaluators that mutate NodeData via the handle, and the SAFETY comment (833-834) asserts non-null without addressing the identity mismatch.
- **Fix:** Pass the already-captured `node` (node_path[i]) to make_handle instead of re-loading move_info.child, ensuring the handle matches the node whose stats were updated.
- **Evidence:** Stats are accumulated on `node` (node_path[i]): line 831 `node.stats.up(...)` and 832 `move_info.stats.replace(&node.stats)`. But the on_backpropagation handle is built from a fresh reload: `self.make_handle(&*move_info.child.load(Ordering::Acquire), tld)` (838). After a delayed-transposition redirect store at 807, `move_info.child` can point to a different node than `node_path[i]` that this playout descended into.
- **Skeptic 1** (confirmed/LOW/multithread-only, tested=False): Read src/search_tree.rs:819-853 (finish_playout), 745-817 (descend), 621-743 (playout), plus make_handle (962) and NodeHandle::data (1267) and the default on_backpropagation (lib.rs:262). The cited code matches: line 831-832 accumulate stats onto node = node_path[i]; line 838 builds the backprop handle from a fresh self.make_handle(&*move_info.child.load(Acquire)). make_handle just wraps whatever 
- **Skeptic 2** (confirmed/LOW/multithread-only, tested=False): Read src/search_tree.rs:819-853 (finish_playout), 745-816 (descend), 656-743 (playout loop), 962-972 (make_handle), and lib.rs:262 (default on_backpropagation). The finding's structural claim is accurate: stats accumulate on node=node_path[i] (831) while the handle is built from a fresh move_info.child.load() (838).

Reachability is narrower than the finding implies. grep shows the ONLY store to .

### F16 · [LOW] · correctness · `src/transposition_table.rs:133-139` · arcade=no
**enough_to_hold can produce capacity 1 (or 0-derived edge) and assumes non-trivial num; tiny num yields useless table**

- **Why:** For num=0 or num=1, the loop body never executes (2 < 0, 2 < 3 is true for num=1 actually -> capacity becomes 2). For num=1: 1*2 < 1*3 (2<3) true -> capacity=2; 2*2<3 false -> stops at capacity=2. OK. But for very large num, `num * 3` can overflow usize (panics in debug, wraps in release producing a tiny capacity). There is no overflow guard. Also capacity=1 (num=0) is a valid power of two but a 1-slot table with PROBE_LIMIT 16 will spin probing the same slot. Edge inputs are silently degenerate.
- **Fix:** Guard against overflow (use saturating/checked arithmetic on `num * 3`) and enforce a sensible minimum capacity; document behavior for num near 0.
- **Evidence:** pub fn enough_to_hold(num: usize) -> Self {
    let mut capacity = 1;
    while capacity * 2 < num * 3 {
        capacity <<= 1;
    }
    Self::new(capacity)
}
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/transposition_table.rs:132-139; code matches the finding verbatim. Verified the arithmetic by compiling the loop standalone: num=0 -> capacity=1, num=1 -> capacity=2 (reporter's self-correction is right), num=2->4, num=3->8, etc. The two real defects: (1) num=0 yields a 1-slot table; (2) `num * 3` has no overflow guard, so num > usize::MAX/3 panics in debug and wraps in release to a bogus
- **Skeptic 2** (uncertain/LOW/unreachable, tested=False): Read src/transposition_table.rs:133-139 (enough_to_hold) plus new() at :117-131 and both probe loops at :188 and :227. Findings:

PARTIALLY CONFIRMED, but the finding's most alarming claim is wrong. (1) Overflow is real: `num * 3` is unguarded and overflows usize when num > usize::MAX/3 (~6.1e18). Verified empirically (checked_mul returns None). In debug this panics; in release it wraps and yields

### F20 · [LOW] · correctness · `src/search_tree.rs:909-953` · arcade=no
**Chance-node proven value and score bounds can be set to mutually contradictory values**

- **Why:** When solver + score_bounded + closed_loop_chance are all enabled, `propagate_proven` may set a chance node's proven value to Draw (per the bug above) while `propagate_score_bounds` computes a converged expectation of, say, +5 and tries to set proven=Win. The CAS at line 947 only writes from Unknown, so whichever propagation runs first wins and the other silently no-ops, leaving `proven_value()` and `score_bounds()` inconsistent on the same node (e.g. proven=Draw but bounds=[5,5]). Downstream selection/PV may then make contradictory choices depending on which accessor it reads.
- **Fix:** Make the two propagation systems agree on chance nodes: derive chance proven value solely from the converged score expectation, and gate the proven write so it cannot conflict with an already-converged bound. Add an assertion in debug builds that proven_value and is_proven()/sign agree.
- **Evidence:** let new_bounds = if parent.is_chance { try_tighten_bounds_chance(parent) } else { try_tighten_bounds(parent) }; ... if self.manager.solver_enabled() && new_bounds.lower == new_bounds.upper { let pv = if new_bounds.lower > 0 { Win } ... }; let _ = parent.proven.compare_exchange(Unknown, pv ...)
- **Skeptic 1** (confirmed/LOW/feature-gated, tested=False): Read src/search_tree.rs:909-953 (propagate_score_bounds), 846-897 (propagate_proven), 435-497 (try_prove_chance_node / try_tighten_bounds_chance), 80-90 (accessors), and 237-262 (chance-node creation). The cited code matches.

The defect is real. For a chance node, try_prove_chance_node (l.435) classifies mixed-proven children (some Win, some Loss/Draw) as proven=Draw, while try_tighten_bounds_cha
- **Skeptic 2** (confirmed/LOW/feature-gated, tested=False): Read src/search_tree.rs:435-497 (chance prove/bounds), 856-960 (both propagations), 840-852 (call order), 996-1018 (PV), 275-296 (terminal invariant). The finding is accurate and the contradiction is real and reproducible by construction. try_prove_chance_node (line 435) returns Draw for any mixed Win/Loss/Draw child set; try_tighten_bounds_chance (line 472) computes a probability-weighted expecta

### F22 · [LOW] · correctness · `src/search_tree.rs:489-496` · arcade=no
**try_tighten_bounds_chance: float accumulation + saturating cast can mis-bound; no overflow/precision guard**

- **Why:** Children's bounds are i32 (terminal_score is arbitrary i32). For large-magnitude scores the f64 weighted sum loses integer precision beyond 2^53 (not reachable from i32 alone, but the floor/ceil rounding still widens bounds by up to 1 each propagation step, which can prevent convergence and thus prevent is_proven()/the cross-system proven derivation from ever firing). The `as i32` cast saturates rather than overflows (safe since Rust 1.45), but combined with floor/ceil a chance node whose true expectation is exact (e.g. 0.5*4 + 0.5*4 = 4) is fine, whereas 0.5*3+0.5*4=3.5 yields [3,4] and never converges even though that's the genuine exact interval. Acceptable, but undocumented as a non-convergence source for chance subtrees.
- **Fix:** Document that chance-node bounds are widened by floor/ceil and therefore chance subtrees may not converge to is_proven() even when children are exact; consider tracking the expectation as a rational or keeping the unrounded f64 to decide proven-ness.
- **Evidence:** lower_sum += prob * child_lower as f64; upper_sum += prob * child_upper as f64; ... ScoreBounds { lower: lower_sum.floor() as i32, upper: upper_sum.ceil() as i32 }
- **Skeptic 1** (confirmed/LOW/feature-gated, tested=False): Code matches the citation exactly. src/search_tree.rs:489-496 accumulates `lower_sum += prob * child_lower as f64` / `upper_sum += prob * child_upper as f64` and returns `ScoreBounds { lower: lower_sum.floor() as i32, upper: upper_sum.ceil() as i32 }`. is_proven() (src/lib.rs:377-379) is `self.lower == self.upper`. So the described non-convergence is real: a chance node whose true weighted expecta
- **Skeptic 2** (uncertain/LOW/feature-gated, tested=False): Read src/search_tree.rs:472-497 (the cited code matches exactly) and the call site at lines 909-953. The mechanism is real: `lower_sum`/`upper_sum` accumulate `prob * child_bound as f64`, then bounds are `{floor(lower_sum), ceil(upper_sum)}`. The cross-system proven derivation (line 939) only fires when `new_bounds.lower == new_bounds.upper`, so a chance node whose true expectation is non-integer 

### F25 · [LOW] · correctness · `src/search_tree.rs:685-723` · arcade=no
**Virtual-loss / propagation length mismatch when a cycle breaks mid-iteration (path longer than node_path)**

- **Why:** In the UseCurrentEvalWhenCycleDetected / UseThisEvalWhenCycleDetected cycle branches, the loop breaks after `path.push`/`players.push` but before `node_path.push`, so path.len() == node_path.len()+1. propagate_proven/propagate_score_bounds index node_path[i-1] for i up to path.len()-1, which stays in bounds (verified: node_path[path.len()-2] == node_path[node_path.len()-1]), so no panic. However finish_playout's backprop zips path/players/node_path and silently truncates the extra path entry, so the virtual loss applied by `choice.stats.down` on that edge is never balanced by an `up`. This is a backprop-bookkeeping leak that also makes the last edge's stats slightly wrong for any node whose bounds/proofs are later read. Only reachable with a non-ZST transposition table AND a UseCurrentEval/UseThisEval cycle policy.
- **Fix:** Either push node_path before the cycle check (so lengths always match), or apply choice.stats.down only after the node is committed to node_path. Add a debug_assert!(path.len() == node_path.len()) at the top of finish_playout to catch the divergence.
- **Evidence:** path.push(choice); ... choice.stats.down(&self.manager); [then UseCurrentEvalWhenCycleDetected: if is_cycle(node_path, node) { break; }] ... node_path.push(node); node.stats.down(...)
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/search_tree.rs:685-743, 819-841, 856-907 and verified the structure. At :685 `choice.stats.down()` and :687 `path.push(choice)` execute before the cycle check at :711-721. Both cycle-eval branches exit early — UseCurrentEval `break`s at :713, UseThisEval `return`s at :718-719 — before :723 `node_path.push(node)` and :724 `node.stats.down()`. So `path.len() == node_path.len()+1`. I empiric
- **Skeptic 2** (confirmed/LOW/feature-gated, tested=False): Read src/search_tree.rs:650-743 (loop), 819-958 (finish_playout/propagate_*), 1346-1367 (NodeStats down/up/replace), 517-519 (is_cycle). The structural claim is CORRECT: choice.stats.down() (685) and path.push(choice) (687) run before the cycle-break at line 713 (UseCurrentEval) / 718 (UseThisEval), while node_path.push(node) (723) is skipped, so path.len() == node_path.len()+1 in those two branch

### F26 · [LOW] · correctness · `src/tree_policy.rs:58-59, 194-196` · arcade=no
**UCT exploration term is sqrt(2)x larger than the documented UCB1 formula**

- **Why:** The code computes C * 2 * sqrt(ln N / n) = C * sqrt(4*ln N / n), but the rustdoc claims C * sqrt(2*ln N / n). The effective exploration is sqrt(2) (~1.41x) larger than documented. Anyone calibrating the exploration constant C against the documented formula (e.g. the arcade difficulty calibration on this branch, or porting C values from literature) will get a meaningfully different tree. This is either a wrong constant or wrong docs; the two disagree.
- **Fix:** Decide on the canonical formula. Either change `2.0 *` to `2.0_f64.sqrt() *` (SQRT_2) to match the documented and classic UCB1 `C*sqrt(2 ln N / n)`, or fix the doc to state `Q + C * 2 * sqrt(ln N / n)` and note C semantics differ from textbook UCB1.
- **Evidence:** Doc (L58-59): `Q(a) + C * sqrt(2 * ln(N) / n(a))`. Code (L194-196): `let explore_term = 2.0 * (ln_adjusted_total / child_visits as f64).sqrt(); ... self.exploration_constant * explore_term + mean_action_value`
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Read src/tree_policy.rs directly. The doc/code mismatch is real and unguarded.

CODE (src/tree_policy.rs:146-148, 194-196): ln_adjusted_total = (total_visits+1).ln() = ln(N+1). explore_term = 2.0 * (ln_adjusted_total / child_visits).sqrt(); value = exploration_constant * explore_term + mean_action_value. So the bonus is C * 2 * sqrt(ln N / n) = C * sqrt(4 ln N / n).

RUSTDOC (src/tree_policy.rs:58
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified at src/tree_policy.rs. Doc L59 states `Q(a) + C * sqrt(2 * ln(N) / n(a))`. Code L194-196 computes `explore_term = 2.0 * sqrt(ln_adjusted_total / child_visits)` then `C * explore_term + mean`, i.e. `C * 2 * sqrt(ln(N+1)/n)` = `C * sqrt(4 * ln(N+1)/n)`. Pulling the 2.0 inside the radical yields a factor of 4 vs the documented 2, so effective exploration is sqrt(2) (~1.41x) larger than the d

### F29 · [LOW] · correctness · `src/lib.rs:675-705` · arcade=no
**Temperature-based best_move() ignores proven values and score bounds, can return a proven-losing move**

- **Why:** With solver_enabled() or score_bounded_enabled() and a nonzero selection_temperature, best_move() can sample a move that the solver has proven to lose (or that has a strictly worse guaranteed bound) purely because it has a high visit count, even though a proven-winning move exists. The two best_move paths are inconsistent about respecting proven/bounded values.
- **Fix:** In select_move_by_temperature, when solver_enabled(): if any child is proven Loss-for-child (parent win), return argmax over those (or restrict the sampling pool to non-losing children). Optionally exclude proven-loss children from the weighted pool and respect score bounds, mirroring select_child_after_search.
- **Evidence:** best_move (L675-682) routes temperature>=1e-8 to select_move_by_temperature. select_move_by_temperature (L684-705) only does `filter(|c| c.visits() > 0)` and weights by `(c.visits()).powf(inv_temp)` — no check of child_proven_value() or child_score_bounds(). Contrast: the temp==0 path uses principal_variation(1) -> select_child_after_search (L173-206) which prefers proven-win/draw and best score-bound.
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Verified against src/lib.rs:675-705. The defect is real as written: best_move() (L676-681) routes selection_temperature>=1e-8 to select_move_by_temperature (L684-705), which only does `.filter(|c| c.visits() > 0)` and weights by `(c.visits()).powf(inv_temp)` — it never consults child_proven_value() or child_score_bounds(). The temp==0 path goes through principal_variation(1) -> select_child_after_
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Code claim verified. src/lib.rs:684-705 select_move_by_temperature filters only on visits()>0 and weights by (visits)^(1/temp); it never consults child_proven_value() or child_score_bounds(). The temp==0 branch (best_move L677-678 -> principal_variation(1) -> select_child_after_search, L173-206) does prefer proven-win/draw and best score-bound, so the two branches are genuinely inconsistent. So th

### F61 · [LOW] · correctness · `treant-wasm/src/difficulty.rs:31-47` · arcade=yes
**pick_weak's two documented safety nets (protect a proven win / take an immediate win) are dead code for every non-solver game**

_also reported as: pick_weak's 'child proven Loss == an immediate win' assumption is only valid in 2-player negamax_

- **Why:** Proven values are only ever stored when `solver_enabled()` returns true (src/search_tree.rs:298 `if solver_enabled { ... node.proven.store(...) }`). The majority of arcade games — Connect6, Mancala, TicTacToe, ConnectFour, Reversi, Pig, Dice, Hex(no), etc. — do NOT enable the solver, so `root_proven_value()` is always `Unknown` and every child's `proven_value` is always `Unknown`. For those games both guards are inert: pick_weak can and will sample a losing move off a winning position at high temp, contradicting the module doc's promise that it 'always takes a proven win' and 'never throws a game the search has proven won'. The safety only functions for the ~22 solver-enabled games, and even there the second guard (line 45) is redundant because a proven-Loss child immediately propagates the root to proven Win (search_tree.rs:367-372), caught by line 32 first.
- **Fix:** Either (a) document precisely that the win-protection only applies to solver-enabled games, or (b) for the 'take an immediate win' case, fall back to a terminal-value probe of root children (clone state + make_move + terminal_value()==Loss) so it works without the solver, or (c) enable the solver on the cheap perfect-information games that currently omit it. At minimum, fix the module-level doc so it does not overclaim a guarantee that holds for a minority of games.
- **Evidence:** // Protect a forced win: never throw a game the search has proven won.
    if matches!(manager.root_proven_value(), ProvenValue::Win) {
        return manager.best_move().map(|m| format!("{m}"));
    }
    ...
    // A child proven Loss (for the opponent) is an immediate winning move — take it.
    if let Some(win) = cand.iter().find(|s| s.proven_value == ProvenValue::Loss) {
        return Some(format!("{}", win.mov));
    }
- **Skeptic 1** (confirmed/LOW/arcade-path, tested=False): Read treant-wasm/src/difficulty.rs:1-70 — the two guards are exactly as cited (line 32 root_proven_value()==Win; line 45 child proven_value==Loss). Confirmed at src/search_tree.rs:298 that node.proven is stored ONLY inside `if solver_enabled { ... }`, and src/lib.rs:240 confirms `solver_enabled()` defaults to false. So for any game not enabling the solver, root_proven_value() is always Unknown and
- **Skeptic 2** (uncertain/LOW/feature-gated, tested=False): Read difficulty.rs:1-71 and search_tree.rs:285-384. The core mechanism is CONFIRMED: proven values are only stored when solver_enabled (search_tree.rs:298), and the trait default is false (lib.rs:240). So for solver-disabled games both pick_weak guards (difficulty.rs:32 root_proven_value==Win, and :45 child proven_value==Loss) are inert, and high-temp sampling can throw a winning position. No test

### F71 · [LOW] · correctness · `treant-dynamic/src/adapter.rs:82-86` · arcade=no
**Non-empty but wrong-length prior vec silently drops moves from the tree**

- **Why:** Uniform fallback only triggers when priors is exactly empty. If a host returns a non-empty priors vec shorter than moves (e.g. it scored only some moves, or an off-by-one), core's create_node zips moves.into_iter().zip(move_eval) (src/search_tree.rs:267-271), which truncates to the shorter length. The unscored moves are dropped from the node's move list entirely — they become unreachable in search, silently degrading move quality with no error. A longer priors vec is likewise truncated, dropping the validated sum invariant per-move.
- **Fix:** Validate priors.len() == moves.len() in the adapter; on mismatch, either fall back to uniform or pad/truncate explicitly with a documented rule. Surface the mismatch rather than letting zip swallow it.
- **Evidence:** if priors.is_empty() && !moves.is_empty() {
    let uniform = 1.0 / moves.len() as f64;
    priors = vec![uniform; moves.len()];
}
- **Skeptic 1** (uncertain/LOW/library-only, tested=False): Code matches the citation. adapter.rs:82-86 does gate the uniform fallback on priors.is_empty() exactly, and search_tree.rs:267-271 zips moves with move_eval, truncating to the shorter length — both as claimed. So a non-empty, wrong-length priors vec is not normalized and can drop moves via zip.

BUT the finding omits a real guard: search_tree.rs:266 calls policy.validate_evaluations(&move_eval) B
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Code claim verified. adapter.rs:83-86 only generates uniform priors when `priors.is_empty()`; a non-empty wrong-length vec is passed through to core's create_node (src/search_tree.rs:267-271) where `moves.into_iter().zip(move_eval)` truncates to the shorter length, silently dropping moves (if priors shorter) or extra priors (if longer). Confirmed no upstream guard catches this: AlphaGoPolicy::vali

### F15 · [LOW] · docs · `src/transposition_table.rs:190, 201, 229` · arcade=no
**Quadratic-probe key/value publication relies on Relaxed key ordering; node contents synchronized only via value pointer**

_also reported as: TranspositionTable::insert/lookup probing failure (PROBE_LIMIT exhaustion returns None) is undocumented behaviour_

- **Why:** All key loads/CAS use Relaxed. This is sound because the SearchNode payload behind the value pointer is only ever dereferenced after an Acquire load of `entry.v` (lines 192, 231) paired with the Release store in get_or_write (line 153). But this is a non-obvious invariant: a future change that reads node fields based purely on the key (without going through the Acquire value load) would be a data race. There is no comment capturing this safety argument, and the unsafe `get_unchecked` (lines 189, 228) plus raw `&*value_here` (line 194) make the file's soundness wholly dependent on it.
- **Fix:** Document the memory-ordering contract: keys are Relaxed and carry no payload; the AtomicPtr value with Release(store)/Acquire(load) is the sole synchronization edge for node contents. Anyone reading the node MUST go through an Acquire load of entry.v.
- **Evidence:** let key_here = entry.k.load(Ordering::Relaxed);
...
.compare_exchange(0, my_hash, Ordering::Relaxed, Ordering::Relaxed)
...
let key_here = entry.k.load(Ordering::Relaxed);
- **Skeptic 1** (confirmed/LOW/multithread-only, tested=False): Read src/transposition_table.rs lines 1-242. The cited code matches exactly: key loads/CAS at lines 190, 201, 229 are all Ordering::Relaxed; value loads at 192 and 231 are Acquire; get_or_write (line 149-157) stores the value pointer with Release (line 153) on CAS success. The finding's technical characterization is correct and is NOT a bug report — it explicitly states the code "is sound." Verifi
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read src/transposition_table.rs lines 90-241. All three cited lines are verbatim: key load is Relaxed (190, 229), key CAS is Relaxed/Relaxed (201), and node payload is only ever dereferenced after an Acquire load of entry.v (192, 231) paired with the Release CAS in get_or_write (153). Entry16 (line 90) holds only k: AtomicU64 (no payload) and v: AtomicPtr<V>, so the Relaxed key is a pure probe-slo

### F27 · [LOW] · docs · `src/tree_policy.rs:89-92, 111-131, 263-267` · arcade=yes
**AlphaGo/PUCT reciprocal makes the real selection formula differ from the documented one (1/n vs 1/(1+n), and hardcoded 2.0 for n=0)**

- **Why:** sum_rewards is the SUM of rewards (not the mean Q), and reciprocal(n) = 1/n for n>=1 and 2.0 for n=0. So the implemented score is (sum + C*P*sqrt(N))/n for n>=1, i.e. mean Q + C*P*sqrt(N)/n, and 2*(C*P*sqrt(N)) for n=0. The documented `(Q + C*P*sqrt(N))/(1+n)` matches neither: for n=0 the doc gives /1 while code multiplies by 2, and for n>=1 the doc divides by (1+n) while code divides by n. The magic 2.0 for unvisited children is an undocumented exploration boost.
- **Fix:** Rewrite the rustdoc to state the actual formula: score = (sum_rewards + C*P*sqrt(N)) * recip(n) where recip(0)=2, recip(n)=1/n. Document the n=0 boost rationale, or switch reciprocal to 1/(1+n) to match the standard AlphaZero PUCT denominator if that was the intent.
- **Evidence:** Doc (L90-92): `(Q(a) + C * P(a) * sqrt(N)) / (1 + n(a))`. Table init (L112): `|x| if x == 0 { 2.0 } else { 1.0 / x as f64 }`; reciprocal (L125-131) returns 1.0/x for x>=1. Use (L265-266): `(sum_rewards + explore_coef * policy_evaln) * self.reciprocal(child_visits as usize)`
- **Skeptic 1** (confirmed/LOW/arcade-path, tested=False): Read src/tree_policy.rs:89-92 (doc), 111-131 (table+reciprocal), 259-267 (use), plus src/lib.rs:158 (default fpu) and treant-dynamic/src/{adapter.rs:145,types.rs:52} (arcade fpu default).

Doc L91 states: (Q + C*P*sqrt(N)) / (1 + n). Actual code at L265-266: (sum_rewards + explore_coef*P) * reciprocal(n), where explore_coef = C*sqrt(total_visits+1) and reciprocal(n)=1/n for n>=1, 2.0 for n=0 (L112
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified against source. tree_policy.rs:91 documents PUCT as `(Q(a) + C*P(a)*sqrt(N)) / (1 + n(a))`. Implementation (L259-267): for an unvisited child the branch `child_visits == 0 && fpu.is_finite()` decides; since the DEFAULT fpu_value() is f64::INFINITY (lib.rs:158-160), `is_finite()` is false, so n=0 children fall into the else branch and compute `(0 + C*sqrt(N)*P) * reciprocal(0)` = `2.0 * C*

### F43 · [LOW] · docs · `src/lib.rs:336-341` · arcade=no
**ProvenValue enum variants lack rustdoc despite a load-bearing #[repr(u8)] discriminant ABI contract**

- **Why:** Each variant is an undocumented public item (rustdoc -W missing_docs flags all four at lib.rs:337-340). More importantly, the explicit discriminants (0/1/2/3) are a load-bearing invariant: `ProvenValue::from_u8` (lib.rs:345) round-trips through these exact numbers and they are almost certainly relied on by the atomic u8 storage in search_tree and by cross-language golden tests. Nothing documents that these discriminant values are stable/part of the public ABI, so a future contributor could renumber them and silently break `from_u8`, the solver's atomic encoding, and any language binding that hardcodes them. The project convention is that ALL public items have rustdoc.
- **Fix:** Add a doc comment to each variant, and add a sentence on the enum (or near from_u8) stating that the numeric discriminants are a stable part of the public/FFI contract and must not be renumbered (Unknown MUST stay 0 so Default-zeroed atomic storage decodes as Unknown).
- **Evidence:** #[repr(u8)]
pub enum ProvenValue {
    Unknown = 0,
    Win = 1,
    Loss = 2,
    Draw = 3,
}
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/lib.rs:332-353 directly. The four ProvenValue variants (Unknown=0, Win=1, Loss=2, Draw=3) genuinely lack rustdoc while the enum, from_u8, and neighboring ScoreBounds items are documented; `rustdoc -W missing_docs` flags all four at lib.rs:337-340 exactly. The discriminant claim is substantiated: search_tree.rs:71/:255 do AtomicU8::new(ProvenValue::Unknown as u8) (relying on Unknown==0 for
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified at src/lib.rs:336-341: the four ProvenValue variants (Unknown=0, Win=1, Loss=2, Draw=3) genuinely have no rustdoc, while the enum itself, from_u8, and ScoreBounds all do — so this is a real violation of the project's "all public items have rustdoc" convention. The #[repr(u8)] discriminant contract is real and load-bearing: search_tree.rs uses `ProvenValue::Unknown as u8` for atomic init (

### F44 · [LOW] · docs · `src/lib.rs:358-362` · arcade=no
**ScoreBounds public fields `lower`/`upper` undocumented**

- **Why:** Both public fields are flagged by rustdoc -W missing_docs (lib.rs:360,361). They carry a non-obvious invariant: `i32::MIN` means 'unbounded below' and `i32::MAX` means 'unbounded above' (sentinels handled by negate_bound), and the values are from the current player's negamax perspective. A consumer reading these fields directly has no in-line documentation of the sentinel meaning.
- **Fix:** Document each field, noting the i32::MIN/i32::MAX sentinels and the current-player perspective.
- **Evidence:** pub struct ScoreBounds {
    pub lower: i32,
    pub upper: i32,
}
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/lib.rs:355-380. The literal claim is accurate: ScoreBounds.lower (line 360) and ScoreBounds.upper (line 361) are pub fields with no per-field /// doc comments. However the finding overstates the impact in two ways. (1) The "rustdoc -W missing_docs warns" claim is not substantiated by the crate config — grep shows no `#![warn(missing_docs)]`/`deny(missing_docs)` anywhere in src/ or Cargo.t
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read src/lib.rs:355-384 directly. Confirmed: pub fields `lower` (line 360) and `upper` (line 361) have no per-field doc comments. However the finding overstates impact in two ways. (1) The struct-level doc (lines 355-357) already states the fields are `[lower, upper]` bounds on the true minimax value "from the current player's perspective" — so the perspective claim is documented at struct level, 

### F45 · [LOW] · docs · `src/lib.rs:143-148` · arcade=no
**MCTS trait associated types are entirely undocumented**

- **Why:** `MCTS` is THE central configuration trait a user must implement, yet none of its six associated types has a doc comment (all flagged at lib.rs:143-148). `NodeData` and `ExtraThreadData` in particular are non-obvious — a new implementer cannot tell from the signature what `NodeData` is for (custom per-node user data) or that `ExtraThreadData` is thread-local scratch space. This is the highest-traffic doc gap in the slice and directly contradicts the 'all public items have rustdoc' convention.
- **Fix:** Add a one-line rustdoc to each associated type (State = game rules, Eval = leaf evaluator, TreePolicy = child selection, NodeData = user data stored per tree node, TranspositionTable = node-sharing table or () to disable, ExtraThreadData = per-thread scratch passed via ThreadData).
- **Evidence:** type State: GameState + Send + Sync + 'static;
    type Eval: Evaluator<Self> + Send + 'static;
    type TreePolicy: TreePolicy<Self> + Send + 'static;
    type NodeData: Default + Sync + Send + 'static;
    type TranspositionTable: TranspositionTable<Self> + Send + 'static;
    type ExtraThreadData: 'static;
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/lib.rs:140-180. The cited lines 143-148 contain exactly the six associated types claimed (State, Eval, TreePolicy, NodeData, TranspositionTable, ExtraThreadData), and none has a rustdoc comment. By contrast, every method on the same trait (virtual_loss, fpu_value, visits_before_expansion, node_limit, select_child_after_search, etc. at lines 150+) carries a doc comment. The trait `MCTS` is
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified at src/lib.rs:142-148: the `MCTS` trait and its methods (virtual_loss, fpu_value, etc.) carry rustdoc, but all six associated types (State, Eval, TreePolicy, NodeData, TranspositionTable, ExtraThreadData) have no doc comments. The claim is factually correct.

No test covers this. grep of tests/ for doc/rustdoc/missing_docs found nothing, and there is no `#![deny(missing_docs)]`/`#![warn(m

### F47 · [LOW] · docs · `src/lib.rs:274-277` · arcade=no
**ThreadData public fields undocumented**

- **Why:** Both public fields (lib.rs:275,276) are flagged by missing_docs. `ThreadData` is handed to user code via `SearchHandle::thread_data()`, so `extra_data` (the user's `ExtraThreadData`) and `policy_data` are part of the public surface user code reads/writes.
- **Fix:** Document policy_data (tree-policy thread-local state, e.g. tie-break RNG) and extra_data (user-provided per-thread scratch).
- **Evidence:** pub struct ThreadData<Spec: MCTS> {
    pub policy_data: TreePolicyThreadData<Spec>,
    pub extra_data: Spec::ExtraThreadData,
}
- **Skeptic 1** (uncertain/LOW/library-only, tested=False): Read src/lib.rs:273-277. The struct exists exactly as cited; `policy_data` (275) and `extra_data` (276) are public and genuinely lack doc comments, while the struct itself is documented (273). ThreadData IS part of the public surface: SearchHandle::thread_data() at src/search_tree.rs:1330 returns `&mut ThreadData<Spec>`, so user code reads/writes both fields. So the raw observation (two undocument
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read src/lib.rs:273-277 directly. The struct matches the cited evidence exactly: `pub struct ThreadData<Spec: MCTS>` with two public, undocumented fields `policy_data: TreePolicyThreadData<Spec>` (line 275) and `extra_data: Spec::ExtraThreadData` (line 276). The struct itself has a doc comment (line 273); only the fields lack docs. The fields are genuinely part of the public surface, reachable fro

### F48 · [LOW] · docs · `src/lib.rs:121-124` · arcade=no
**pub mod batch / transposition_table / tree_policy lack module-level docs**

- **Why:** All three public modules (batch, transposition_table, tree_policy) are flagged by rustdoc -W missing_docs at the module declaration (lib.rs:121,123,124). They appear as top-level entries in the generated API docs with no summary, leaving the docs.rs landing page modules undescribed.
- **Fix:** Add a `//!`-style module doc (or `/// ...` on the `pub mod`) summarizing each module's purpose.
- **Evidence:** pub mod batch;
mod search_tree;
pub mod transposition_table;
pub mod tree_policy;
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Verified src/lib.rs:120-128. Lines 121/123/124 are exactly `pub mod batch;`, `pub mod transposition_table;`, `pub mod tree_policy;` with no `///` doc comment on the declarations. Confirmed the module files themselves lack a leading `//!` module-level doc: batch.rs starts with `use crate::*;`, transposition_table.rs with `use super::*;`, tree_policy.rs with `use rand::...`. All three are genuinely 
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read src/lib.rs:119-128 directly: `pub mod batch;` (121), `pub mod transposition_table;` (123), `pub mod tree_policy;` (124) all lack any `///` or `//!` doc comment, and the module files (src/batch.rs, src/transposition_table.rs, src/tree_policy.rs) begin directly with `use` statements — no module-level docs. So the factual claim is accurate; these public modules render undescribed on docs.rs. Thi

### F49 · [LOW] · docs · `src/lib.rs:516, 530, 720, 792, 793, 818, 822` · arcade=no
**Several public MCTSManager / AsyncSearch methods have no rustdoc**

- **Why:** These are all undocumented public API methods flagged by missing_docs. `AsyncSearch::halt(self){}` (line 792) is an especially user-hostile foot-gun: it is an empty body whose only effect is to drop `self` (and thus stop search via Drop). A caller reading the name expects `halt()` to do something explicit; nothing documents that the stop happens via Drop and that the borrowed variant cannot return the manager (unlike AsyncSearchOwned::halt which does). `playout_until` (530) takes a predicate with no doc on when it is evaluated (before each playout).
- **Fix:** Document each. For AsyncSearch::halt specifically, state that it consumes the handle and stops search on drop, and contrast with AsyncSearchOwned::halt which recovers the manager.
- **Evidence:** pub fn print_on_playout_error(&mut self, v: bool) -> &mut Self {   // 516
    pub fn playout_until<Predicate: FnMut() -> bool>(&mut self, mut pred: Predicate) {  // 530
    pub fn perf_test_to_stderr(&mut self, num_threads: usize) {  // 720
    pub fn halt(self) {}  // 792 (AsyncSearch)
    pub fn num_threads(&self) -> usize {  // 793
    pub fn halt(mut self) -> MCTSManager<Spec> {  // 818 (AsyncSearchOwned)
    pub fn num_threads(&self) -> usize {  // 822
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/lib.rs around all cited lines. All seven methods are confirmed public (`pub fn`) on public types and lack rustdoc: print_on_playout_error (516), playout_until (530), perf_test_to_stderr (720), AsyncSearch::halt (792), AsyncSearch::num_threads (793), AsyncSearchOwned::halt (818), AsyncSearchOwned::num_threads (822). MCTSManager (481), AsyncSearch (785), and AsyncSearchOwned (807) are all p
- **Skeptic 2** (confirmed/LOW/library-only, tested=True): Read src/lib.rs at all cited lines (516, 530, 720, 792-795, 818, 822). All seven methods exist exactly as quoted and genuinely lack method-level rustdoc, so the core claim (undocumented public API methods) is CONFIRMED and accurate.

However the finding is overstated on three points:

1. "flagged by missing_docs" is FALSE. There is no #![deny(missing_docs)] or #![warn(missing_docs)] anywhere in th

### F50 · [LOW] · docs · `src/batch.rs:16, 114` · arcade=no
**BatchEvaluator::StateEvaluation and BatchedEvaluatorBridge::new undocumented**

- **Why:** `BatchEvaluator::StateEvaluation` (batch.rs:16) and the primary constructor `BatchedEvaluatorBridge::new` (batch.rs:114) are flagged by missing_docs. `new` is the entry point named in the struct's own example yet has no rustdoc describing that it spawns a background collector thread that lives until Drop.
- **Fix:** Document the associated type and add rustdoc to `new` noting it spawns a dedicated collector thread joined on Drop.
- **Evidence:** pub trait BatchEvaluator<Spec: MCTS>: Send + Sync + 'static {
    type StateEvaluation: Sync + Send + Clone;   // line 16
...
    pub fn new(batch_eval: B, config: BatchConfig) -> Self {   // line 114
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/batch.rs directly. Both citations are accurate: line 16 `type StateEvaluation: Sync + Send + Clone;` has no doc comment, and line 114 `pub fn new(batch_eval: B, config: BatchConfig) -> Self` has no rustdoc despite being named in the BatchedEvaluatorBridge struct example (batch.rs:85,91). `new` does spawn a background collector thread (std::thread::spawn at line 119) stored in eval_thread,
- **Skeptic 2** (uncertain/LOW/library-only, tested=False): Read src/batch.rs directly. The literal facts hold: line 16 `type StateEvaluation: Sync + Send + Clone;` has no doc comment, and line 114 `pub fn new(batch_eval: B, config: BatchConfig) -> Self` has no rustdoc. The constructor does spawn a dedicated collector thread (std::thread::spawn at line 119, collector_loop) that is joined on Drop (Drop impl at ~line 185: takes sender to close channel, then 

### F51 · [LOW] · docs · `src/lib.rs:217-221` · arcade=no
**rng_seed() doc is scoped too narrowly: omits the selection RNG and chance RNG offsets**

- **Why:** The doc claims the seeding scheme is simply `seed + thread_id`, but there are three distinct RNG streams with three different offsets, none disclosed here: (1) the per-thread tree-policy RNG uses `seed + thread_id` (search_tree.rs:588), (2) the per-thread chance RNG uses `seed + thread_id + 0xCAFE_BABE` (search_tree.rs:592), and (3) the manager-level selection_rng used by best_move()/temperature sampling uses `seed.wrapping_add(u64::MAX/2)` (lib.rs:503 and 731). A user reading only this doc cannot reproduce best_move() temperature selection, and the doc gives a false impression that thread_id is the only offset. (The docs/ site DOES document the 0xCAFE_BABE offset, so the rustdoc is the lagging copy.)
- **Fix:** Expand the rustdoc to note that there are separate deterministic streams: per-thread policy (seed+thread_id), per-thread chance (offset by 0xCAFE_BABE), and the manager's selection/temperature RNG (offset by u64::MAX/2), all derived from this seed.
- **Evidence:** /// Optional RNG seed for deterministic search. When set, each thread gets
    /// a reproducible RNG seeded from `seed + thread_id`.
    fn rng_seed(&self) -> Option<u64> {
        None
    }
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): All four cited locations verified against source. lib.rs:217-221 rustdoc for rng_seed() states "each thread gets a reproducible RNG seeded from seed + thread_id" — accurate but only describes one of three derived streams. search_tree.rs:588 confirms per-thread policy RNG = base_seed.wrapping_add(thread_id). search_tree.rs:592 confirms chance RNG = seed.wrapping_add(0xCAFE_BABE). lib.rs:503 (MCTSMa
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): All cited code verified verbatim. src/lib.rs:217-218 rustdoc says only "seed + thread_id". The three offsets exist exactly as claimed: policy RNG = base_seed.wrapping_add(thread_id) (search_tree.rs:588); chance RNG = seed.wrapping_add(0xCAFE_BABE) (search_tree.rs:592); selection_rng = seed.wrapping_add(u64::MAX/2) (lib.rs:503 and the reset path at lib.rs:731). So the rustdoc genuinely omits two of

### F52 · [LOW] · docs · `src/lib.rs:493-505` · arcade=no
**MCTSManager::new rustdoc omits that the seed sets the temperature-selection RNG with a fixed offset**

- **Why:** The constructor reads `manager.rng_seed()` and derives the selection RNG (used by best_move() temperature sampling) with the undocumented `+ u64::MAX/2` offset. The same magic offset is duplicated in `reset()` (lib.rs:731). This implicit contract — that seeding makes temperature-based best_move() reproducible too — is documented nowhere in the rustdoc, and the duplicated magic constant is a refactor hazard (change one, forget the other → reset() and new() diverge).
- **Fix:** Note in the `new` rustdoc that when rng_seed() is Some, temperature move-selection is also reproducible. Extract the seed-offset derivation into a single private helper used by both new() and reset() to remove the duplicated magic constant.
- **Evidence:** /// Create a new search manager with the given game state, config, evaluator,
    /// tree policy, and transposition table.
    pub fn new(...) -> Self {
        let selection_rng = match manager.rng_seed() {
            Some(seed) => Rng64::seed_from_u64(seed.wrapping_add(u64::MAX / 2)),
            None => Rng64::from_rng(rand::thread_rng()).unwrap(),
        };
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/lib.rs:493-514 (new) and 725-740 (reset). The finding is accurate. new() at line 502-505 derives selection_rng via `Rng64::seed_from_u64(seed.wrapping_add(u64::MAX / 2))` from manager.rng_seed(); reset() at line 730-733 contains the identical magic-offset expression — a true duplicated constant / refactor hazard. selection_rng is consumed at line 697 by best_move temperature sampling (`se
- **Skeptic 2** (confirmed/LOW/library-only, tested=True): Code confirmed verbatim: src/lib.rs:502-505 (new()) and 730-733 (reset()) both derive selection_rng via `seed.wrapping_add(u64::MAX / 2)` — the magic offset is genuinely duplicated. The new() method rustdoc (493-494) does NOT mention that seeding makes temperature-based best_move() reproducible; the struct-level rustdoc (478-480) only notes the RefCell<Rng64> exists for temperature selection, not 

### F55 · [LOW] · docs · `src/search_tree.rs:129-158` · arcade=no
**MoveInfo accessor 'Acquire-load-before-deref' child invariant is undocumented at the public API boundary**

- **Why:** These public accessors (child/child_proven_value/child_score_bounds), exposed via MoveInfoHandle (the type the tree policy and best-child code in lib.rs select_child_after_search operate on), all rely on an Acquire load of the child pointer to safely deref node data published by another thread's Release store (the lock-free publication protocol at search_tree.rs:789). None of these public methods documents that the Acquire ordering is load-bearing for memory safety / data-race freedom, nor that the proven_value/score_bounds they return are best-effort snapshots that can change concurrently. A maintainer could downgrade to Relaxed and introduce UB. The slice's stated hunt list explicitly calls out the 'Acquire-load-before-deref invariant' as a key undocumented invariant. (search_tree.rs is cross-check, not a primary slice file, but these accessors are the contract surface the slice's tree_policy.rs / lib.rs code consumes.)
- **Fix:** Add rustdoc to these accessors noting (a) the returned proven_value/score_bounds are concurrency snapshots that may be stale, and (b) a module-level note that the Acquire load pairs with the Release store that publishes child nodes and must not be weakened.
- **Evidence:** pub fn child(&self) -> Option<NodeHandle<'_, Spec>> {
        let ptr = self.child.load(Ordering::Acquire);   // 130
...
    pub fn child_proven_value(&self) -> ProvenValue {
        let ptr = self.child.load(Ordering::Acquire);   // 142
...
    pub fn child_score_bounds(&self) -> ScoreBounds {
        let ptr = self.child.load(Ordering::Acquire);   // 154
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Verified against src/search_tree.rs. The cited code matches exactly: child() (line 130), child_proven_value() (line 142), and child_score_bounds() (line 154) each perform self.child.load(Ordering::Acquire) followed by an unsafe deref of the resulting raw pointer. The pairing Release store is real: line 789 publishes a freshly-created child via choice.child.compare_exchange(null_mut(), created, Ord
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Factually accurate. Read src/search_tree.rs:128-160: child() (130), child_proven_value() (142), child_score_bounds() (154) all use Ordering::Acquire loads on the child AtomicPtr. These pair with the Release publication at line 789 (compare_exchange success ordering) and the store at 807. The doc comments on these accessors describe return semantics (null→None/Unknown/UNBOUNDED, mover perspective) 

### F57 · [LOW] · docs · `src/lib.rs:169-206` · arcade=no
**select_child_after_search Panics doc is correct but the score-bounded tie-break path is undocumented and subtle**

- **Why:** The rustdoc only describes the default (max-visits) behaviour and the empty-slice panic, but the body implements three layered selection regimes depending on solver_enabled()/score_bounded_enabled(): (1) prefer proven-Loss (=parent win) then proven-Draw children, (2) else pick max guaranteed score via negate_bound(child.upper), (3) else max visits. A user overriding this method, or reasoning about why best_move() sometimes ignores visit counts, has no doc explaining that solver/score-bounded modes change the post-search choice. This is an implicit behavioural contract (the negamax 'child Loss = parent win' sign convention) documented only inline as comments.
- **Fix:** Expand the rustdoc to describe the precedence: proven win/draw children (solver), then best guaranteed lower bound (score-bounded), then most-visited; and note the perspective negation.
- **Evidence:** /// Select the best child after search completes. Override for custom post-search selection.
    ///
    /// # Panics
    /// Panics if `children` is empty. Only call on non-terminal nodes.
    fn select_child_after_search<'a>(&self, children: &'a [MoveInfo<Self>]) -> &'a MoveInfo<Self> {
- **Skeptic 1** (confirmed/LOW/feature-gated, tested=False): Read src/lib.rs:169-206. The code matches the finding exactly. The rustdoc (lines 169-172) only states "Select the best child after search completes" plus the empty-slice panic, while the body (lines 174-205) implements three layered selection regimes: (1) solver_enabled() prefers proven-Loss then proven-Draw children (lines 174-189); (2) score_bounded_enabled() picks max guaranteed lower bound vi
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified src/lib.rs:169-206. The rustdoc on select_child_after_search documents only "best child" semantics and the empty-slice panic, but the body implements three layered selection regimes exactly as the finding describes: (1) under solver_enabled(), prefer a child whose child_proven_value()==Loss (child Loss = parent win), then a proven Draw; (2) under score_bounded_enabled(), pick max negate_b

### F58 · [LOW] · docs · `src/transposition_table.rs:45-48, 213-219` · arcade=no
**ApproxTable::clear overrides the no-op trait default but neither the override nor the re-rooting requirement is documented on the impl**

- **Why:** The trait default `clear()` is a no-op (transposition_table.rs:47). The doc says clear 'is called during tree re-rooting to prevent dangling pointers' — meaning a custom TranspositionTable that holds real SearchNode pointers and does NOT override clear() will retain dangling pointers across advance_root/reset, a memory-safety-adjacent foot-gun. The trait is already `unsafe`, but this specific obligation (you MUST override clear() if you cache node pointers, or advancing the root yields dangling references) is stated only as a passive 'Called during...' note, not as an implementer requirement in the trait's # Safety contract.
- **Fix:** Add to the trait-level # Safety doc that any implementation caching SearchNode references MUST implement clear() to drop them, since the search re-roots by clearing the table; tie it to the existing unsafe contract.
- **Evidence:** /// Clear all entries from the table.
    /// Called during tree re-rooting to prevent dangling pointers.
    fn clear(&mut self) {}
...
    fn clear(&mut self) {   // ApproxTable impl, line 213
        for entry in self.arr.iter_mut() {
            *entry.k.get_mut() = 0;
            *entry.v.get_mut() = std::ptr::null_mut();
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Citations verified. src/transposition_table.rs:45-48 — trait default clear() is a no-op with only a passive "/// Called during tree re-rooting to prevent dangling pointers" note. The trait's # Safety block (lines 5-7) covers ONLY the insert double-free contract, NOT the clear() obligation, so the finding's core claim (the re-rooting requirement is undocumented as an implementer requirement) is acc
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified against source. The cited code is accurate: the trait default clear() at src/transposition_table.rs:47 is a no-op {}, the doc comment at lines 45-46 says only "Called during tree re-rooting to prevent dangling pointers," and ApproxTable overrides it at lines 213-219 (zeroing keys and nulling value pointers). The mechanism is real and confirmed in src/search_tree.rs:1239-1243: advance_root

### F60 · [LOW] · docs · `src/lib.rs:706-719` · arcade=no
**perf_test rustdoc says '10-second' but the duration is hardcoded and not parameterized**

- **Why:** Minor but accurate: the doc's '10-second' is correct (10 iterations × 1s sleep) but brittle — `perf_test_to_stderr` (line 720) is undocumented and just wraps this. The pair is a public benchmarking API where the fixed 10s is baked in with no parameter; callers cannot shorten it. Low severity, noted for completeness as the duration contract is implicit.
- **Fix:** Note in the doc that the duration is fixed at 10 seconds (not configurable), and document perf_test_to_stderr.
- **Evidence:** /// Run a 10-second performance benchmark, calling `f` with nodes/sec each second.
    pub fn perf_test<F>(&mut self, num_threads: usize, mut f: F)
...
        for _ in 0..10 {
            ...
            std::thread::sleep(Duration::from_secs(1));
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/lib.rs:706-724. The factual claims are accurate: line 706 doc says "Run a 10-second performance benchmark"; the loop at line 712 is `for _ in 0..10` with `std::thread::sleep(Duration::from_secs(1))` at line 714 — so the doc's "10-second" is correct (10 iterations x 1s). The duration is indeed hardcoded with no parameter; callers cannot shorten or lengthen it. `perf_test_to_stderr` (line 7
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read src/lib.rs:706-724 directly. Facts confirmed: doc on perf_test (line 706) says "10-second"; loop is `for _ in 0..10` with `Duration::from_secs(1)` sleeps (lines 712-718), so 10s is hardcoded and not parameterized. `perf_test_to_stderr` (line 720) has no rustdoc comment in source and just wraps perf_test — this does violate the repo convention "All public API items have rustdoc comments" (CLAU

### F3 · [LOW] · perf · `src/search_tree.rs:808-811, 1245-1246` · arcade=no
**Orphaned nodes accumulate unbounded for the tree's lifetime (deferred-free never drained mid-search)**

_also reported as: Delayed-transposition orphaned nodes accumulate for the tree's lifetime instead of being reclaimed_

- **Why:** These Boxes cannot be freed immediately because another thread may have loaded the briefly-published `created` pointer (CAS at 789) before the redirect store at 807, so the node must outlive in-flight readers. But with no epoch/quiescence reclamation they are pinned until advance_root or tree drop, an effective memory leak proportional to transposition contention during a single search.
- **Fix:** Add a quiescent reclamation point (e.g. drain orphaned when no playouts are in flight, gated by thread_counter/sentinel), or document the growth and the fact that diagnose() exposes the count so callers can advance_root to reclaim.
- **Evidence:** On every delayed-transposition redirect, descend does `self.orphaned.lock().unwrap().push(unsafe { Box::from_raw(created) });` (808-811). The only drain is `self.orphaned.lock().unwrap().clear();` in advance_root (1246). A long `playout_n`/`playout_n_parallel` that never advances the root therefore grows `orphaned` without bound, holding one heap `SearchNode` Box alive per delayed-transposition contention event.
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Code reads exactly as cited. src/search_tree.rs:808-811 pushes the briefly-published `created` Box onto `self.orphaned` on the delayed-transposition redirect path (after CAS at 789, store at 807). The ONLY drains are advance_root at line 1246 (`self.orphaned.lock().unwrap().clear()`) and tree Drop. `orphaned` is declared once (line 29), pushed once (808), cleared once (1246); confirmed via grep. d
- **Skeptic 2** (uncertain/LOW/library-only, tested=False): Mechanics confirmed against source. src/search_tree.rs:808-811 pushes one Box<SearchNode> into `orphaned` on every delayed-transposition redirect; grep confirms this is the ONLY pusher. The only drain is the `.clear()` in advance_root at 1246. So within a single search that never advances the root, `orphaned` grows by one node per delayed-transposition contention event, with no mid-search reclamat

### F37 · [LOW] · perf · `src/batch.rs:142-163, 211-231` · arcade=no
**BatchedEvaluatorBridge serializes single-threaded search and adds max_wait latency per leaf**

- **Why:** With a single search thread (`MCTSManager::playout`/`playout_n`), only one request can ever be in flight, so every leaf evaluation waits the full `max_wait` (default 1ms) for a batch that can never fill, capping single-threaded search at ~1000 playouts/sec. With N search threads and `max_batch_size > N` the same partial-fill stall happens on the last partial batch. The docstring example uses `playout_n_parallel(10000, 4)` with `max_batch_size: 32`, which will frequently time out waiting for 32 that 4 threads cannot supply.
- **Fix:** Document that max_batch_size should be <= number of concurrent search threads, and/or fire the batch as soon as `batch_requests.len()` cannot grow (track in-flight request count) instead of always waiting for the deadline. At minimum warn that the bridge is pointless for single-threaded search.
- **Evidence:** `evaluate_new_state` does `sender.lock().unwrap().send(request)` then `response_rx.recv()` (blocks). The collector `recv()`s one request then loops `while batch_requests.len() < config.max_batch_size { ...recv_timeout(remaining)... }` until `max_wait` (default 1ms) elapses.
- **Skeptic 1** (confirmed/LOW/library-only, tested=True): Read src/batch.rs:142-163 and 211-231. The code matches the finding exactly. evaluate_new_state (142-163) creates a sync_channel(1), locks the Mutex<Sender>, sends the request, then blocks on response_rx.recv(). The collector_loop (211-231) does receiver.recv() for the first request, then computes deadline = now + max_wait (default 1ms, confirmed at line 61) and loops recv_timeout(remaining) until
- **Skeptic 2** (confirmed/LOW/library-only, tested=True): Read src/batch.rs:142-163 (evaluate_new_state: send under mutex then response_rx.recv() blocks) and 211-231 (collector_loop: recv() one request, then while batch_requests.len() < max_batch_size loop with recv_timeout until deadline = max_wait). The mechanism is exactly as described: with one request in flight, the collector waits the full max_wait for a batch that can never fill.

ALREADY TESTED: 

### F79 · [LOW] · perf · `treant-dynamic/src/adapter.rs:101-108` · arcade=yes
**evaluate_existing_state recomputes available_moves and re-runs full host evaluate purely to discard priors**

_also reported as: `DynEvaluator::evaluate_existing_state` re-allocates a `Vec<String>` of all moves on every open-loop chance re-evaluation_

- **Why:** This path (open-loop chance re-evaluation) builds the full move-string list and invokes the host's evaluate() — which may compute and allocate a priors vector — only to throw the priors away (bound to _). Across an FFI boundary this is the expensive call. For a neural-net host evaluator it re-runs inference every chance-node revisit. Combined with the perspective-labeling bug above, the simplest correct-and-fast option is often to not re-evaluate at all.
- **Fix:** If re-evaluation is needed, add a value-only host hook to avoid prior computation/allocation; otherwise return existing_evaln unchanged and document that open-loop chance nodes reuse the stored evaluation.
- **Evidence:** let move_strings: Vec<String> = state.0.available_moves().into_iter().collect();
let (_, value) = self.0.evaluate(&*state.0, &move_strings);
- **Skeptic 1** (confirmed/LOW/arcade-path, tested=False): Read adapter.rs:95-108 — code matches evidence verbatim: line 102 rebuilds the move-string list via state.0.available_moves(), line 103 calls the host-implemented EvalCallbacks::evaluate (callbacks.rs:61) and binds the priors with `let (_, value)`, discarding them. The value-only result is wrapped into DynStateEval.

Reachability confirmed: evaluate_existing_state is invoked at search_tree.rs:734 
- **Skeptic 2** (uncertain/LOW/library-only, tested=True): Cited code is accurate (treant-dynamic/src/adapter.rs:95-108): evaluate_existing_state rebuilds move_strings via state.0.available_moves() and calls self.0.evaluate(...) binding priors to `_`. The wasted-priors mechanism is real: EvalCallbacks has only one hook, `evaluate(&self, state, moves) -> (Vec<f64>, f64)` (callbacks.rs:61) with no value-only variant, so a host NN/rollout genuinely computes 

### F35 · [LOW] · refactor · `src/tree_policy.rs:152-187, 219-258` · arcade=no
**Solver + score-bounded pre-pass and select-by-key prelude duplicated across UCTPolicy, AlphaGoPolicy, and select_child_after_search**

- **Why:** This is correctness-critical negamax sign logic (negate child.upper for parent.lower, etc.). Three copies means a future fix to the pruning or sign convention must be applied in three places or the policies silently diverge from post-search selection. The Draw branch's `sum_rewards/visits` average is also duplicated verbatim.
- **Fix:** Extract a shared `fn proven_and_bounds_prefilter(mov, best_lower, solver, score_bounded) -> Option<f64>` (returns Some when the move's score is forced) and a `fn best_lower_bound(moves)` helper used by both policies and select_child_after_search.
- **Evidence:** Identical blocks in both policies: `let best_lower = if score_bounded { moves.clone().map(|m| negate_bound(m.child_score_bounds().upper)).max().unwrap_or(i32::MIN) } else { i32::MIN };` plus the per-move solver match (`Loss=>INFINITY, Win=>NEG_INFINITY, Draw=>avg, Unknown=>{}`) and bounds-pruning `if score_bounded && best_lower > i32::MIN { let parent_upper = negate_bound(mov.child_score_bounds().lower); if parent_upper < best_lower { return NEG_INFINITY; } }`. A third near-copy of the proven/bounds selection logic lives in `lib.rs::select_child_after_search` (lib.rs:173-206).
- **Skeptic 1** (confirmed/MEDIUM/feature-gated, tested=False): Read src/tree_policy.rs:152-198 (UCTPolicy) and :225-269 (AlphaGoPolicy): the solver/score-bounded pre-pass and select_by_key prelude are byte-for-byte identical between the two policies. The best_lower computation (negate_bound of child.upper, max, unwrap_or(i32::MIN)), the solver match (Loss=>INFINITY, Win=>NEG_INFINITY, Draw=>sum_rewards/visits average, Unknown=>{}), and the bounds-pruning bloc
- **Skeptic 2** (confirmed/LOW/library-only, tested=True): Read src/tree_policy.rs:152-199 (UCTPolicy) and 225-269 (AlphaGoPolicy), plus lib.rs:173-206. The duplication is real and verbatim: `diff` of lines 153-187 vs 225-258 shows the ONLY differences are inline comments — the `best_lower` pre-pass, the solver match (Loss=>INFINITY, Win=>NEG_INFINITY, Draw=>sum_rewards/visits avg, Unknown=>{}), and the bounds-pruning block (`negate_bound(mov.child_score_

### F65 · [LOW] · refactor · `treant-wasm/src/gridpack.rs:496-564` · arcade=yes
**Macro-vs-hand-written duplication: OrderChaosWasm reimplements the entire cell_game_wasm! surface by hand, diverging only in move parsing**

- **Why:** Roughly 60 lines duplicate the macro body verbatim purely to swap the `apply_move` parse logic and the UCT constant (1.5, hardcoded in four separate places: 509, 558, 562). Any future change to the shared surface (e.g. fixing the result() format, adding a method to GameHandle, changing get_stats) must be made in both the macro and this hand copy, and the hardcoded 1.5 can drift from the macro games' explicit `$c`. This is exactly the kind of drift the macro was meant to prevent.
- **Fix:** Extend cell_game_wasm! to accept the move type + a parse expression (or a small trait `CellMove: FromStr + Display`) so Order & Chaos goes through the macro, eliminating the hand copy and the repeated 1.5 literal.
- **Evidence:** // Order & Chaos has a (cell, symbol) move, so its own thin wrapper.
#[wasm_bindgen]
pub struct OrderChaosWasm { manager: MCTSManager<OcCfg>, cols: usize, rows: usize, }
... (full duplicate of cols/rows/playout_n/get_stats/get_board/current_player/is_terminal/result/best_move/weak_move/reset)
- **Skeptic 1** (confirmed/LOW/arcade-path, tested=False): Read treant-wasm/src/gridpack.rs:419-489 (the cell_game_wasm! macro) and 497-564 (OrderChaosWasm). OrderChaosWasm does hand-reimplement the full macro surface verbatim — struct + cols/rows/playout_n/get_stats/get_board/current_player/is_terminal/result/best_move/weak_move/apply_move/reset — diverging only in (1) apply_move parsing "cell,sym" into OcMove{cell,sym} vs the macro's plain u16, and (2) 
- **Skeptic 2** (confirmed/LOW/arcade-path, tested=True): Verified at treant-wasm/src/gridpack.rs. The macro cell_game_wasm! (lines 420-488, instantiated for Connect6/Squava/Notakto/SquareUp at 491-494 with explicit $c constants) and the hand-written OrderChaosWasm (497-564) are near-verbatim duplicates of the same WASM surface: cols/rows/playout_n/get_stats/get_board/current_player/is_terminal/result/best_move/weak_move/reset are identical, diverging on

### F7 · [LOW] · smell · `src/search_tree.rs:786-812` · arcade=no
**Delayed-transposition redirect discards stats already accrued into the briefly-published 'created' node**

- **Why:** Any visits/rewards other threads accumulated into `created` during that window are silently discarded when the edge is redirected to `existing`. Not a memory-safety bug (orphaned keeps `created` alive, no UAF), but it is wasted work and a small statistical bias whose likelihood scales with thread count and transposition rate.
- **Fix:** Insert into the transposition table BEFORE publishing the child pointer (check-then-publish), so the edge is only ever pointed at the canonical node and no stats are stranded; or fold created's stats into existing on redirect.
- **Evidence:** `created` is published into the tree via the winning CAS at 787-789 `compare_exchange(null_mut(), created, Release, Acquire)`. Only afterward does `table.insert` (799) possibly return an `existing`, at which point `choice.child.store(existing_ptr, Release)` (807) redirects the edge and `created` is moved to `orphaned`. Between 789 and 807, concurrent threads can load `created` (descend 752), push it on their path, and call `down`/`up` on its stats.
- **Skeptic 1** (confirmed/LOW/multithread-only, tested=False): Read src/search_tree.rs:745-817 (descend) and confirmed the cited sequence exactly: `created` is published via the winning CAS at lines 787-789 BEFORE `table.insert` at 799, and on a delayed-transposition hit the edge is redirected via `choice.child.store(existing_ptr, ...)` at 807 with `created` pushed to `orphaned` at 808-811. Verified the stat-accrual mechanism that makes stranding real: anothe
- **Skeptic 2** (confirmed/LOW/multithread-only, tested=False): Verified against src/search_tree.rs:786-816. The sequence is exactly as claimed: `created` is published via the winning CAS at 787-789 (`compare_exchange(null_mut(), created, Release, Acquire)`), and only afterward does `table.insert` (799) run; on a delayed-transposition hit it redirects the edge with `choice.child.store(existing_ptr, Release)` (807) and pushes `created` to `orphaned` (808-811). 

### F13 · [LOW] · smell · `src/batch.rs:250-252` · arcade=no
**collector_loop swallows response-send failures silently, hiding dropped/abandoned search threads**

- **Why:** If a search thread that issued a request has gone away (panicked, been cancelled), its `response_rx` is dropped and this `send` errors; the result is discarded with `let _ =`. That is correct for liveness, but it is completely silent — there is no counter/diagnostic, so an evaluator paying GPU cost for results nobody consumes is invisible. Given the bridge already panics aggressively on the search side (previous finding), the asymmetry (search side panics, collector side silently ignores) is a smell and a debuggability gap.
- **Fix:** Track a dropped-response counter or at least a debug-log on send failure so abandoned requests are observable; document the asymmetry.
- **Evidence:** for (request, result) in batch_requests.drain(..).zip(results.into_iter()) {
    let _ = request.response.send(result);
}
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/batch.rs:250-252 directly; the cited code is quoted exactly: `for (request, result) in batch_requests.drain(..).zip(results.into_iter()) { let _ = request.response.send(result); }`. The silent swallow of send failures is real — no counter, log, or metric exists in collector_loop (src/batch.rs:196-254). The claimed asymmetry is also real: the search side (evaluate_new_state, src/batch.rs:1
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read src/batch.rs:250-252 — code matches the finding verbatim: `for (request, result) in batch_requests.drain(..).zip(results.into_iter()) { let _ = request.response.send(result); }`. The asymmetry is real and confirmed: the search side (evaluate_new_state, lines 154-162) panics aggressively via three .expect() calls ("bridge already shut down", "batch collector thread died", "batch collector drop

### F23 · [LOW] · smell · `src/search_tree.rs:856-897` · arcade=no
**propagate_proven early-break can leave a provable ancestor unproven within a single playout**

- **Why:** Propagation walks only the current playout path and breaks as soon as the on-path child is Unknown. A parent could already be provable via a different (off-path) proven child (e.g. Win because some sibling is a proven Loss), but that proof is deferred until a future playout happens to traverse the proving child. This is correct (proofs are monotone and eventually found) but means root can stay Unknown longer than necessary, and contrasts with try_prove_node which already re-scans all children. Pure efficiency/latency, not soundness.
- **Fix:** Optionally always call try_prove_node(parent) (which already rescans all children) regardless of the on-path child's value, only breaking when the parent itself stays Unknown. The on-path child check is then redundant.
- **Evidence:** let child_proven = unsafe { (*child_ptr).proven_value() }; if child_proven == ProvenValue::Unknown { break; } ... if parent_proven == ProvenValue::Unknown { break; }
- **Skeptic 1** (confirmed/LOW/feature-gated, tested=False): Read src/search_tree.rs:856-897 (propagate_proven) and the helper try_prove_node at lines 342-392. The finding is accurate as written. propagate_proven walks only the current playout path bottom-up and breaks early at line 865-867 when the on-path child (path[i].child) is ProvenValue::Unknown, before ever calling try_prove_node on the parent. Meanwhile try_prove_node (line 351-379) scans ALL of a 
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read src/search_tree.rs:856-897 (propagate_proven), :342-392 (try_prove_node), :435-468 (try_prove_chance_node). The cited code matches exactly. The mechanism is real: at line 865-867, if the on-path child is Unknown the loop breaks, never calling try_prove_node(parent) on this playout. try_prove_node (line 372) can prove a parent Win from a single proven-Loss SIBLING child even when the on-path c

### F33 · [LOW] · smell · `src/tree_policy.rs:170-177, 242-249` · arcade=no
**Proven-Draw child in choose_child returns raw mean with no exploration term, competing unevenly against UCT/PUCT scores**

- **Why:** A proven-draw child is scored on raw mean reward while siblings get mean + exploration. There is no value in re-exploring a proven node, but mixing a no-bonus score against with-bonus scores means the proven-draw can be systematically over- or under-selected relative to its true standing, distorting visit allocation among the remaining unproven siblings. Behavior is defensible but undocumented and asymmetric.
- **Fix:** Document the intent (proven-draw competes on exploitation value only), or return a fixed draw value (e.g. 0.0 on the reward scale) so proven draws are treated as a known-value option rather than re-deriving a noisy mean from sum_rewards/cv.
- **Evidence:** UCT L170-177 and PUCT L242-249: ProvenValue::Draw returns `mov.sum_rewards() as f64 / cv as f64` (or 0.0 if cv==0) — a bare mean with no exploration bonus, while sibling unproven children get full UCT/PUCT scores including the exploration term.
- **Skeptic 1** (confirmed/LOW/feature-gated, tested=False): Read src/tree_policy.rs L162-200 (UCTPolicy::choose_child) and L234-269 (AlphaGoPolicy::choose_child). The code matches the citation exactly: in both policies, when `solver` is true and a child's `child_proven_value()` is `ProvenValue::Draw`, the selector returns `if cv == 0 { 0.0 } else { mov.sum_rewards() as f64 / cv as f64 }` (L170-177 and L242-249) — a bare mean with no exploration term, while
- **Skeptic 2** (confirmed/LOW/feature-gated, tested=False): Read src/tree_policy.rs:166-198 (UCT) and 238-268 (PUCT). Both match the finding exactly: when `solver` is enabled and a child's `child_proven_value()` is `ProvenValue::Draw`, the branch returns `mov.sum_rewards() as f64 / cv as f64` (or `0.0` if `cv==0`) — a bare exploitation mean with NO exploration term, while unproven siblings (the `ProvenValue::Unknown => {}` fall-through) get the full UCT (`

### F38 · [LOW] · smell · `src/batch.rs:101-103` · arcade=no
**hand-written `unsafe impl Sync for BatchedEvaluatorBridge` is broad and unaudited against StateEvaluation**

- **Why:** The manual Sync is needed because `mpsc::Sender` is `!Sync`, but the SAFETY note omits the response channel: `EvalRequest` carries `mpsc::SyncSender<(Vec<MoveEvaluation>, SE)>`, and the struct itself does not name `Spec::State`/`MoveEvaluation`/`SE` in the justification. The bound `B::StateEvaluation: Sync + Send + Clone` (trait def line 16) does hold, so it is sound today, but the hand-rolled `unsafe impl` will not be re-checked by the compiler if someone later adds a `!Sync` field (e.g. a `Cell` cache) to the bridge. This is exactly the kind of place a future edit silently introduces UB.
- **Fix:** Prefer wrapping the `Sender` in a `Mutex` only and deriving Sync structurally (the Mutex<Sender> + Arc<B> + Option<JoinHandle> are all Sync/Send), or scope the unsafe impl as tightly as possible and expand the SAFETY comment to enumerate every field including the response sender type carried through the channel.
- **Evidence:** `// SAFETY: The Mutex<Sender> is Sync, Arc<B> is Sync (B: Sync), Option<JoinHandle> is Send.
// Evaluator requires Sync, which this satisfies through the Mutex wrapper.
unsafe impl<Spec: MCTS, B: BatchEvaluator<Spec>> Sync for BatchedEvaluatorBridge<Spec, B> {}`
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/batch.rs:95-103 (struct + unsafe impl) and 131-194 (Evaluator impl, Drop, collector). The cited code exists verbatim. Core smell is real: an UNCONDITIONAL hand-rolled `unsafe impl<Spec, B> Sync for BatchedEvaluatorBridge` whose SAFETY comment doesn't enumerate all type parameters, and which by construction won't be re-checked by the compiler if a future edit adds a `!Sync` field. So the m
- **Skeptic 2** (confirmed/LOW/multithread-only, tested=False): Read src/batch.rs:95-103. The struct fields are `sender: Option<Mutex<mpsc::Sender<EvalRequest<...>>>>`, `batch_eval: Arc<B>`, `eval_thread: Option<JoinHandle<()>>`. The finding's factual claims hold: the manual `unsafe impl Sync` is sound today (the trait bound `B::StateEvaluation: Sync + Send + Clone` at line 16 plus `B: Send + Sync` at line 15 cover the channel payload, including the `SyncSende

### F40 · [LOW] · smell · `src/tree_policy.rs:111-131, 259-267` · arcade=yes
**reciprocal(0) overloaded to mean 2.0 is an undocumented magic constant entangling FPU semantics**

- **Why:** `reciprocal(0)` mathematically is +inf but is hard-coded to 2.0, which silently encodes the PUCT `1/(1+n)` denominator (1/(1+0)=1, not 2) AND a different unvisited-child weighting. The value 2.0 is unexplained, differs from the standard PUCT denominator, and is only reached when `fpu` is infinite — a subtle coupling between `fpu_value()` and this table. A future reader tuning PUCT will not know why unvisited children get a 2x prior boost rather than the standard `1/(1+n)=1`.
- **Fix:** Document that the table approximates `1/(1+n)` (so index n should map to `1/(1+n)`, making reciprocal(0)=1.0) — the current 2.0 suggests the index convention is actually `1/n` with a special unvisited case; clarify which, add a comment, and a unit test pinning the formula for n=0..3.
- **Evidence:** `reciprocals = (0..RECIPROCAL_TABLE_LEN).map(|x| if x == 0 { 2.0 } else { 1.0 / x as f64 })` and in choose_child the `child_visits == 0` (with infinite fpu) path falls through to `(sum_rewards + explore_coef * policy_evaln) * self.reciprocal(child_visits as usize)`, i.e. multiply-by-2 for an unvisited child.
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/tree_policy.rs:111-131 (table build + reciprocal()) and 259-267 (the choose_child fallthrough), plus the doc comment at 89-92, lib.rs:158-160 (default fpu), and treant-dynamic/src/{types.rs:52,adapter.rs:145}. The code is cited accurately: reciprocals = (0..128).map(|x| if x==0 {2.0} else {1.0/x}), and the unvisited child (child_visits==0) with infinite fpu falls through to (sum_rewards +
- **Skeptic 2** (confirmed/LOW/arcade-path, tested=False): Read src/tree_policy.rs:111-131 and 259-267, src/lib.rs:158-160 (fpu default), and tests/mcts_tests.rs (fpu tests + AlphaGo evaluators). The code matches the finding: `reciprocals = (0..LEN).map(|x| if x==0 {2.0} else {1.0/x})` and choose_child does `(sum_rewards + explore_coef*P) * reciprocal(child_visits)`.

Reachability correction: The finding calls the reciprocal(0)=2.0 path a "subtle coupling

### F62 · [LOW] · smell · `treant-wasm/src/difficulty.rs:29-63` · arcade=yes
**pick_weak determinism is illusory: same seed does not reproduce the move because the underlying search RNG is unseeded**

_also reported as: Meaning of AiConfig.playouts drifts between rebuild-based and advance-based games, biasing difficulty calibration_

- **Why:** The `seed` argument only seeds the final softmax draw. The visit counts it samples over come from `manager.playout_n(playouts)`, and none of the cell-game / mancala configs set `rng_seed()`, so the MCTS search itself draws from `thread_rng()`. The same `seed` therefore yields different visit distributions (and frequently different moves) run to run. Callers in pickAiMove (gameTypes.ts:148) generate a fresh random seed per move anyway, so this is benign in production, but the `seed` parameter advertises a reproducibility it cannot deliver, and any test or golden harness that passes a fixed seed expecting determinism (e.g. tictactoe weak_move tests) is relying on visit-mass concentration, not the seed.
- **Fix:** Document that `seed` only makes the tie-break draw reproducible and that full reproducibility additionally requires a seeded search config; or have pick_weak seed/reset the manager's search RNG from `seed` when the spec supports it.
- **Evidence:** manager.playout_n(playouts);
    ...
    let mut rng = SmallRng::seed_from_u64(seed as u64);
    let mut r = rng.gen::<f64>() * sum;
- **Skeptic 1** (confirmed/LOW/arcade-path, tested=False): Read difficulty.rs:1-71. The cited code matches exactly: manager.playout_n(playouts) at line 29 runs the MCTS search, then only the final softmax tie-break draw (lines 62-63, SmallRng::seed_from_u64(seed)) consumes `seed`. The visit counts it samples over (root_child_stats, line 36) come from that unseeded search.

Verified the search RNG mechanism: MCTS::rng_seed() defaults to None (src/lib.rs:21
- **Skeptic 2** (confirmed/LOW/arcade-path, tested=False): Verified all claims by reading the source. difficulty.rs:29 calls manager.playout_n(playouts) and difficulty.rs:62-63 seeds a SmallRng from `seed` only for the final softmax tie-break draw. The MCTS search itself uses a selection RNG seeded from rng_seed() (src/lib.rs:502-504): when rng_seed() returns None the RNG comes from thread_rng(), i.e. non-deterministic. The default rng_seed() (src/lib.rs:

### F66 · [LOW] · smell · `treant-wasm/src/gridpack.rs:448-451` · arcade=yes
**get_stats / get_tree unwrap serde_wasm_bindgen::to_value, panicking into WASM on serialization failure**

- **Why:** Every macro-generated game (and OrderChaosWasm line 521, and mancala.rs:425/434) unwraps the serde conversion. A serialization failure would panic across the WASM boundary; with console_error_panic_hook it surfaces as an unrecoverable abort rather than a recoverable JS error. The structures are simple so failure is unlikely, but the pattern is replicated ~35 times and there is no test exercising a serialization-failure path. Minor, but it is an unwrap on a nominally-fallible path in the shared surface.
- **Fix:** Use `.unwrap_or(JsValue::NULL)` or return a Result/Option to JS, so a serialization edge case degrades to 'no stats' rather than aborting the WASM instance.
- **Evidence:** pub fn get_stats(&self) -> JsValue {
                let stats = types::build_stats(&self.manager, |_| None);
                serde_wasm_bindgen::to_value(&stats).unwrap()
            }
- **Skeptic 1** (confirmed/LOW/arcade-path, tested=False): Read treant-wasm/src/gridpack.rs:448-451 — code matches the cited evidence verbatim (get_stats unwraps serde_wasm_bindgen::to_value). Grep confirms the pattern is replicated ~40 times across the WASM surface, including OrderChaos at gridpack.rs:521 and mancala.rs:425/434, plus get_tree variants (connectfour:88, nim:169, etc.). lib.rs:72 installs console_error_panic_hook::set_once(), so a panic sur
- **Skeptic 2** (uncertain/LOW/unreachable, tested=False): Code verified at treant-wasm/src/gridpack.rs:448-451 — matches the finding exactly, and grep confirms the pattern is replicated ~40 times across the WASM crate (e.g. mancala.rs:425/434, gridpack.rs:521, every macro-generated game, plus all the *.rs get_stats/get_tree methods). So the descriptive claim (unwrap on a nominally-fallible serde conversion, no failure-path test) is factually correct: I c

### F73 · [LOW] · smell · `treant-dynamic/src/evaluators.rs:54` · arcade=yes
**RandomRollout depth cap is a hardcoded 1000, ignoring DynConfig::max_playout_length / max_playout_depth**

- **Why:** The rollout length cap is a magic constant unrelated to the user's configured max_playout_length (default 1_000_000) or max_playout_depth. Games whose random rollouts routinely exceed 1000 plies will silently truncate and fall into the 0.0 trap above; games that want shorter caps cannot tighten it. The evaluator has no access to DynConfig, so the knob is unreachable.
- **Fix:** Make the cap configurable (constructor arg or plumb config) and/or document the 1000-ply limit prominently. At minimum hoist it to a named const with a doc comment.
- **Evidence:** let max_depth = 1000;
for _ in 0..max_depth {
- **Skeptic 1** (confirmed/LOW/arcade-path, tested=False): Verified treant-dynamic/src/evaluators.rs:54. The code reads exactly `let max_depth = 1000;` followed by `for _ in 0..max_depth {` (line 56) — a hardcoded magic constant with no doc comment. The RandomRollout evaluator (struct at line 20) holds only an Option<Mutex<Rng64>> and its EvalCallbacks::evaluate (callbacks.rs:53-60) receives only &self + game state + moves, with no access to DynConfig. So
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read treant-dynamic/src/evaluators.rs:54 directly — the code matches the finding exactly: `let max_depth = 1000;` then `for _ in 0..max_depth`. On truncation, sim is non-terminal, terminal_value() returns None, and execution falls to the `(priors, 0.0)` branch (line 92) — the "0.0 trap" the finding describes. Confirmed DynConfig defaults in treant-dynamic/src/types.rs:56-57 (max_playout_length=1_0

### F34 · [LOW] · test-gap · `src/tree_policy.rs:194-196, 263-267` · arcade=no
**No test pins the exact UCT/PUCT selection score, so the formula/doc mismatches are uncaught**

- **Why:** The two doc/code discrepancies above (UCT factor, PUCT denominator) survive because no unit test computes the expected selection score for a small hand-constructed node and asserts equality. Any future refactor of the formula would also go unnoticed.
- **Fix:** Add a focused unit test that builds a node with known visits/sum_rewards/priors and asserts choose_child returns the move matching the exact arithmetic of the intended formula, for both UCTPolicy and AlphaGoPolicy, including the n==0 and tie-break branches.
- **Evidence:** grep of tests/ for `reciprocal`, `1 + n`, `(1 +` returned no hits; the exploration-constant factor (2.0 vs sqrt(2)) and reciprocal(0)=2.0 are not asserted anywhere.
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/tree_policy.rs at the cited lines. Both formula facts are confirmed:

UCT (lines 194-196): `explore_term = 2.0 * (ln_adjusted_total / child_visits).sqrt()` then `exploration_constant * explore_term + mean_action_value` = `2C * sqrt(ln/n) + Q`. The docstring (line 59) claims `Q(a) + C * sqrt(2 * ln(N) / n(a))`. The `2.0` is OUTSIDE the sqrt in code but the `2` is INSIDE the sqrt in the doc
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read src/tree_policy.rs:194-196 (UCT) and 263-267 (PUCT) plus surrounding context, the docstrings (lines 59, 91), and reciprocal()/new() (lines 111-131). Searched tests/ and src/tree_policy.rs for any score-arithmetic assertions.

The test-gap claim is CONFIRMED. The 123 integration tests (tests/mcts_tests.rs) and 26 golden tests all run full searches and assert on outcomes (best move, score bound

### F39 · [LOW] · test-gap · `src/transposition_table.rs:180-211` · arcade=no
**ApproxQuadraticProbingHashTable size guard over/under-admits and is never tested for fullness/eviction behavior**

- **Why:** Under concurrency the load-then-fetch_add is not atomic, so the 2/3 load-factor cap is approximate (multiple threads can pass the guard simultaneously). That is acceptable by the 'approximate' design, but the 16-probe give-up means inserts silently fail (return None) well before the table is full when hashes cluster, and `size` can drift above the real occupancy if two threads both fetch_add for the same hash that one of them then loses the value CAS on. There appear to be no tests exercising near-capacity insert, the 16-probe cliff, or that lookup still finds a key after a failed/aliased insert. A magic `2/3` and `PROBE_LIMIT=16` (line 167) drive correctness with no test pinning them.
- **Fix:** Add tests filling the table to >2/3 and verifying graceful degradation (no panic, lookups consistent), and document the load-factor and probe-limit constants as tuning knobs. Consider asserting that `size` never exceeds capacity.
- **Evidence:** `if self.size.load(Ordering::Relaxed) * 3 > self.capacity * 2 { return self.lookup(key, handle); }` then later `self.size.fetch_add(1, Ordering::Relaxed)` only on successful key CAS; the probe loop `for inc in 1..(PROBE_LIMIT+1)` gives up after 16 probes returning `None`.
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read src/transposition_table.rs:115-241. The code matches the finding (the type is ApproxQuadraticProbingHashTable, aliased as ApproxTable — the finding's name/lines are correct). Verified the specific mechanics: (1) line 180 the load-factor guard `size*3 > capacity*2` (2/3) is a plain Relaxed load then later (line 204) fetch_add, so it is non-atomic and approximate under concurrency; (2) the inse
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read src/transposition_table.rs:115-239. Code matches the finding: line 180 guard `size*3 > capacity*2` (~2/3 load factor) redirects to lookup; line 188/227 probe loop `1..(PROBE_LIMIT+1)` with PROBE_LIMIT=16 (line 167) returns None on give-up; line 204 `size.fetch_add(1)` only after a winning key CAS. The test-gap claim is the substantive and accurate part: searched tests/mcts_tests.rs and the sr

### F59 · [LOW] · test-gap · `src/batch.rs:78-93` · arcade=no
**BatchedEvaluatorBridge example marked ```ignore``` so it is never compile-checked, risking doc rot**

- **Why:** The only usage example for the batching feature is `ignore`d, so it is excluded from doc-tests. The crate-level example in lib.rs IS run as a doc-test (lib.rs:5), establishing the convention that examples compile. This one can silently drift out of sync with the real API (note `mcts.playout_n_parallel` is called on a value bound with `let` not `let mut`, which would not compile — evidence it was never checked).
- **Fix:** Convert to a compiling `no_run` doctest with a minimal stub BatchEvaluator, or at minimum fix the `let mut mcts` so the snippet is coherent.
- **Evidence:** /// ```ignore
/// // 1. Implement BatchEvaluator for your NN evaluator
/// impl BatchEvaluator<MyMCTS> for MyNNEvaluator { ... }
...
/// let mcts = MCTSManager::new(state, MyMCTS, bridge, policy, table);
/// mcts.playout_n_parallel(10000, 4);
/// ```
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Verified against source. The example at src/batch.rs:80-93 is marked ```ignore``` and is therefore excluded from doc-tests, while the crate-level example at src/lib.rs:5 is a non-ignored, compiled doctest — confirming the claimed convention that examples normally compile. The incoherence claim is also correct: the snippet binds `let mcts = MCTSManager::new(...)` then calls `mcts.playout_n_parallel
- **Skeptic 2** (confirmed/LOW/library-only, tested=True): Both factual claims verified at source. src/batch.rs:80 uses ```ignore``` for the sole batching-feature usage example, excluding it from doctests. The snippet is genuinely non-compiling: src/lib.rs:611 declares `pub fn playout_n_parallel(&mut self, n, num_threads)`, but batch.rs:91-92 binds `let mcts = MCTSManager::new(...)` (no mut) and then calls `mcts.playout_n_parallel(...)` — calling an &mut 

### F78 · [LOW] · test-gap · `tests/golden/golden_tests.json:1-7 (all 7 tests)` · arcade=no
**Cross-language golden tests omit chance nodes, score-bounded MCTS, custom priors, and evaluate_existing_state**

_also reported as: Cross-language golden JSON only covers counting + nim; no chance, score-bounded, draw, dirichlet, temperature, or closed-loop scenarios_

- **Why:** The golden JSON is the shared cross-language contract (CLAUDE.md: 'shared across all language bindings'). It exercises only deterministic single/two-player games with the solver. The perspective-sensitive and easy-to-get-wrong paths — open-loop vs closed-loop chance nodes (chance_outcomes), score_bounded_enabled + terminal_score, non-uniform host priors, custom interpret_for_player, and the evaluate_existing_state re-evaluation path flagged above — have NO cross-language golden coverage. The dice-game and prior tests exist only as Rust-side unit tests in golden.rs, so other-language ports can pass the golden suite while silently mishandling chance/score-bounded semantics.
- **Fix:** Add golden_tests.json entries for a chance game (both closed_loop and open_loop), a score_bounded game asserting root_score_bounds, and a custom-priors game, with deterministic seeds and visit/bound expectations.
- **Evidence:** 7 tests: counting_basic, counting_high_playouts, nim_solver_{5,4,3,6}_stones, nim_no_solver_5_stones — games 'counting' and 'nim' only; expects cover best_move/child_stats/proven_value.
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Read /home/peter/code/treant/tests/golden/golden_tests.json (full file, 113 lines) and the runner /home/peter/code/treant/treant-dynamic/tests/golden.rs. All finding claims verified:

(1) JSON contains exactly 7 tests: counting_basic, counting_high_playouts, nim_solver_{5,4,3,6}_stones, nim_no_solver_5_stones. Only games "counting" and "nim". Expects cover only best_move/child_stats/proven_value. 
- **Skeptic 2** (confirmed/LOW/library-only, tested=True): Read tests/golden/golden_tests.json (exactly 7 tests, games "counting" + "nim" only — counting_basic, counting_high_playouts, nim_solver_{5,4,3,6}_stones, nim_no_solver_5_stones) and treant-dynamic/tests/golden.rs. The factual core of the finding is correct: chance nodes (DiceGame open+closed loop), score-bounded MCTS (ScoreGame + terminal_score), and custom/misleading priors (PriorGame/Misleading

### F85 · [LOW] · test-gap · `tests/mcts_tests.rs:522-541, 3371-3382` · arcade=no
**AdvanceError::ChildNotOwned has no behavioral regression test (only Display is asserted)**

- **Why:** ChildNotOwned is returned (src/search_tree.rs:1204-1206: `if !move_info.owned.load(Ordering::SeqCst) { return Err(AdvanceError::ChildNotOwned); }`) when a transposition table makes the chosen root child a non-owned alias of another node. This is the only failure path of advance() that depends on the lock-free table/ownership machinery, the most subtle part of re-rooting (detaching+freeing a subtree without double-free). It is reachable only with a TranspositionTable enabled, and a regression that mistakenly returned Ok and then `Box::from_raw`'d a non-owned pointer would be a use-after-free / double-free — exactly the class of bug a test should pin. The WASM Nim apply_move (treant-wasm/src/nim.rs:199-210) calls advance() but only handles the Ok/Err-fallback split, so the arcade wouldn't surface it either.
- **Fix:** Add a test using make_counting_mcts() (ApproxTable enabled) that drives enough playouts to create a transposition alias on a root child, then asserts advance(&that_move) == Err(AdvanceError::ChildNotOwned), and that a subsequent playout_n still runs without UB (run under `cargo test` and ideally `cargo +nightly miri` / ASAN).
- **Evidence:** test_advance_root_move_not_found asserts Err(AdvanceError::MoveNotFound); test_advance_root_child_not_expanded asserts Err(AdvanceError::ChildNotExpanded). The third variant is only checked in test_advance_error_display: `let e3 = AdvanceError::ChildNotOwned; ... assert!(format!("{}", e3).contains("alias"));`. No test ever drives advance() into the ChildNotOwned branch.
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Verified all cited locations. tests/mcts_tests.rs:522-533 (test_advance_root_move_not_found) asserts Err(AdvanceError::MoveNotFound); 535-541 (test_advance_root_child_not_expanded) asserts Err(AdvanceError::ChildNotExpanded). The only reference to ChildNotOwned in test code is tests/mcts_tests.rs:3375 inside test_advance_error_display, which merely asserts format!("{}", e3).contains("alias"). grep
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): VERIFIED the cited code. tests/mcts_tests.rs:521-541 are test_advance_root_move_not_found (asserts Err(MoveNotFound)) and test_advance_root_child_not_expanded (asserts Err(ChildNotExpanded)). The only other ChildNotOwned reference in the whole repo is test_advance_error_display at line 3375-3378, which merely asserts `format!("{}", e3).contains("alias")`. grep across tests/, src/, treant-dynamic c

### F87 · [LOW] · test-gap · `treant-wasm/tests/mcts_tests.rs:1-20` · arcade=no
**Shared WASM integration test covers only 10 of 36 exported game types and only weak_move()**

- **Why:** This is the only test that exercises games through the public wasm-bindgen-exported surface (`use treant_wasm::*`). The 26 uncovered games rely solely on in-module `#[cfg(test)]` tests that use internal types, so a broken or missing `pub use`/`#[wasm_bindgen]` export, or a panic in the shared weak_move/result path for those games, would not be caught by any integration test. apply_move(), result(), current_player(), is_terminal() are not exercised through the public surface for ANY game.
- **Fix:** Extend the integration test to construct and smoke-test every exported *Wasm type (a macro over the full list), calling weak_move, best_move, current_player, is_terminal, result, and a sample apply_move, asserting no panic and contract-shaped outputs.
- **Evidence:** every_game_weak_move_returns_something_or_terminal ck!()s exactly 10 games (ConnectFour, TicTacToe, Reversi, Hex, Frontline, Mancala, DotsBoxes, Konane, Domineering, Nim) and only asserts `g.weak_move(50,5,1.5,1).is_some()`. lib.rs exports 36 `*Wasm` types (e.g. Amazons, CaptureGo, Chomp, Clobber, Col, Euclid, FoxHounds, Game2048, Connect6, Notakto, OrderChaos, SquareUp, Squava, MuTorere, NoGo, Pig, Prior(2), Shift, Sim, SubtractSquare, Trails, Treblecross, Wythoff, Counting, Dice). The integration crate touches none of the other 26.
- **Skeptic 1** (confirmed/MEDIUM/library-only, tested=False): Read treant-wasm/tests/mcts_tests.rs (the entire 21-line file) and treant-wasm/src/lib.rs. Every concrete claim is accurate. The single test every_game_weak_move_returns_something_or_terminal ck!()s exactly 10 games — ConnectFour, TicTacToe, Reversi, Hex, Frontline, Mancala, DotsBoxes, Konane, Domineering, Nim — matching the cited list verbatim, and asserts only `g.weak_move(50,5,1.5,1).is_some()`
- **Skeptic 2** (uncertain/LOW/library-only, tested=True): Read treant-wasm/tests/mcts_tests.rs (all 20 lines): the integration test does cover exactly 10 games (ConnectFour, TicTacToe, Reversi, Hex, Frontline, Mancala, DotsBoxes, Konane, Domineering, Nim) and asserts only g.weak_move(50,5,1.5,1).is_some(). Confirmed lib.rs exports 36 distinct *Wasm types via `pub use` (counted: comm shows 26 exported-but-not-in-integration-test). So the literal citation 

### F89 · [LOW] · test-gap · `treant-wasm/src/dice.rs:1-172 (whole file)` · arcade=yes
**Several WASM games with non-trivial logic have zero tests: dice (chance), nim, counting, prior**

- **Why:** DiceGameWasm exercises the open/closed-loop chance machinery through the wasm surface and has no test at all; a regression in chance_outcomes wiring or the get_tree/get_stats export would be invisible. NimWasm.apply_move's advance-with-fallback path (distinct from every other game's MCTSManager::new rebuild) is untested at the WASM layer, so the ChildNotOwned/ChildNotExpanded fallback `self.manager.playout_n(100); self.manager.advance(&m)` is unverified.
- **Fix:** Add a dice.rs test that plays Roll, advances through a chance outcome, and checks is_terminal/result; add a nim.rs test that exercises apply_move both when the child is already expanded (advance Ok) and when it is not (fallback playout path).
- **Evidence:** Per-file #[test] counts: dice.rs=0, nim.rs=0, counting.rs=0, prior.rs=0, difficulty.rs=0, gridlib.rs=0 (gridlib helpers max_run/line_symbol/has_square are exercised only indirectly). dice.rs is the only WASM chance-node game and DiceGameWasm is exported (lib.rs:45) but never instantiated in any test; nim.rs is the only WASM game whose apply_move uses the advance()/subtree-preserving path (nim.rs:199-210) rather than the rebuild path.
- **Skeptic 1** (confirmed/LOW/arcade-path, tested=False): Read treant-wasm/src/dice.rs (whole file, 1-172), nim.rs:196-223, lib.rs, and treant-wasm/tests/mcts_tests.rs. Claims substantiated:

1. Per-file #[test] counts: grep over treant-wasm/src confirms dice.rs=0, nim.rs=0, counting.rs=0, prior.rs=0, difficulty.rs=0, gridlib.rs=0. Nearly every other game file has 1-15 tests. Confirmed.

2. dice.rs is the only WASM chance-node game: chance_outcomes (dice
- **Skeptic 2** (uncertain/LOW/library-only, tested=True): Read dice.rs (whole file, 172 lines), nim.rs (whole file), and surveyed all of treant-wasm/src. The literal claim is TRUE: dice.rs, nim.rs, counting.rs, prior.rs, difficulty.rs, gridlib.rs each contain 0 #[test] blocks (verified via grep -c), while ~30 sibling game files (pig, tictactoe, connectfour, mancala, etc.) carry native #[test] unit tests. DiceGameWasm/NimWasm are exported (lib.rs:45,:57) 

### F90 · [LOW] · test-gap · `treant-wasm/src/gridlib.rs:165-191` · arcade=yes
**gridlib helper has_square is asserted only positively; no negative/edge test for the O(n^2) square detector**

- **Why:** SquareUp's entire terminal condition (gridpack.rs:357-368) rests on has_square. A bug in the perpendicular-offset logic could either miss tilted squares (game never ends → infinite-ish search) or false-positive (premature win). The diagonal/tilted branch and the negative path are the parts most likely to be wrong and are completely unexercised.
- **Fix:** Add unit tests in gridlib (or gridpack) for has_square: a tilted square (e.g. (0,1),(1,0),(1,2),(2,1)) returns true, three collinear points return false, and a sparse board returns false.
- **Evidence:** has_square iterates point pairs and checks both perpendicular offsets to detect a square of any size/orientation. The only coverage is gridpack.rs square_up_detects_a_square (positive, axis-aligned 2x2). There is no test for a rotated/tilted square (the (-dc,dr) perpendicular branch), no negative test (3 collinear or near-square that must NOT trigger), and no test that fewer than 4 same-symbol cells returns false.
- **Skeptic 1** (confirmed/LOW/arcade-path, tested=False): Read gridlib.rs:165-191: has_square is exactly as cited — it enumerates same-symbol point pairs, treats each as an edge, and checks both perpendicular offsets [(-dc,dr),(dc,-dr)] to detect a square of any size/orientation. Verified the only test exercising it is gridpack.rs:607 square_up_detects_a_square, which is a single positive, axis-aligned 2x2 case (cells 0,1,4,5 on a 4x4). grep confirms gri
- **Skeptic 2** (confirmed/LOW/arcade-path, tested=False): Verified by reading gridlib.rs:165-191 and searching tests. has_square uses an O(n^2) point-pair scan with a perpendicular-offset branch [(-dc,dr),(dc,-dr)] that detects squares of any orientation. gridlib.rs contains NO #[cfg(test)] module at all (grep found none). The sole coverage is gridpack.rs:607 square_up_detects_a_square, which is exactly the positive axis-aligned 2x2 case the finding desc

### F91 · [LOW] · test-gap · `tests/mcts_tests.rs:3335-3368` · arcade=no
**test_on_backpropagation_called does not actually verify the callback fires**

- **Why:** on_backpropagation is a public extension point. As written, the test would still pass if on_backpropagation were never invoked by the search loop (e.g. a refactor accidentally dropped the call site), giving false confidence that the hook works.
- **Fix:** Use interior mutability (e.g. store an Arc<AtomicUsize> in ExtraThreadData or a thread-local) so the callback can increment a counter, then assert the count is > 0 and consistent with the number of backpropagation steps.
- **Evidence:** The test's own comment admits it: `// We can't easily count calls via the trait (no mutable state in &self), but we verify the method exists and doesn't panic when called.` The MCTS::on_backpropagation impl body is empty and the assertions only check num_nodes>1 and best_move==Add — identical to a search with no callback at all.
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Read tests/mcts_tests.rs:3335-3368. The finding is accurate. `test_on_backpropagation_called` (3354-3368) only asserts `mcts.tree().num_nodes() > 1` and `mcts.best_move().unwrap() == Move::Add` — both are generic properties of any successful search, independent of the callback. The `BackpropCountMCTS::on_backpropagation` impl body (3346-3348) is empty and has no side effect, so the test would pass
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Confirmed by reading tests/mcts_tests.rs:3335-3368. The test's own comment (line 3356-3357) admits it does not count callback invocations, and the assertions are only `num_nodes() > 1` and `best_move() == Move::Add` — both identical to a search with no callback. The `on_backpropagation` impl body (line 3346) is empty. Tellingly, the comment at line 3347 references a "backprop_counter below" that d

### F92 · [LOW] · test-gap · `treant-dynamic/tests/golden.rs:269-1233` · arcade=no
**Dynamic adapter: single-playout playout() and playout_parallel_for() are untested**

- **Why:** playout_parallel_for is a time-bounded threaded entry point through the trait-object adapter; thread spawning/joining through the dyn boundary is a distinct code path from playout_n_parallel and is unexercised, so a deadlock or join-panic there would ship undetected in language bindings.
- **Fix:** Add a dynamic-adapter test calling playout_parallel_for(Duration, 2) and a single playout() and assert num_nodes grows without panic.
- **Evidence:** DynMCTSManager exposes pub fn playout, playout_n, playout_n_parallel, playout_parallel_for (confirmed by grep of treant-dynamic/src). The test file only ever calls mgr.playout_n and mgr.playout_n_parallel (grep of `mgr\.` shows: advance, best_move, num_nodes, playout_n, playout_n_parallel, principal_variation, reset, root_child_stats, root_proven_value, root_score_bounds, tree_snapshot). Neither playout() nor playout_parallel_for() is called.
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Verified against source. treant-dynamic/src/manager.rs:40-57 confirms DynMCTSManager exposes playout(), playout_n(), playout_n_parallel(), and playout_parallel_for(). grep of treant-dynamic/tests/golden.rs confirms the exact set of mgr.* calls claimed (advance, best_move, num_nodes, playout_n, playout_n_parallel, principal_variation, reset, root_child_stats, root_proven_value, root_score_bounds, t
- **Skeptic 2** (confirmed/LOW/library-only, tested=True): Verified directly. treant-dynamic/src/manager.rs:40-57 exposes four public playout methods; playout() and playout_parallel_for() are pure one-line delegations to MCTSManager<DynSpec> (inner: MCTSManager<DynSpec>, manager.rs:17). Grep of treant-dynamic/tests/golden.rs confirms the literal coverage gap: only playout_n (25+ call sites) and playout_n_parallel (one call, line 1092: playout_n_parallel(5

### F93 · [LOW] · test-gap · `tests/mcts_tests.rs:2362-2368, 2731-2749` · arcade=no
**No test pins root_score_bounds for a solver-only / value-only game producing UNBOUNDED, nor the bounds-vs-proven one-directionality at the adapter level**

- **Why:** The one-directional bounds<->proven coupling is a stated key invariant and a likely sign/feature-gating regression site. A test that solver-only leaves bounds UNBOUNDED (and that score_bounded-only leaves proven Unknown until bounds converge) would pin the gating that keeps the two opt-in features independent.
- **Fix:** Add a two-player solver-only test asserting root_score_bounds()==ScoreBounds::UNBOUNDED, and a score_bounded-only test asserting root_proven_value()==Unknown before convergence and the expected value after.
- **Evidence:** test_score_bounded_unbounded_without_feature asserts UNBOUNDED only for the single-player CountingGame. test_terminal_value_derives_score_bounds (Loss->-1, Win->1) covers value->bounds derivation in core, but the documented invariant 'Bounds->proven is one-directional (converged bounds set proven value, NOT the reverse)' (CLAUDE.md) is asserted only implicitly. There is no test asserting that enabling solver alone (score_bounded disabled) leaves root_score_bounds == UNBOUNDED for a two-player game.
- **Skeptic 1** (confirmed/LOW/feature-gated, tested=False): Read tests/mcts_tests.rs at all cited locations and traced the relevant game/MCTS specs. The finding's factual claims hold:

1. test_score_bounded_unbounded_without_feature (lines 2362-2368) asserts root_score_bounds()==UNBOUNDED only on CountingGame, which is single-player (type Player = (), line 31). Confirmed.

2. test_terminal_value_derives_score_bounds (2731-2748) uses ValueOnlyBoundsMCTS (sc
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Read tests/mcts_tests.rs:2362-2368 and 2731-2749 plus surrounding helpers and src/search_tree.rs. The finding's factual claims hold:

1. test_score_bounded_unbounded_without_feature (2362-2368) uses make_no_transposition_mcts() which is the single-player CountingGame(0) (Player = (), tests/mcts_tests.rs:6-13, 29-34). So the UNBOUNDED-without-feature assertion is only for a single-player value-only


## DISPUTED (2)

### F80 · [none] · contract · `treant-dynamic/src/callbacks.rs:32-36` · arcade=no
**score_bounded_enabled silently does nothing because dyn terminal_score defaults to None**

- **Why:** DynConfig exposes score_bounded_enabled (types.rs:22-23) as a first-class knob, and DynSpec wires it to the core (adapter.rs:177-179). But Score-Bounded MCTS only does anything when terminal_score() returns Some on terminals (core sets exact bounds from it). The GameCallbacks default is None, so a host that flips score_bounded_enabled on without overriding terminal_score gets a no-op feature with no warning — an easy mistake given the two settings live in different traits.
- **Fix:** Cross-reference in both docs: note in score_bounded_enabled (types.rs) that it requires terminal_score to be implemented, and in terminal_score that it is required for Score-Bounded MCTS to have any effect.
- **Evidence:** fn terminal_score(&self) -> Option<i32> { None }  // default
// DynConfig::score_bounded_enabled toggles core score-bounded MCTS
- **Skeptic 1** (refuted/none/library-only, tested=False): The finding's quotes are accurate (callbacks.rs:34-36 defaults terminal_score to None; types.rs:22-23 exposes score_bounded_enabled; adapter.rs:177-179 wires it through), but its central claim is false. The finding asserts "Score-Bounded MCTS only does anything when terminal_score() returns Some," making score_bounded_enabled a silent no-op unless terminal_score is overridden. The core actually ha
- **Skeptic 2** (confirmed/LOW/library-only, tested=False): Verified against source. callbacks.rs:34-36 confirms `terminal_score()` defaults to `None`. types.rs:23 confirms `score_bounded_enabled` is a separate first-class config field defaulting to false, and adapter.rs:46-47 / 177-179 confirm both are independently wired to the core (terminal_score forwarded to GameCallbacks, score_bounded_enabled forwarded to MCTS::score_bounded_enabled). The mechanics 

### F18 · [none] · perf · `src/batch.rs:115, 148` · arcade=no
**Unbounded mpsc channel for eval requests; no backpressure if collector stalls**

- **Why:** The request channel is an unbounded `mpsc::channel`. Each search thread blocks on its own `response_rx.recv()` after sending, so in steady state at most N requests (N=thread count) are in flight, bounding the queue. But if `evaluate_batch` is slow and the design ever changed to non-blocking submission, the unbounded queue would grow without limit. The per-response `sync_channel(1)` is correct (each request expects exactly one reply). Minor: the asymmetry (unbounded request, bounded response) is a latent foot-gun worth a comment, since the bound currently relies entirely on the blocking handshake in evaluate_new_state.
- **Fix:** Comment that backpressure is provided implicitly by the synchronous handshake (each search thread blocks on its response_rx), so the request channel never exceeds the worker count; or use a bounded sync_channel for the request side to make the invariant explicit.
- **Evidence:** let (sender, receiver) = mpsc::channel::<EvalRequest<Spec, B::StateEvaluation>>();
...
let (response_tx, response_rx) = mpsc::sync_channel(1);
- **Skeptic 1** (confirmed/LOW/library-only, tested=False): Citations are accurate. src/batch.rs:115 is `let (sender, receiver) = mpsc::channel::<EvalRequest<...>>()` (unbounded request channel); :148 is `let (response_tx, response_rx) = mpsc::sync_channel(1)` (bounded per-response). The finding's reasoning is correct AND it self-concedes there is no actual unbounded-growth bug: evaluate_new_state (lines 142-163) sends a request then blocks on response_rx.
- **Skeptic 2** (refuted/none/unreachable, tested=True): Code at src/batch.rs:115 (unbounded mpsc::channel for requests) and :148 (sync_channel(1) for responses) matches the finding verbatim. But this is not a defect. evaluate_new_state (lines 142-163) performs a synchronous handshake: it sends one request, then immediately blocks on response_rx.recv() before returning. So each search thread can have at most ONE request in flight; the request queue is b


## UNCERTAIN (4)

### F46 · [LOW] · docs · `src/lib.rs:394-396, 449` · arcade=no
**GameState and Evaluator associated types undocumented**

- **Why:** `GameState::Move/Player/MoveList` (lib.rs:394-396) and `Evaluator::StateEvaluation` (lib.rs:449) are public associated types with no rustdoc, flagged by missing_docs. These are part of the primary user-facing trait surface; the convention requires docs.
- **Fix:** Add brief rustdoc to each associated type.
- **Evidence:** pub trait GameState: Clone {
    type Move: Sync + Send + Clone;
    type Player: Sync;
    type MoveList: std::iter::IntoIterator<Item = Self::Move>;
...
pub trait Evaluator<Spec: MCTS>: Sync {
    type StateEvaluation: Sync + Send;
- **Skeptic 1** (uncertain/LOW/library-only, tested=False): Read src/lib.rs:392-465 directly. The factual surface claim is accurate: GameState's associated types Move/Player/MoveList (lib.rs:394-396) and Evaluator::StateEvaluation (lib.rs:449) are public associated types with no rustdoc comments (the surrounding trait METHODS are documented; the types are not).

However, the finding's stated justification is false. It claims these are "flagged by missing_d
- **Skeptic 2** (uncertain/LOW/library-only, tested=False): Read src/lib.rs:392-465. The code claim is accurate: GameState's associated types Move/Player/MoveList (lines 394-396) and Evaluator's StateEvaluation (line 449) have no rustdoc, while every method in both traits is documented. CLAUDE.md does state "All public API items have rustdoc comments," so this is a genuine (if trivial) convention gap.

However, the finding's central justification is FALSE:

### F56 · [LOW] · docs · `src/lib.rs:725-757` · arcade=no
**MCTSManager::reset / advance panic-on-async contract relies on Arc strong-count, only partially documented**

- **Why:** `reset()` has no rustdoc-documented Panics section even though it panics via Arc::try_unwrap when an AsyncSearchOwned holds a second Arc; `advance()` mentions the panic in prose ('Panics if an async search is still running', line 750) but neither documents that the detection mechanism is the Arc strong count — meaning the panic also fires if the user has cloned the tree Arc for any other reason (e.g. holding a NodeHandle's parent or another reference), not only 'async search'. The panic message is thus narrower than the real trigger condition.
- **Fix:** Add a `# Panics` doc section to reset(); broaden both messages/docs to 'Panics if any other reference to the search tree is held (e.g. a running async search).'
- **Evidence:** pub fn reset(self) -> Self {
        let search_tree = Arc::try_unwrap(self.search_tree)
            .unwrap_or_else(|_| panic!("Cannot reset while async search is running"));
...
    pub fn advance(&mut self, mov: &Move<Spec>) -> Result<(), AdvanceError> {
        let tree = Arc::get_mut(&mut self.search_tree)
            .expect("Cannot advance while async search is running");
- **Skeptic 1** (uncertain/LOW/library-only, tested=False): Read src/lib.rs:725-757 (reset/advance) plus surrounding context, src/lib.rs:478-485 (struct def), and src/search_tree.rs:1261-1269 (NodeHandle). The finding splits into two claims with opposite verdicts.

CONFIRMED (the actual LOW/docs core): reset() at lib.rs:727 has only a one-line doc comment and no `# Panics` section, yet it panics via Arc::try_unwrap (line 728-729). advance() (line 751) ment
- **Skeptic 2** (uncertain/LOW/library-only, tested=False): Read src/lib.rs:725-757 (reset/advance) and verified the code matches the finding verbatim. The narrow factual doc gap is real: reset() (line 725-740) has no `# Panics` rustdoc section at all, and advance() (line 750) only mentions the panic in one line of prose; neither documents that detection is via Arc strong count. No test covers these panics — reset() is used once in a happy-path test (mcts_

### F12 · [LOW] · perf · `src/batch.rs:96, 154-159` · arcade=no
**Mutex<Sender> serializes all leaf enqueues, defeating parallel batch collection**

- **Why:** `mpsc::Sender` is already `Send + Sync`-able via clone and `send` takes `&self`; wrapping it in a `Mutex` is unnecessary and forces every search thread to take a global lock for each leaf evaluation. Under high parallelism (playout_n_parallel with many threads) this lock becomes the bottleneck precisely where batching is supposed to help. The SAFETY comment at line 101-102 even asserts the Mutex is what provides Sync, but `std::sync::mpsc::Sender<T>` is `Send` and, since Rust 1.72, `Sender` is `Sync` when `T: Send`; the mutex is not required for soundness, only (per the comment's framing) to satisfy a Sync bound that a bare clone-based design would also satisfy.
- **Fix:** Replace `Mutex<mpsc::Sender<..>>` with a plain `mpsc::Sender` (it is Sync for T: Send) or hand each worker its own cloned Sender, eliminating the lock on the enqueue hot path.
- **Evidence:** sender: Option<Mutex<mpsc::Sender<EvalRequest<Spec, B::StateEvaluation>>>>,
...
sender
    .lock()
    .unwrap()
    .send(request)
- **Skeptic 1** (uncertain/LOW/library-only, tested=False): Code matches the citation exactly: src/batch.rs:96 declares `sender: Option<Mutex<mpsc::Sender<...>>>`, lines 101-103 have the SAFETY comment + `unsafe impl Sync`, and lines 154-159 do `sender.lock().unwrap().send(request)`.

The finding's CORE technical claim is correct: `std::sync::mpsc::Sender<T>` is `Send` and (on this toolchain, rustc 1.94.1) `Sync` when `T: Send`. So the `Mutex` is genuinely
- **Skeptic 2** (uncertain/LOW/library-only, tested=True): Read src/batch.rs:96, 101-103, 154-159, plus new()/Drop/collector_loop. The factual claims are mostly correct but the severity is inflated.

FACTUAL CLAIMS — verified:
- The field IS `Option<Mutex<mpsc::Sender<...>>>` (line 96), and the enqueue path does `sender.lock().unwrap().send(request)` (lines 155-159). Confirmed.
- `std::sync::mpsc::Sender<T>: Sync where T: Send` since Rust 1.72 — correct. 

### F42 · [LOW] · smell · `src/lib.rs:516, 542, 561, 793, 822; src/tree_policy.rs:67-85` · arcade=no
**Grouped minor API/style nits: missing #[must_use], inconsistent assertion messages, dead trait_data threading**

- **Why:** Individually trivial, but together they show builder-pattern inconsistency (config split between the MCTS trait and a lone &mut-Self setter) and inconsistent #[must_use]/assert-message conventions that invite drift. Worth one cleanup pass.
- **Fix:** Add #[must_use] to pure getters; give the num_threads asserts messages; either make print_on_playout_error a plain setter returning () or document the builder intent; consider moving it onto the MCTS trait for consistency.
- **Evidence:** `print_on_playout_error(&mut self, v) -> &mut Self` (lib.rs:516) is a builder-style setter but is not used in chained form anywhere and returns &mut Self inconsistently with other config (which lives on the MCTS trait). `num_threads(&self)` (793, 822) and `exploration_constant(&self)` (tree_policy 82,121) are pure getters lacking `#[must_use]` while most other read-only methods on MCTSManager have it. `playout_parallel_async`/`into_playout_parallel_async` assert `num_threads != 0` with no message (lib.rs:543,562) whereas `UCTPolicy::new` gives a formatted message. `choose_child` takes `mut han
- **Skeptic 1** (uncertain/LOW/library-only, tested=False): Read all cited lines. Most sub-claims of this grouped style nit are factually accurate but one is wrong, so the bundle is partly confirmed / partly refuted — net uncertain.

CONFIRMED sub-claims:
- src/lib.rs:516 `print_on_playout_error(&mut self, v: bool) -> &mut Self` is a lone builder-style setter; all 3 callers (tests/mcts_tests.rs:313,582,3419) call it as a standalone statement, never chained
- **Skeptic 2** (uncertain/LOW/library-only, tested=False): Read all cited lines. The grouped finding is partly accurate and partly wrong.

ACCURATE (genuine cosmetic nits, severity LOW/none is correct):
- src/lib.rs:516 print_on_playout_error(&mut self, v: bool) -> &mut Self is a builder-style setter. It IS used (tests/mcts_tests.rs:313,582,3419) but always as a standalone statement, never chained, so the &mut Self return is inconsistent with the rest of 


## REFUTED (7)

### F84 · [LOW] · contract · `src/search_tree.rs:479, 508, 246-260` · arcade=no
**`chance_probs`/`moves` length agreement is an unenforced invariant; mismatches silently truncate via `zip`**

- **Why:** Chance nodes store `moves` and `chance_probs` as two parallel `Vec`s built from `chance_outcomes()`. Every consumer `zip`s them, so if a future code path ever produced mismatched lengths (or a game's `chance_outcomes()` returned probabilities not summing to 1), the shorter is silently used: `sample_chance_child` would never reach trailing outcomes and the cumulative-probability sampler would fall through to `moves.last().unwrap()`. The contract ('probabilities must be positive and sum to 1.0') is documented on the trait but enforced nowhere — no `debug_assert` on length or sum. Not currently violated by in-tree games, but a foot-gun for library users.
- **Fix:** In `create_node`'s closed-loop branch, `debug_assert_eq!(probs.len(), moves.len())` and `debug_assert!((probs.iter().sum::<f64>() - 1.0).abs() < 1e-6)`; or store outcomes as a single `Vec<(Move, f64)>` to make desync unrepresentable.
- **Evidence:** for (mi, &prob) in node.moves.iter().zip(node.chance_probs.iter()) { ... }  // try_tighten_bounds_chance
...
for (mi, &prob) in node.moves.iter().zip(node.chance_probs.iter()) { ... }  // sample_chance_child
- **Skeptic 1** (refuted/LOW/library-only, tested=False): Read src/search_tree.rs:230-289 (create_node), 470-515 (try_tighten_bounds_chance, sample_chance_child), and lib.rs:430-443 (chance_outcomes trait doc). The cited zip lines (479, 508) and the moves.last().unwrap() fallback (514) all exist as quoted.

The central claim — that moves and chance_probs can desync in length and silently truncate via zip — is REFUTED by construction. chance_probs is popu
- **Skeptic 2** (uncertain/LOW/library-only, tested=False): Read src/search_tree.rs:230-262 (sole chance-node construction), 472-515 (the two cited zip loops), and lib.rs:430-443 (the trait contract). The zip pattern is exactly as quoted at lines 479 and 508.

The headline "length-mismatch" claim is largely REFUTED: moves (lines 246-249) and chance_probs (line 242) are both built from the same `outcomes: Vec<(Move,f64)>` returned by chance_outcomes(). grep

### F8 · [LOW] · test-gap · `tests/mcts_tests.rs:560-569` · arcade=no
**Multithreaded transposition-table contention path (delayed redirect + concurrent readers) has no test coverage**

- **Why:** The riskiest concurrency code in this slice — brief publication of `created`, redirect to `existing`, orphaning while other threads may hold the pointer — is only exercised single-threaded, where the race window cannot manifest. Regressions in node publication/orphan lifecycle would not be caught.
- **Fix:** Add a test using a transposition-heavy game (e.g. CountingGame-style cyclic states) run with playout_n_parallel(N, 4+) under ThreadSanitizer/Miri or at least a high-iteration stress run, asserting no crash and consistent root stats.
- **Evidence:** test_advance_root_with_transposition_table (560-569) exercises the table only single-threaded. The parallel tests (test_parallel_best_move 260-262, test_score_bounded_parallel_correctness 2348-2353) do not combine a transposition table with multiple threads, so descend()'s delayed-transposition redirect (807) and the orphan path (808-811) are never hit under concurrency.
- **Skeptic 1** (refuted/LOW/multithread-only, tested=True): The finding's central factual claim is false. It states "The parallel tests (test_parallel_best_move 260-262, test_score_bounded_parallel_correctness 2348-2353) do not combine a transposition table with multiple threads." But test_parallel_best_move (tests/mcts_tests.rs:260-263) calls make_counting_mcts() then playout_n_parallel(10000, 4). make_counting_mcts (line 237-245) constructs MCTSManager<C
- **Skeptic 2** (uncertain/LOW/multithread-only, tested=False): Read tests/mcts_tests.rs:560-569 and 237-264, and src/search_tree.rs:790-817 and src/transposition_table.rs:149-238.

The finding's PREMISE is factually wrong: it claims the parallel tests "do not combine a transposition table with multiple threads." False. test_parallel_best_move (tests/mcts_tests.rs:259-264) uses make_counting_mcts() which builds ApproxTable::new(1024) (line 243) and runs playou

### F17 · [LOW] · test-gap · `src/transposition_table.rs:1-242` · arcade=no
**Test coverage gap: no concurrency stress test for transposition table inserts or the batched evaluator collector**

- **Why:** The most defect-prone code in this slice (lock-free insert with key-then-value publication, size accounting under contention, probe-limit drop behavior) and the entire batch collector lifecycle (channel close on drop, collector-death panic paths, mismatched result length) have no direct or multithreaded tests. The single transposition test only checks a diagnostic substring. Bugs like the size double-increment (first finding) would never be caught. A loom-based or high-thread-count stress test would substantiate the soundness claims the `unsafe impl Sync` depends on.
- **Fix:** Add a multithreaded stress test inserting overlapping hashes from many threads asserting no double-free/leak and bounded size; add a BatchedEvaluatorBridge test that runs playout_n_parallel with >1 thread and one that triggers an evaluate_batch length mismatch / panic to pin down the documented failure mode.
- **Evidence:** tests/mcts_tests.rs:398 test_transposition_table_hits only asserts the diagnostic string contains 'transposition table hits'; no test exercises concurrent insert/lookup contention, the null-value publication window, the size-overcount path, probe-limit exhaustion, or BatchedEvaluatorBridge under playout_n_parallel.
- **Skeptic 1** (refuted/LOW/multithread-only, tested=True): Read src/transposition_table.rs:1-242 in full and the cited tests/mcts_tests.rs. The finding's headline — "no concurrency stress test for transposition table inserts or the batched evaluator collector" — is largely FALSE.

TT under concurrency: make_counting_mcts() (tests/mcts_tests.rs:237-245) builds CountingMCTS, whose `type TranspositionTable = ApproxTable<Self>` (line 88), backed by ApproxTabl
- **Skeptic 2** (uncertain/LOW/library-only, tested=True): Read src/transposition_table.rs:1-242 and searched tests/. The finding is a test-gap claim (not a bug), and it is partially correct but overstated.

REFUTED portions: The claim "no test exercises concurrent insert/lookup contention ... or BatchedEvaluatorBridge under playout_n_parallel" is inaccurate as literally written. tests/mcts_tests.rs:237-244 make_counting_mcts() builds with ApproxTable::ne

### F24 · [none] · correctness · `src/lib.rs:193-204` · arcade=no
**select_child_after_search treats a genuine i32::MIN guaranteed score as 'no bound'**

- **Why:** `best_lower == i32::MIN` is used as the sentinel for 'no child has a usable bound' (falls through to visit-count selection). But i32::MIN is also a representable terminal_score, and `negate_bound(child.upper)` can legitimately equal i32::MIN when a child's upper is i32::MAX (unexpanded) — conflating 'unbounded' with 'worst possible guaranteed score'. If a user game uses terminal scores near i32::MIN this branch silently ignores real bound information. Extreme edge case given typical small integer scores.
- **Fix:** Track whether any child produced a finite bound with a bool flag instead of overloading i32::MIN as a sentinel; or reserve i32::MIN exclusively for 'unbounded' and document that terminal_score must avoid it.
- **Evidence:** let best_lower = children.iter().map(|c| negate_bound(c.child_score_bounds().upper)).max().unwrap_or(i32::MIN); if best_lower > i32::MIN { ... } 
- **Skeptic 1** (refuted/none/feature-gated, tested=False): Read src/lib.rs:173-205 (select_child_after_search), negate_bound at src/lib.rs:384-390, ScoreBounds::UNBOUNDED at src/lib.rs:359-368, child_score_bounds at src/search_tree.rs:153-160, and terminal-score seeding at src/search_tree.rs:315-328. The cited code matches verbatim: best_lower uses unwrap_or(i32::MIN) and gates on best_lower > i32::MIN.

The finding is REFUTED on its central evidence. It 
- **Skeptic 2** (refuted/none/unreachable, tested=True): Read src/lib.rs:173-205 (select_child_after_search) and src/lib.rs:382-390 (negate_bound). The finding's central claim is backwards. negate_bound is sentinel-safe: it maps i32::MIN->i32::MAX and i32::MAX->i32::MIN. terminal_score() sets score_lower=score_upper=s (search_tree.rs:325-328). So a child with a GENUINE terminal_score == i32::MIN has child.upper == i32::MIN, and negate_bound(i32::MIN) ==

### F41 · [none] · docs · `src/lib.rs:684-705` · arcade=no
**select_move_by_temperature can return a non-argmax move while best_move()'s doc implies temperature only reweights**

- **Why:** Two gaps: (1) the `roll <= 0.0` loop plus float rounding means the last-element fallback path is reachable even though the loop 'should' always pick; harmless but the fallback returns the last (lowest-visit, since not sorted) move on rare float underflow. (2) `selection_temperature()` between 0 and 1e-8 is treated as exactly argmax, and there is no #[must_use] mismatch but the magic `1e-8` threshold is undocumented. Minor, but `best_move` is the headline API and temperature behavior near the 1e-8 boundary and the unsorted last-element fallback are unspecified.
- **Fix:** Document the 1e-8 cutoff and that temperature sampling ignores zero-visit children; make the fallback return the max-weight move rather than `weighted.last()`.
- **Evidence:** `fn selection_temperature` doc (lib.rs:230-235): '0.0 (default) = argmax by visits. 1.0 = proportional to visits.' `best_move` branches at `temperature < 1e-8`. `select_move_by_temperature` filters `c.visits() > 0`, weights `visits^(1/temperature)`, then samples; final fallback `Some(weighted.last().unwrap().0.clone())`.
- **Skeptic 1** (refuted/none/library-only, tested=True): Read src/lib.rs:673-705 and the doc at 230-235; code matches the citation exactly. The finding's headline is false: best_move()'s doc (lib.rs:230-232) does NOT imply temperature "only reweights" — it explicitly states "0.0 (default) = argmax by visits. 1.0 = proportional to visits," which documents that non-zero temperature performs stochastic, non-argmax sampling. Returning a non-argmax move is t
- **Skeptic 2** (refuted/none/library-only, tested=True): Read src/lib.rs:673-705 (best_move + select_move_by_temperature) and lib.rs:230-235 (selection_temperature doc). Code matches the finding's quotes, but the substantive claims are wrong:

(1) The "last-element fallback returns the lowest-visit move" claim is factually false. `weighted` is built from `root_node().moves()` iteration order, NOT sorted by visits, so weighted.last() is arbitrary order, 

### F83 · [none] · perf · `src/search_tree.rs:272` · arcade=yes
**`create_node` always sorts the move vector even when the policy's comparator is the no-op default (UCT)**

- **Why:** `compare_move_evaluations` defaults to `Ordering::Equal` (used by every UCT-based game, including all gridpack games and most arcade games). The sort then performs O(n log n) comparator calls per node creation that can never change the order, on the node-expansion path that runs once per new tree node. For high-branching games (e.g. 9x9 Connect6 with ~81 moves) this is measurable wasted work in the hottest allocation path.
- **Fix:** Skip the sort when progressive widening is not in use, e.g. gate on `state.max_children(0) != usize::MAX`, or add a `TreePolicy::needs_move_ordering() -> bool` defaulting to false and only sort when true.
- **Evidence:** moves.sort_by(|a, b| policy.compare_move_evaluations(&a.move_evaluation, &b.move_evaluation));
- **Skeptic 1** (refuted/none/arcade-path, tested=False): Read src/search_tree.rs:272 (sort line present as cited), src/tree_policy.rs:36-42 (compare_move_evaluations defaults to Ordering::Equal), and confirmed UCTPolicy (impl at tree_policy.rs:134) does NOT override it while only AlphaGoPolicy (line 290) does. The vast majority of arcade/wasm games use UCTPolicy (verified via grep: gridpack, Amazons, NoGo, Reversi, Hex, etc.), so the no-op sort is reach
- **Skeptic 2** (uncertain/none/arcade-path, tested=True): Source confirmed: src/search_tree.rs:272 unconditionally calls moves.sort_by(policy.compare_move_evaluations(...)). The default impl (src/tree_policy.rs:36-42) returns Ordering::Equal; UCTPolicy (impl at line 134) does NOT override it, so every UCT-based game sorts with a constant-Equal comparator. AlphaGoPolicy (line 290) overrides it with a real comparator. So the factual premise — UCT games sor

### F67 · [none] · test-gap · `treant-wasm/src/tictactoe.rs:173-184` · arcade=yes
**weak_move test name overstates coverage: 'protects a win' is not actually exercised for solver-less TicTacToe**

- **Why:** TicTacToeWasm's config does not enable `solver_enabled`, so the `root_proven_value()==Win` / proven-Loss branches in pick_weak never fire here. The assertion that high-temp weak play still returns the winning move "2" passes only because 2000 playouts concentrate visit mass on "2", not because any win-protection logic ran. The test name implies the safety net is validated when it is not — and there is no test anywhere that exercises pick_weak's proven-win guard on a solver-enabled game, so the most safety-critical branch of the weakening dial is untested.
- **Fix:** Add a test on a solver-enabled game (e.g. one of the gridpack solver games) at high temp/top-K that asserts pick_weak returns the proven-winning move even when temperature would otherwise scatter; rename this test to reflect that it checks visit-concentration, not proven-win protection.
- **Evidence:** fn weak_move_temp0_equals_best_and_protects_a_win() {
        ...
        let weak = g.weak_move(2000, 9, 3.0, 1);
        assert_eq!(weak.as_deref(), Some("2"));
    }
- **Skeptic 1** (refuted/none/arcade-path, tested=True): The finding's central premise is factually false. It claims "TicTacToeWasm's config does not enable solver_enabled, so the root_proven_value()==Win / proven-Loss branches in pick_weak never fire here." But treant-wasm/src/tictactoe.rs:30 constructs the manager with `GridMcts { solver: true }`, and treant-games/src/grid.rs:290-292 makes `solver_enabled()` return that `solver` field — so the solver 
- **Skeptic 2** (refuted/none/unreachable, tested=True): The finding's central premise is factually wrong. It claims "TicTacToeWasm's config does not enable solver_enabled," but treant-wasm/src/tictactoe.rs:30 (new_manager) constructs `GridMcts { solver: true }`, and treant-games/src/grid.rs:290-292 returns `self.solver` from `solver_enabled()`. The solver IS enabled for TicTacToeWasm.

I empirically reproduced the exact test scenario (X at {0,1}, O at 
