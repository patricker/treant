# AI Difficulty Calibration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the arcade's one-size-fits-all `{easy:200/ε0.5, medium:2000/ε0.1, hard:10000/ε0}` AI with a value-aware weakening mechanism plus *measured*, per-game difficulty numbers, so "Easy" is actually beatable.

**Architecture:** Add one generic Rust helper, `difficulty::pick_weak`, that runs N playouts then selects a move by **visit-count temperature restricted to the top-K most-visited children** (no long-tail blunders) while **protecting forced wins** (`root_proven_value()==Win ⇒ play best_move`). Every WASM game exposes it as `weak_move(playouts, top_k, temp, seed)`. A native Rust self-play tournament (`examples/calibrate.rs`) drives that *same* method to measure each game's real strength curve — because the literature's numbers are regime-specific (neural-net + small games) and do **not** transfer to our pure-rollout engines. The arcade's `pickAiMove` calls `weak_move` with per-game `AiConfig`s filled in from the measured results.

**Tech Stack:** Rust (`treant-wasm` crate, `wasm-bindgen`, `rand` SmallRng), TypeScript/React (Docusaurus arcade), `wasm-pack`.

**Why this design (from an adversarial literature review):**
- MCTS strength is ~log in playouts but the plateau location depends on game size + whether a neural-net evaluator is present. Our engines use *pure random rollouts* (eval=0) + an exact solver on small games, so we cannot import published numbers — we must measure ours.
- ε-greedy over argmax is a poor knob (it still plays the optimal move 1−ε of the time). Raw AlphaZero temperature `N(a)^(1/τ)` over *all* moves picks 1-visit blunders (Leela cut τ to 0.25 to stop exactly this). **Restricting the softmax to the top-K by visits** keeps the "smooth dial" while removing the tail-blunder failure, and protecting `root_proven_value()==Win` keeps it from throwing a won game.

---

## File Structure

| File | Responsibility | New/Modify |
|---|---|---|
| `treant-wasm/src/difficulty.rs` | Generic `pick_weak` helper (the whole mechanism) | Create |
| `treant-wasm/src/lib.rs` | `mod difficulty;` | Modify |
| `treant-wasm/src/*.rs` (31 game classes) | one-line `weak_move(...)` method each | Modify |
| `treant-wasm/examples/calibrate.rs` | self-play tournament harness | Create |
| `plans/ai-calibration-results.md` | measured per-game numbers (data artifact) | Create (by Task 3) |
| `docs/src/components/arcade/gameTypes.ts` | `AiConfig`, `DEFAULT_DIFFICULTY`, `aiConfig()`, new `pickAiMove`, `GameHandle.weakMove`, `GameDefinition.difficulty?` | Modify |
| `docs/src/components/arcade/games/frontline.tsx` (`moveHandle`), `gridpack.tsx` (`cellHandle`/`ocHandle`), and other handle factories | add `weakMove` to each handle | Modify |
| `docs/src/components/arcade/useGameSession.ts` | resolve `aiConfig(def, kind)` and pass to `pickAiMove` | Modify |
| `docs/src/components/arcade/games/*.tsx` (calibrated games) | add `difficulty:` override | Modify |

---

## Task 1: The weakening mechanism (`pick_weak`) + first game method

**Files:**
- Create: `treant-wasm/src/difficulty.rs`
- Modify: `treant-wasm/src/lib.rs`
- Modify: `treant-wasm/src/tictactoe.rs` (add `weak_move`, add test)

- [ ] **Step 1: Write the failing test** in `treant-wasm/src/tictactoe.rs` inside its `mod tests` block (append after the existing test):

```rust
    #[test]
    fn weak_move_temp0_equals_best_and_protects_a_win() {
        // A board where X (player 0) has an immediate winning move.
        // X at 0,1 ; O at 3,4 ; X to move can win at 2 (top row 0-1-2).
        let mut g = TicTacToeWasm::new(3, 3, 3, 2);
        assert!(g.apply_move("0")); // X
        assert!(g.apply_move("3")); // O
        assert!(g.apply_move("1")); // X
        assert!(g.apply_move("4")); // O
        // temp 0 must return the engine's best move (deterministic)
        let strong = g.weak_move(2000, 1, 0.0, 1);
        assert_eq!(strong.as_deref(), g.best_move().as_deref());
        // even a very weak/random setting must still take the proven win (protection)
        let weak = g.weak_move(2000, 9, 3.0, 1);
        assert_eq!(weak.as_deref(), Some("2"));
    }

    fn empty_cells(g: &TicTacToeWasm) -> Vec<String> {
        g.get_board()
            .as_bytes()
            .iter()
            .enumerate()
            .filter(|&(_, &c)| c == b' ')
            .map(|(i, _)| i.to_string())
            .collect()
    }

    #[test]
    fn weak_move_returns_a_legal_move() {
        let mut g = TicTacToeWasm::new(3, 3, 3, 2);
        let mv = g.weak_move(50, 5, 1.5, 7).unwrap();
        assert!(empty_cells(&g).contains(&mv));
    }
```

