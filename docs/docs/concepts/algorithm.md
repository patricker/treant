---
sidebar_position: 1
id: algorithm
---

import TreeGrowthDemo from '@site/src/components/demos/TreeGrowthDemo';

# How MCTS Works

Monte Carlo Tree Search is a best-first search algorithm that uses random sampling to evaluate positions. Unlike alpha-beta, it needs no evaluation function to start working. Unlike pure Monte Carlo simulation, it builds a persistent tree that focuses sampling on the most promising branches.

This page goes deeper than the tutorial. It covers the mathematical foundations, asymptotic guarantees, and the design decisions that make MCTS practical.

## The multi-armed bandit foundation

MCTS reduces the problem of choosing which branch to explore to a well-studied problem in statistics: the multi-armed bandit.

You face a row of slot machines. Each has an unknown payout distribution. Each pull gives you information but costs an opportunity. Pull the best-looking machine and you exploit current knowledge. Pull an unknown machine and you explore for better options. The optimal strategy balances both.

Auer, Cesa-Bianchi, and Fischer (2002) proved that the UCB1 policy achieves logarithmic cumulative regret -- the gap between what you earned and what you would have earned always pulling the best arm grows as O(ln n), which is the theoretical minimum. The formula:

```
UCB1(a) = Q(a) + C * sqrt(ln(N) / n(a))
```

where Q(a) is the average reward of arm a, N is total pulls, and n(a) is pulls of arm a.

The shape of the exploration term matters. `sqrt(ln(N)/n(a))` grows slowly with total trials (logarithmically) but shrinks quickly as an arm is sampled more (inverse square root). Early on, uncertainty dominates and every arm gets tried. Later, the best arms dominate. The logarithm ensures exploration never fully stops -- even after a million pulls, there is still some probability of revisiting a weak-looking arm.

## From bandits to trees

Kocsis and Szepesvari (2006) applied UCB1 to tree search, creating UCT (Upper Confidence bounds applied to Trees). Each node in the search tree is treated as an independent bandit problem: the arms are the available moves, and the rewards are the simulation outcomes propagated up from below.

This is a leap. UCB1 assumes stationary reward distributions, but in a tree, the reward distribution of a move changes as the subtree below it gets more samples. Kocsis and Szepesvari proved that UCT is consistent despite this violation: given enough playouts, the probability of selecting a suboptimal move at the root converges to zero.

The key insight is that the non-stationarity is benign. As child nodes get more samples, their reward estimates improve. The parent node's bandit problem becomes easier over time, not harder. Convergence is slower than in the stationary case, but it still happens.

There is a subtlety here that is often glossed over. In a bandit problem, the arms are independent. In a tree, the reward distribution of one move depends on the exploration of its subtree. If the search has explored move A's subtree deeply and move B's subtree shallowly, move A's reward estimate is more reliable. UCB1 handles this automatically -- the confidence interval for move B is wider precisely because it has fewer visits. The formula does not need to know why the variance is high; it just reacts to the sample count.

## The four phases, revisited

Each playout runs four steps:

1. **Selection.** Walk from root to leaf, choosing the child with highest UCB score at each node. This descends the tree along the current best path while occasionally detouring to explore.

2. **Expansion.** At a leaf, create a new child node for the selected move. The tree grows by one node per playout. The `visits_before_expansion` parameter can delay expansion, requiring multiple visits to a leaf before it becomes a tree node. This reduces memory usage at the cost of slower convergence.

3. **Simulation (rollout).** From the new node, play random moves until the game ends. This gives a noisy but unbiased estimate of the position's value. In this library, the simulation step is generalized: the `Evaluator` can return any reward signal, not just a game outcome. Random rollout, heuristic evaluation, and neural network inference are all valid implementations.

4. **Backpropagation.** Walk back up to the root, updating visit counts and reward sums at each node along the path. Each node's statistics are updated atomically, allowing concurrent playouts to backpropagate simultaneously. The reward is interpreted per-player via `interpret_evaluation_for_player`, ensuring that a win for one player is a loss for the other in adversarial games.

<TreeGrowthDemo />

## Anytime minimax

MCTS is an anytime algorithm: you can stop it at any point and get a usable answer. More playouts mean better answers, but the first hundred playouts already capture the gross structure of the game tree.

With infinite playouts, UCT converges to the minimax value of the game. It builds the same information as exhaustive minimax search, but prioritizes the branches that matter. In practice, MCTS achieves strong play with a tiny fraction of the tree explored.

