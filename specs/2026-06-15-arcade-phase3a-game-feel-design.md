# Phase 3a — Arcade game feel (animations + sound)

**Date:** 2026-06-15
**Status:** Approved scope, ready for implementation
**Depends on:** Phase 2b (arcade with 6 games). Complete.

> Spec in repo-root `specs/`. Phase 3 is split: **3a = game feel (this)**;
> **3b = destination** (landing/hero, desktop layout, share links) comes next,
> with browser mockups for the landing page.

## Goal

Make the arcade *feel* like a real game, not a state machine — movement
animations and a little sound — without new dependencies or asset files, and
without changing any game logic.

## A. Animations (CSS, diff-driven)

The boards re-render from a board string each move. To animate only what changed,
each `Board` keeps a `useRef` of the previous board and derives the changed
cell(s); a transient CSS class on those cells plays a keyframe. No game-logic
changes, no per-tile identity model.

- **Connect Four — disc drop.** Diff finds the newly-filled cell; that disc gets a
  `dropIn` animation (translateY from above the board → 0, slight ease-out bounce).
- **2048 — spawn + merge.** Diff each of 16 tiles vs previous: a cell that went
  `0 → n` is a **spawn** (`popIn`: scale 0→1); a cell whose value **increased**
  (merge) gets `mergeBump` (scale 1→1.15→1). Full directional sliding is out of
  scope (needs a tile-identity model) — pop/bump gives the feel cheaply.
- **Tic-Tac-Toe / Shift — place / slide.** The changed cell (newly `me`) gets
  `popIn`. (Shift already highlights the selected piece.)
- **Win / game-over celebration.** The overlay card + icon animate in
  (`overlayPop`: scale/opacity) and the 🏆/🎮 icon does a short `celebrate`
  wiggle. Applies to every game.
- **Reduced motion:** wrap the keyframes in `@media (prefers-reduced-motion: no-preference)`
  so users who opt out get instant updates.

Mancala/Nim get the celebration + sound but no per-piece animation in 3a (their
multi-stone moves don't diff to a single cell cleanly; revisit if wanted).

## B. Sound (Web Audio, synthesized — no files)

A tiny `sound.ts` module lazily creates one `AudioContext` and plays short
synthesized blips (oscillator + gain envelope). No asset files, no deps.

- API: `sound.move()`, `sound.drop()`, `sound.merge()`, `sound.win()`,
  `sound.lose()`, `sound.draw()`. Each is a 1–2 oscillator envelope (e.g. `move`
  = 120ms square blip; `win` = quick rising arpeggio; `lose` = falling tone).
- **Mute** persisted in `localStorage` (`arcade.muted`). The module reads it before
  every sound; a no-op when muted or before first user gesture (AudioContext is
  resumed on first interaction to satisfy autoplay policies).
- **Triggers (centralized in `useGameSession`, so all games get it free):**
  - each applied move (human or AI) → `sound.move()` (Connect Four can use
    `sound.drop()` via an optional `def.moveSound`).
  - 2048 merge (score increased on a move) → `sound.merge()`.
  - terminal → `sound.win()` / `sound.lose()` / `sound.draw()` based on result and
    whether the human won (for solo: win if `max_tile` milestone, else `lose`).
- **Mute toggle UI:** a 🔊/🔇 button in the arcade header (`ArcadeShell`),
  persisted; visible on every screen.

### Win/lose attribution
- Multiplayer human-v-AI: `win` if the human seat (0) won, else `lose`; `draw` on
  draw. Pass-and-play / watch: just `win` (someone won) / `draw`.
- Solo: terminal always `lose` fanfare unless `endText` indicates a 2048 milestone
  (then `win`). Simplest: solo terminal → `draw`/neutral tone; a reached-2048 →
  `win`. (Keep it gentle — 2048 "game over" shouldn't sound punishing.)

## Implementation surface
- New: `arcade/sound.ts` (module), `arcade/controls/MuteToggle.tsx`.
- Modify: `useGameSession.ts` (trigger sounds; expose nothing new), `ArcadeShell.tsx`
  (mute toggle in header), `arcade.module.css` (keyframes + animation classes),
  and each animated `Board` (`connectFour.tsx`, `game2048.tsx`, `ticTacToe.tsx`,
  `shift.tsx`) to add the diff-driven class. `GamePlay.tsx` overlay gets the
  celebration classes.
- Optional `GameDefinition.moveSound?: 'move' | 'drop'` so Connect Four uses the
  drop blip. (Optional/additive.)

## Testing
JS-only; verify via Playwright + manual (no console errors):
- Connect Four: dropping a disc adds the `dropIn` class to the landed disc;
  game-over overlay animates.
- 2048: a move adds `popIn`/`mergeBump` to changed tiles; score-up triggers merge
  sound (can't hear in Playwright — assert the class/flag path instead).
- Mute toggle flips `localStorage.arcade.muted` and persists across reload.
- All six games still play correctly (animations/sound are additive).

## Out of scope (3b / later)
- Landing/hero page, desktop/tablet layout, share/deep links (Phase 3b).
- Full 2048 directional tile sliding (needs a tile-identity model).
- Win-line highlighting (needs per-game winning-line computation in JS).
- Mancala/Nim per-piece animation.

## Risks
- **AudioContext autoplay policy** — must `resume()` on first user gesture, else
  the first sounds are silent; gate all sound behind "context started".
- **Animation jank on re-render** — only the changed cell gets the class, and it's
  cleared next render; keep keyframes short (≤250ms) and GPU-friendly (transform/
  opacity only).
- **Don't animate on initial mount** (the whole board would animate). Skip the
  diff animation when there's no previous board (first render / reset).
