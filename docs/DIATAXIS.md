# Diataxis Classification & Learning Objectives

## Page Classification

Every page belongs to exactly one Diataxis type. No mixing.

| Page | Type | Audience |
|---|---|---|
| **intro.md** | Navigation | All |
| **tutorials/01-what-is-mcts** | Tutorial (conceptual) | Beginner |
| **tutorials/02-first-search** | Tutorial | Beginner |
| **tutorials/03-two-player-games** | Tutorial | Beginner |
| **tutorials/04-solving-games** | Tutorial | Beginner-Intermediate |
| **tutorials/05-stochastic-games** | Tutorial | Intermediate |
| **tutorials/06-neural-network-priors** | Tutorial | Intermediate |
| **tutorials/07-advanced-search** | Tutorial | Intermediate |
| **how-to/parallel-search** | How-To | Intermediate |
| **how-to/tree-reuse** | How-To | Intermediate |
| **how-to/progressive-widening** | How-To | Intermediate |
| **how-to/batched-evaluation** | How-To | Advanced |
| **how-to/hyperparameter-tuning** | How-To | Intermediate |
| **how-to/custom-tree-policy** | How-To | Advanced |
| **how-to/wasm-integration** | How-To | Intermediate |
| **concepts/algorithm** | Explanation | All |
| **concepts/exploration-exploitation** | Explanation | All |
| **concepts/tree-policies** | Explanation | Intermediate |
| **concepts/solver-and-bounds** | Explanation | Intermediate |
| **concepts/chance-nodes** | Explanation | Intermediate |
| **concepts/parallel-mcts** | Explanation | Intermediate |
| **concepts/architecture** | Explanation | Advanced |
| **reference/traits** | Reference | All |
| **reference/configuration** | Reference | All |
| **reference/glossary** | Reference | All |

## Tutorial Learning Objectives (ABCD Model)

### Tutorial 1: What is MCTS?

**By the end of this tutorial, the reader will be able to:**
1. Explain the four phases of MCTS (selection, expansion, simulation, backpropagation) and the role of each
2. Describe how the UCT formula balances exploration and exploitation
3. Predict how the search tree grows asymmetrically based on move quality

**Bloom's level:** Remember, Understand

### Tutorial 2: Your First Search

**By the end of this tutorial, the reader will be able to:**
1. Implement `GameState` for a single-player game with two moves and a terminal condition
2. Implement `Evaluator` with state evaluation and player-perspective reward
3. Configure and run MCTS search using `MCTSManager`, and interpret the principal variation and child statistics

**Bloom's level:** Apply

### Tutorial 3: Two-Player Games

**By the end of this tutorial, the reader will be able to:**
1. Implement `GameState` with alternating players and a `Player` enum
2. Implement `interpret_evaluation_for_player` to produce opposite-sign rewards for each player
3. Explain how negamax perspective eliminates the need for separate min/max logic

**Bloom's level:** Apply, Understand

### Tutorial 4: Proving Wins and Losses

**By the end of this tutorial, the reader will be able to:**
1. Implement `terminal_value()` and `terminal_score()` to provide game outcomes at terminal nodes
2. Enable MCTS-Solver and Score-Bounded search via configuration methods
3. Read and interpret `root_proven_value()` and `root_score_bounds()` after search

**Bloom's level:** Apply

### Tutorial 5: Games with Chance

**By the end of this tutorial, the reader will be able to:**
1. Implement `chance_outcomes()` to define stochastic transitions with probabilities
2. Implement `evaluate_existing_state` correctly for open-loop stochastic games
3. Distinguish when to use open-loop vs closed-loop chance node modes

**Bloom's level:** Apply, Analyze

### Tutorial 6: Neural Network Priors

**By the end of this tutorial, the reader will be able to:**
1. Build an evaluator that returns prior probabilities (Vec<f64>) for the PUCT policy
2. Configure `AlphaGoPolicy` with Dirichlet noise, FPU, temperature, and seeded RNG
3. Explain when PUCT outperforms UCT and how search overcomes misleading priors

**Bloom's level:** Apply, Analyze

### Tutorial 7: Advanced Search Features

**By the end of this tutorial, the reader will be able to:**
1. Add transposition table support with `TranspositionHash` and `ApproxTable`
2. Preserve search across turns using `advance()` and handle its error cases
3. Configure progressive widening and depth limiting for production search

**Bloom's level:** Apply

## How-To Learning Objectives

### Parallel Search
1. Choose between `playout_n_parallel`, `playout_parallel_for`, and `playout_parallel_async` based on requirements
2. Configure virtual loss for correct parallel exploration

### Tree Reuse
1. Call `advance()` after each move to preserve the search tree
2. Handle all three `AdvanceError` variants

### Progressive Widening
1. Implement `max_children()` with a widening schedule
2. Order `available_moves()` by priority for effective widening

### Batched Evaluation
1. Implement `BatchEvaluator` for GPU/NN inference
2. Configure `BatchConfig` and wrap with `BatchedEvaluatorBridge`

### Hyperparameter Tuning
1. Select appropriate values for C, FPU, Dirichlet alpha, and temperature
2. Diagnose search pathologies using `root_child_stats()` and `diagnose()`

### Custom Tree Policy
1. Implement `TreePolicy` with `choose_child` and thread-local data
2. Wire the custom policy into an MCTS configuration

### WASM Integration
1. Compile a game to WASM with `wasm-pack` and `wasm-bindgen`
2. Call MCTS from JavaScript/TypeScript in the browser

## Design Decisions

### Tutorial 1 is conceptual, not hands-on
Tutorial 1 teaches MCTS theory with an interactive demo. It contains no Rust code. This is intentional — the reader needs the mental model before they can write code. The interactive demo provides the "doing" element. The explanation-heavy content is appropriate because the reader has zero context.

### Code regions are the source of truth
All tutorials embed code via `remark-code-region` from compiled, tested example files. This is a deliberate choice per SITE.md ("code is the source of truth"). Readers see the exact code that compiles and runs. The tradeoff (no inline editing) is accepted.

### Prerequisites are implicit via ordering
Tutorials are numbered and progressive. Each assumes the reader completed the previous one. This is stated in the intro page ("Build working MCTS programs step by step"). Individual tutorials do not repeat this.
