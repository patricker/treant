# Phase 3D+3E — Gale & Slimetrail Implementation Plans

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (one implementer task per game, sequential; review each). Master-plan Global Constraints apply. Program lessons apply (verification non-optional; Display/parse single-sourced + round-trip test; rules are load-bearing — cite sources in code comments; build guards need category + glyph; i18n ×6; explicit-path commits with the `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` trailer; never push; calibration scratch untracked).

## 3D — Gale (Shannon switching game)

**Identity:** id `gale`, name `Gale`, icon 🌉 + monochrome glyph, category Connect & line, playerLabels `['Red', 'Gold']` (arcade convention). Blurb: bridge-building duel — connect your two sides before your rival cuts you off. NAME NOTE: "Gale" (academic, safe); do NOT use "Bridg-It" (former trademark) anywhere user-facing.

**Rules (verify against the Wikipedia "Shannon switching game" page + hexwiki Bridg-It page; cite both in engine comments):** two interleaved dot grids — Red's (n+1)×n and Gold's n×(n+1), offset. Players alternately draw an edge between two orthogonally adjacent OWN dots; edges may not cross. Red connects left↔right, Gold top↔bottom. Exactly one player always succeeds (no draws).

**Engine shape (`treant-wasm/src/gale.rs`):** the elegant encoding — every potential Red edge crosses exactly one potential Gold edge, so the game is "claim a crossing point": an n×(n+1)+(n+1)×n… derive the crossing-point lattice (it's 2 interleaved sets; total crossings = 2·n·(n+... hand-derive and assert in a test). Move = crossing-point index (Display = bare index; round-trip test). Winner via union-find to virtual side nodes (reuse the pattern from hex.rs, cite it). Solver ON. winning_cells() → the connecting edge set (for the win glow; the Board maps them to drawn edges). `new(n: u32, pie: u32)` — n = dots-per-side clamps 3–7 (default 5, classic Bridg-It); pie rule mirrors hex.rs's swap implementation (swap = the mirrored crossing claim, recolored; verify mirror geometry makes sense for the asymmetric grids — if not cleanly definable, ship WITHOUT pie and say so; do not force it).

**Tile:** presets Classic 5 ⭐ / Small 3 ⚡ / Big 7 🤯 (+ Tournament ♻️ with pie if pie ships); knob size 3–7 (+ pie 0/1 if shipped; default 0 per the family-default precedent). Board: SVG — two dot lattices in player colours, claimed edges drawn thick, tap a crossing point to claim (hit target ≥44px at default size; at n=7 verify tap targets on 390px width — if too small, cap the knob at 6 and note it). Setup preview must render the empty lattice.

**Tests:** crossing-lattice count assert; edge-cross exclusivity (claiming a point blocks the perpendicular edge); union-find win both players; no-draw property (full board ⇒ exactly one winner — fuzz 200 random fills); round-trip; ai_plays; solver proves a 3×3 endgame. Calibrate via the example directly (`gale 20`); paste or reuse-with-comment per precedent.

## 3E — Slimetrail

**Identity:** id `slimetrail`, name `Slimetrail`, icon 🐌 + monochrome glyph, category Family classics (kid-first game), playerLabels `['Red', 'Gold']`.

**Rules (verify against the Combinatorial Game Theory blog post naming the game — http://combinatorialgametheory.blogspot.com/2017/07/game-description-slimetrail.html — and any Ludii/other source found; cite in comments):** ONE shared token on a grid; players alternate moving it one step (orthogonal+diagonal — verify the sourced adjacency; if sources use orthogonal-only or hex, follow the source and note it); the vacated cell becomes permanent slime (never re-enterable); Red wins if the token ever reaches RED's goal corner, Gold likewise; if the mover has no legal move, ADJUDICATE FROM THE SOURCE (mover loses? draw?) — encode what the source says and test it.

**Engine shape (`treant-wasm/src/slimetrail.rs`):** near-trivial — token position + visited bitmask; `new(size: u32, /* goals fixed: opposite corners */)` size clamps 5–9, token starts centre (verify start position from source; centre is the common form). Move = destination cell index (Display bare index; round-trip test). Solver ON (tiny state space); no chance. winning_cells() → the goal cell on a win (glow).

**Tile:** presets Classic 7 ⭐ / Quick 5 ⚡ / Big 9 🤯; knob size 5–9. Board: reuse the grid rendering conventions (tttGrid-family) — token rendered large (🟢-style disc), slime cells visibly filled, goal corners tinted per player colour with a small flag/star marker, legal targets highlighted (the standing legal-move affordance). The "whose goal is which" must be self-evident (colour-tinted corners + caption).

**Tests:** slime permanence; goal win for both seats; the sourced no-move rule; full-game termination fuzz; round-trip; ai_plays. Calibrate per precedent.

## Order & checkpoint

3D then 3E (sequential implementers, one review each, fix loops as needed). After both approved: critic checkpoint (play both vs Hard; judge kid-legibility of Slimetrail's goals and Gale's crossing-claim interaction), then push the pair.
