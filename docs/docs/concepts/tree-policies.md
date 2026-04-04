---
sidebar_position: 3
id: tree-policies
---

# UCT vs PUCT

The tree policy determines which child to explore at each node during selection. This library provides two built-in policies: `UCTPolicy` (classic UCB1) and `AlphaGoPolicy` (PUCT with prior probabilities). They share the same goal -- balancing exploration and exploitation -- but differ in how they treat moves before the search has gathered evidence about them.

Choosing the right policy is a design decision that depends on your game's branching factor, whether you have a source of prior knowledge, and how many playouts you can afford.

## UCT: Upper Confidence Bounds for Trees

UCT applies UCB1 at each tree node. The selection score for child a is:

```
UCT(a) = Q(a) + C * sqrt(2 * ln(N) / n(a))
```

Q(a) is the average reward of child a. N is the parent's visit count. n(a) is the child's visit count. C is the exploration constant.

The derivation is direct from the multi-armed bandit literature. Auer et al. (2002) showed that UCB1 achieves O(ln n) regret, the theoretical optimum. Kocsis and Szepesvari (2006) applied it to trees, proving that the resulting search converges to the minimax value.

**Strengths.** UCT needs no domain knowledge. Every move starts equal. The search discovers move quality entirely through playouts. This makes UCT the right choice when you have no prior information about move quality -- simple games, custom games, prototyping.

**Weaknesses.** Treating all moves equally means the search must visit every child at least once before revisiting any. In a position with 250 legal moves (typical in Go), the first 250 playouts are pure round-robin with no exploitation at all. This is catastrophically wasteful when most moves are obviously bad. A Go player knows that playing in the center of an empty board is more promising than playing on the 1-1 point, but UCT cannot express this.

## PUCT: Predictor + UCB for Trees

PUCT modifies the UCB formula to incorporate a prior probability P(a) for each move:

```
PUCT(a) = Q(a) + C * P(a) * sqrt(N) / (1 + n(a))
```

P(a) is the prior probability of move a, typically from a neural network policy head. The exploration term is now proportional to the prior: high-probability moves get more exploration, low-probability moves less.

This is the formula used by AlphaGo, AlphaZero, and every major game-playing system since 2016. It replaced UCT not because of better theoretical guarantees (PUCT has weaker theoretical properties), but because it converges faster in practice when good priors are available.

**Strengths.** PUCT focuses early visits on promising moves. In a Go position with 250 legal moves, if the network assigns 30% probability to the best move and 0.01% to the worst, the best move gets visited roughly 3000x more often before the exploration term decays. The search effectively ignores bad moves without explicitly pruning them.

Neural network integration is natural. The `Evaluator::evaluate_new_state` method returns per-move evaluations (the priors) alongside the state evaluation. These flow directly into PUCT's P(a) term.

**Weaknesses.** PUCT requires a prior source. Without one, all moves have equal prior (1/k for k moves), and the formula degenerates to a slightly different variant of UCT. Worse, bad priors actively hurt: if the network assigns high probability to a losing move, the search will waste visits confirming it's bad before exploring the real best move.

The formula also lacks UCT's theoretical regret bound. The `sqrt(N) / (1 + n(a))` shape was chosen empirically, not derived from a proof. In practice this does not matter -- the constant-factor improvement from good priors overwhelms the theoretical advantage of optimal regret.

## When UCT wins

UCT is the better choice when:

- **Branching factor is small.** In Tic-Tac-Toe (max 9 moves), round-robin costs nothing. The search completes the exploration phase in 9 playouts and exploits from playout 10 onward.
- **No prior source exists.** If you cannot produce per-move probabilities, UCT's equal treatment is correct rather than deficient.
- **The game is simple.** For toy games, prototypes, and benchmarks, UCT's zero-configuration property is valuable.
- **You want theoretical guarantees.** UCT has proven convergence and optimal regret. If you need to reason formally about your search, UCT is the foundation.

## When PUCT wins

PUCT is the better choice when:

