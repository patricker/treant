# Arcade Phase 2 (Nim, Mancala, Shift) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add Nim, Mancala, and Shift to the arcade as `GameDefinition`s over their existing WASM classes, after generalizing the Phase-1 abstraction (params, legal-moves, board-as-component).

**Architecture:** Generalize `GameParams`, move `legalMoves` onto `GameHandle`, and turn `renderBoard` into a `Board` component. Then add three adapters + Board components that normalize each WASM's quirks into the common `GameHandle`. Reuse modes + epsilon-greedy difficulty.

**Tech Stack:** React 19, Docusaurus, TS, `treant-wasm` (`NimWasm`, `MancalaWasm`, `ShiftWasm`). Spec: `specs/2026-06-15-arcade-phase2-multiplayer-games-design.md`.

---

## File Structure
- Modify: `arcade/gameTypes.ts` — generalize params, `GameHandle.legalMoves`, `Board` on def, `pickAiMove`.
- Modify: `arcade/useGameSession.ts` — `pickAiMove` call.
- Modify: `arcade/GamePlay.tsx` — render `<def.Board/>`.
- Modify: `arcade/games/connectFour.tsx`, `ticTacToe.tsx` — handle `legalMoves`, `Board` component.
- Create: `arcade/games/nim.tsx`, `mancala.tsx`, `shift.tsx`.
- Modify: `arcade/games/index.ts` — register the three.
- Modify: `arcade/arcade.module.css` — Nim/Mancala/Shift board styles.

---

## Task 1: Generalize the abstraction (refactor, CF/TTT stay green)

**Files:** `arcade/gameTypes.ts`, `useGameSession.ts`, `GamePlay.tsx`, `games/connectFour.tsx`, `games/ticTacToe.tsx`

- [ ] **Step 1: `gameTypes.ts` edits**

Replace the `GameParams` type:
```ts
export type GameParams = { numPlayers: number } & Record<string, number>;
```
Add to `GameHandle`:
```ts
  legalMoves(): string[];
```
In `GameDefinition`, remove `legalMoves(...)` and replace `renderBoard` with:
```ts
  Board: React.ComponentType<BoardProps>;
```
Add the import at top: `import type { ComponentType, JSX } from 'react';` and use `ComponentType`.
Replace `pickAiMove` with (no `def`/`params` needed now):
```ts
export function pickAiMove(
  handle: GameHandle,
  diff: Difficulty,
  rng: () => number = Math.random,
): string | undefined {
  const { playouts, epsilon } = DIFFICULTY[diff];
  const legal = handle.legalMoves();
  if (legal.length === 0) return undefined;
  if (rng() < epsilon) return legal[Math.floor(rng() * legal.length)];
  handle.playoutN(playouts);
  return handle.bestMove() ?? legal[0];
}
```

- [ ] **Step 2: `useGameSession.ts`** — update the call site:

Change `const mv = pickAiMove(h, def, params, difficulty);` to `const mv = pickAiMove(h, difficulty);`. Remove now-unused `def`/`params` from `pickAiMove` import usage (keep `def`/`params` in the hook signature — still used by `def.create`).

- [ ] **Step 3: `GamePlay.tsx`** — render the Board component:

Replace the `def.renderBoard({...})` call with:
```tsx
<def.Board
  board={s.board}
  params={params}
  currentPlayer={s.current}
  interactive={interactive}
  onMove={s.onHumanMove}
/>
```

- [ ] **Step 4: `connectFour.tsx`** — move `legalMoves` into the handle; rename render to `Board`:

In `makeHandle`, add to the returned object:
```ts
    legalMoves: () => {
      const board = g.get_board();
      const out: string[] = [];
      for (let c = 0; c < p.cols; c++) if ((board[c] ?? ' ') === ' ') out.push(String(c));
      return out;
    },
```
Delete the `legalMoves:` property from the `connectFour` definition object. Rename `renderBoard: ({ board, params, interactive, onMove }: BoardProps) => { … }` to a named component and assign it:
```tsx
function ConnectFourBoard({ board, params, interactive, onMove }: BoardProps) {
  /* …existing body… */
}
```
and in the def: `Board: ConnectFourBoard,`.

- [ ] **Step 5: `ticTacToe.tsx`** — same treatment:

Add to `makeHandle`:
```ts
    legalMoves: () => {
      const board = g.get_board();
      const out: string[] = [];
      for (let i = 0; i < p.cols * p.rows; i++) if ((board[i] ?? ' ') === ' ') out.push(String(i));
      return out;
    },
```
Delete `legalMoves` from the def; extract `function TicTacToeBoard(props: BoardProps) {…}` and set `Board: TicTacToeBoard,`.

- [ ] **Step 6: Build check**