(Both the helper `fn empty_cells` and the two tests go inside the existing `mod tests { use super::*; ... }` block in `tictactoe.rs`.)

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p treant-wasm tictactoe::tests::weak_move 2>&1 | tail -20`
Expected: FAIL — `no method named weak_move found` (and `pick_weak` unresolved).

- [ ] **Step 3: Create the helper** `treant-wasm/src/difficulty.rs`:

```rust
//! Difficulty: a value-aware MCTS weakening dial shared by every game.
//!
//! `pick_weak` runs `playouts` MCTS iterations, then selects a move by a
//! temperature softmax over visit counts **restricted to the top-K most-visited
//! children** (so it never samples a 1-visit blunder off the tail), while always
//! taking a proven win. temp→0 = the engine's best move; larger temp = flatter
//! choice among the K strongest moves; more playouts = stronger search.
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use treant::{MCTSManager, MoveEvaluation, Move, ProvenValue, MCTS};
use std::fmt::Display;

pub fn pick_weak<Spec>(
    manager: &mut MCTSManager<Spec>,
    playouts: u64,
    top_k: usize,
    temp: f64,
    seed: u32,
) -> Option<String>
where
    Spec: MCTS,
    Spec::ExtraThreadData: Default,
    MoveEvaluation<Spec>: Clone,
    Move<Spec>: Display + Clone,
{
    if playouts == 0 {
        return None; // caller falls back to a random legal move
    }
    manager.playout_n(playouts);

    // Protect a forced win: never throw a game the search has proven won.
    if matches!(manager.root_proven_value(), ProvenValue::Win) {
        return manager.best_move().map(|m| format!("{m}"));
    }

    let mut stats = manager.root_child_stats();
    if stats.is_empty() {
        return None;
    }
    stats.sort_by(|a, b| b.visits.cmp(&a.visits));
    let k = top_k.clamp(1, stats.len());
    let cand = &stats[..k];

    if temp <= 0.0001 || cand.len() == 1 {
        return Some(format!("{}", cand[0].mov)); // deterministic best
    }

    // softmax over (visits / maxvisits)^(1/temp), candidates only (no tail).
    let maxv = cand[0].visits.max(1) as f64;
    let weights: Vec<f64> = cand
        .iter()
        .map(|s| (s.visits as f64 / maxv).powf(1.0 / temp))
        .collect();
    let sum: f64 = weights.iter().sum();
    let mut rng = SmallRng::seed_from_u64(seed as u64);
    let mut r = rng.gen::<f64>() * sum;
    for (i, w) in weights.iter().enumerate() {
        r -= w;
        if r <= 0.0 {
            return Some(format!("{}", cand[i].mov));
        }
    }
    Some(format!("{}", cand[0].mov))
}
```

- [ ] **Step 4: Register the module** — add to `treant-wasm/src/lib.rs` in the alphabetical `mod` list (between `mod dice;`/`mod domineering;` region — put it after `mod dice;`):

```rust
mod difficulty;
```

- [ ] **Step 5: Add the `weak_move` method to `TicTacToeWasm`** — in `treant-wasm/src/tictactoe.rs`, inside the `#[wasm_bindgen] impl TicTacToeWasm { ... }` block, right after the `best_move` method:

```rust
    /// Difficulty-aware move: see `crate::difficulty::pick_weak`.
    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p treant-wasm tictactoe::tests::weak_move 2>&1 | tail -20`
Expected: PASS (2 tests).

- [ ] **Step 7: Lint**

Run: `cargo clippy -p treant-wasm 2>&1 | tail -3`
Expected: `Finished` with 0 warnings.

- [ ] **Step 8: Commit**

