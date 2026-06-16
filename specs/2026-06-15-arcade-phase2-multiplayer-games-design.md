# Phase 2 — Arcade: the multiplayer games (Nim, Mancala, Shift)

**Date:** 2026-06-15
**Status:** Approved scope (multiplayer-first), ready for implementation
**Depends on:** Phase 1 (arcade foundation: `GameDefinition`, `useGameSession`, screens). Complete.

> Spec in repo-root `specs/` (not `docs/`, the Docusaurus site).

## Context & scope

Phase 1 shipped the arcade with two grid games. Phase 2 adds the three remaining
**two-player tap games** — **Nim**, **Mancala**, **Shift** — as arcade
`GameDefinition`s wrapping their existing WASM classes (`NimWasm`, `MancalaWasm`,
`ShiftWasm`). They reuse the existing modes (pass-and-play / vs-AI / watch) and
epsilon-greedy difficulty.

**Out of scope (Phase 2b):** 2048 and Dice. They are single-player and need a new
*solo* mode + swipe/roll input; deferred to keep this phase tight.

**Not doing:** folding Shift into the `treant-games` grid engine (a separate
backend cleanup). Phase 2 wraps the existing `ShiftWasm` as-is (it already has
the draw fix from this session).

## The problem: the Phase-1 abstraction was grid-shaped

Three things in Phase 1 assumed grid placement games and must generalize:

1. **`GameParams` is `{cols, rows, k, numPlayers}`** — but Nim is `{stones}`,
   Mancala is `{pits, stones, numPlayers}`, Shift is `{cols, rows, k, numPlayers,
   pieces}`.
2. **`GameDefinition.legalMoves(board, params)`** is player-independent — but
   Mancala/Nim legal moves depend on the *current player* and live state.
3. **`GameDefinition.renderBoard(props) => JSX`** is a pure render call — but Shift
   needs local UI state (selected piece) for its two-tap move.

### Abstraction changes (in `docs/src/components/arcade/`)

**a. Generalize `GameParams`** (`gameTypes.ts`):
```ts
export type GameParams = { numPlayers: number } & Record<string, number>;
```
`numPlayers` stays a required field (the mode/seat system needs it); all other
knobs (cols, rows, k, pits, stones, pieces) are game-specific entries. The CF/TTT
defs already only read `cols/rows/k/numPlayers`, so they keep working unchanged.

**b. Move `legalMoves` onto `GameHandle`** (it has live state):
```ts
interface GameHandle {
  /* …existing… */
  legalMoves(): string[];   // current player's legal move strings
}
```
Remove `GameDefinition.legalMoves`. `pickAiMove` now calls `handle.legalMoves()`
instead of `def.legalMoves(handle.getBoard(), params)`. Grid adapters implement it
by deriving from the board (empty cells / non-full columns); Mancala uses the
WASM `legal_moves()`; Nim derives from stone count.

**c. `renderBoard` → a `Board` component** so it can hold hooks/state:
```ts
interface GameDefinition {
  /* …existing… */
  Board: React.ComponentType<BoardProps>;   // was: renderBoard(props) => JSX
}
```
`GamePlay` renders `<def.Board {...props} />` instead of calling
`def.renderBoard(props)`. CF/TTT convert their render functions to components
(mechanical — they use no hooks). Shift's `Board` uses `useState` for the
selected piece.

## Result-string normalization (the adapters' job)

Each WASM reports terminal results differently. Every adapter's `result()` returns
the **common form**: a **1-indexed winner number string** (`"1".."4"`), `"Draw"`,
or `""`. (`GamePlay.winnerLabel` already maps `"1"→Red`, `"2"→Yellow`, etc.)

| Game | WASM `result()` | Adapter normalization |
|------|-----------------|-----------------------|
| Connect Four / Tic-Tac-Toe | `"1".."4"` / `"Draw"` / `""` | pass through |
| Shift | `"1".."4"` / `"Draw"` / `""` (draw fix landed) | pass through |
| Mancala | `"P1".."P4"` / `"Draw"` / `""` | strip leading `P` |
| Nim | *(none)* | when `is_terminal()` (stones 0), winner = the **non-current** player (normal play, last-to-take wins): `current_player()=="P1" → "2"`, `"P2" → "1"` |

`currentPlayer()` normalization: Mancala returns `u32` (pass through); Nim returns
`"P1"/"P2"` → map to `0/1`.

## Per-game definitions

### Nim (`games/nim.tsx`)
- **Fixed 2 players.** `defaultParams { numPlayers: 2, stones: 15 }`.
- **Knobs:** `stones` 3–30 (no players knob).
- **Presets:** Classic-15, Quick-7, Marathon-25.
- **Handle (over `NimWasm(stones)`):**
  - `getBoard()` → `String(current_stones())` (synthetic; the Board renders that
    many stone icons).
  - `currentPlayer()` → `"P1"→0, "P2"→1`.
  - `legalMoves()` → `stones>=2 ? ["Take1","Take2"] : stones===1 ? ["Take1"] : []`.
  - `applyMove("Take1"|"Take2")`, `bestMove()`, `playoutN`, `isTerminal()`
    (`current_stones()===0`), `result()` per table above, `free()`.
- **`Board`:** a pile of `stones` stone icons + two big buttons **"Take 1"** /
  **"Take 2"** (Take-2 disabled when only 1 left), enabled only when
  `interactive`. Calls `onMove("Take1"|"Take2")`.

