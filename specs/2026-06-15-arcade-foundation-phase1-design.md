# Phase 1 — Arcade foundation (mobile game arcade + two grid games)

**Date:** 2026-06-15
**Status:** Approved (design settled during brainstorming), ready for implementation
**Depends on:** Phase 0 (`treant-games` grid engine + facade WASM classes) — complete.

> Spec in repo-root `specs/` (not `docs/`, which is the Docusaurus site).

## Context

The treant docs "playground" demos turned out to be genuinely fun pass-the-phone
family games. Phase 1 turns that into a real **mobile-first game arcade** at
`/arcade` on mcts.dev (static GitHub Pages, WASM AI, no backend, no accounts),
proven end-to-end on the two Phase-0 grid games (Connect Four, Tic-Tac-Toe). It
establishes the reusable shell (`useGameSession` + `GameDefinition`) so Phase 2
(porting the remaining games) is mechanical.

### Settled decisions (from brainstorming)
- **Public destination**, same site/domain (mcts.dev), no accounts, static only.
- **Aesthetic A — playful arcade**: bright, chunky, emoji, big touch targets.
  Scoped to `/arcade` so the docs keep their restrained look.
- **Locked flow**: set up a game (mode + difficulty + rules) → it loads → play to
  completion. **No mid-game rule/mode changes.** Game over → replay / change
  setup / back to arcade.
- **Quick-play + wild presets**: setup opens with a one-tap Classic start, a shelf
  of tappable wild presets, and a Custom panel of full sliders.
- **Modes**: Human-v-Human (pass-and-play), Human-v-AI (with difficulty),
  AI-v-AI (watch). Solo-puzzle deferred (grid games are all 2+ player).
- **Difficulty** lives in the JS layer (no Rust changes): epsilon-greedy over the
  WASM `best_move()` after N playouts.

## Architecture

All client-side React inside the existing Docusaurus site. New route `/arcade`;
the pedagogical `/playground` demos are untouched.

### New files (`docs/src/components/arcade/`)
- `gameTypes.ts` — interfaces + pure helpers (see below).
- `useGameSession.ts` — the shared game-driver hook.
- `games/connectFour.tsx` — `GameDefinition` for Connect Four.
- `games/ticTacToe.tsx` — `GameDefinition` for Tic-Tac-Toe.
- `games/index.ts` — the game registry (array of `GameDefinition`).
- `ArcadeShell.tsx` — screen state machine (`launcher | setup | playing`).
- `Launcher.tsx`, `GameSetup.tsx`, `GamePlay.tsx` — the three screens.
- `ModePicker.tsx`, `DifficultyPicker.tsx`, `PresetChips.tsx`, `CustomKnobs.tsx` — shared controls.
- `arcade.module.css` — the playful skin.
- `docs/src/pages/arcade.tsx` — route; wraps `ArcadeShell` in `WasmProvider` + `BrowserOnly`.

Reuses the existing `components/treant/WasmProvider`.

### `GameDefinition` (the reusable boundary)
```ts
type GameParams = { cols: number; rows: number; k: number; numPlayers: number };

interface Preset { label: string; params: GameParams; }
interface Knob { key: keyof GameParams; label: string; min: number; max: number; step: number; }

interface GameHandle {            // thin uniform wrapper over the WASM class
  applyMove(move: string): boolean;
  getBoard(): string;
  currentPlayer(): number;        // 0-indexed seat
  isTerminal(): boolean;
  result(): string;               // "1".."4" winner (1-indexed), "Draw", or ""
  bestMove(): string | undefined;
  playoutN(n: number): void;
  free(): void;
}

interface GameDefinition {
  id: string;
  name: string;
  icon: string;                   // emoji
  blurb: string;
  defaultParams: GameParams;
  presets: Preset[];
  knobs: Knob[];                  // bounds for the Custom panel
  create(wasm: WasmModule, p: GameParams): GameHandle;
  legalMoves(board: string, p: GameParams): string[];   // for epsilon-random
  renderBoard(props: BoardProps): JSX.Element;          // per-game board + input
}
```
`legalMoves` is derived from the board string per game (empty cells for
placement; non-full columns for gravity), so the AI can pick a random legal move
without new WASM API.

### Modes → seat assignment (any player count)
```ts
type Mode = 'pvp' | 'pvai' | 'aivai';
function seatTypes(mode: Mode, numPlayers: number): ('human'|'ai')[] {
  if (mode === 'pvp')   return Array(numPlayers).fill('human');
  if (mode === 'aivai') return Array(numPlayers).fill('ai');
  return ['human', ...Array(numPlayers - 1).fill('ai')]; // pvai: seat 0 human
}
```