```bash
git add treant-wasm/src/difficulty.rs treant-wasm/src/lib.rs treant-wasm/src/tictactoe.rs
git commit -m "feat(difficulty): value-aware weak_move helper (top-K visit temperature + win protection)"
```

---

## Task 2: Roll `weak_move` out to every game class

**Files:**
- Modify (add the same method to each `#[wasm_bindgen] impl`): `treant-wasm/src/` — `connectfour.rs`, `gridpack.rs` (all 5 classes: Connect6/Squava/Notakto/SquareUp/OrderChaos), `shift.rs`, `mancala.rs`, `pig.rs`, `nim.rs`, `hex.rs`, `reversi.rs`, `frontline.rs`, `clobber.rs`, `konane.rs`, `trails.rs`, `capturego.rs`, `chomp.rs`, `wythoff.rs`, `dotsboxes.rs`, `mutorere.rs`, `domineering.rs`, `nogo.rs`, `col.rs`, `sim.rs`, `amazons.rs`, `foxhounds.rs`, `treblecross.rs`, `subtractsquare.rs`, `euclid.rs`, `game2048.rs`, `counting.rs`, `dice.rs`, `prior.rs`.

> Note: the method body is **identical** for every class because each has a field `manager: MCTSManager<...>`. The generic bounds (`ExtraThreadData: Default`, `Move: Display + Clone`, `MoveEvaluation: Clone`) are satisfied by all games (they use `()` thread data, `Display` moves, and `()`/`f32` evaluations).

- [ ] **Step 1: Add this exact method to each class's `#[wasm_bindgen] impl` block** (place it right after that class's `best_move` method). For `OrderChaosWasm` in `gridpack.rs` and any class generated by the `cell_game_wasm!` macro, add it inside the macro's generated impl OR as a separate `#[wasm_bindgen] impl Name { ... }` block in the same file:

```rust
    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
```

For classes built by the `cell_game_wasm!` macro in `gridpack.rs`, add `weak_move` to the macro body once (inside the `impl` the macro generates) so all five inherit it; verify by grepping:

Run: `grep -c "fn weak_move" treant-wasm/src/gridpack.rs`
Expected: `5` (one per generated class) **or** add a standalone `#[wasm_bindgen] impl <Name>` per class if the macro can't host it.

- [ ] **Step 2: Add a smoke test** to `treant-wasm/tests/mcts_tests.rs` (create the test fn at the end of the file):

```rust
#[test]
fn every_game_weak_move_returns_something_or_terminal() {
    use treant_wasm::*;
    // A representative sample across constructors; each must return a legal-ish
    // move string (Some) from the opening position under a weak setting.
    macro_rules! ck {
        ($g:expr) => {{
            let mut g = $g;
            assert!(g.weak_move(50, 5, 1.5, 1).is_some(), "weak_move returned None at start");
        }};
    }
    ck!(ConnectFourWasm::new(7, 6, 4, 2));
    ck!(TicTacToeWasm::new(3, 3, 3, 2));
    ck!(ReversiWasm::new(6, 6));
    ck!(HexWasm::new(7));
    ck!(FrontlineWasm::new(6, 6));
    ck!(MancalaWasm::new(6, 4, 2));
    ck!(DotsBoxesWasm::new(3, 3));
    ck!(KonaneWasm::new(6, 6));
    ck!(DomineeringWasm::new(6, 6));
    ck!(NimWasm::new(15));
}
```

- [ ] **Step 3: Run the smoke test**

Run: `cargo test -p treant-wasm every_game_weak_move 2>&1 | tail -10`
Expected: PASS. If any constructor signature differs, fix the call to match (`cargo build` errors name the exact arity).

- [ ] **Step 4: Full test + lint**

Run: `cargo test -p treant-wasm 2>&1 | grep "test result" | head -1 && cargo clippy -p treant-wasm 2>&1 | tail -2`
Expected: all tests OK; clippy 0 warnings.

- [ ] **Step 5: Commit**

```bash
git add treant-wasm/src/*.rs treant-wasm/tests/mcts_tests.rs
git commit -m "feat(difficulty): expose weak_move on every game engine"
```

---

## Task 3: Self-play calibration harness + measured numbers

**Files:**
- Create: `treant-wasm/examples/calibrate.rs`
- Create (output, written by hand from the run): `plans/ai-calibration-results.md`

- [ ] **Step 1: Write the harness** `treant-wasm/examples/calibrate.rs`:

