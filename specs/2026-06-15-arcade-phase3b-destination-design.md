# Phase 3b — Arcade as a destination (landing, desktop, share)

**Date:** 2026-06-15
**Status:** Approved (landing direction chosen via mockups), ready for implementation
**Depends on:** Phase 3a (game feel). Complete.

> Spec in repo-root `specs/`. Implement the **landing** first (decided), then
> desktop layout, then share links.

## Scope
Three polish items to make `/arcade` feel like a public destination:
1. **Landing page — direction C ("Featured spotlight")** — chosen from browser mockups.
2. **Desktop / tablet layout** — stop being a centered phone column on big screens.
3. **Share / deep links** — URL params to open a specific game/preset.

## 1. Landing — Featured spotlight (C)

Rewrite `Launcher.tsx`:
- **Rotating featured hero**: a gradient card (tinted by the featured game's seat-1
  color) with a `✨ FEATURED` label, the game's big icon, name, blurb, a small
  decorative **board-art** motif, and a **▶ Play** button → that game's setup.
  - Auto-rotates through `GAMES` on an interval (~5s) with a gentle fade; pauses
    on hover/focus. Starts on a stable index (e.g. 0) — **not** `Math.random`
    (SSR-safe; Docusaurus pre-renders).
- **Board-art per game** (`heroArt`): a tiny inline motif (a few colored discs for
  Connect Four, X/O for Tic-Tac-Toe, two tiles for 2048, pit dots for Mancala,
  stones for Nim, pieces for Shift). Decorative only — not a live game.
- **"More games" grid** below: a compact icon+name grid of **all six** games
  (the featured one is also here, so nothing is unreachable), each → setup.
- Keep the title/tagline; the mute toggle stays in the shell header.

## 2. Desktop / tablet layout

The arcade is `max-width: 560px` centered — fine on a phone, sparse on a laptop.
- On wider viewports (`min-width: 720px`), widen the container and lay the launcher
  out with the hero and grid side-by-side (hero left, games grid right), or a wider
  grid. In-game screens stay centered/comfortable (a game board shouldn't stretch).
- Use CSS only (media queries in `arcade.module.css`); no JS layout logic.
- Verify the six game boards still look right at desktop width (cap board width so
  e.g. Tic-Tac-Toe doesn't become huge).

## 3. Share / deep links

- `arcade.tsx` reads URL params on load: `?game=<id>&mode=<mode>&difficulty=<diff>`
  (and game-specific params later). If `game` matches, `ArcadeShell` opens that
  game's **setup** (not auto-start — respects the locked-setup flow) preselected.
- A **Share** button on the setup screen copies a link (`navigator.clipboard`) with
  the current `game`/`mode`/`difficulty` so you can text "play this" to someone.
- Pure client-side (static hosting); no server. Unknown/absent params → normal
  launcher.

## Testing (Playwright + manual, no console errors)
- Landing shows the rotating featured hero + the six-game grid; Play and grid tiles
  reach setup.
- Resize to desktop width → layout reflows (hero + grid not a thin column); boards
  still sized sensibly.
- `/arcade?game=mancala&mode=pvp` opens Mancala setup with pass-and-play selected.
- Share button copies a URL containing the current selection.
- All games still play.

## Out of scope
- Live auto-playing board in the hero (decorative art only this phase).
- Per-game param deep-links beyond game/mode/difficulty (add later if wanted).
- SEO/meta polish, social preview images (could be a tiny follow-on).

## Risks
- **SSR/pre-render**: Docusaurus statically renders pages — no `Math.random`/
  `Date.now` at module/render top level; the rotation interval lives in a
  `useEffect` (client-only), initial featured index is constant.
- **Clipboard API** needs HTTPS or localhost; on the LAN IP it may be blocked —
  fall back to showing the URL in a text field to copy manually.