Run: `cd docs && npm run build 2>&1 | tail -6`
Expected: success (CF/TTT still compile under the new contract).

- [ ] **Step 7: Commit**
```bash
git add docs/src/components/arcade
git commit -m "refactor(arcade): generalize params, legalMoves on handle, Board component"
```

---

## Task 2: Nim

**Files:** Create `arcade/games/nim.tsx`; modify `games/index.ts`, `arcade.module.css`

- [ ] **Step 1: `games/nim.tsx`**
```tsx
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.NimWasm(p.stones);
  const seat = () => (g.current_player() === 'P1' ? 0 : 1);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => String(g.current_stones()),
    currentPlayer: seat,
    isTerminal: () => g.current_stones() === 0,
    result: () => (g.current_stones() === 0 ? String(seat() === 0 ? 2 : 1) : ''),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const s = g.current_stones();
      return s >= 2 ? ['Take1', 'Take2'] : s === 1 ? ['Take1'] : [];
    },
    free: () => g.free(),
  };
}

function NimBoard({ board, interactive, onMove }: BoardProps) {
  const stones = Number(board) || 0;
  return (
    <div className={styles.nimWrap}>
      <div className={styles.nimPile}>
        {Array.from({ length: stones }, (_, i) => (
          <span key={i} className={styles.nimStone} />
        ))}
      </div>
      <div className={styles.nimCount}>{stones} stones left</div>
      <div className={styles.nimButtons}>
        <button className={styles.nimTake} disabled={!interactive || stones < 1} onClick={() => onMove('Take1')}>Take 1</button>
        <button className={styles.nimTake} disabled={!interactive || stones < 2} onClick={() => onMove('Take2')}>Take 2</button>
      </div>
    </div>
  );
}

export const nim: GameDefinition = {
  id: 'nim',
  name: 'Nim',
  icon: '🪨',
  blurb: 'Take 1 or 2 stones. Take the last one to win.',
  defaultParams: { numPlayers: 2, stones: 15 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, stones: 15 } },
    { label: 'Quick', emoji: '⚡', params: { numPlayers: 2, stones: 7 } },
    { label: 'Marathon', emoji: '🏃', params: { numPlayers: 2, stones: 25 } },
  ],
  knobs: [{ key: 'stones', label: 'Stones', min: 3, max: 30, step: 1 }],
  create: makeHandle,
  Board: NimBoard,
};
```
Note: the player-count knob is absent (Nim is fixed 2-player); `numPlayers` stays 2 in every preset.

- [ ] **Step 2: register + CSS**

In `games/index.ts`: import `nim`, add to `GAMES` after `ticTacToe`.
In `arcade.module.css` add: `.nimWrap` (column, centered), `.nimPile` (flex wrap of stones), `.nimStone` (28px circle, `background: var(--arc-p3)`), `.nimCount` (bold), `.nimButtons` (row, gap), `.nimTake` (big chunky button, `background: var(--arc-accent)`, white, disabled dimmed, min-height 48px).

- [ ] **Step 3: Build + commit**

Run: `cd docs && npm run build 2>&1 | tail -4` → success.
```bash
git add docs/src/components/arcade
git commit -m "feat(arcade): Nim game"
```

---

## Task 3: Mancala

**Files:** Create `arcade/games/mancala.tsx`; modify `index.ts`, `arcade.module.css`

- [ ] **Step 1: `games/mancala.tsx`**

