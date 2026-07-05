# Knobs, Variant Tiles & New Games — Program Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand the arcade from 40 games to ~60 findable tiles by (1) exposing engine capability the UI hides, (2) adding rule flags that turn existing engines into named-variant families, and (3) staging the genuinely new engines — with every step honest about AI strength.

**Architecture:** Two-layer arcade (Rust `GameState` + `XxxWasm` in `treant-wasm/src/` ↔ config-only `GameDefinition` in `docs/src/components/arcade/games/`). Variant tiles are pure config over shared engines (the Gomoku-on-`TicTacToeWasm` pattern, policy in ARCADE.md §6). Rule variants are constructor flags on existing engines, never forked engines.

**Tech Stack:** Rust (treant MCTS, wasm-bindgen), TypeScript/React (Docusaurus), self-play calibration harness (`treant-wasm/examples/calibrate.rs` via `scripts/calibrate.sh`).

## Global Constraints

- Naming: public names must be non-trademarked (ARCADE.md §6). Names used here were vetted in GAME-IDEAS.md §6z/§6n/§6o.
- `result()` contract: `""` (undecided) / `"Draw"` / bare 1-indexed seat digit. Never break it — the calibration harness and `useGameSession` both parse it.
- Every engine keeps the uniform WASM surface: `new`, `playout_n`, `get_board`, `current_player`, `is_terminal`, `result`, `best_move`, `weak_move(playouts, top_k, temp, seed)`, `apply_move`, `reset`.
- `cargo clippy` stays at 0 warnings; `cargo test -p treant-wasm` green before every commit.
- TSX layer has no unit-test runner: its test cycle is `cd docs && npm run build` (typecheck) + a Playwright DOM assertion against the static build served on :3939 (NOT `docusaurus start` — see memory: dev server gives false positives).
- After ANY TSX string change: `cd docs && node scripts/i18n-keys.mjs --write`, top up all six `i18n/locales/*.json`, then `--check` reports 0 missing/0 orphaned.
- After ANY clamp widening: run the harness audit (`cargo run --release --example calibrate -p treant-wasm -- audit`) — its Pass 2 catches silent-clamp bugs (engine board ≠ `cols×rows` at the largest UI values).
- After ANY rule flag ships: register the variant in `calibrate.rs` `games()` and run `scripts/calibrate.sh <id> 20`; paste the emitted `difficulty:` block into the tile. Misère flags especially — they can invert what "hard" means.
- Every new tile id must be added to a `CATEGORIES[].ids` array in `Launcher.tsx` — there is NO fallback bucket; an uncategorized id silently disappears from the shelf (only hero/search show it).
- Commit trailer: `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.

---

# Part I — The architecture answers (Peter's questions)

## I.1 What we change in EXISTING games/knobs to enable the future waves

| Enabler | Why future games need it | Effect |
|---|---|---|
| **Fix `PresetChips.same()`** (compares only `cols/rows/k/numPlayers` today) | Variant tiles are preset-heavy and keyed on OTHER params (`target`, `stones`, `men`, `size`, future `anti`/`lasker`/`variant` flags). Today two presets can both show "active"; with flag params the highlight would be wrong on almost every new tile. | UI-only, one function. |
| **Launcher category enforcement** | We're about to add ~20 tiles. The "falls under More" comment in `Launcher.tsx` is a lie — an uncategorized id vanishes. A build-time check makes forgetting impossible. | Script + CI; no UX change. |
| **Flag-knob convention** | Rule flags ride the existing knob machinery as `0/1` number knobs (proven by Nine Men's Morris `flying`). No new UI concept needed — YAGNI on toggle widgets until a round of use proves the stepper annoying. | None now. |
| **Calibration per variant** | `difficulty:` blocks are per-tile constants. A flag that changes who's favored (misère!) silently mis-labels Easy/Hard if we reuse the parent's block. Every variant tile gets its own `calibrate.rs` registration. | AI-quality guardrail; ~3 min compute per tile. |
| **Clamp-audit habit** | Engines clamp inputs silently (by design). Widening a UI knob past a real clamp yields a board that lies (UI says 12, engine plays 10). The harness audit's Pass 2 exists precisely for this. | Guardrail; no code. |

**Where the real clamps live** (investigated, exact): `tictactoe.rs` `MAX_DIM=15` (u8 index → cols×rows ≤ 256 is the hard wall) · `connectfour.rs` `MAX_DIM=10` · `gridlib.rs` `MAX_DIM=12` (gridpack games) · `shift.rs` 10×10 · `mancala.rs` pits 2–8, stones 1–8, players {2,4} · `pig.rs` target 20–200 · `wythoff.rs` 4–12 · `hex.rs` 4–11 · `trails.rs` 4–9 · `reversi.rs` even 4–10 · unclamped: `col.rs`, `domineering.rs`, `foxhounds.rs`, `nim.rs` (u8). `treant-games/grid.rs` `MAX_DIM=10` is dead code — wrappers own their clamps.

## I.2 How this affects UI / UX / AI-training

- **UI:** knobs and presets auto-render; setup preview auto-works (it builds a throwaway engine from current params). Renderer ceiling is phone cell size ~19 cols (Pinch-Five proves it) and the 1×N Treblecross strip needs `overflow-x` checking past ~20.
- **UX:** each variant gets its own tile (name/icon/blurb/rules), so discoverability is by name, and the tile still exposes the full knob panel. Category shelves grow; the 🤯 emoji convention marks "silly" presets whose AI is honestly softer.
- **AI training:** no training in the ML sense — treant is search. "Training requirements" = per-tile difficulty calibration via self-play ladder (`STD`/`LIGHT` in calibrate.rs) + the win-protection caveat: solver-off games (Connect Four, Reversi, Mancala, Pig, Connect-6) can't protect proven wins at low playouts, so their Easy is genuinely beatable — keep it that way. Misère flags need their eval terms flipped or re-derived (`reversi.rs eval0` disc-diff/corners; `gridlib.rs squava_safety`), then re-calibrated.

## I.3 Reuse-vs-refactor-vs-new, per future game

**Refactor of existing engines (flags, no forks):** Col (adjacency polarity), Domineering (either-orientation), Reversi (anti), Hex (pie rule), Nine Men's Morris (Lasker place-or-move), Trails (knight movement), Connect Four (k≥2, MAX_DIM 12, pop-out move, cylinder wrap — the last two touch shared `treant-games/grid.rs`: `winner()` needs column-wrap and `terminal_value()` needs mover-relative winner comparison once pops can complete an *opponent's* line), Pig (dice count + doubles variants).

**Small engine over ≥80% existing libraries:** World Threes (Shift's place-then-slide phases over per-board adjacency tables; new graph-board renderer shared with Morris-style SVG), Gale (Hex's union-find over an edge-claim board), Slimetrail (Trails' walk-and-burn on a shared token), Toads & Frogs (1-D like Treblecross; near-trivial), Pawn Duel (Frontline + double-step/en-passant), Strand (Trails + remove-any-tile; needs a 3-part move and 2-phase board input — that's why it is NOT a flag), Heap Nim (multi-heap/max-take/misère — new move encoding + new board UI, replaces the single-pile toy), Len Choa (Bagh-Chal's hunt patterns on a triangle graph).

**Full new engines (from scratch):** Draughts (capture-chain machinery with maximum-capture variants — deliberately built as a reusable draughts-lib so Dama, Alquerque, Latrunculi later become small engines over it), Surakarta (unique loop-capture path generation), Salvo (hidden-info + determinization + the pass-screen UX primitive), Bulls & Cows (small but a new duel/deduction category), Ambush (hidden ranks; hardest AI; last).

## I.4 The classification Peter asked for (end state)

- **Config-only tiles (name + icon + blurb + default flags):** today: Gomoku. After Phase 2: Snort, Cram, Anti-Reversi, Lasker Morris, Joust, Pop Out, Cylinder Four, Two-Dice Pig, Big Pig. After later phases: Giveaway/American/Brazilian/Pool/Russian/Spanish/Italian/Frisian Draughts (one engine → up to 9 tiles), Rex (after misère-hex eval work), Hexapawn (Pawn Duel preset), Fibonacci/Moore's Nim (Heap Nim flags), Caro (grid-engine ruleset flag; full Renju forks deferred).
- **Small-engine-over-libs (≈80% reuse):** World Threes, Gale, Slimetrail, Toads & Frogs, Pawn Duel, Strand, Heap Nim, Len Choa, and later Dama/Latrunculi/Alquerque over draughts-lib.
- **Full engines:** Draughts, Surakarta, Salvo (+pass-screen), Bulls & Cows, Ambush.

---

# Part II — Tasks

Phases 0–2 are fully specified below (the current execution horizon). Phases 3–5 are scoped as follow-on plan documents (one per game/subsystem, per the writing-plans scope rule) — their architecture is pinned in Part I and GAME-IDEAS.md §6x.

## Phase 0 — Foundation (2 tasks)

### Task 0.1: Preset-active comparator compares all params

**Files:**
- Modify: `docs/src/components/arcade/PresetChips.tsx:5-7`

**Interfaces:**
- Produces: `same(a: GameParams, b: GameParams): boolean` comparing the union of keys of both objects. Later variant tiles rely on flag params (e.g. `anti: 1`) affecting preset highlighting.

- [ ] **Step 1: Make the change**

Replace (current code, `PresetChips.tsx:5-7`):

```ts
function same(a: GameParams, b: GameParams) {
  return a.cols === b.cols && a.rows === b.rows && a.k === b.k && a.numPlayers === b.numPlayers;
}
```

with:

```ts
// Compare the FULL param set: presets are keyed on target/stones/men/size and
// (soon) rule flags, not just grid dims — two presets must never both light up.
function same(a: GameParams, b: GameParams) {
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  return [...keys].every((k) => (a[k] ?? 0) === (b[k] ?? 0));
}
```

- [ ] **Step 2: Typecheck + build**

Run: `cd /home/peter/code/treant/docs && npm run build`
Expected: `[SUCCESS] Generated static files in "build".`

- [ ] **Step 3: Browser-verify against the static build**

Serve: `cd /home/peter/code/treant/docs && (npx serve -l 3939 build &)`. With Playwright on `http://localhost:3939/arcade?game=pig`, assert exactly ONE preset chip carries the active class after clicking "Quick (50)" (previously "Classic (100)" also matched because `same()` ignored `target`):