```rust
//! Self-play difficulty calibration. Round-robins a ladder of AiConfigs per game
//! using the SHIPPING weak_move, prints a win matrix + each config's field score,
//! and flags adjacent ladder steps where strength really changes.
//! Run: cargo run --release --example calibrate -p treant-wasm
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use treant_wasm::*;

#[derive(Clone, Copy)]
struct Cfg { name: &'static str, playouts: u32, top_k: usize, temp: f64 }

trait Eng {
    fn weak(&mut self, p: u32, k: usize, t: f64, seed: u32) -> Option<String>;
    fn apply(&mut self, m: &str) -> bool;
    fn terminal(&self) -> bool;
    fn result(&self) -> String;
    fn current(&self) -> u32;
}
macro_rules! eng {
    ($t:ty) => {
        impl Eng for $t {
            fn weak(&mut self, p: u32, k: usize, t: f64, seed: u32) -> Option<String> { self.weak_move(p, k, t, seed) }
            fn apply(&mut self, m: &str) -> bool { self.apply_move(m) }
            fn terminal(&self) -> bool { self.is_terminal() }
            fn result(&self) -> String { self.result() }
            fn current(&self) -> u32 { self.current_player() }
        }
    };
}
eng!(ConnectFourWasm); eng!(TicTacToeWasm); eng!(ReversiWasm); eng!(HexWasm); eng!(FrontlineWasm);

fn play(make: &dyn Fn() -> Box<dyn Eng>, a: &Cfg, b: &Cfg, rng: &mut SmallRng) -> u32 {
    let mut g = make();
    for _ in 0..2000 {
        if g.terminal() { break; }
        let c = if g.current() == 0 { a } else { b };
        let seed = rng.gen::<u32>();
        match g.weak(c.playouts, c.top_k, c.temp, seed) {
            Some(m) => { let _ = g.apply(&m); }
            None => break,
        }
    }
    match g.result().as_str() { "1" => 0, "2" => 1, _ => 2 }
}

fn tournament(name: &str, make: &dyn Fn() -> Box<dyn Eng>, pool: &[Cfg], n: u32) {
    let mut rng = SmallRng::seed_from_u64(0xC0FFEE);
    let m = pool.len();
    let mut pts = vec![vec![0.0f64; m]; m];
    let mut field = vec![0.0f64; m];
    let mut games = vec![0.0f64; m];
    for i in 0..m {
        for j in (i + 1)..m {
            for g in 0..n {
                let (x, y) = if g % 2 == 0 { (i, j) } else { (j, i) };
                let (sx, sy) = match play(make, &pool[x], &pool[y], &mut rng) {
                    0 => (1.0, 0.0), 1 => (0.0, 1.0), _ => (0.5, 0.5),
                };
                pts[x][y] += sx; pts[y][x] += sy;
                field[x] += sx; field[y] += sy; games[x] += 1.0; games[y] += 1.0;
            }
        }
    }
    println!("\n==== {name} ====");
    for i in 0..m {
        print!("{:>12} ", pool[i].name);
        for j in 0..m {
            if i == j { print!("    ·"); }
            else { print!(" {:>3.0}%", 100.0 * pts[i][j] / n as f64); }
        }
        println!("   field {:>3.0}%", 100.0 * field[i] / games[i]);
    }
    println!("  ladder steps (upper beats lower):");
    for i in 1..m {
        let wr = 100.0 * pts[i][i - 1] / n as f64;
        let tag = if wr >= 70.0 { "  <-- real step" } else if wr <= 58.0 { "  (no real change)" } else { "" };
        println!("    {:>12} vs {:<12} {:>3.0}%{}", pool[i].name, pool[i - 1].name, wr, tag);
    }
}

fn main() {
    // Weak -> strong ladder, all via the shipping weak_move (no pure-random anchor needed).
    let ladder = [
        Cfg { name: "p8 t3 k6",   playouts: 8,    top_k: 6, temp: 3.0 },
        Cfg { name: "p30 t2 k5",  playouts: 30,   top_k: 5, temp: 2.0 },
        Cfg { name: "p100 t1 k4", playouts: 100,  top_k: 4, temp: 1.0 },
        Cfg { name: "p300 t.6 k3",playouts: 300,  top_k: 3, temp: 0.6 },
        Cfg { name: "p1000 t.3",  playouts: 1000, top_k: 2, temp: 0.3 },
        Cfg { name: "p4000 t0",   playouts: 4000, top_k: 1, temp: 0.0 },
    ];
    let n = 40;
    tournament("Tic-Tac-Toe 3x3", &|| Box::new(TicTacToeWasm::new(3, 3, 3, 2)), &ladder, n);
    tournament("Connect Four 7x6", &|| Box::new(ConnectFourWasm::new(7, 6, 4, 2)), &ladder, n);
    tournament("Gomoku 9x9 k5", &|| Box::new(TicTacToeWasm::new(9, 9, 5, 2)), &ladder, n);
    tournament("Reversi 6x6", &|| Box::new(ReversiWasm::new(6, 6)), &ladder, n);
    tournament("Hex 7x7", &|| Box::new(HexWasm::new(7)), &ladder, n);
    tournament("Frontline 6x6", &|| Box::new(FrontlineWasm::new(6, 6)), &ladder, n);
}
```