Board string = comma-separated counts, ring order: `P0 pit0..pits-1, P0 store, P1 pit0..pits-1, P1 store, …`. So player `pl`'s pits start at `pl*(pits+1)` and their store is at `pl*(pits+1)+pits`.
```tsx
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';

const SEAT_COLOR = ['var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)'];

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.MancalaWasm(p.pits, p.stones, p.numPlayers);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => { const r = g.result(); return r.startsWith('P') ? r.slice(1) : r; },
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => { const s = g.legal_moves(); return s ? s.split(',') : []; },
    free: () => g.free(),
  };
}

function MancalaBoard({ board, params, currentPlayer, interactive, onMove }: BoardProps) {
  const pits = params.pits, np = params.numPlayers;
  const counts = board.split(',').map(Number);
  const ringStride = pits + 1;
  const pitVal = (pl: number, i: number) => counts[pl * ringStride + i] ?? 0;
  const storeVal = (pl: number) => counts[pl * ringStride + pits] ?? 0;
  return (
    <div className={styles.mancalaWrap}>
      {Array.from({ length: np }, (_, pl) => (
        <div key={pl} className={styles.mancalaRow} style={{ borderColor: SEAT_COLOR[pl] }}>
          <div className={styles.mancalaStore} style={{ background: SEAT_COLOR[pl] }}>{storeVal(pl)}</div>
          <div className={styles.mancalaPits}>
            {Array.from({ length: pits }, (_, i) => {
              const v = pitVal(pl, i);
              const mine = pl === currentPlayer;
              return (
                <button key={i} className={styles.mancalaPit}
                  disabled={!interactive || !mine || v === 0}
                  onClick={() => onMove(String(i))}
                  style={{ borderColor: SEAT_COLOR[pl] }}>{v}</button>
              );
            })}
          </div>
          <div className={styles.mancalaLabel}>P{pl + 1}</div>
        </div>
      ))}
    </div>
  );
}

export const mancala: GameDefinition = {
  id: 'mancala',
  name: 'Mancala',
  icon: '🫘',
  blurb: 'Sow seeds, capture, and fill your store. Kalah rules.',
  defaultParams: { numPlayers: 2, pits: 6, stones: 4 },
  presets: [
    { label: 'Kalah', emoji: '⭐', params: { numPlayers: 2, pits: 6, stones: 4 } },
    { label: 'Quick', emoji: '⚡', params: { numPlayers: 2, pits: 3, stones: 3 } },
    { label: '4-Player Ring', emoji: '🎉', params: { numPlayers: 4, pits: 4, stones: 3 } },
  ],
  knobs: [
    { key: 'pits', label: 'Pits', min: 3, max: 8, step: 1 },
    { key: 'stones', label: 'Stones', min: 2, max: 6, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 4, step: 1 },
  ],
  create: makeHandle,
  Board: MancalaBoard,
};
```
Note: `apply_move` takes the **local** pit index `i` (0..pits-1) for the current player; the WASM rejects empty/illegal pits, so disabling `v===0`/non-current is just UX.

- [ ] **Step 2: register + CSS**

`index.ts`: add `mancala` to `GAMES`.
`arcade.module.css`: `.mancalaWrap` (column, gap), `.mancalaRow` (flex row, 2px border, rounded, padding), `.mancalaStore` (rounded rect, white bold number, min 44px), `.mancalaPits` (flex row, gap), `.mancalaPit` (circle ~48px, white bg, 2px border, bold count, disabled dimmed), `.mancalaLabel` (small).

- [ ] **Step 3: Build + commit**

`cd docs && npm run build 2>&1 | tail -4` → success.
```bash
git add docs/src/components/arcade
git commit -m "feat(arcade): Mancala game (bonus turns + captures via WASM)"
```

---

## Task 4: Shift (two-phase Board with selection)

**Files:** Create `arcade/games/shift.tsx`; modify `index.ts`, `arcade.module.css`

- [ ] **Step 1: `games/shift.tsx`**