```js
// browser_evaluate on the setup page after clicking the "Quick (50)" chip
() => {
  const on = [...document.querySelectorAll('[class*="chipOn"]')];
  return { activeCount: on.length, label: on[0]?.textContent };
}
// Expected: { activeCount: 1, label: '⚡ Quick (50)' }
```

- [ ] **Step 4: Commit**

```bash
git add docs/src/components/arcade/PresetChips.tsx
git commit -m "fix(arcade): preset highlight compares all params, not just grid dims"
```

### Task 0.2: Build-time launcher-category completeness check

**Files:**
- Create: `docs/scripts/check-categories.mjs`
- Modify: `docs/package.json` (wire into `build` as a prebuild step)

**Interfaces:**
- Consumes: `Launcher.tsx` CATEGORIES literal, `games/index.ts` imports.
- Produces: `npm run build` fails if any GAMES id is missing from CATEGORIES (protects every tile task in this plan).

- [ ] **Step 1: Write the check (it greps source, no imports needed)**

```js
// docs/scripts/check-categories.mjs
// Every game id registered in games/index.ts must appear in a Launcher
// category — there is no fallback bucket; missing ids vanish from the shelf.
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const launcher = readFileSync(join(root, 'src/components/arcade/Launcher.tsx'), 'utf8');
const catBlock = launcher.match(/const CATEGORIES[\s\S]*?\n\];/)?.[0] ?? '';
const categorized = new Set([...catBlock.matchAll(/'([a-z0-9-]+)'/g)].map((m) => m[1]));

const gamesDir = join(root, 'src/components/arcade/games');
const { readdirSync } = await import('node:fs');
const ids = [];
for (const f of readdirSync(gamesDir)) {
  if (!f.endsWith('.tsx')) continue;
  const src = readFileSync(join(gamesDir, f), 'utf8');
  for (const m of src.matchAll(/^\s*id:\s*'([a-z0-9-]+)'/gm)) ids.push(m[1]);
}
const missing = ids.filter((id) => !categorized.has(id));
if (missing.length) {
  console.error(`check-categories: not in any Launcher category: ${missing.join(', ')}`);
  process.exit(1);
}
console.log(`check-categories: ${ids.length} game ids all categorized.`);
```

- [ ] **Step 2: Run it — must pass on the current tree**

Run: `cd /home/peter/code/treant/docs && node scripts/check-categories.mjs`
Expected: `check-categories: 40 game ids all categorized.`

- [ ] **Step 3: Verify it FAILS when a game is uncategorized**

Temporarily add `id: 'zz-test'` in a scratch file `src/components/arcade/games/zztest.tsx` containing only `export const x = { id: 'zz-test' };`, rerun, expect exit 1 with `zz-test` named. Delete the scratch file.

- [ ] **Step 4: Wire into the build**

In `docs/package.json` scripts, change `"build": "docusaurus build"` to:

```json
"build": "node scripts/check-categories.mjs && node scripts/i18n-keys.mjs --check && docusaurus build"
```

(If `i18n-keys.mjs --check` is not already in the build script, this also locks locale completeness; if it is, keep the existing form and prepend only the category check.)

Run: `npm run build` — expected: both checks print, then `[SUCCESS]`.

- [ ] **Step 5: Commit**

```bash
git add docs/scripts/check-categories.mjs docs/package.json
git commit -m "chore(arcade): build fails if a game id is missing from launcher categories"
```