This is the fundamental tradeoff: MCTS gives no finite-time guarantee about which move is best. After 10,000 playouts, the most-visited move is probably the best, but "probably" is doing work. Alpha-beta with a perfect evaluation function gives you the minimax value exactly (up to the search depth). MCTS gives you a statistical estimate that improves with time.

The asymmetric growth pattern is where MCTS gains its practical edge. A balanced tree of depth d with branching factor b has b^d nodes. MCTS explores an asymmetric subtree that is deep along the principal variation and shallow everywhere else. The ratio of explored nodes to total tree size can be vanishingly small while still producing strong play. This is why MCTS succeeded in Go, where the full game tree has roughly 10^360 nodes.

## The simulation question

The original MCTS papers use random rollouts: play uniformly random moves until the game ends, then use the game outcome as the reward signal. This works surprisingly well. Random play is a terrible strategy, but averaged over thousands of samples, it produces a usable signal about which positions are better than others.

Random rollouts have a critical advantage: they require zero domain knowledge. You can apply MCTS to any game where you can enumerate legal moves and detect terminal states. No evaluation function, no heuristics, no training data.

But random rollouts are also wasteful. In Go, a random game might last 200 moves past the point where the outcome is determined. In chess, random play produces absurd positions that tell you nothing about the real game.

Three responses emerged:

**Heavy playouts.** Add simple heuristics to the rollout policy -- capture moves in Go, checks in chess. This keeps the "no neural network required" advantage while improving signal quality. The academic Go programs (pre-AlphaGo) used this extensively. The tradeoff is speed: a heavy rollout takes longer per simulation but produces a better reward estimate, so fewer simulations may suffice.

**Learned value functions.** Replace the rollout with a neural network that estimates position value directly. One forward pass replaces hundreds of random moves. This is what AlphaGo (2016) did, combining a value network with rollouts.

**No rollouts at all.** AlphaZero (2017) dropped rollouts entirely. The value network alone provides the reward signal. Each playout reaches a leaf, evaluates it with the network, and backpropagates. The tree becomes purely a search structure, not a simulation engine. This changes the per-playout cost profile: instead of many cheap random moves, each playout does one expensive neural network evaluation. The total playout count drops (hundreds or thousands, not millions), but each playout is more informative.

This library supports all three approaches. The `Evaluator` trait's `evaluate_new_state` method determines what happens at leaf nodes. Return a game outcome for rollout-based search. Return a neural network evaluation for rollout-free search. The tree structure and selection logic remain identical.

The `max_playout_depth` method provides a softer version of depth control. Unlike `max_playout_length` (a hard safety cap that panics), `max_playout_depth` causes the search to evaluate the current node as a leaf and backpropagate. This is useful with neural network evaluators: the network can evaluate any position, so there is no reason to descend deeper than the evaluation signal can usefully guide. Setting a depth limit also bounds the per-playout cost, making search time more predictable.

The separation between the `Evaluator` trait and the tree structure is what makes this flexibility possible. The search engine does not know or care how leaf values are produced. It receives a number and backpropagates it. This clean interface is discussed further in [Library Architecture](./architecture).

## The tree as a learned evaluation function

A useful mental model: the search tree is itself an evaluation function, constructed online during search.

Each node stores a reward average. That average, informed by the samples below it, is an estimate of the position's minimax value. Near the root, thousands of samples make the estimate tight. Near the leaves, a handful of samples make it loose. The tree policy decides where to invest the next sample by comparing these estimates against their uncertainty.

In this view, running more playouts is equivalent to training a better evaluation function. The "training data" is the simulated game outcomes. The "model" is the tree of reward averages. The "generalization" is the UCB formula's ability to interpolate between well-explored and poorly-explored siblings.

This is also why MCTS and neural networks complement each other so well. The network provides a prior (initial evaluation function). MCTS refines it through search (online training). The refined evaluation is better than either component alone.

## Comparison with alpha-beta

Alpha-beta pruning is the classical game tree search. It evaluates every position with a heuristic function and uses minimax with pruning to find the best move up to a fixed depth.

MCTS and alpha-beta make different assumptions and different tradeoffs:

**Evaluation function.** Alpha-beta requires one. Without a good evaluation function, alpha-beta is useless. MCTS works without one (via rollouts) but benefits from one (via learned evaluators). This makes MCTS the default choice when you lack domain expertise to write an evaluation function.

**Move ordering.** Alpha-beta's efficiency depends critically on examining the best move first. Good move ordering turns O(b^d) into O(b^(d/2)). Poor ordering gives no pruning at all. MCTS discovers move ordering automatically through the UCB selection -- good moves accumulate visits, bad moves are tried once and mostly ignored.