Board string = row-major symbols (`X O A B`, space empty). Placement move `P{i}`; movement move `M{from},{to}`.
```tsx
import { useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';

const SYM = ['X', 'O', 'A', 'B'];
const SEAT_COLOR: Record<string, string> = { X: 'var(--arc-p1)', O: 'var(--arc-p2)', A: 'var(--arc-p3)', B: 'var(--arc-p4)' };

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.ShiftWasm(p.cols, p.rows, p.k, p.numPlayers, p.pieces);
  const inPlacement = () => g.in_placement_phase();
  const legalMoves = () => {
    const board = g.get_board();
    const n = p.cols * p.rows;
    const empties: number[] = [];
    for (let i = 0; i < n; i++) if ((board[i] ?? ' ') === ' ') empties.push(i);
    if (inPlacement()) return empties.map((i) => 'P' + i);
    const me = SYM[g.current_player()];
    const mine: number[] = [];
    for (let i = 0; i < n; i++) if (board[i] === me) mine.push(i);
    const out: string[] = [];
    for (const f of mine) for (const t of empties) out.push(`M${f},${t}`);
    return out;
  };
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves,
    free: () => g.free(),
    // extra, used by the Board:
    inPlacement,
  } as GameHandle & { inPlacement(): boolean };
}

function ShiftBoard({ board, params, currentPlayer, interactive, onMove }: BoardProps) {
  const { cols, rows } = params;
  const me = SYM[currentPlayer];
  const [sel, setSel] = useState<number | null>(null);
  // infer phase from the board: placement until every seat reached its quota is
  // engine-side; here we derive "can I place?" = there exist empty cells AND the
  // current player has fewer than `pieces` on the board.
  const cells = Array.from({ length: cols * rows }, (_, i) => board[i] ?? ' ');
  const myCount = cells.filter((c) => c === me).length;
  const placement = myCount < params.pieces;

  const tap = (i: number) => {
    if (!interactive) return;
    const ch = cells[i];
    if (placement) {
      if (ch === ' ') onMove('P' + i);
      return;
    }
    if (sel === null) {
      if (ch === me) setSel(i);
    } else if (i === sel) {
      setSel(null);
    } else if (ch === ' ') {
      onMove(`M${sel},${i}`);
      setSel(null);
    } else if (ch === me) {
      setSel(i);
    }
  };

  return (
    <div>
      <div className={styles.shiftCaption}>
        {placement ? 'Place your pieces' : sel === null ? 'Tap a piece to pick it up' : 'Tap an empty square to slide'}
      </div>
      <div className={styles.shiftGrid} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
        {cells.map((ch, i) => (
          <button key={i} className={`${styles.shiftCell} ${sel === i ? styles.shiftSel : ''}`}
            disabled={!interactive}
            onClick={() => tap(i)}
            style={{ color: ch === ' ' ? 'transparent' : SEAT_COLOR[ch] }}
            aria-label={`Cell ${i + 1}: ${ch === ' ' ? 'empty' : ch}`}>
            {ch === ' ' ? '·' : ch}
          </button>
        ))}
      </div>
    </div>
  );
}

export const shift: GameDefinition = {
  id: 'shift',
  name: 'Shift',
  icon: '🔀',
  blurb: 'Place 3 pieces, then slide them. Line them up to win.',
  defaultParams: { numPlayers: 2, cols: 3, rows: 3, k: 3, pieces: 3 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, cols: 3, rows: 3, k: 3, pieces: 3 } },
    { label: 'Big', emoji: '🔲', params: { numPlayers: 2, cols: 5, rows: 4, k: 4, pieces: 4 } },
    { label: '3-Player', emoji: '👨‍👩‍👦', params: { numPlayers: 3, cols: 4, rows: 4, k: 3, pieces: 2 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 2, max: 8, step: 1 },
    { key: 'rows', label: 'Height', min: 2, max: 8, step: 1 },
    { key: 'k', label: 'In a row', min: 2, max: 6, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 4, step: 1 },
    { key: 'pieces', label: 'Pieces', min: 1, max: 4, step: 1 },
  ],
  create: makeHandle,
  Board: ShiftBoard,
};
```
Note: the Board derives "placement vs movement for *me*" from `myCount < pieces` (matches the engine's per-player `in_placement_phase`), avoiding a handle round-trip mid-render.

- [ ] **Step 2: register + CSS**

`index.ts`: add `shift` to `GAMES`.
`arcade.module.css`: `.shiftCaption` (centered, bold, accent), `.shiftGrid` (grid, gap, soft bg, rounded), `.shiftCell` (aspect 1, white, rounded, big bold), `.shiftSel` (outline `3px solid var(--arc-accent)`, scale 1.05).

- [ ] **Step 3: Build + commit**

`cd docs && npm run build 2>&1 | tail -4` → success.
```bash
git add docs/src/components/arcade
git commit -m "feat(arcade): Shift game (two-phase place-then-slide)"
```

---

## Task 5: End-to-end verification

- [ ] **Step 1:** Start dev server: `cd docs && npm start -- --host 0.0.0.0 --port 3939 --no-open` (background). Wait for "compiled successfully". (Rebuild WASM first only if changed — Phase 2 is JS-only, so not needed.)

- [ ] **Step 2: Playwright** — for each game, no console errors:
  - Launcher shows 5 tiles (Connect Four, Tic-Tac-Toe, Nim, Mancala, Shift).
  - **Nim** pass-and-play: repeatedly Take to 0 → overlay names the winner (the player who took the last stone).
  - **Mancala** vs-AI Easy: human sows a non-empty pit → board changes and AI responds; verify a store-landing keeps the human's turn (bonus turn).
  - **Shift** pass-and-play: place all pieces (caption flips to movement) → select a piece, tap empty → it slides; small board fills → draw overlay.
  - One **wild preset** each (Nim Marathon, Mancala 4-Player, Shift Big) loads and is playable.

- [ ] **Step 3:** Stop the dev server (or leave running for the user). Remove `.playwright-mcp/` if created (gitignored).

- [ ] **Step 4:** Final commit if cleanup needed.

---

## Self-Review Notes
- **Spec coverage:** abstraction generalization (Task 1), three games incl. result/`currentPlayer`/`legalMoves` normalization and two-phase Shift (Tasks 2–4), registry + CSS, e2e per game (Task 5). Mapped.
- **Type consistency:** `GameParams` (`{numPlayers} & Record<string,number>`), `GameHandle.legalMoves()`, `GameDefinition.Board`, `pickAiMove(handle, diff, rng)` used consistently across Tasks 1–4. Each adapter returns the normalized `result()` (1-indexed/`Draw`/`""`).
- **Constraint checks:** Nim fixed 2 players; Mancala/Shift player knobs ≤4; Shift `pieces ≤ cols*rows/numPlayers` enforced by the WASM constructor (presets respect it).