## Phase 1 — Tier-1 pure-UI knob wave (6 tasks; engine already supports everything)

> Shared Definition of Done for every Phase-1 task: (a) `npm run build` green, (b) i18n `--write` + six-locale top-up + `--check` 0/0, (c) harness audit run once at the end of the phase (Task 1.6), (d) Playwright spot-check of the wildest new preset on the static build, (e) commit per task.

### Task 1.1: Fox & Hounds — Width/Height knobs + wild presets

**Files:**
- Modify: `docs/src/components/arcade/games/foxhounds.tsx:71-91`

**Interfaces:**
- Consumes: `FoxHoundsWasm(cols, rows)` — unclamped; hounds auto-populate odd columns of the top row (verified `foxhounds.rs:32-55`).

- [ ] **Step 1: Replace the presets/knobs block**

In the `foxHounds` definition replace the `presets` and `knobs` fields (current values quoted in the plan's investigation record) with:

```ts
  presets: [
    { label: 'Classic 8×8', emoji: '⭐', params: { numPlayers: 2, cols: 8, rows: 8 } },
    { label: 'Small 6×6', emoji: '🔳', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Hound Wall 12×8', emoji: '🐕', params: { numPlayers: 2, cols: 12, rows: 8 } },
    { label: 'Long Chase 8×12', emoji: '🏃', params: { numPlayers: 2, cols: 8, rows: 12 } },
    { label: 'Thunderdome 12×12', emoji: '🤯', params: { numPlayers: 2, cols: 12, rows: 12 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 6, max: 12, step: 1 },
    { key: 'rows', label: 'Height', min: 6, max: 12, step: 1 },
  ],
```

- [ ] **Step 2: Build + i18n**

Run: `cd /home/peter/code/treant/docs && node scripts/i18n-keys.mjs --write && npm run build`
Top up the four new preset labels in all six `i18n/locales/*.json`, then `node scripts/i18n-keys.mjs --check` → 0 missing.

- [ ] **Step 3: Browser-verify Thunderdome**

Playwright on `:3939/arcade?game=fox-hounds`: click "Thunderdome 12×12", assert the setup preview grid contains 144 cells and 6 hound pieces (odd columns of 12 = 6 hounds):

```js
() => {
  const preview = document.querySelector('[class*="setupPreview"]');
  const cells = preview.querySelectorAll('[class*="tttCell"]').length;
  return { cells };
}
// Expected: { cells: 144 }
```

- [ ] **Step 4: Commit**

```bash
git add docs/src/components/arcade/games/foxhounds.tsx docs/src/components/arcade/i18n
git commit -m "feat(arcade): Fox & Hounds width/height knobs — up to 6 hounds on 12-wide boards"
```

### Task 1.2: Wythoff's Queen — Size knob

**Files:**
- Modify: `docs/src/components/arcade/games/wythoff.tsx:4-25`

**Interfaces:**
- Consumes: `WythoffWasm(n)` clamps 4–12; board renders via shared `MoveBoard` reading `params.cols/rows`. The engine is square-only, so ONE knob drives both dims.

- [ ] **Step 1: Add the knob; keep params square via a single `cols` knob mirrored to `rows` in `create` and `Board`**

Replace `knobs: [],` and the `create`/`Board` lines with:

```ts
  knobs: [{ key: 'cols', label: 'Size', min: 4, max: 12, step: 1 }],
  create: (wasm, p) => moveHandle(new wasm.WythoffWasm(p.cols)),
  // The engine is square (n×n): mirror the single Size knob into rows so the
  // shared MoveBoard renders the same board the engine plays.
  Board: (props) => <MoveBoard {...props} params={{ ...props.params, rows: props.params.cols }} />,
```

Also update every preset's `params` to keep `rows` equal to `cols` (they already are).

- [ ] **Step 2: Build + verify**

`npm run build`; Playwright: set Size to 5 via the knob stepper, assert preview cell count is 25 and exactly one 👑 piece renders. No i18n change ('Size' already exists as a key from Hex).

- [ ] **Step 3: Commit**

```bash
git add docs/src/components/arcade/games/wythoff.tsx
git commit -m "feat(arcade): Wythoff's Queen gets a Size knob (engine was always parameterized)"
```

### Task 1.3: Tic-Tac-Toe + Gomoku — widen to the engine's true 15/k ceiling

**Files:**
- Modify: `docs/src/components/arcade/games/ticTacToe.tsx:86-91` (knobs), `:78-84` (presets)
- Modify: `docs/src/components/arcade/games/gomoku.tsx:24-29` (knobs)

**Interfaces:**
- Consumes: `TicTacToeWasm` clamps `cols/rows ∈ [2,15]`, `k ∈ [2, max(cols,rows)]`, players ≤ 6 (verified `tictactoe.rs:13-30`).

- [ ] **Step 1: Tic-Tac-Toe knobs → 15, plus two presets**

```ts
  knobs: [
    { key: 'cols', label: 'Width', min: 2, max: 15, step: 1 },
    { key: 'rows', label: 'Height', min: 2, max: 15, step: 1 },
    { key: 'k', label: 'In a row', min: 2, max: 15, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
  ],
```

Append presets:

```ts
    { label: 'Speed Duel 2×10', emoji: '🏎️', params: { cols: 10, rows: 2, k: 2, numPlayers: 2 } },
    { label: 'Land War 15×15', emoji: '🤯', params: { cols: 15, rows: 15, k: 10, numPlayers: 6 } },
```

- [ ] **Step 2: Gomoku k knob → 8, add the 🤯 preset**

```ts
  knobs: [
    { key: 'cols', label: 'Width', min: 9, max: 15, step: 1 },
    { key: 'rows', label: 'Height', min: 9, max: 15, step: 1 },
    { key: 'k', label: 'In a row', min: 4, max: 8, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
  ],
```

Append preset: `{ label: '8-in-a-row', emoji: '🤯', params: { numPlayers: 2, cols: 15, rows: 15, k: 8 } },`

- [ ] **Step 3: Build + i18n top-up (3 new labels) + verify**

Playwright: TTT "Land War" preset → preview renders 225 cells; play 3 moves vs easy AI on 15×15 to confirm responsiveness.

- [ ] **Step 4: Commit**

```bash
git add docs/src/components/arcade/games/ticTacToe.tsx docs/src/components/arcade/games/gomoku.tsx docs/src/components/arcade/i18n
git commit -m "feat(arcade): TTT/Gomoku knobs reach the engine's real 15×15 / k ceilings"
```

### Task 1.4: Gridpack four — widen to gridlib's 12

**Files:**
- Modify: `docs/src/components/arcade/games/gridpack.tsx` (four `knobs` blocks: noTacToe `:140-143`, trapThree `:114-117`, squareUp `:166-169`, orderChaos `:264-267`)

**Interfaces:**
- Consumes: `gridlib.rs MAX_DIM = 12`; all four gridpack `new()`s clamp to it (verified).

- [ ] **Step 1: Widen all four knob blocks' max to 12** (mins unchanged), e.g. No-Tac-Toe:

```ts
  knobs: [
    { key: 'cols', label: 'Width', min: 3, max: 12, step: 1 },
    { key: 'rows', label: 'Height', min: 3, max: 12, step: 1 },
  ],
```

(trap-three min 4, square-up min 4, order-chaos min 5 — keep each game's existing mins.)

- [ ] **Step 2: Add one 🤯 preset per game**

```ts
// noTacToe:  { label: 'Endurance 12×12', emoji: '🤯', params: { numPlayers: 2, cols: 12, rows: 12 } },
// trapThree: { label: 'Minefield 12×12', emoji: '🤯', params: { numPlayers: 2, cols: 12, rows: 12 } },
// squareUp:  { label: 'Acreage 12×12', emoji: '🤯', params: { numPlayers: 2, cols: 12, rows: 12 } },
// orderChaos:{ label: 'Pandemonium 12×12', emoji: '🤯', params: { numPlayers: 2, cols: 12, rows: 12 } },
```

- [ ] **Step 3: Build + i18n (4 labels) + Playwright preview check (144 cells on No-Tac-Toe 🤯) + commit**

```bash
git add docs/src/components/arcade/games/gridpack.tsx docs/src/components/arcade/i18n
git commit -m "feat(arcade): gridpack knobs reach gridlib's true 12×12 — misère TTT endurance mode"
```

### Task 1.5: Mancala stones/pits + Pig target ranges

**Files:**
- Modify: `docs/src/components/arcade/games/mancala.tsx:93-97` (knobs) + presets
- Modify: `docs/src/components/arcade/games/pig.tsx:90-93` (knobs) + presets

**Interfaces:**
- Consumes: `MancalaWasm` clamps pits 2–8, stones 1–8 (`mancala.rs:383-386`); `PigWasm` clamps target 20–200 (`pig.rs:149-158`).

- [ ] **Step 1: Mancala**

```ts
  knobs: [
    { key: 'pits', label: 'Pits', min: 2, max: 8, step: 1 },
    { key: 'stones', label: 'Stones', min: 1, max: 8, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 4, step: 2 },
  ],
```

Append presets: `{ label: 'Micro', emoji: '🤏', params: { numPlayers: 2, pits: 2, stones: 1 } },` and `{ label: 'Overflow', emoji: '🤯', params: { numPlayers: 4, pits: 8, stones: 8 } },`

- [ ] **Step 2: Pig**

```ts
  knobs: [
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
    { key: 'target', label: 'Target', min: 20, max: 200, step: 5 },
  ],
```

Append presets: `{ label: 'Sprint (20)', emoji: '🏎️', params: { numPlayers: 2, target: 20 } },` and `{ label: 'Marathon (200)', emoji: '🤯', params: { numPlayers: 2, target: 200 } },`

- [ ] **Step 3: Build + i18n (4 labels) + Playwright: Mancala Micro preview shows 2 pits/side with 1 stone; Pig Sprint plays a full vs-AI game quickly. Commit.**

```bash
git add docs/src/components/arcade/games/mancala.tsx docs/src/components/arcade/games/pig.tsx docs/src/components/arcade/i18n
git commit -m "feat(arcade): Mancala 1-stone/2-pit oddities + Pig sprint-to-20 and marathon-to-200"
```

### Task 1.6: Shift widening + phase-1 clamp audit + calibration spot-check

**Files:**
- Modify: `docs/src/components/arcade/games/shift.tsx:125-131` (knobs)

**Interfaces:**
- Consumes: `shift.rs` MAX 10×10, pieces self-clamped to `cols*rows/num_players` (`shift.rs:365-372`).

- [ ] **Step 1: Widen Shift**

```ts
  knobs: [
    { key: 'cols', label: 'Width', min: 2, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 2, max: 10, step: 1 },
    { key: 'k', label: 'In a row', min: 2, max: 8, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
    { key: 'pieces', label: 'Pieces', min: 1, max: 6, step: 1 },
  ],
```

Append preset: `{ label: '1-piece duel', emoji: '🏎️', params: { numPlayers: 2, cols: 3, rows: 3, k: 2, pieces: 1 } },`

- [ ] **Step 2: Run the harness audit over ALL games (Pass 2 = silent-clamp check at max UI values)**

Run: `cd /home/peter/code/treant && cargo run --release --example calibrate -p treant-wasm -- audit`
Expected: every game PASS; specifically fox-hounds/wythoff/gridpack/mancala/pig/shift at their new maxima. Any FAIL → the UI knob exceeded a real clamp → fix the knob max, not the engine, in this phase.

- [ ] **Step 3: Spot-recalibrate the two games whose feel changed most**

Run: `scripts/calibrate.sh fox-hounds 20` and `scripts/calibrate.sh no-tac-toe 20`; paste each emitted `difficulty:` block into the tile if it differs from the current block.

- [ ] **Step 4: Build + i18n (1 label) + commit**

```bash
git add docs/src/components/arcade/games/shift.tsx docs/src/components/arcade/i18n
git commit -m "feat(arcade): Shift to 10×10/6-piece + phase-1 clamp audit green"
```

## Phase 2 — Rule flags + variant tiles (7 tasks)

> Pattern for every task: Rust flag (TDD) → wasm ctor param → update parent tile's `create` call → add variant tile in the SAME game file (gridpack multi-export pattern) → register in `games/index.ts` + `Launcher.tsx` category → `rules.ts` entry → i18n → calibrate BOTH parent and variant → commit. The variant tile's `GameDefinition` is config-only — that's the whole point.

### Task 2.1: Col polarity flag → **Snort** tile

**Files:**
- Modify: `treant-wasm/src/col.rs` (struct, `is_legal`, ctor), `docs/src/components/arcade/games/col.tsx`, `docs/src/components/arcade/games/index.ts`, `docs/src/components/arcade/Launcher.tsx` (Brain-teasers), `docs/src/components/arcade/rules.ts`
- Test: `col.rs` `#[cfg(test)]`

**Interfaces:**
- Produces: `ColWasm::new(cols: usize, rows: usize, avoid_enemy: u32)`; `avoid_enemy=0` is exactly today's Col; `1` = Snort. JS: `new wasm.ColWasm(p.cols, p.rows, p.avoid ?? 0)`.

- [ ] **Step 1: Write the failing test** (append to col.rs tests)

```rust
#[test]
fn snort_cannot_colour_next_to_enemy_but_own_is_fine() {
    let mut g = ColWasm::new(3, 3, 1); // avoid_enemy = Snort
    assert!(g.apply_move("4")); // P0 takes centre
    // P1 may NOT play any orthogonal neighbour of the centre…
    assert!(!g.apply_move("1"));
    assert!(g.apply_move("0")); // …but a diagonal-only contact is legal
    // P0 may now play NEXT TO OWN centre stone (own-adjacency is fine in Snort)
    assert!(g.apply_move("1"));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p treant-wasm snort_cannot -- --nocapture`
Expected: FAIL — ctor takes 2 args (compile error), driving the signature change.

- [ ] **Step 3: Implement** — add `avoid_enemy: bool` to the `Col` struct, thread through `new`, and change `is_legal` (current code at `col.rs:42-49`) to:

```rust
fn is_legal(&self, i: usize) -> bool {
    if self.grid[i] != -1 {
        return false;
    }
    // Col: never touch your OWN colour. Snort (avoid_enemy): never touch the ENEMY.
    let banned = if self.avoid_enemy { 1 - self.current as i8 } else { self.current as i8 };
    !self.neighbors(i).iter().any(|&n| self.grid[n] == banned)
}
```

Wasm ctor: `pub fn new(cols: usize, rows: usize, avoid_enemy: u32) -> Self` passing `avoid_enemy != 0`.

- [ ] **Step 4: Run tests** — `cargo test -p treant-wasm col` → all green (old tests updated to pass `0`). `cargo clippy` → 0 warnings.

- [ ] **Step 5: Rebuild wasm + update tiles**

`cd treant-wasm && wasm-pack build --target web && cd ../docs && npm install ../treant-wasm/pkg 2>/dev/null || npm install treant-wasm && git checkout package.json` (follow the repo's existing install step for the wasm pkg — ARCADE.md §8).

In `col.tsx`: change create to `(wasm, p) => moveHandle(new wasm.ColWasm(p.cols, p.rows, 0))`, then append and export:

```ts
// Snort — Col's mirror twin (Conway): you may not colour next to the ENEMY.
export const snort: GameDefinition = {
  id: 'snort',
  name: 'Snort',
  icon: '🐂',
  blurb: 'Colour the map — but never next to the enemy. Spread wide, fence them in.',
  difficulty: col.difficulty, // placeholder until Step 7 calibration
  defaultParams: { numPlayers: 2, cols: 5, rows: 5 },
  presets: [
    { label: 'Classic 5×5', emoji: '⭐', params: { numPlayers: 2, cols: 5, rows: 5 } },
    { label: 'Big 7×7', emoji: '🔲', params: { numPlayers: 2, cols: 7, rows: 7 } },
    { label: 'Mega 10×10', emoji: '🤯', params: { numPlayers: 2, cols: 10, rows: 10 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 10, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.ColWasm(p.cols, p.rows, 1)),
  Board: ColBoard,
  playerLabels: ['Red', 'Yellow'],
};
```

Register `snort` in `games/index.ts` (import from `./col`, insert after `col` in GAMES) and add `'snort'` to the Brain-teasers category ids. Add to `rules.ts`:

```ts
  snort: 'Take turns colouring any empty cell — but you may never colour a cell orthogonally touching an ENEMY-coloured cell. Your own cells are fine to touch. The first player with no legal cell loses.',
```

- [ ] **Step 6: Build + i18n** — `node scripts/i18n-keys.mjs --write`, top up 6 locales (name/blurb/labels/rules), `--check` 0/0, `npm run build` green (category check passes because Step 5 added the id).

- [ ] **Step 7: Register + calibrate** — add `("snort", STD, || Box::new(...ColWasm::new(5,5,1)...))` to `calibrate.rs games()` mirroring the `col` entry; run `scripts/calibrate.sh snort 20`; paste the emitted difficulty block over the placeholder. Re-run `scripts/calibrate.sh col 20` (ctor change is behavior-neutral for `0` but verify).

- [ ] **Step 8: Playwright verify** (static build): open `?game=snort`, play 2 moves vs easy AI, assert the adjacency rule visually (legal-cell count shrinks around ENEMY stones, not own).

- [ ] **Step 9: Commit**

```bash
git add treant-wasm/src/col.rs treant-wasm/examples/calibrate.rs docs/src/components/arcade
git commit -m "feat(arcade): Snort — Col's mirror twin as a config tile on a one-flag engine"
```

### Task 2.2: Domineering either-orientation flag → **Cram** tile

**Files:**
- Modify: `treant-wasm/src/domineering.rs` (`gen` at `:25-45`, `partner` at `:46-48`, ctor at `:113-120`), `docs/src/components/arcade/games/domineering.tsx`, `games/index.ts`, `Launcher.tsx` (Brain-teasers), `rules.ts`

**Interfaces:**
- Produces: `DomineeringWasm::new(cols, rows, cram: u32)`. Cram: BOTH players may place either orientation (impartial); win = last to place (unchanged terminal). Move strings stay `"anchor-partner"` so the existing `MoveBoard`/`moveHandle` UI works untouched.

- [ ] **Step 1: Failing test**

```rust
#[test]
fn cram_lets_either_player_place_both_orientations() {
    let g = DomineeringWasm::new(4, 4, 1);
    let moves = g.legal_moves();
    // 4×4 empty board: 12 vertical + 12 horizontal anchors = 24 moves for P0.
    assert_eq!(moves.split(',').count(), 24);
}
```

Run: `cargo test -p treant-wasm cram_lets` → FAIL (ctor arity).

- [ ] **Step 2: Implement** — add `cram: bool`; in `gen()`, when `cram` is set emit BOTH `(i, i+cols)` and `(i, i+1)` candidate pairs for the current player (guarding right-edge wrap for horizontal exactly as the existing player-1 branch does); keep move encoding `"from-to"`. `partner()` becomes unused in cram paths — derive the partner from the parsed move string in `make_move` (it already receives both cells).

- [ ] **Step 3: Green + clippy.** Old tests updated to pass `0`.

- [ ] **Step 4: Tile work** — `domineering.tsx` create gains `, 0`; append `export const cram: GameDefinition` with `id: 'cram'`, name `'Cram'`, icon `'🀫'`, blurb `'Dominoes for two — but BOTH of you can place either way. Last to fit one wins.'`, same knobs/presets shape as Domineering (4–10), create `new wasm.DomineeringWasm(p.cols, p.rows, 1)`, `playerLabels: ['Player 1', 'Player 2']`. Register in index.ts + Brain-teasers + rules.ts:

```ts
  cram: 'Take turns placing a domino on any two free side-by-side cells — vertical or horizontal, your choice (unlike Domineering, both players may use both). The first player who cannot fit a domino loses.',
```

- [ ] **Step 5: i18n + build + calibrate (`scripts/calibrate.sh cram 20` after registering) + Playwright spot-check + commit**

```bash
git add treant-wasm/src/domineering.rs treant-wasm/examples/calibrate.rs docs/src/components/arcade
git commit -m "feat(arcade): Cram — impartial Domineering via an either-orientation flag"
```

### Task 2.3: Reversi anti flag → **Anti-Reversi** tile

**Files:**
- Modify: `treant-wasm/src/reversi.rs` (`term` at `:85-96`, `eval0` at `:150-154`, ctor at `:175-180`), `docs/src/components/arcade/games/reversi.tsx`, `games/index.ts`, `Launcher.tsx` (Family classics), `rules.ts`

**Interfaces:**
- Produces: `ReversiWasm::new(cols, rows, anti: u32)`. Anti: FEWEST discs wins at game end; eval sign-flips its material/corner terms (mobility keeps its sign — moves are still good).

- [ ] **Step 1: Failing test**

```rust
#[test]
fn anti_reversi_awards_the_win_to_fewer_discs() {
    let mut g = ReversiWasm::new(4, 4, 1);
    // Drive to terminal with any legal sequence (reuse the pattern from the
    // existing full-game test), then assert result is the LOW-count seat.
    while !g.is_terminal() {
        let m = g.legal_moves().split(',').next().unwrap().to_string();
        assert!(g.apply_move(&m));
    }
    let b = g.get_board();
    let x = b.chars().filter(|&c| c == 'X').count();
    let o = b.chars().filter(|&c| c == 'O').count();
    let expect = if x < o { "1" } else if o < x { "2" } else { "Draw" };
    assert_eq!(g.result(), expect);
}
```

- [ ] **Step 2: Implement** — `anti: bool` on the struct; in `term()` swap `Greater`→`Loss` / `Less`→`Win` when `anti`; in `eval0()` negate the disc-diff and corner terms when `anti` (keep `2*(mobility)` positive):

```rust
let material = (me as i32 - you as i32) + 12 * (my_corners - your_corners);
let signed = if self.anti { -material } else { material };
signed + 2 * (my_moves - your_moves)
```

(Adapt names to the file's real locals when editing — the shape above is the requirement.)

- [ ] **Step 3: Green + clippy; old tests pass `0`.**

- [ ] **Step 4: Tile** — `reversi.tsx` create gains `, 0`; append `export const antiReversi` with `id: 'anti-reversi'`, name `'Anti-Reversi'`, icon `'🙃'`, blurb `'Reversi upside-down: flip everything at the ENEMY — fewest discs wins.'`, same presets/knobs, create `new wasm.ReversiWasm(p.cols, p.rows, 1)`, playerLabels `['Red', 'Yellow']`. Register (index.ts + Family classics) + rules.ts:

```ts
  'anti-reversi': 'Play exactly like Reversi — flank enemy discs to flip them — but the goal is reversed: when the board fills or nobody can move, the player with the FEWEST discs wins. Try to give discs away.',
```

- [ ] **Step 5: i18n + build + calibrate anti-reversi (fresh `calibrate.rs` entry) AND re-run reversi + Playwright + commit**

```bash
git add treant-wasm/src/reversi.rs treant-wasm/examples/calibrate.rs docs/src/components/arcade
git commit -m "feat(arcade): Anti-Reversi — one sign flip, endless family arguments"
```

### Task 2.4: Nine Men's Morris Lasker flag → **Lasker Morris** tile

**Files:**
- Modify: `treant-wasm/src/ninemorris.rs` (`gen` at `:166-198`, ctor at `:311-320`), `docs/src/components/arcade/games/ninemorris.tsx`, `games/index.ts`, `Launcher.tsx` (Move & capture), `rules.ts`

**Interfaces:**
- Produces: `NineMorrisWasm::new(men_per_player, flying, lasker: u32)`. Lasker: while unplaced men remain, `gen()` offers BOTH placements AND slide moves (the hard place-then-slide phase gate is relaxed); default men for the tile = 10.

- [ ] **Step 1: Failing test**

```rust
#[test]
fn lasker_offers_slides_during_placement() {
    let mut g = NineMorrisWasm::new(10, 0, 1);
    assert!(g.apply_move("0"));  // P0 places
    assert!(g.apply_move("12")); // P1 places
    // P0 still has 9 unplaced men, but sliding the placed man must ALSO be legal:
    let moves = g.legal_moves();
    assert!(moves.split(',').any(|m| m.contains('-')), "expected a slide move, got {moves}");
}
```

- [ ] **Step 2: Implement** — `lasker: bool`; in `gen()` change the phase branch from `if self.placing() { placements } else { slides }` to emit placements while `placing()` AND (placements ∪ slides) when `lasker && on_board > 0`; keep mill-removal variants untouched.

- [ ] **Step 3: Green + clippy; existing tests pass `0`.**

- [ ] **Step 4: Tile** — `ninemorris.tsx` create gains `, 0`; append `export const laskerMorris` with `id: 'lasker-morris'`, name `'Lasker Morris'`, icon `'♟️'`, blurb `'The chess champion's morris: ten men, and you may slide before you finish placing.'`, defaultParams `{ numPlayers: 2, men: 10, flying: 0 }`, knobs same as parent, create `new wasm.NineMorrisWasm(p.men, p.flying, 1)`. Register (index.ts + Move & capture) + rules.ts entry (place-or-move each turn; mills capture; reduce to two or block to win).

- [ ] **Step 5: i18n + build + calibrate both + Playwright + commit**

```bash
git add treant-wasm/src/ninemorris.rs treant-wasm/examples/calibrate.rs docs/src/components/arcade
git commit -m "feat(arcade): Lasker Morris — place-or-move flag on the morris engine"
```

### Task 2.5: Trails knight-movement flag → **Joust** tile

**Files:**
- Modify: `treant-wasm/src/trails.rs` (`orth` at `:42-58`, `gen` at `:59-66`, ctor at `:148-153`), `docs/src/components/arcade/games/trails.tsx`, `games/index.ts`, `Launcher.tsx` (Move & capture), `rules.ts`

**Interfaces:**
- Produces: `TrailsWasm::new(cols, rows, knight: u32)`. Knight mode: moves are the 8 knight leaps instead of 4 orthogonal steps; vacated cell still becomes a wall; move encoding `"from-to"` unchanged (UI untouched).

- [ ] **Step 1: Failing test**

```rust
#[test]
fn joust_moves_like_a_knight() {
    let g = TrailsWasm::new(6, 6, 1);
    let moves = g.legal_moves();
    // Every move must span a (1,2) or (2,1) offset.
    for m in moves.split(',') {
        let (f, t) = m.split_once('-').unwrap();
        let (f, t) = (f.parse::<i32>().unwrap(), t.parse::<i32>().unwrap());
        let (dr, dc) = ((t / 6 - f / 6).abs(), (t % 6 - f % 6).abs());
        assert!((dr, dc) == (1, 2) || (dr, dc) == (2, 1), "non-knight move {m}");
    }
}
```

- [ ] **Step 2: Implement** — `knight: bool`; add a `leaps()` neighbour fn with the 8 knight offsets (bounds-checked like `orth()`), and pick per-flag in `gen()`.

- [ ] **Step 3: Green + clippy; existing tests pass `0`.**

- [ ] **Step 4: Tile** — trails.tsx create gains `, 0`; append `export const joust` with `id: 'joust'`, name `'Joust'`, icon `'🐴'`, blurb `'Knights on a burning board: leap, scorch the square you left, outlast your rival.'`, presets 6×6 ⭐ / 8×8 🔲 / 10×10 🤯 — wait, engine clamps 4–9, so 🤯 is 9×9 — knobs 5–9, create `new wasm.TrailsWasm(p.cols, p.rows, 1)`. Register (Move & capture) + rules.ts entry.

- [ ] **Step 5: i18n + build + calibrate both + Playwright (verify knight-leap targets highlight) + commit**

```bash
git add treant-wasm/src/trails.rs treant-wasm/examples/calibrate.rs docs/src/components/arcade
git commit -m "feat(arcade): Joust — knight-leap flag turns Trails into the burning-board classic"
```

### Task 2.6: Hex pie rule (swap on move 2)

**Files:**
- Modify: `treant-wasm/src/hex.rs` (ctor `:201-205`, `available_moves` `:152-157`, `make_move` `:158-161`), `docs/src/components/arcade/games/hex.tsx` (handle + HexBoard swap button)

**Interfaces:**
- Produces: `HexWasm::new(n, pie: u32)`. With `pie=1`, after the first stone the second player's legal moves include the literal move `"swap"`, which reflects the first stone across the long diagonal and recolors it (standard pie implementation). `legalMoves()` surfaces it; the Hex tile gains knob `{ key: 'pie', label: 'Pie rule', min: 0, max: 1, step: 1 }` defaulting 1.

- [ ] **Step 1: Failing test**

```rust
#[test]
fn pie_swap_mirrors_and_recolors_the_first_stone() {
    let mut g = HexWasm::new(7, 1);
    assert!(g.apply_move("9"));            // P0: (r1,c2)
    assert!(g.legal_moves().split(',').any(|m| m == "swap"));
    assert!(g.apply_move("swap"));
    let b = g.get_board();
    assert_eq!(b.chars().nth(9), Some(' '));   // original cell cleared
    assert_eq!(b.chars().nth(15), Some('O'));  // mirrored (r2,c1) is now P1's
    assert_eq!(g.current_player(), 0);
}
```

- [ ] **Step 2: Implement** — track `ply`; when `pie && ply == 1`, append the `Swap` move; `make_move(Swap)` clears `(r,c)`, sets `(c,r)` to player 1, advances turn. Solver/winning-cells paths unaffected (post-swap it's a normal position).

- [ ] **Step 3: Green + clippy; old tests pass `0`.**

- [ ] **Step 4: UI** — hex.tsx: `makeHandle` passes `p.pie ?? 1`; add the knob + a "♻ Swap sides" pill in `HexBoard` rendered only when `legalMoves` includes `'swap'` (calls `onMove('swap')`); add one line to the tile's `rules` text explaining the swap. i18n for the new strings.

- [ ] **Step 5: Build + calibrate hex (re-run — first-move advantage changed) + Playwright (swap pill appears for P2 on move 2 and works) + commit**

```bash
git add treant-wasm/src/hex.rs docs/src/components/arcade/games/hex.tsx docs/src/components/arcade/i18n docs/src/components/arcade/rules.ts
git commit -m "feat(arcade): Hex pie rule — the swap Hex players ask for first"
```

### Task 2.7: Connect Four family — k≥2, MAX_DIM 12, Pop Out + Cylinder tiles

**Files:**
- Modify: `treant-games/src/grid.rs` (`GridConfig`, `winner` `:87-115`, `terminal_value` `:250-258`, `make_move` `:237-248`), `treant-wasm/src/connectfour.rs` (clamps `:8-24`, ctor `:54-61`, `apply_move` `:146-157`), `docs/src/components/arcade/games/connectFour.tsx`, `games/index.ts`, `Launcher.tsx` (Family classics), `rules.ts`
- Test: `treant-games/src/grid.rs` + `connectfour.rs` tests

**Interfaces:**
- Produces: `GridConfig` gains `wrap_cols: bool` and `allow_pop: bool`. `ConnectFourWasm::new(cols, rows, k, num_players, variant: u32)` with `variant: 0=classic, 1=pop-out, 2=cylinder`. Pop move encoding: `"pop<col>"`. CRITICAL: with pops, a move can complete the OPPONENT's line, so `terminal_value` must become mover-relative (`winner == just-moved ? previous-mover-wins` logic) — implement as: `winner().map(|w| if w == self.current { Lose-for-mover…} )` exactly per the existing mover-relative convention, now comparing seats instead of assuming the previous mover won.

- [ ] **Step 1: Failing tests (three)**

```rust
#[test]
fn k2_is_accepted() {
    let g = ConnectFourWasm::new(7, 6, 2, 2, 0);
    // two moves by P0 in adjacent columns should win at k=2 …drive and assert result "1"
}
#[test]
fn cylinder_wraps_the_win_check() {
    // place k=4 discs in cols 10,11,0,1 on a 12-wide cylinder → P0 wins
}
#[test]
fn pop_that_completes_opponents_line_loses() {
    // construct a stack where popping P0's bottom disc drops P1's discs into a 4-line;
    // P0 plays "pop3"; assert result "2".
}
```

(Each test constructs its position with explicit `apply_move` sequences — write them against the real drop physics while implementing; the assertion contracts above are fixed.)

- [ ] **Step 2: Implement grid.rs** — `wrap_cols`: in `winner()`, when set, step columns modulo `cols` (rows never wrap); `allow_pop`: `GridMove` stays `u8` — encode pops as `col + 128` (documented; cols ≤ 12 so no collision); `make_move` on a pop shifts the column down one cell; `terminal_value` becomes seat-aware as specified. `connectfour.rs`: `MAX_DIM` 10→12, k clamp `.clamp(3, …)`→`.clamp(2, …)`, ctor takes `variant`, `apply_move` parses `"pop<n>"`, `legal_moves` includes pops for columns whose bottom disc is the mover's (pop-out rule).

- [ ] **Step 3: Green + clippy across BOTH crates** (`cargo test -p treant-games -p treant-wasm`) — TicTacToeWasm shares grid.rs, so its full suite is the regression net for the terminal change.

- [ ] **Step 4: Tiles** — connectFour.tsx: create gains `, 0`; widen cols/rows knobs to 12 and k min to 2; append `export const popOut` (`id: 'pop-out'`, icon `'⤵️'`, blurb `'Connect Four where discs can leave: pop your own bottom disc and watch the column fall.'`, create variant 1) and `export const cylinderFour` (`id: 'cylinder-four'`, icon `'🛢️'`, blurb `'Connect Four on a tube — lines wrap around the edges.'`, create variant 2, default cols 12). The Pop Out BOARD needs a pop affordance: add a small `⤵` button row under each column, rendered only when `legalMoves` contains `pop<c>` (same pattern as the existing drop-arrow header row in `ConnectFourBoard`). Register both (Family classics) + rules.ts entries.

- [ ] **Step 5: i18n + build + calibrate connect-four AND pop-out AND cylinder-four + audit run + Playwright (pop a disc, see the column fall; win across the seam on cylinder) + commit**

```bash
git add treant-games/src/grid.rs treant-wasm/src/connectfour.rs treant-wasm/examples/calibrate.rs docs/src/components/arcade
git commit -m "feat(arcade): Connect Four family — k=2 chaos, 12-wide, Pop Out + Cylinder tiles"
```

### Task 2.8: Pig dice variants → **Two-Dice Pig** + **Big Pig** tiles

**Files:**
- Modify: `treant-wasm/src/pig.rs` (ctor `:149-158`, bust rule `:79-88`, `chance_outcomes` `:90-96`), `docs/src/components/arcade/games/pig.tsx`, `games/index.ts`, `Launcher.tsx` (Dice & solo), `rules.ts`

**Interfaces:**
- Produces: `PigWasm::new(num_players, target, variant: u32)` — `0` classic 1-die; `1` Two-Dice (roll 2: any single 1 busts the TURN, double-1 wipes the SCORE); `2` Big Pig (double-1 scores +25, other doubles score double, no wipes). Chance nodes enumerate the 21 unordered two-die outcomes with correct weights (6 doubles at 1/36, 15 pairs at 2/36).

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn two_dice_double_one_wipes_banked_score() { /* seat with banked 30, force Die2(1,1), assert score 0 and turn passes */ }
#[test]
fn big_pig_double_one_scores_twenty_five() { /* force Die2(1,1), assert turn_total += 25 and turn continues */ }
```

(Forcing outcomes: extend the existing test-only hook the file uses for deterministic rolls — pig.rs tests already drive `make_move` with explicit `PigMove::Die(v)` values; add a `Die2(u8, u8)` move arm and drive it the same way.)

- [ ] **Step 2: Implement** — `variant: u8` field; `PigMove::Die2(a, b)`; `chance_outcomes` emits the 21 weighted pairs when `variant > 0`; bust logic per the interface table above. `Hold` unchanged.

- [ ] **Step 3: Green + clippy; classic tests pass `variant 0` untouched.**

- [ ] **Step 4: Tiles** — pig.tsx create gains `, 0`; append `twoDicePig` (`id: 'two-dice-pig'`, icon `'🎲'`, blurb `'Two dice, double the pace — but snake eyes eat your whole score.'`, `noUndo: true`, variant 1) and `bigPig` (`id: 'big-pig'`, icon `'🐷'`, blurb `'Doubles pay double and snake eyes pay 25. Greed, rewarded. Mostly.'`, `noUndo: true`, variant 2), both with the parent's knobs (players 2–6, target 20–200). Register (Dice & solo) + rules.ts entries.

- [ ] **Step 5: i18n + build + calibrate all three pig tiles + Playwright (watch-AI mode finishes a Two-Dice game) + commit**

```bash
git add treant-wasm/src/pig.rs treant-wasm/examples/calibrate.rs docs/src/components/arcade
git commit -m "feat(arcade): Two-Dice Pig + Big Pig — published variants as one-flag tiles"
```

## Phase 3 — Small engines over existing libraries (follow-on plan docs, one per game)

Priority order and the ≥80% reuse claims, pinned here so each follow-on plan starts from the same map:

1. **Heap Nim upgrade** (`plans/…-heap-nim.md`) — multi-heap + max-take + misère; replaces the single-pile toy; new heap board UI. Reuses: difficulty/weak_move plumbing, NimBoard layout patterns. Absorbs: Moore's Nim, Fibonacci Nim as flags/tiles.
2. **World Threes** (`…-world-threes.md`) — Shift's place/slide phases over per-board adjacency tables (7 boards); new SVG graph-board renderer (shared with Morris/Mu Tōrere patterns). Absorbs nine catalogue entries as board presets.
3. **Gale** (`…-gale.md`) — Hex's union-find over edge-claim cells; new edge-grid renderer.
4. **Slimetrail** (`…-slimetrail.md`) — Trails-style walk of one shared token toward per-player goals.
5. **Toads & Frogs** (`…-toads-frogs.md`) — 1-D hop engine (Treblecross-shaped); trivial board.
6. **Pawn Duel** (`…-pawn-duel.md`) — Frontline + double-step/en-passant; Hexapawn ships as a preset tile.
7. **Strand** (`…-strand.md`) — Trails + remove-any-tile; needs a 3-part move + 2-phase board input (the reason it isn't a Phase-2 flag).
8. **Len Choa** (`…-len-choa.md`) — Bagh-Chal hunt logic on a triangle graph.

## Phase 4 — Full engines (follow-on plan docs)

1. **Draughts** (`…-draughts.md`) — THE flagship. Build as `treant-games/draughtslib` (capture-chain generator with max-capture policies, promotion rules, flying kings, orthogonal option, misère) + one `DraughtsWasm`. Ships up to 9 config-only tiles (American ⭐, Brazilian, Pool, Russian, Spanish, Italian, Frisian, Giveaway 🙃, International 10×10). Later small engines over the lib: Dama, Alquerque, Latrunculi.
2. **Surakarta** (`…-surakarta.md`) — loop-capture path generation; arced-board SVG renderer (the visual showpiece).

## Phase 5 — Hidden information (follow-on plan docs; new UX capability)

1. **Pass-screen primitive + Bulls & Cows** (`…-pass-screen-bulls-cows.md`) — blackout handoff screen + per-player secret views built once, proven on the cheapest hidden-info game (near-perfect AI via consistent-set search wrapped in the treant evaluator).
2. **Salvo** (`…-salvo.md`) — fleet editor, determinized-MCTS gunner (placement sampling + hit-probability voting via treant chance nodes).
3. **Ambush** (`…-ambush.md`) — hidden ranks, determinization; LAST — hardest AI, ship only if Salvo's determinization proves strong.

---

## Self-review record

- **Spec coverage:** Peter's four questions map to Part I.1 (enabling updates), I.2 (UI/UX/AI effects), I.3 (refactor/reuse/new per game), I.4 (config-only vs 80%-lib vs full-engine classification); "build out the full plan" → Phases 0–2 fully tasked, 3–5 scoped as per-subsystem follow-on plans per the writing-plans scope rule.
- **Placeholder scan:** no TBDs. Two tests in Task 2.7 Step 1 and one in 2.8 intentionally specify the assertion contract and instruct the implementer to derive the move sequence against real drop physics — the contract (who wins) is fixed, the setup is mechanical.
- **Type consistency:** ctor extensions are always a trailing `u32` flag defaulting to `0` = old behavior; every parent tile's `create` gains `, 0` in the same task; `result()` seat-digit contract restated where terminal logic changes (2.3, 2.7).