### Difficulty (per AI seat, uniform in v1)
```ts
type Difficulty = 'easy' | 'medium' | 'hard';
const DIFFICULTY = {
  easy:   { playouts: 200,   epsilon: 0.5 },
  medium: { playouts: 2000,  epsilon: 0.1 },
  hard:   { playouts: 10000, epsilon: 0.0 },
};
```
AI move = with probability `epsilon` pick a uniform random legal move; else
`playoutN(playouts)` then `bestMove()`. Epsilon-random is what actually lets a
kid beat "Easy" (low playouts alone still plays tiny games near-perfectly).

### `useGameSession` (the driver)
Owns the `GameHandle`, the seat-type array, and difficulty. Generalizes the loop
currently copy-pasted in `ConnectFourDemo`:
- State: `board`, `currentPlayer`, `phase` (`'playing' | 'thinking' | 'over'`),
  `result`.
- `onHumanMove(move)`: ignore unless current seat is human and not over; apply,
  sync; if not terminal and next seat is AI, schedule the AI turn.
- AI turn: after a short delay (~400ms, for visible "thinking"), pick a move
  (epsilon-greedy), apply, sync, and **chain** if the next seat is also AI
  (watch mode / multi-AI), with a delay between each.
- `reset()`: rebuild the handle from the same params (replay) — config is
  immutable mid-game.
- Frees the WASM handle on unmount / rebuild.

### Screens (state machine in `ArcadeShell`)
- **launcher** — grid of game tiles (the 2 grid games; others shown as
  "coming soon"). Tap → setup for that game.
- **setup** — `ModePicker` (👥/🤖/👀), `DifficultyPicker` (shown only when the
  mode has AI), `PresetChips` (Classic + wild presets), `CustomKnobs`
  (cols/rows/k/players within bounds), and a big **Start**. Picking a preset sets
  params; Custom edits params; Start locks them.
- **playing** — `GamePlay` renders the game's board via `renderBoard`, a
  whose-turn banner, a `⋯` quit/restart (confirm → back to setup), and on
  terminal an overlay: result + **Play again** (same params) / **Change setup** /
  **Arcade**.

### Presets (respecting engine clamps: dims 2–10, **players 2–4**, k ≤ max dim)
- **Connect Four** (`defaultParams {7,6,4,2}`):
  - Classic `{7,6,4,2}` · Connect-5 `{9,7,5,2}` · 4-Player Frenzy `{9,8,4,4}` · Giant `{10,10,5,2}`
- **Tic-Tac-Toe** (`defaultParams {3,3,3,2}`):
  - Classic `{3,3,3,2}` · Gomoku-lite `{9,9,5,2}` · Big Board `{6,6,4,2}` · 3-Player `{6,6,4,3}`

Knob bounds: Connect Four cols/rows 3–10, k 3–10, players 2–4; Tic-Tac-Toe
cols/rows 2–10, k 2–10, players 2–4. (k is auto-clamped by the WASM to ≤ max dim.)

## Testing

No JS unit-test runner exists in the docs package; introducing one is out of
scope. Verification is:
- **Pure helpers** (`seatTypes`, `pickAiMove`/epsilon-greedy, each game's
  `legalMoves`, difficulty mapping) are written as exported pure functions and
  exercised via a Playwright `browser_evaluate` probe (deterministic by injecting
  a fixed RNG where randomness matters).
- **End-to-end (Playwright)** against a dev server with freshly-built WASM:
  - `/arcade` launcher renders both game tiles.
  - Connect Four, **pass-and-play**: play a scripted line to a win → overlay shows
    a winner.
  - Connect Four, **vs-AI Easy**: human move → AI responds with a legal move.
  - Tic-Tac-Toe, **watch (AI-v-AI)**: both seats auto-play to a terminal.
  - A **wild preset** (Connect-5 / Gomoku-lite) loads and is playable.
- **No console errors** during any of the above.

## Out of scope (later phases)
- Shift / Mancala / Nim / 2048 / Dice (Phase 2); solo-puzzle mode.
- Landing page, SEO, share/deep links, sound, elaborate animations (Phase 3).
- Per-seat individual difficulty; online multiplayer (never — static).
- Replacing or removing the existing `/playground` pedagogical demos.

## Risks
- **Playful skin clashing with docs theme** — mitigated by scoping all arcade CSS
  to `arcade.module.css` / the `/arcade` route.
- **AI blocking the UI thread** during high-playout "Hard" turns — mitigated by
  the deferred (`setTimeout`) AI turn + a visible "thinking" state; same approach
  the current demos already use acceptably.
- **Over-building the shell before Phase 2 games exist** — mitigated by keeping
  `GameDefinition` minimal (only what CF + TTT need) and adding fields when a real
  Phase-2 game requires them.
