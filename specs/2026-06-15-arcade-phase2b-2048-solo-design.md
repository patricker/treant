# Phase 2b — Arcade: 2048 + solo mode

**Date:** 2026-06-15
**Status:** Approved scope (2048-only), ready for implementation
**Depends on:** Phase 2 (arcade foundation + 5 games, generalized `GameDefinition`). Complete.

> Spec in repo-root `specs/` (not `docs/`, the Docusaurus site).

## Context & scope

Phase 2 added the multiplayer games. Phase 2b adds **2048**, the first
*single-player* arcade game, and the **solo-mode** abstraction it needs.

**Dropped: Dice.** As built it is not a playable game — `DiceGameWasm` exposes no
`apply_move`/`is_terminal`/`best_move` (it's a search-visualization demo), and the
rules have no real decision (`Roll` adds 1–6 with no bust, game ends at score ≥ 20,
objective = maximize score → *always roll*; `Stop` is strictly dominated). Making
it a game would need both WASM interactivity and a rules redesign (a bust/target).
Out of scope; it stays the chance-node demo in the Playground.

**This phase is JS-only** — 2048's WASM is complete and the solo AI uses
`best_move()` directly, so no WASM rebuild.

## The new concept: solo games

2048 is single-player (`numPlayers = 1`): no opponent, no turn-taking against
someone. The two sensible modes are **You play** (you make the moves, with an
optional **Hint**) and **Watch AI** (the AI plays it — a great showcase). vs-AI
and pass-and-play don't apply.

### Abstraction additions (`docs/src/components/arcade/`)

**a. `GameDefinition.solo?: boolean`** (`gameTypes.ts`). 2048 sets `solo: true`.

**b. New `Mode` value `'solo'`** = "you play". `seatTypes('solo', n)` →
`Array(n).fill('human')` (= `['human']` for solo). "Watch AI" reuses `'aivai'`
(→ `['ai']`). Add `'solo'` to the `Mode` union and the `seatTypes` switch.

**c. Optional solo display hooks on `GameHandle`** (backward-compatible — other
games don't implement them):
```ts
interface GameHandle {
  /* …existing… */
  statusText?(): string;  // live status, e.g. "Score 1234 · Best 128"
  endText?(): string;     // game-over text, e.g. "Game over — score 1234"
}
```

**d. `useGameSession`** gains:
- `statusText` / `endText` in its return (computed from the handle's optional
  hooks on each sync; `''` when absent).
- `getHint(): string | undefined` — `h.playoutN(HINT_PLAYOUTS); return h.bestMove()`.
- **Solo AI strength is fixed** (no epsilon — random 2048 moves are just bad). In
  `runAiTurn`, when `def.solo`: `h.playoutN(SOLO_AI_PLAYOUTS); h.bestMove()`
  (bypass `pickAiMove`'s epsilon path). `SOLO_AI_PLAYOUTS ≈ 800`,
  `HINT_PLAYOUTS ≈ 1500`.

**e. `GameSetup`** — for `def.solo`, render a **2-option** mode toggle
("🙂 You play" = `solo`, "🤖 Watch AI" = `aivai`), default `solo`, and **no
difficulty picker**. Non-solo games keep the 3-way picker + difficulty.

**f. `GamePlay`** — solo branches:
- **Banner:** if `def.solo` and `statusText`, show `statusText` instead of the
  turn banner.
- **Hint button:** when `def.solo && mode === 'solo' && phase === 'playing'`, show
  a **💡 Hint** button; clicking calls `getHint()` and shows the suggestion
  briefly (e.g. "Try ⬆️"). Move→label via optional `def.formatHint?(move)`.
- **Overlay:** if `def.solo`, show `endText` (and a 🎮/🏆 icon) instead of
  `winnerLabel(result)`; keep Play again / Change setup / Arcade.

## 2048 game (`games/game2048.tsx`)

- **No params** beyond `numPlayers: 1`. `defaultParams { numPlayers: 1 }`; **no
  knobs, no presets** (fixed 4×4). `solo: true`.
- **Handle (over `Game2048Wasm()`):**
  - `getBoard()` → `g.get_board()` returns a flat 16-number JS array; the adapter
    returns `arr.join(',')` (board is a `string` in the common interface).
  - `currentPlayer()` → `0`.
  - `isTerminal()` → `g.is_terminal()`.
  - `applyMove(dir)` → `g.apply_move(dir)` (dir ∈ `"Up"|"Down"|"Left"|"Right"`;
    returns `false` for a no-op direction — the session ignores it, so illegal
    swipes are harmless).
  - `bestMove()` → `g.best_move()` (a direction).
  - `playoutN`, `free`.
  - `result()` → `''` (solo has no winner; overlay uses `endText`).
  - `legalMoves()` → `['Up','Down','Left','Right']` (unused in the solo flow, but
    satisfies the interface).
  - `statusText()` → `Score ${g.score()} · Best ${g.max_tile()}`.
  - `endText()` → `g.max_tile() >= 2048 ? '🎉 You made ' + g.max_tile() + '! Score ' + g.score() : 'Game over — score ' + g.score() + ', best tile ' + g.max_tile()`.
- **`Board`:** a 4×4 grid of tiles parsed from the comma board (`0` = empty), each
  tile colored by value (standard 2048 ramp). Input, all three (only when
  `interactive`):
  - **on-screen arrow buttons** (⬆️⬇️⬅️➡️) → `onMove(dir)` — primary mobile control.
  - **keyboard** arrows (desktop) via a `useEffect` keydown listener.
  - **swipe** (touchstart/touchend on the grid; dominant axis → direction).
- **`formatHint(move)`** → arrow + word, e.g. `"Up" → "⬆️ Up"`.

## Registry & launcher
- `games/index.ts`: add `game2048` to `GAMES` (after `shift`). Six games total.
- Launcher: drop the "More soon" tile (nothing concrete pending) or keep it
  generic — implementer's call; prefer dropping it.

## Testing
JS-only; verify as before (dev server + existing WASM, no console errors):
- **2048 You-play:** arrow/swipe moves change the board and the score climbs;
  reaching a dead board shows the game-over overlay with the final score.
- **2048 Watch-AI:** the AI auto-plays, score climbs, tiles merge upward.
- **Hint:** in You-play, the Hint button shows a direction suggestion.
- **Other games unaffected** (the `GameHandle`/`useGameSession` additions are
  optional/additive): a quick Connect Four vs-AI still works.

## Out of scope
- Dice (dropped; needs WASM + rules redesign).
- Solo difficulty levels (fixed AI strength this phase).
- 2048 variants/knobs (fixed 4×4).
- Phase 3 polish (animations, sound, landing).

## Risks
- **Swipe gesture conflicts** with page scroll on mobile — call
  `preventDefault` on the board's touch handlers / set `touch-action: none` on the
  grid so swipes don't scroll the page.
- **Watch-AI move cadence**: 800 playouts × a long 2048 game could feel slow;
  the ~400ms delay makes it watchable, and 800 keeps each move fast. Tune if
  needed.
- **Additions must not break the 5 existing games** — `solo`, `statusText`,
  `endText`, `formatHint` are all optional; the non-solo code paths are unchanged.
