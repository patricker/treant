# Phase 3C — World Threes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (one implementer task + review). Master-plan Global Constraints apply.

**Goal:** ONE new game, `world-threes`, wrapping the global family of tiny place-and-slide three-in-a-row games — the board itself is the customization (each preset is a country's traditional game).

**Architecture:** New engine `treant-wasm/src/worldthrees.rs` — per-board static config (points, adjacency edges, win-lines, piece count, start layout, win mode), one `GameState` over it. Phases: placement (unless the board starts pre-placed) then slide-along-edges. New SVG graph-board renderer in the tile (points+edges from per-board coordinates; study `ninemorris.tsx`/`mutorere.tsx` for the house SVG pattern). Solver ON (all boards are tiny; treant should play perfectly).

## Boards (RULES ARE LOAD-BEARING — do not invent; verify each against its Wikipedia page before encoding, cite the URL in a code comment per board)

| # | Preset label | Points | Pieces | Start | Win |
|---|---|---|---|---|---|
| 0 | Achi (Ghana) ⭐ | 3×3, orthogonal + both diagonals through centre | 4 | place then slide | 3-in-line |
| 1 | Tapatan (Philippines) | 3×3 + diagonals | 3 | place then slide | 3-in-line |
| 2 | Shisima (Kenya) | octagon rim (8) + centre | 3 | place then slide (rim↔neighbours, rim↔centre) | 3-in-line THROUGH the centre |
| 3 | Tant Fant (India) | 3×3 + diagonals | 3 | PRE-PLACED on home rows, slide only | 3-in-line NOT on your own home row |
| 4 | Nine Holes (England) | 3×3, NO diagonals | 3 | place then slide (any empty? verify — historical rule is move to ANY empty hole) | 3-in-line orthogonal only |
| 5 | Tsoro Yematatu (Zimbabwe) | 7-point triangle | 3 | place then slide (may jump? verify — jumping over one piece is attested) | 3-in-line |
| 6 | Picaria (Zuni) | 3×3 + 4 mid-edge points = 13 | 3 | place then slide | 3-in-line (verify which lines count — mid-point lines included) |
| 7 | Pong Hau K'i (China) 🤯 | 5 points | 2 | PRE-PLACED, slide only | opponent BLOCKED (no legal move) |

If a board's sources conflict irreconcilably, pick the most-cited ruleset, note it in the rules text ("rules follow the common form"), and say so in the report. If one board proves unimplementable in budget, ship without it (drop its preset) and report — 7 good boards beat 8 with a wrong one.

## Binding shape

- `WorldThreesWasm::new(board: u32, /* future-proof */)` — board 0–7 clamps. Move encoding: placement `"<point>"`, slide `"<from>-<to>"` (dash convention; Display/parse single-sourced + round-trip test — program lesson).
- Draw guard: these games can cycle; add a no-progress ply cap → Draw (mirror ninemorris's `DRAW_PLY_CAP` pattern) EXCEPT Pong Hau K'i (blockade games end; still cap defensively).
- Per-board unit tests: adjacency edge-count assertion (hand-counted from the cited source), a scripted win in ≤8 moves where feasible, phase-transition test, and `ai_plays`. Tant Fant: test the own-home-row line does NOT win. Shisima: test a rim-only line does NOT win. Pong Hau K'i: blockade terminal test.
- Tile: id `world-threes`, name `World Threes`, icon `🌍` + REQUIRED monochrome glyph in icons.tsx (build guard enforces). NO knobs — the 8 presets ARE the selector (each sets `{ numPlayers: 2, board: n }`; preset-highlight comparator handles it). Category: Brain-teasers? NO — Family classics (it's the world's folk games; also balances shelves). rules.ts: one general entry + per-board specifics live in each preset's... rules.ts is per-game — write a compact general text naming the boards; the setup preview shows the selected board.
- Difficulty: register in calibrate.rs; run the calibrate example directly (`world-threes 20`, default board); paste if sane, else DEFAULT_DIFFICULTY with comment (these are solver-tiny; expect degenerate ladders — that's fine, solver-perfect Hard is the point).
- Board renderer must render from the SAME adjacency/coordinate data the engine's board uses (single source: export coordinates via a get_layout() string from wasm, or mirror a TS table with a unit-style build assertion — implementer's choice, justify it; divergence between rendered edges and legal moves is the failure mode that matters).

## Verification (non-optional)

cargo TDD (red first) per board · clippy 0 · wasm rebuild + docs install flow · audit run (all games ok incl. world-threes) · i18n --write/top-up ×6/--check 0/0 (8 preset labels with country names — check each locale's country-name conventions) · npm run build green (category+glyph guards) · Playwright static :3939: Shisima preset renders the octagon; play a full vs-easy game on Achi to terminal; Pong Hau K'i preset renders 5 points and blockade-ends; setup preview switches boards when presets are tapped. Screenshot the Shisima board for the report.

**Commit:** explicit paths, trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`, do NOT push.