- [ ] **Step 2: Build and run it**

Run: `cargo run --release --example calibrate -p treant-wasm 2>&1 | tail -80`
Expected: one win-matrix block per game. Reading guide: each game's "field %" column orders the ladder by strength; the "ladder steps" lines flag where adjacent configs differ by ≥70% (a real, perceptible step) vs ≤58% (a wasted notch).

- [ ] **Step 3: If a step says "(no real change)"** for low-playout configs in tiny/solved games (expected for Tic-Tac-Toe — the solver makes search irrelevant), that confirms the diagnosis: for those games only `temp` and `top_k` move strength. Note which games are search-sensitive vs temperature-only.

- [ ] **Step 4: Record the results** in `plans/ai-calibration-results.md`. For each game pick three ladder rungs that are each separated by a "real step", with the **lowest** rung as Easy (so a casual human wins a fair share) and a Hard rung under ~1s/move. Template (fill the real measured choices):

```markdown
# AI calibration results (run: <date>, seed 0xC0FFEE, n=40/pair)

| Game | Easy {playouts,topK,temp} | Medium | Hard | notes |
|---|---|---|---|---|
| tic-tac-toe | {8,6,3.0} | {100,4,1.0} | {2000,1,0.0} | solver: search ~irrelevant, temp is the lever |
| connect-four | … | … | … | search-sensitive |
| gomoku | … | … | … | … |
| reversi | … | … | … | … |
| hex | … | … | … | … |
| frontline | … | … | … | … |
```

- [ ] **Step 5: Commit** (the harness is a kept research tool; the results doc is the data artifact):

```bash
git add treant-wasm/examples/calibrate.rs plans/ai-calibration-results.md
git commit -m "feat(difficulty): self-play calibration harness + measured per-game numbers"
```

---

## Task 4: Wire the new mechanism into the arcade `pickAiMove`

**Files:**
- Modify: `docs/src/components/arcade/gameTypes.ts`
- Modify: `docs/src/components/arcade/games/frontline.tsx` (`moveHandle`), `docs/src/components/arcade/games/gridpack.tsx` (`cellHandle`, `ocHandle`), and the per-game `makeHandle` factories in `nim.tsx`, `pig.tsx`, `euclid.tsx`, `subtractsquare.tsx`, `mancala.tsx`, `game2048.tsx`, `connectFour.tsx`, `ticTacToe.tsx`, `gomoku.tsx`, `hex.tsx`, `shift.tsx`, `reversi.tsx`, `dotsboxes.tsx`, `capturego.tsx`, `trails.tsx`, etc.
- Modify: `docs/src/components/arcade/useGameSession.ts`

- [ ] **Step 1: Add `weakMove` to the `GameHandle` interface** in `gameTypes.ts` (inside `export interface GameHandle { ... }`, after `legalMoves()`):

```ts
  /** Difficulty-aware move (value-aware top-K visit temperature). Optional:
   *  handles without it fall back to playout+best in pickAiMove. */
  weakMove?(playouts: number, topK: number, temp: number, seed: number): string | undefined;
```

- [ ] **Step 2: Add the `AiConfig` model** in `gameTypes.ts` — replace the existing `DIFFICULTY` block. First add the types/defaults:

