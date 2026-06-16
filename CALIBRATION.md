# Calibrating AI difficulty in the Treant Arcade

How the arcade decides what "Easy", "Medium", and "Hard" mean for each of its
~30 Monte-Carlo-Tree-Search games — and how to re-measure it. Everything here is
reproducible from the repo: one Rust harness, one shell script, one results file.

- **Mechanism:** [`treant-wasm/src/difficulty.rs`](treant-wasm/src/difficulty.rs)
- **Harness:** [`treant-wasm/examples/calibrate.rs`](treant-wasm/examples/calibrate.rs)
- **Re-run:** [`scripts/calibrate.sh`](scripts/calibrate.sh)
- **Latest numbers:** [`plans/ai-calibration-results.md`](plans/ai-calibration-results.md)

---

## The problem

The arcade's AI is the [`treant`](https://crates.io/crates/treant) MCTS library
searching the game tree. Difficulty *used* to be one global table —
`{easy: 200 playouts + ε0.5, medium: 2000 + ε0.1, hard: 10000 + ε0}` — applied to
every game. In play-testing, **even "Easy" felt hard**. A short literature review
explained why (sources at the bottom):

1. **MCTS strength is ~logarithmic in the number of playouts**, with strong
   diminishing returns. 200 playouts is already well up the curve on small games,
   so "Easy" was near full strength.
2. **ε-greedy over the best move is a poor weakening knob.** At ε=0.5 the AI still
   plays the *optimal* move half the time — enough to beat a casual player.
3. **One global table can't fit 30 different games.** "200 playouts" is perfect on
   3×3 Tic-Tac-Toe and weak on 15×15 Gomoku. And the published "where the curve
   flattens" numbers come from *neural-net* engines on small games; they do **not**
   transfer to our **pure-rollout** engines (uniform priors, eval = 0, plus an
   exact solver on small games). So we can't import numbers — we have to measure.

## The weakening mechanism

Instead of ε-greedy, every engine exposes one method,
`weak_move(playouts, top_k, temp, seed)`
([`difficulty.rs`](treant-wasm/src/difficulty.rs)), used identically by the arcade
and by the calibration harness:

1. Run `playouts` MCTS iterations.
2. **Protect forced moves:** if the position is a proven win (root proven `Win`,
   or a child move proven to lose for the opponent), play it. The AI never throws
   a game it has already won.
3. Otherwise, take the **top-K most-visited** children and sample one with a
   temperature softmax over their visit counts: `weight ∝ (visits/maxVisits)^(1/temp)`.

Two knobs span the whole strength range:
- **`playouts`** — how deep it looks. The primary dial; native to MCTS (this is
  what KataGo/Leela use to set strength).
- **`temp`** (with `top_k`) — how often it picks a *strong-but-not-best* move.
  `temp → 0` = always the best move; larger `temp` = flatter choice among the
  K best moves.

Restricting the softmax to the **top-K by visits** is deliberate: raw AlphaZero
temperature over *all* moves occasionally samples a barely-searched move and
produces an alien, random-looking blunder (Leela Zero's developers cut their
temperature for exactly this reason). Top-K keeps the dial smooth while making
the mistakes *plausible* — the bot picks a reasonable second-best move, not
nonsense.

## The calibration method

For each game we run a **self-play round-robin** of a ladder of configs
(weak → strong), at the game's Classic board, alternating who moves first to
cancel the first-move advantage:

```
ladder (STD)   = p8/t3 · p30/t2 · p100/t1 · p300/t0.6 · p800/t0.35 · p2000/t0
ladder (LIGHT) = p8/t3 · p30/t2 · p80/t1 · p200/t0.5 · p500/t0.2   (branchy games)
```

Every config plays every other `n` games (default 20/pair). From the win matrix
we compute each rung's **field score** (its overall win rate across the field)
and auto-pick the three levels:

- **Easy** = the weakest rung — a casual human should win a fair share.
- **Hard** = the *cheapest* rung that reaches within 5% of peak field strength.
  This auto-caps solver/small games at a low playout count (where they already
  play near-perfectly — more search is wasted time) while letting open games
  climb to thousands of playouts.
- **Medium** = the rung whose field score sits closest to halfway between Easy
  and Hard.

The harness prints, per game, the matrix **and** a ready-to-paste block:

```
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 300, topK: 3, temp: 0.6 },
  },
  (Hard capped at p300: strength peaked early — extra playouts add nothing.)
```

### What the data shows

A clean split, exactly as the theory predicts:
- **Solver / small games** (Tic-Tac-Toe, Connect Four, Gomoku, Sim, Nim-likes):
  the matrix flattens after a few dozen–few hundred playouts — Hard caps low and
  `temp` is the real lever.
- **Open, high-branching games** (Hex, Reversi, Frontline, Amazons): strength
  keeps climbing the whole ladder — Hard goes to the top rung.

## Re-running it (one command)

```bash
scripts/calibrate.sh                 # all games (~minutes)
scripts/calibrate.sh connect-four    # one game
scripts/calibrate.sh connect-four 40 # one game, 40 games/pair (less noise)
```

Then paste each emitted `difficulty: { … }` block into the matching
`docs/src/components/arcade/games/<id>.tsx` `GameDefinition`, and rebuild the WASM
(`cd treant-wasm && wasm-pack build --target web`). Games with no `difficulty`
override fall back to `DEFAULT_DIFFICULTY` in `gameTypes.ts`.

### Validating "Easy is beatable"

Calibration measures *relative* strength; it can't measure "fun vs a human." We
sanity-check by hand: e.g. a dumb human-proxy (always drop the leftmost legal
column) should beat calibrated Easy Connect Four a fair share, and Hard should
beat Easy decisively. If Easy is still too strong, lower its `playouts` or raise
its `temp` in that game's `difficulty` block and re-verify.

### Two games are hand-set

`nim` and `subtract-square` don't expose the uniform WASM result/seat API the
harness needs, so they're set by analogy to the measured solver games (tiny,
solver-dominated: low playouts, `temp` is the lever). 2048 is single-player and
has no difficulty levels.

---

## Sources

- *On Strength Adjustment for MCTS-Based Programs* (AAAI)
- *AlphaDDA: adjusting AlphaZero strength to a human partner* (arXiv 2111.06266)
- *AlphaViT* (arXiv 2408.13871) — the Connect-4 Elo-vs-simulations curve
- Stockfish "Skill Level" docs; Leela Zero temperature discussion (issue #67) —
  the top-K / tail-blunder lesson
- AlphaGo Zero / AlphaZero (Nature 2017 / Science 2018) — visit-count temperature