### Mancala (`games/mancala.tsx`)
- **`defaultParams { pits: 6, stones: 4, numPlayers: 2 }`.**
- **Knobs:** `pits` 3–8, `stones` 2–6, `numPlayers` 2–4.
- **Presets:** Kalah (6,4,2), Quick (3,3,2), 4-Player Ring (4,3,4).
- **Handle (over `MancalaWasm(pits, stones, numPlayers)`):**
  - `getBoard()` → WASM `get_board()` (comma-separated counts in ring order:
    P0 pits, P0 store, P1 pits, P1 store, …).
  - `currentPlayer()` → `u32`.
  - `legalMoves()` → split WASM `legal_moves()` on `,` (current player's non-empty
    local pit indices); `[]` if empty.
  - `applyMove(localPit)`, `bestMove()`, `isTerminal()`, `result()` strips `P`,
    plus a `scores()` passthrough for the Board, `free()`.
- **Bonus turns:** handled for free — `useGameSession` re-reads `currentPlayer()`
  after every move, so a Kalah "land-in-your-store, go again" keeps the same seat
  (human stays interactive; AI chains via `runAiTurn`).
- **`Board`:** two (or N) rows of pit buttons with stone counts + each player's
  store; current player's non-empty pits are tappable (`onMove(localPit)`). Show
  the score per player. (Parse `getBoard()` into pits/stores by ring layout from
  `params.pits`/`numPlayers`.)

### Shift (`games/shift.tsx`)
- **`defaultParams { cols: 3, rows: 3, k: 3, numPlayers: 2, pieces: 3 }`.**
- **Knobs:** `cols` 2–8, `rows` 2–8, `k` 2–6, `numPlayers` 2–4, `pieces` 1–4.
  (Constructor clamps `pieces ≤ cols*rows/numPlayers`.)
- **Presets:** Classic (3,3,3,2,3), Big (5,4,4,2,4), 3-Player (4,4,3,3,2).
- **Handle (over `ShiftWasm(cols,rows,k,numPlayers,pieces)`):**
  - `getBoard()` (row-major symbols), `currentPlayer()` u32, `isTerminal()`,
    `result()` (pass through; draw works), `bestMove()` (returns `P{i}`/`M{f},{t}`),
    `applyMove`, `playoutN`, `free()`.
  - extra: `inPlacementPhase()` passthrough; `legalMoves()` — derive from the board
    + phase: placement → `P{i}` for each empty cell; movement → `M{from},{to}` for
    each of the current player's pieces × each empty cell. (Needed for
    epsilon-random and to disable illegal taps.)
- **`Board` (two-phase, holds `selectedPiece` state):**
  - **Placement phase** (`inPlacementPhase()`): tap an empty cell → `onMove("P"+i)`.
  - **Movement phase:** tap your own piece → select (highlight); tap an empty cell
    → `onMove("M"+sel+","+i)` and clear selection; tap your other piece →
    re-select; tap selected piece → deselect. (Mirrors the working playground
    logic, but with clear highlight + only empty cells as legal targets.)
  - A small caption tells the player which phase they're in ("Place your pieces" /
    "Slide a piece to an empty square").

## Registry, modes, difficulty (reused)
- `games/index.ts`: `GAMES = [connectFour, ticTacToe, nim, mancala, shift]`. The
  launcher shows all five tiles + the "More soon" tile (now just 2048/Dice).
- Modes: all three are multiplayer → `pvp/pvai/aivai` unchanged. Nim is 2-player;
  Mancala/Shift expose the players knob.
- Difficulty: reuse `DIFFICULTY` (epsilon-greedy). Works for all three (Nim's
  solver makes "Hard" perfect; that's fine/intended).

## Testing
No JS unit runner; verify as in Phase 1:
- **Pure helpers:** the result/`currentPlayer` normalizations and `legalMoves`
  derivations are small pure functions, spot-checked via a Playwright
  `browser_evaluate` probe.
- **End-to-end (Playwright, dev server + fresh WASM), no console errors:**
  - Nim **pass-and-play**: take stones to 0 → overlay names the correct winner.
  - Mancala **vs-AI**: human sows a pit → AI replies; a bonus-turn move keeps the
    human's turn.
  - Shift **pass-and-play**: place all pieces (phase flips to movement) → select &
    slide a piece → it moves; fill a small board → draw overlay.
  - A **wild preset** for each loads and is playable.
- Rust/WASM already covered (incl. the new Shift draw tests).

## Out of scope (later)
- 2048, Dice, the solo mode, swipe/roll input (Phase 2b).
- Folding Shift into the `treant-games` engine.
- Landing page / polish / animations / sound (Phase 3).

## Risks
- **Mancala board parsing** (ring layout → per-player pits/stores) is the fiddliest
  bit; the WASM `get_board()` doc string pins the order, and `scores()` cross-checks
  store counts. Mitigated by an e2e that plays real moves.
- **Shift two-tap on touch**: ensure the selected piece is clearly highlighted and
  only empty cells are valid targets, so a mis-tap re-selects rather than silently
  failing (the original UX confusion).
- **`GameParams` generalization** could loosen typing; keep `numPlayers` required
  and treat the rest as named numeric knobs validated by each def's `knobs` bounds.
