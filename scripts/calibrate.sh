#!/usr/bin/env bash
#
# Re-run the arcade AI difficulty calibration. Full method in CALIBRATION.md.
#
# Usage:
#   scripts/calibrate.sh                 # calibrate ALL games (n=20/pair, ~minutes)
#   scripts/calibrate.sh connect-four    # calibrate one game
#   scripts/calibrate.sh connect-four 40 # one game, 40 games/pair (less noise)
#
# It self-plays a ladder of weak_move configs per game, prints a win matrix, and
# emits a ready-to-paste `difficulty: { easy, medium, hard }` block. Paste each
# block into the matching docs/src/components/arcade/games/<id>.tsx, then rebuild
# the WASM (cd treant-wasm && wasm-pack build --target web).
set -euo pipefail
cd "$(dirname "$0")/.."

GAME="${1:-}"
N="${2:-20}"
OUT="plans/ai-calibration-results.txt"

if [[ -n "$GAME" ]]; then
  echo "Calibrating '$GAME' (n=$N games/pair)…"
  cargo run --release --example calibrate -p treant-wasm -- "$GAME" "$N" | tee "$OUT"
else
  echo "Calibrating ALL games (n=20 games/pair) — this takes several minutes…"
  cargo run --release --example calibrate -p treant-wasm | tee "$OUT"
fi

echo
echo "Full output saved to $OUT."
echo "Paste each emitted 'difficulty: { … }' block into the matching"
echo "docs/src/components/arcade/games/<id>.tsx GameDefinition, then rebuild the WASM."