- **Branching factor is large.** In Go (250 moves), chess (30-40 moves), or any game where most moves are bad, priors prevent wasting visits. This is PUCT's decisive advantage.
- **A neural network provides priors.** Self-play training systems need PUCT. The network's policy output is exactly the P(a) term. Training improves the priors, which improves the search, which generates better training data.
- **Playout budget is limited.** With only 800 playouts (AlphaZero's per-move budget), you cannot afford round-robin on 250 moves. PUCT makes every playout count.
- **You want to integrate domain knowledge.** Even without a neural network, you can provide hand-crafted priors -- higher values for captures, checks, center play. PUCT will focus the search on these moves without ignoring alternatives entirely.

## FPU as the bridge

First Play Urgency (FPU) mediates how UCT and PUCT handle unvisited children.

With UCT, infinity FPU (the default) is correct. Unvisited children get infinite urgency, forcing round-robin. Since all moves start equal, you need to visit each one to learn anything. The exploration constant C takes over after the round-robin phase.

With PUCT, infinity FPU undermines the priors. A move with P(a) = 0.001 should rarely be visited, but infinity FPU visits it before revisiting the P(a) = 0.3 move. Setting FPU to a finite value (e.g., 0.0) lets the prior control early exploration. The P(a) = 0.001 move stays unvisited unless the search exhausts better options.

The combination of PUCT with finite FPU and good priors creates a qualitatively different search. Instead of "try everything, then exploit," the search follows a path of decreasing prior probability, only exploring unlikely moves when the likely ones have been thoroughly investigated. This is how AlphaZero achieves superhuman play with 800 playouts per move.

## Progressive widening

An alternative to FPU for managing high branching factors. Instead of exposing all children and using FPU to limit exploration, progressive widening limits how many children are visible:

```rust
fn max_children(&self, visits: u64) -> usize {
    (visits as f64).sqrt() as usize
}
```

At 1 visit, only 1 child is available. At 100 visits, 10 children. At 10,000, 100. The search starts narrow and widens as it gathers evidence. Moves are expanded in the order returned by `available_moves()`, so sort by prior quality.

Progressive widening composes with either policy. With UCT, it prevents round-robin waste on high-branching positions. With PUCT, it provides a hard cap that the prior-based soft gating cannot.

## Future directions

The tree policy design is an active research area. Several alternatives to UCT and PUCT exist but are not yet in this library:

**Gumbel-Top-k (Policy Improvement Through Planning, 2022).** Adds Gumbel noise and uses the completed Q-values to improve the policy. Provably improves the policy with any number of simulations, even one. Promising for very low simulation budgets.

**Thompson Sampling.** Instead of upper confidence bounds, sample from the posterior distribution of each arm's reward. Naturally explores uncertain arms. Harder to implement efficiently in trees.

**RAVE (Rapid Action Value Estimation).** Shares information between sibling nodes -- if a move is good in one context, it is probably good in similar contexts. Effective in Go but less general.

Each of these could be implemented as a `TreePolicy` in this library's architecture. The trait boundary is the key design decision: the game, evaluator, and policy are independent, so new policies can be developed without modifying the search engine.

## Solver integration

Both UCT and PUCT integrate with the [MCTS-Solver and Score-Bounded](./solver-and-bounds) systems. When the solver is enabled, proven children override the normal UCB calculation:

- A child proven as Loss (from the child's perspective) means the parent wins by choosing it. The selection score becomes `f64::INFINITY`.
- A child proven as Win means the parent loses. The selection score becomes `f64::NEG_INFINITY`.
- A child proven as Draw receives its empirical average reward, allowing the search to still prefer an unproven potential win.

When score bounds are enabled, children whose upper bound (from the parent's perspective) falls below the best guaranteed lower bound from any sibling are pruned. They receive `f64::NEG_INFINITY` and are skipped during selection. This pruning logic is identical in both UCT and PUCT.

## Tie-breaking

Both policies use a thread-local RNG for tie-breaking. When multiple children have identical UCB scores (common early in search when several children have zero visits and infinity FPU), one is chosen uniformly at random. The `PolicyRng` tracks the number of tied candidates and uses reservoir sampling to select among them in a single pass.

Deterministic tie-breaking (e.g., always pick the first child) creates systematic bias. In games with symmetric positions, deterministic tie-breaking favors whichever move happens to appear first in the move list. Random tie-breaking eliminates this bias. When `rng_seed()` is set, the tie-breaking RNG is seeded deterministically per thread, making the search reproducible.

## Choosing a policy: decision guide

If you are building a game with fewer than 20 legal moves per position and no neural network, use `UCTPolicy`. Set C = sqrt(2) and leave FPU at infinity. This configuration requires zero tuning and works out of the box.

If you are building a system with a neural network (or any source of per-move prior probabilities), use `AlphaGoPolicy`. Set C between 1.0 and 2.5, set FPU to 0.0 or a small negative value, and return prior probabilities from your evaluator's `evaluate_new_state`. This is the configuration used by every major game AI since 2016.

If you are unsure, start with `UCTPolicy`. It is simpler, has stronger theoretical guarantees, and exposes problems clearly (if the search is bad, the game is hard or the playout budget is too small). Switch to `AlphaGoPolicy` when you have a prior source and evidence that it helps.
