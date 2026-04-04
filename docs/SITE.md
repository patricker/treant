# Site Design Brief

## One sentence

A documentation site that feels like a textbook you can run.

## The problem we're solving

Technical documentation lives in one of two failure modes. Academic resources explain beautifully but give you nothing to touch. Library docs give you API signatures but no understanding. The gap between "I read about it" and "I can use it" is where most people give up.

## The goal

Every concept on this site is backed by running code. Every code sample is pulled from a tested source file. Every non-trivial idea has an interactive demo you can manipulate in your browser. The site is a single artifact where theory, code, and interaction are the same thing — not three separate efforts duct-taped together.

## Design principles

### 1. Code is the source of truth

No prose-only code blocks. Every snippet is extracted from a compiled, tested Rust file via `remark-code-region`. If the code changes, the docs update. If the code breaks, the build breaks. Stale examples are structurally impossible.

### 2. Interaction before explanation

When a concept can be demonstrated, demonstrate it first. Let the reader build intuition by manipulating a live system, then explain what they just saw. Sliders and buttons teach faster than paragraphs.

### 3. Progressive disclosure

The site serves two audiences — people learning MCTS for the first time and experienced practitioners evaluating this library. The structure goes shallow-to-deep: tutorials assume nothing, concepts assume curiosity, reference assumes competence. A reader should never hit a wall of jargon without a link to the page that explains it.

### 4. Minimal, dense, precise

No filler. No "in this section we will discuss." Every sentence either teaches something or shows something. White space is generous. Text is tight. Code blocks are complete — never a `// ...` that hides the part you actually needed.

## Visual identity

### Tone: Workshop, not lecture hall

The site should feel like a well-lit workbench — clean, purposeful, slightly technical. Not playful (no mascots, no emoji headers). Not corporate (no gradients, no stock illustrations). Think: a good O'Reilly book if it could run in your browser.

### Color

- Light mode: white background, near-black text, muted blue accents
- Dark mode: true dark (not grey), same accent palette
- Demos use color functionally: green/red/yellow for proven values, blue intensity for visit density, orange for highlighted paths. Color carries meaning — never decoration.

### Typography

- Monospace for code (Prism-highlighted Rust and TOML)
- Clean sans-serif for prose (system font stack or Inter)
- Generous line height. Comfortable reading width. No walls of text.

### Layout

- Sidebar navigation, always visible on desktop
- Content column: 720px max, centered
- Demos are full-width within the content column — they need room to breathe
- Mobile: sidebar collapses, demos stack vertically, sliders remain usable

## Interactive demos

### Philosophy

Demos are not decorations. Each one exists because the concept it teaches is genuinely harder to understand without interaction. A slider for the exploration constant C teaches more about exploration-exploitation in 10 seconds than a page of prose.

### Aesthetic

- SVG tree visualizations with clean lines, circle nodes, readable labels
- Muted palette until interaction — then color highlights the active element
- Smooth CSS transitions (not flashy animations)
- Controls are simple: sliders, buttons, dropdowns. No custom widgets.
- Stats panels are plain text with monospace numbers — no charts for the sake of charts

### Technical constraints we embrace

- Single-threaded WASM. We don't pretend to demo parallelism — we explain it with diagrams and prose.
- 137KB WASM binary. Lazy-loaded. No spinner — the demo simply appears when ready.
- Small games only. The demos use tiny game trees (Nim, counting, dice) because they're legible at demo scale. We don't try to simulate Go.

## What this site is not

- Not a paper. We cite Browne et al. and Kocsis & Szepesvari, but we don't reproduce proofs.
- Not a blog. No dates, no "updates," no opinions section. Content is evergreen or it doesn't exist.
- Not a marketing page. No testimonials, no "trusted by" logos, no pricing. The code speaks.
- Not a playground-first app. The playground supports the docs, not the other way around. Reading path is primary. Demos are inline where they teach; the standalone playground is a bonus.

## Success criteria

A reader who completes the tutorial sequence can implement any game with any feature of the library. A reader who visits a single concept page leaves understanding one thing deeply. A reader who opens the playground learns something without reading a word.