**Search depth.** Alpha-beta searches to a fixed depth with iterative deepening. MCTS has no fixed depth; it automatically searches deeper in forcing lines and shallower in quiet positions. A forced checkmate sequence might be explored 30 moves deep while a quiet middlegame position stays at depth 5. This adaptive depth allocation is one of MCTS's strongest practical advantages.

**Branching factor.** Alpha-beta struggles with high branching factors (Go: ~250 legal moves). MCTS handles them naturally because it only expands promising branches. Most of the 250 moves in a Go position will receive zero or one visit. With PUCT and neural network priors, the effective branching factor drops further -- the search focuses on the 5-10 moves the network considers most promising.

**Parallelism.** Alpha-beta is difficult to parallelize effectively because pruning creates sequential dependencies. A prune decision at one node depends on results from sibling nodes. MCTS parallelizes naturally -- multiple threads can descend the tree simultaneously with minimal contention. See [Lock-Free Parallel Search](./parallel-mcts).

**Quiescence.** Alpha-beta needs special handling at the search horizon to avoid evaluating unstable positions (the "horizon effect"). MCTS has no horizon -- it searches until terminal states (with rollouts) or until the evaluator produces a value (without rollouts). The problem does not arise in the same form, though MCTS has its own issues with tactical depth (it may not search deeply enough to see a distant checkmate).

## Asymptotic properties

Three properties characterize MCTS in the limit:

**Consistency.** As playouts approach infinity, the probability of selecting the best move at the root approaches 1. This is the fundamental correctness guarantee: MCTS eventually finds the right answer.

**Optimality.** UCB1 achieves the theoretical minimum regret bound of O(ln n). No policy can do better asymptotically. In practice, PUCT often converges faster despite lacking this theoretical guarantee, because prior knowledge reduces the constant factor.

**No finite-time guarantees.** After any fixed number of playouts, there is no bound on the probability that the most-visited move is suboptimal. You can construct adversarial games where 10 million playouts are insufficient to distinguish the best move. In practice this is rarely a problem, but it means MCTS results should always be treated as estimates, not proofs.

The exception is MCTS-Solver, which propagates game-theoretic proofs through the tree. When a subtree is fully explored and all terminal states agree, the result is exact. See [Game-Theoretic Proving](./solver-and-bounds).

## Beyond two-player games

MCTS generalizes naturally to multi-player, single-player, and cooperative games. The core mechanism -- sample, evaluate, backpropagate -- does not assume two players or zero-sum rewards. The `interpret_evaluation_for_player` method converts a state evaluation to a per-player reward, which is all the search needs.

Single-player MCTS (planning, optimization) treats each decision point as a bandit problem with no opponent. The search explores different decision sequences and converges toward the best one. The evaluation is a scalar objective value.

Multi-player MCTS maintains a reward for each player. During selection, each node uses the reward for the player currently deciding. Coalitions, asymmetric information, and non-zero-sum payoffs all work within this framework, though convergence may be slower when the game-theoretic structure is complex.

Stochastic games (dice, cards, random events) add another dimension. The tree must account for nature's random choices in addition to player decisions. See [Open-Loop vs Closed-Loop](./chance-nodes) for a full treatment of how this library handles randomness.

## Further reading

- Kocsis, L. and Szepesvari, C. (2006). "Bandit based Monte-Carlo Planning." *European Conference on Machine Learning.*
- Browne, C. et al. (2012). "A Survey of Monte Carlo Tree Search Methods." *IEEE Transactions on Computational Intelligence and AI in Games.*
- Auer, P., Cesa-Bianchi, N., and Fischer, P. (2002). "Finite-time Analysis of the Multiarmed Bandit Problem." *Machine Learning.*
- Silver, D. et al. (2017). "Mastering the game of Go without human knowledge." *Nature.*
- Silver, D. et al. (2018). "A general reinforcement learning algorithm that masters chess, shogi, and Go through self-play." *Science.*
- Coulom, R. (2007). "Efficient Selectivity and Backup Operators in Monte-Carlo Tree Search." *Computers and Games.*

## Summary

MCTS builds a search tree through repeated sampling. Each playout descends from root to leaf using UCB-based selection, expands one node, evaluates the leaf (via rollout, heuristic, or neural network), and backpropagates the result. The tree grows asymmetrically, focusing on promising branches.

The algorithm is backed by solid theory: UCB1 gives optimal regret bounds, UCT gives convergence to the minimax value, and the anytime property means the search can be stopped whenever the time budget runs out. The practical limitations -- no finite-time guarantees, sensitivity to the exploration constant, potential weakness in deep tactical situations -- are well-understood and addressable through the mechanisms described in the following concept pages.
