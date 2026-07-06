# Phase 3B — Launcher Variant Families Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (one implementer task + review). Master-plan Global Constraints apply (plans/2026-07-05-knobs-variants-and-new-games.md).

**Goal:** Group variant tiles under their parent on the launcher shelf — parent tile + a "+N" chip expanding the family — shrinking the visible tile count (~49 → ~38) and making opaque variant names discoverable as "more ways to play a game you recognize."

**Design (locked with Peter's critic — do not relitigate):**
- Data model: optional `variantOf?: string` (parent game id) on `GameDefinition` (docs/src/components/arcade/gameTypes.ts).
- Launcher grouping: within the existing CATEGORIES shelves, a child tile is HIDDEN from the grid and instead reachable via its parent's family expansion. A parent with children renders its normal tile PLUS a "+N" chip (N = child count).
- Interaction: tile body tap = play the parent (one tap preserved, unchanged). Chip tap = expand. Expansion renderer is a single `renderFamily(parent, kids)` seam dispatched ON CHILD COUNT: families ≤3 render an INLINE ROW of the child tiles directly below the parent's row; families ≥4 render a BOTTOM SHEET (none exist yet — implement the seam + inline row now; the sheet may be a stub that falls back to inline with a `// TODO(draughts)` note, clearly marked).
- Chip: ≥44px touch target (arcade standing rule), accessible name like "Show N more ways to play {name}", aria-expanded; Escape/second tap collapses; only one family expanded at a time is acceptable (simplest state).
- Search behavior UNCHANGED: children remain individually searchable and playable (search results render flat, no grouping).
- Hero rotation UNCHANGED (children still rotate). Recently-played strip UNCHANGED (flat).

**Families to declare now (child → variantOf):**
pop-out → connect-four · cylinder-four → connect-four · two-dice-pig → pig · big-pig → pig · anti-reversi → reversi · lasker-morris → nine-morris · joust → trails · cram → domineering · snort → col. (Gomoku stays standalone — its name recognition exceeds Tic-Tac-Toe association.)

**Guard updates:** `docs/scripts/check-categories.mjs` currently requires every id in a CATEGORIES array. Children may now legitimately be absent from CATEGORIES (they render via family expansion). Update the check: every game id must be EITHER in a category OR declare `variantOf` pointing at a categorized parent (validate the parent id exists and is categorized; reject variantOf chains — a child's parent must not itself have variantOf). Update the success message accordingly.

**i18n:** new UI strings (the chip's aria label, any "More ways to play" caption) via the standard t() flow, --write + six-locale top-up + --check 0/0, alphabetical insert.

**Verification (non-optional):** `npm run build` green (including the updated guard, plus a deliberate-failure check: a scratch child with variantOf to a non-existent parent must fail the build; remove after proving); Playwright on static :3939 (kill animations): (1) Family classics shelf shows Connect Four with a "+2" chip and does NOT show Pop Out/Cylinder Four as flat tiles; (2) chip tap expands an inline row with both children; tapping Pop Out opens its setup; (3) parent tile body tap still goes straight to Connect Four setup; (4) search "pop" still surfaces Pop Out flat; (5) chip computed size ≥44×44; (6) collapsed by default on load; (7) phone width 390 — expansion doesn't overflow horizontally. Screenshot the expanded state for the report.

**Review gate scrutiny:** guard correctness (chain rejection, dangling parent), no regression for standalone tiles, keyboard/a11y on the chip, CSS in both light-base and neon skin, i18n keys.

**Commit:** explicit paths, trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`, do NOT push (ships with the Phase-3 checkpoint batch).
