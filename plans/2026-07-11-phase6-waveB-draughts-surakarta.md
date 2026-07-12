# Phase 6 — Wave B: Spanish + Italian Draughts + Surakarta Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. FOUR implementer tasks (6-1 draughts flags, 6-2 Spanish/Italian tiles, 6-3 Surakarta engine, 6-4 Surakarta tile/board), sequential, each reviewed; TWO pushes (after 6-2 gates, after 6-4 gates) each with a critic checkpoint. Master-plan Global Constraints + ALL standing program rules apply (verification non-optional; sourced rules quoted in code; single-sourced encodings + round-trip; wave-A bit-identity; i18n ×6; wasm rebuild flow; audit; guards; calibration policy Hard=max rung; explicit-path commits with trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`; never push — the controller pushes at gates).

**Goal:** Two national draughts (Spanish, Italian) as near-config tiles over `draughts.rs` via 2 net new flags, plus Surakarta — the arc-capture showpiece — as a from-scratch engine. Research + all sourced rule quotes: `plans/2026-07-11-draughts-waveB-surakarta-research.md` (READ IT; every load-bearing rule below is quoted there with URLs — re-verify, don't re-derive). Frisian is deferred (structurally incompatible); Turkish is a stretch, NOT in this plan.

## Task 6-1 — draughts engine: 2 new flags

`treant-wasm/src/draughts.rs`. Ctor grows to `new(size, men_rows, flying_kings, men_capture_back, max_capture, promote_mid_chain, misere, men_cannot_capture_kings, capture_priority)`. Both new flags default 0; **the six wave-A variants must stay bit-identical** (assert: extend the existing flag-table test AND a self-play transcript identity test at fixed seed for at least American + International with new flags 0).

1. **`men_cannot_capture_kings`** (0/1): in `chain()`, when the mover is a man and the jumped piece is a king, skip that continuation. NOTE the subtle consequence (test it): if a man's ONLY jump is over a king, the man has no capture — capture obligation falls to other pieces or quiet moves become legal.
2. **`capture_priority`** enum (0=none, 1=Spanish, 2=Italian): a post-`max_capture` lexicographic retain over generated chains, computed against the pre-capture board at generation time:
   - Mode 1 (Spanish): `(count, kings_captured)` — "as much kings as possible must be captured" (ludoteka, quoted in research §1).
   - Mode 2 (Italian): `(count, capturing_piece_is_king, kings_captured, king_captured_earliest)` — the verbatim 4-level Wikipedia hierarchy (research §2). "king_captured_earliest" = among remaining ties, prefer the chain whose first-captured king comes earliest in the jump sequence.
   - `capture_priority > 0` requires `max_capture = 1` semantics as its base — decide and document: either it implies max_capture, or constructor rejects/normalizes the combo (pick one, test it).
3. **Sourcing gate before coding Italian:** the research flags Italian king movement (non-flying, step-1) as lacking a clean Wikipedia verbatim. WebFetch a FID (Federazione Italiana Dama) or equivalent federation source and quote it in the code comment. Blocked-over-invent if it can't be pinned.
4. Tests (~12): each priority tier broken by a crafted tie (count tie → king-capturer wins; that tie → more kings; that tie → earliest king), man-blocked-by-king positions (incl. the only-jump-is-king case), wave-A bit-identity, round-trip unchanged, audit re-run (`cargo run --release --example calibrate -p treant-wasm -- audit` → `ok draughts`), clippy 0.
5. Update `treant-dynamic` golden tests ONLY if the draughts ctor is exposed there (check; document either way).

## Task 6-2 — Spanish + Italian tiles

`docs/src/components/arcade/games/draughts.tsx` (extend the existing family; these are `variantOf` draughts tiles like the six shipped):
- **Spanish (Damas) ⭐**: 8,3,1,0,1,0,0 + mcck=0, priority=1. Flying kings, forward-only men, most-kings tiebreak.
- **Italian (Dama) 🇮🇹→ no flag emoji, pick a glyph-consistent icon**: 8,3,**0**,0,1,0,0 + mcck=1, priority=2. Non-flying kings, men can't jump kings.
- Update ALL existing six tile configs for the widened ctor (two trailing 0s). Expose the two new flags in the family's custom-knobs panel ("go crazy" philosophy): `men_cannot_capture_kings` toggle, `capture_priority` 3-way.
- Wasm rebuild flow; i18n ×6 (`--check` 0/0); check-categories + GLYPHS entries (real monochrome SVGs — the 5-missing-glyphs lesson); calibration: run the calibrate example for both tiles, Hard = max rung per CALIBRATION.md, record in plans/ai-calibration-results.md.
- Verify: production build + Playwright — play both tiles vs AI; craft a browser-reachable position asserting the Italian man-can't-jump-king rule visibly binds (legal-move highlights exclude the king jump); wave-A tiles still launch and play.

## Task 6-3 — Surakarta engine (`treant-wasm/src/surakarta.rs`)

From scratch; precedent for structure: any small engine (e.g. `gale.rs`). Research §5 has the full sourced rules — quote them in the header.
- **State:** 6×6 points, 12 pieces/side starting on each player's two nearest rows — but FIRST confirm the starting rows against a second source (rules PDF / second encyclopedia; the research flags Wikipedia as not verbatim on this). Blocked-over-invent.
- **Moves:** `(from,to)` encoding `"r,c-r,c"`-style consistent with existing engines (check draughts/gale conventions first; single-sourced Display/parse + round-trip incl. edge points). Quiet = 8-neighbour step to empty. Capture = land on enemy via a clear loop arc: move GENERATION does circuit-reachability (walk the loop tracks from `from`, all traversed points empty, capture the first enemy landed on). `(from,to)` is unambiguous — distinct arcs to the same target are the same move (research §5 settles this; assert it in a test: a position with two clear arcs to one target generates ONE move).
- **Terminal:** win = opponent has 0 pieces. Draw cap: N plies without a capture → Draw (research recommends the 40-ply precedent from draughts.rs — reuse that constant/rationale, document).
- **Also expose** a `canonical_arc(from, to) -> String` helper (comma-joined point path, shortest clear route, deterministic) for the UI's mandatory capture animation — engine owns the geometry, the board just draws it.
- Tests: loop-track geometry (the 8 corner loops — get the track definitions right; this is the Picaria lesson: derive from a drawn diagram in comments, reviewer re-derives independently), capture legality (blocked arcs, multi-loop routes, may-pass-same-point-twice), one-move-per-(from,to) dedup, round-trip, draw cap, termination, ai_plays, audit-registered. Calibrate: measured ladder if self-play works (it should — perfect info), Hard = max rung.

## Task 6-4 — Surakarta tile + arc board

- Tile: id `surakarta`, name "Surakarta", category Strategy classics (check real category names in Launcher.tsx), standalone. Knobs: conservative — this game's identity IS its board; draw-cap plies knob (20–80) and maybe board-size only if the engine parameterized it (do NOT force size params the engine lacks). Presets: Classic ⭐ + one more max (judgment).
- **Board (the big build):** SVG — 6×6 points on grid lines with the two nested corner-loop tracks drawn as the signature rounded arcs. Tap piece → engine-authoritative highlights (empty neighbours + arc-reachable enemies). **Mandatory capture animation:** the piece travels the `canonical_arc` path (CSS/SVG path animation, ~600ms, respects prefers-reduced-motion) — a teleport capture is a UX failure, not a nice-to-have. lastCells marks the capture. 390px-up comfortable; dark palette vars only.
- resultFlavor: capture-all win line (e.g. "🏵️ {winner} swept the board!" — i18n'd); draw-cap flavor for the stalemate ending.
- Verify: production build + Playwright — full game vs AI, a capture animating along the arc (screenshot mid-animation or assert the animation class/path), highlights match engine legal_moves exactly for a crafted position, i18n ×6, guards, audit.

## Gates

Reviews per task (opus): 6-1 = comparator correctness vs quoted hierarchy + wave-A bit-identity evidence + the only-jump-is-king edge; 6-2 = config values vs research tables + calibration honesty; 6-3 = independent re-derivation of the loop-track geometry (Picaria lesson) + (from,to) dedup + sourcing of starting rows; 6-4 = arc animation present + engine-authoritative highlights + §7 gotchas. Critic checkpoint 8 after 6-2 (play both national variants cold, does the priority rule ever confuse a family player? verdict on exposing the flags as knobs) → push 1. Critic checkpoint 9 after 6-4 (Surakarta cold-legibility: does the arc capture teach itself? animation feel, Easy/Hard) → push 2.
