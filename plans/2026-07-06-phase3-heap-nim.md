# Phase 3A — Heap Nim Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (one implementer task + review). Follows the master plan (plans/2026-07-05-knobs-variants-and-new-games.md) Global Constraints verbatim.

**Goal:** Upgrade the single-pile Nim toy into real multi-heap Nim (heaps, max-take, misère) while keeping the shipped Nim tile's default feel byte-identical.

**Architecture:** Rework `treant-wasm/src/nim.rs` (new move encoding + heap state) and `docs/src/components/arcade/games/nim.tsx` (new heap-rows board UI). No new tile this task — the existing `nim` tile gains knobs/presets. Moore's-Nim and Fibonacci-Nim flags are explicitly deferred (backlog).

## Verified facts (from prior investigation — do re-verify on read)

- Current engine: `Nim { stones: u8, current: Player }`; moves `Take1`/`Take2` only; `NimWasm::new(stones: u8)`, no upper clamp; terminal `stones == 0 → Loss` (normal play: taker of last stone WINS).
- Current tile: `defaultParams { numPlayers: 2, stones: 15 }`, one knob stones 3–60, `NimBoard` renders a single pile.
- Difficulty is HAND-SET (CALIBRATION.md exception — nim lacks the uniform seat API in the harness); keep it hand-set, verify the weak-move path still returns applyable tokens (quadline lesson: Display/parse single source).
- Build guards now enforce: category membership AND an icons.tsx GLYPHS entry for every game id (nim already has both).

## Binding shape (adapt exact code to the real file)

1. **Engine** — `NimWasm::new(stones: u32, heaps: u32, max_take: u32, misere: u32)`:
   - Heap sizes descend from `stones` by 2 with floor 1: stones=7, heaps=4 → [7,5,3,1] (Marienbad). heaps clamps 1–8, stones 1–60, max_take 0=unlimited else 1–10, misère 0/1.
   - `heaps=1, max_take=2, misere=0` must reproduce today's game exactly (same legal moves, same terminal semantics) — regression test required.
   - Move encoding: `"<heap>-<count>"` (dash convention, e.g. "2-3" = take 3 from heap 2, 0-indexed heap). Display, encode, decode single-sourced; round-trip test required (gen → Display → apply_move on a clone accepts every move).
   - Terminal: no stones left → normal: mover-to-move LOSES (previous taker won); misère: mover-to-move WINS. `result()` contract unchanged ("", "Draw" never occurs, seat digit).
   - Solver stays enabled (Nim is the flagship solver demo). Add a test: solver proves the known first-player-win on [3,5,7] normal play... only if the existing test style supports `root_proven_value()` cheaply; otherwise a played-to-terminal verdict test.
2. **Tile** (`nim.tsx`, same id `nim`):
   - defaultParams `{ numPlayers: 2, stones: 15, heaps: 1, maxTake: 2, misere: 0 }` — the shipped default game is unchanged.
   - Knobs: stones 3–60 step 1 · heaps 1–8 · maxTake 0–10 (label it so 0 reads as "no limit" — e.g. label 'Max take (0 = any)') · misere 0–1.
   - Presets (all params explicit — the preset-highlight comparator compares the full key union): `Classic ⭐` (current default) · `Marienbad 🎩` {stones 7, heaps 4, maxTake 0, misere 1} · `Ten Heaps 🤯` {stones 21, heaps 10→clamped? NO — heaps knob max 8; use heaps 8, stones 17} · `Last Loses 🙃` {stones 15, heaps 1, maxTake 2, misere 1}.
   - Blurb/rules updated for the general game (count-agnostic of heap count; mention misère knob). rules.ts entry updated to match.
3. **Board UI** (`NimBoard` rework): render each heap as a row of stone dots; tapping the k-th stone from the row's end proposes taking k stones (visual selection), with a confirm tap/button; must satisfy ARCADE.md touch rules (≥44px targets, no dead taps — illegal counts beyond maxTake get the head-shake). Single-heap default must still look/feel like today's pile. `lastCells` marker: optional — if heap rows don't map 1:1 to board-string indices, skip it and say so (the master plan's rule).
4. **get_board encoding**: choose and document (e.g. comma-joined heap counts "7,5,3"); keep variant-0-style backward shape only if the current board string is already that — check first; the Board and engine must agree.

## Task (single implementer dispatch)

Steps: failing tests first (ctor arity + heap-gen + round-trip + misère terminal + regression single-pile) → engine → tile+board → wasm rebuild (`wasm-pack build --target web`; docs `npm install treant-wasm && git checkout package.json`) → audit run (calibrate example `audit` — nim must stay ok) → i18n --write + six-locale top-up + --check 0/0 → `npm run build` green → Playwright on static :3939 (kill animations): Marienbad preset renders 4 rows [7,5,3,1]; take 3 from the 7-heap via stone taps; easy AI responds; play a misère single-pile game to the end and assert the loser/winner banner matches misère semantics → commit (explicit paths, trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`), do NOT push.

## Review gate

Standard two-verdict review (spec + quality) with scrutiny on: single-pile regression byte-equivalence; move round-trip; misère terminal polarity (and whether the hand-set difficulty still makes sense — easy must remain beatable in misère); board-string/Board agreement; preset param completeness; translations ×6.

## Ship

Push after review approval + critic checkpoint (bundled with the next Phase-3 game if it lands the same session).