```ts
/** Concrete AI knobs for one difficulty level. */
export interface AiConfig {
  playouts: number;
  topK: number;
  temp: number;
}

/** Fallback ladder for games without a calibrated override. Placed LOW on the
 *  simulation curve and weakened with top-K visit temperature, not epsilon. */
export const DEFAULT_DIFFICULTY: Record<Difficulty, AiConfig> = {
  easy: { playouts: 40, topK: 6, temp: 1.6 },
  medium: { playouts: 500, topK: 3, temp: 0.5 },
  hard: { playouts: 5000, topK: 1, temp: 0.0 },
};

/** Resolve the AiConfig for a game + level (per-game override wins). */
export function aiConfig(def: GameDefinition, d: Difficulty): AiConfig {
  return def.difficulty?.[d] ?? DEFAULT_DIFFICULTY[d];
}
```

Keep the emoji/label metadata the UI needs (the DifficultyPicker was already removed; `GameSetup`'s per-seat picker uses `DIFFICULTY[d].label`/`.emoji`). So **retain** a metadata table:

```ts
export const DIFFICULTY: Record<Difficulty, { label: string; emoji: string }> = {
  easy: { label: 'Easy', emoji: '😊' },
  medium: { label: 'Medium', emoji: '😎' },
  hard: { label: 'Hard', emoji: '🔥' },
};
```

- [ ] **Step 3: Rewrite `pickAiMove`** in `gameTypes.ts` to take an `AiConfig`:

```ts
/**
 * Choose an AI move under an AiConfig. Uses the engine's value-aware weakening
 * (weakMove) when available; otherwise a graceful fallback (random for the
 * weakest setting, else playout+best).
 */
export function pickAiMove(
  handle: GameHandle,
  cfg: AiConfig,
  rng: () => number = Math.random,
): string | undefined {
  const legal = handle.legalMoves();
  if (legal.length === 0) return undefined;
  if (cfg.playouts > 0 && handle.weakMove) {
    const seed = Math.floor(rng() * 0xffffffff) >>> 0;
    const mv = handle.weakMove(cfg.playouts, cfg.topK, cfg.temp, seed);
    if (mv != null) return mv;
  }
  if (cfg.playouts === 0 || !handle.weakMove) {
    return legal[Math.floor(rng() * legal.length)];
  }
  handle.playoutN(cfg.playouts);
  return handle.bestMove() ?? legal[0];
}
```

- [ ] **Step 4: Add `difficulty?` to `GameDefinition`** in `gameTypes.ts` (inside the interface, after `rules?`):

```ts
  /** Per-game calibrated AI knobs per level (else DEFAULT_DIFFICULTY). */
  difficulty?: Record<Difficulty, AiConfig>;
```

- [ ] **Step 5: Add `weakMove` to the shared handles.** In `frontline.tsx` `moveHandle` (the returned object), add:

```ts
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
```

In `gridpack.tsx` `cellHandle` and `ocHandle` add the same line. In every per-game `makeHandle` (e.g. `nim.tsx`, `pig.tsx`, `euclid.tsx`, `subtractsquare.tsx`, `mancala.tsx`, `connectFour.tsx`, `ticTacToe.tsx`, `gomoku.tsx` (uses `cellHandle`), `hex.tsx`, `shift.tsx`, `reversi.tsx` (uses `moveHandle`), etc.) add the same `weakMove` line to the returned handle object. (Handles that wrap a wasm object as `g` all gain one line.)

- [ ] **Step 6: Update `useGameSession.ts`** to resolve the config. Change the import line:

```ts
import { aiConfig, pickAiMove } from './gameTypes';
```

and in `runAiTurn`, replace the `pickAiMove(h, kind)` call:

```ts
      const mv = def.solo ? (h.playoutN(SOLO_AI_PLAYOUTS), h.bestMove()) : pickAiMove(h, aiConfig(def, kind));
```

(`kind` is the seat's `Difficulty`; `def` is in scope.)

- [ ] **Step 7: Build the WASM and the site to typecheck**

```bash
cd treant-wasm && wasm-pack build --target web 2>&1 | tail -2
cd ../docs && npm install treant-wasm 2>&1 | tail -1 && git checkout package.json
npm run build 2>&1 | tail -4
```
Expected: `[SUCCESS] Generated static files`.

- [ ] **Step 8: Commit**

```bash
cd /home/peter/code/treant
git add treant-wasm/pkg 2>/dev/null; git add docs/src/components/arcade/gameTypes.ts docs/src/components/arcade/useGameSession.ts docs/src/components/arcade/games/*.tsx
git commit -m "feat(arcade): drive AI from per-game AiConfig via engine weak_move"
```

---

## Task 5: Apply the measured per-game difficulty overrides

**Files:**
- Modify: the calibrated games' defs — `connectFour.tsx`, `ticTacToe.tsx`, `gomoku.tsx`, `reversi.tsx`, `hex.tsx`, `frontline.tsx` (read the chosen rows from `plans/ai-calibration-results.md`).

- [ ] **Step 1: Add a `difficulty:` field to each calibrated game's `GameDefinition`**, using the three rungs picked in Task 3 Step 4. Example for `connectFour.tsx` (replace the numbers with the *measured* ones from the results doc):

```ts
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2.0 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 4000, topK: 1, temp: 0.0 },
  },
```

Place it right after `rules:` (or `blurb:`). Repeat per calibrated game with that game's measured numbers.

- [ ] **Step 2: Typecheck**

Run: `cd docs && npm run build 2>&1 | tail -3`
Expected: SUCCESS.

- [ ] **Step 3: Commit**

```bash
git add docs/src/components/arcade/games/connectFour.tsx docs/src/components/arcade/games/ticTacToe.tsx docs/src/components/arcade/games/gomoku.tsx docs/src/components/arcade/games/reversi.tsx docs/src/components/arcade/games/hex.tsx docs/src/components/arcade/games/frontline.tsx
git commit -m "feat(arcade): calibrated per-game Easy/Medium/Hard from self-play measurement"
```

---

## Task 6: Browser verification

**Files:** none (manual + Playwright DOM checks against the dev server on `0.0.0.0:3939`).

- [ ] **Step 1: Restart the dev server** (WASM changed):

```bash
pkill -f "docusaurus start"; sleep 1
cd docs && npm start -- --host 0.0.0.0 --port 3939   # run in background
```
Wait for HTTP 200 on `http://localhost:3939/arcade`.

- [ ] **Step 2: Beat "Easy" Connect Four as a human-proxy.** Drive Playwright: open `?game=connect-four`, start vs-AI **Easy**, and have the script play a deliberately simple strategy (e.g., always drop in the leftmost legal column). Play to the end.
Expected: across ~5 such games the human-proxy wins a meaningful share (target ≳40%). Record the win count. If Easy still wins ≳80%, its `playouts`/`temp` are too strong — lower playouts / raise temp in `connectFour.tsx` and re-verify.

- [ ] **Step 3: Confirm the levels are actually different.** Watch-AI Easy-vs-Hard is not directly selectable per seat in one game, but use the per-seat setup: start a game with **Player 1 = Easy AI, Player 2 = Hard AI** (Customize players), Watch. Run 6 games.
Expected: Hard wins the clear majority (≳70%). If ~50/50, the calibration rungs are too close — pick wider-separated rungs from the results doc.

- [ ] **Step 4: Console + regression**

Run a DOM check: 0 console errors during a full game on 3 games (Connect Four, Tic-Tac-Toe, Gomoku). Confirm `cargo test -p treant-wasm` and `cargo clippy --workspace` are still green.

- [ ] **Step 5: Commit any tuning adjustments** made in Steps 2–3:

```bash
git add docs/src/components/arcade/games/*.tsx
git commit -m "fix(arcade): tune calibrated difficulty rungs after human-proxy verification"
```

---

## Notes for the implementer

- **Do not push.** Commits stay local (a push triggers a prod deploy). The human pushes when ready.
- **`npm install treant-wasm` re-alphabetizes `docs/package.json`** — run `git checkout docs/package.json` after each install.
- **Solver games (Tic-Tac-Toe, Nim, etc.):** expect the harness to show that playout count barely matters; their Easy rung must rely on high `temp` + larger `topK` (occasionally pick a 2nd/3rd-best *trusted* move) so a child can win. Tic-Tac-Toe is a *draw* under perfect play, so "Easy" there means "lets you draw, and sometimes errs so you can win," not "lets you crush it."
- **The `temp` knob is monotone but game-relative.** Don't assume one temp transfers across games — that's exactly why Task 3 measures each game.
- **Rollout coverage:** Tasks 4–5 calibrate a representative six. Uncalibrated games use `DEFAULT_DIFFICULTY`, which is already far easier than today's 200/2000/10000. Calibrating the rest is a repeat of Task 3 (add them to the harness `tournament(...)` calls) + Task 5 (paste numbers) — no new mechanism.
