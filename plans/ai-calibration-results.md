# AI calibration results

Run: cargo run --release --example calibrate -p treant-wasm (seed 0xC0FFEE, 40 games/pair)

Ladder rungs (`{playouts, top_k, temp}`):
- `p8 t3 k6` = {8, 6, 3.0}
- `p30 t2 k5` = {30, 5, 2.0}
- `p100 t1 k4` = {100, 4, 1.0}
- `p300 t.6 k3` = {300, 3, 0.6}
- `p1000 t.3` = {1000, 2, 0.3}
- `p4000 t0` = {4000, 1, 0.0}

| Game | Easy {playouts,topK,temp} | Medium | Hard | notes |
|---|---|---|---|---|
| tic-tac-toe | {8,6,3.0} | {30,5,2.0} | {100,4,1.0} | Forced draw above p100: only the p30->p100 step is real; everything p300+ plateaus at ~50% (draws). temp/top_k is the early lever, playouts irrelevant past 100. |
| connect-four | {8,6,3.0} | {30,5,2.0} | {300,3,0.6} | Steep low end (p8->p30 79%, p30->p100 90%); strength saturates above p300, so Hard caps at the top *meaningful* rung. |
| gomoku | {8,6,3.0} | {30,5,2.0} | {100,4,1.0} | Big real steps p8->p30 (95%) and p30->p100 (95%), then a hard plateau; p100 is the strongest meaningful rung (field 68%, more playouts add nothing). |
| reversi | {8,6,3.0} | {100,4,1.0} | {4000,1,0.0} | Strength climbs monotonically the whole ladder; every step real except the top. Picked a wide, clearly-separated spread (p100 beats p8 95%, p4000 beats p100 100%). |
| hex | {8,6,3.0} | {100,4,1.0} | {4000,1,0.0} | p8->p30 is NOT a real step (62%), so Medium jumps to p100 (100% over p8); playouts dominate, strength keeps rising to p4000. |
| frontline | {8,6,3.0} | {100,4,1.0} | {4000,1,0.0} | Strongly playout-driven: every ladder step is a real step; p100 beats p8 100%, p4000 beats p100 98%. Hard = strongest rung. |

Low-playout solver games (tic-tac-toe, connect-four, gomoku) showed the expected `(no real change)` flags for the high-playout rungs: once the rollout count is past the point where the game's forced outcome is found, extra playouts/lower temperature stop moving the win rate, so the matrix flattens to ~50%. The high-branching games (reversi, hex, frontline) do not solve out at these playout counts and keep gaining strength up the ladder.

## Raw output
```
==== Tic-Tac-Toe 3x3 ====
    p8 t3 k6     ·  35%  18%  24%  21%  25%   field  24%
   p30 t2 k5   65%    ·  26%  29%  34%  35%   field  38%
  p100 t1 k4   82%  74%    ·  44%  46%  48%   field  59%
 p300 t.6 k3   76%  71%  56%    ·  49%  50%   field  60%
   p1000 t.3   79%  66%  54%  51%    ·  50%   field  60%
    p4000 t0   75%  65%  52%  50%  50%    ·   field  58%
  ladder steps (upper beats lower):
       p30 t2 k5 vs p8 t3 k6      65%
      p100 t1 k4 vs p30 t2 k5     74%  <-- real step
     p300 t.6 k3 vs p100 t1 k4    56%  (no real change)
       p1000 t.3 vs p300 t.6 k3   51%  (no real change)
        p4000 t0 vs p1000 t.3     50%  (no real change)

==== Connect Four 7x6 ====
    p8 t3 k6     ·  21%   2%   0%   2%   4%   field   6%
   p30 t2 k5   79%    ·  10%  12%  18%  22%   field  28%
  p100 t1 k4   98%  90%    ·  40%  35%  40%   field  60%
 p300 t.6 k3  100%  88%  60%    ·  48%  46%   field  68%
   p1000 t.3   98%  82%  65%  52%    ·  54%   field  70%
    p4000 t0   96%  78%  60%  54%  46%    ·   field  67%
  ladder steps (upper beats lower):
       p30 t2 k5 vs p8 t3 k6      79%  <-- real step
      p100 t1 k4 vs p30 t2 k5     90%  <-- real step
     p300 t.6 k3 vs p100 t1 k4    60%
       p1000 t.3 vs p300 t.6 k3   52%  (no real change)
        p4000 t0 vs p1000 t.3     46%  (no real change)

==== Gomoku 9x9 k5 ====
    p8 t3 k6     ·   5%   0%   5%  14%  22%   field   9%
   p30 t2 k5   95%    ·   5%  30%  39%  36%   field  41%
  p100 t1 k4  100%  95%    ·  48%  50%  49%   field  68%
 p300 t.6 k3   95%  70%  52%    ·  50%  49%   field  63%
   p1000 t.3   86%  61%  50%  50%    ·  50%   field  60%
    p4000 t0   78%  64%  51%  51%  50%    ·   field  59%
  ladder steps (upper beats lower):
       p30 t2 k5 vs p8 t3 k6      95%  <-- real step
      p100 t1 k4 vs p30 t2 k5     95%  <-- real step
     p300 t.6 k3 vs p100 t1 k4    52%  (no real change)
       p1000 t.3 vs p300 t.6 k3   50%  (no real change)
        p4000 t0 vs p1000 t.3     50%  (no real change)

==== Reversi 6x6 ====
    p8 t3 k6     ·  22%   5%   0%   0%   0%   field   6%
   p30 t2 k5   78%    ·  10%   0%   2%   0%   field  18%
  p100 t1 k4   95%  90%    ·  26%   9%   0%   field  44%
 p300 t.6 k3  100% 100%  74%    ·  29%  11%   field  63%
   p1000 t.3  100%  98%  91%  71%    ·  44%   field  81%
    p4000 t0  100% 100% 100%  89%  56%    ·   field  89%
  ladder steps (upper beats lower):
       p30 t2 k5 vs p8 t3 k6      78%  <-- real step
      p100 t1 k4 vs p30 t2 k5     90%  <-- real step
     p300 t.6 k3 vs p100 t1 k4    74%  <-- real step
       p1000 t.3 vs p300 t.6 k3   71%  <-- real step
        p4000 t0 vs p1000 t.3     56%  (no real change)

==== Hex 7x7 ====
    p8 t3 k6     ·  38%   0%   0%   0%   0%   field   8%
   p30 t2 k5   62%    ·   0%   0%   0%   0%   field  12%
  p100 t1 k4  100% 100%    ·  25%   0%   0%   field  45%
 p300 t.6 k3  100% 100%  75%    ·  22%   8%   field  61%
   p1000 t.3  100% 100% 100%  78%    ·  18%   field  79%
    p4000 t0  100% 100% 100%  92%  82%    ·   field  95%
  ladder steps (upper beats lower):
       p30 t2 k5 vs p8 t3 k6      62%
      p100 t1 k4 vs p30 t2 k5    100%  <-- real step
     p300 t.6 k3 vs p100 t1 k4    75%  <-- real step
       p1000 t.3 vs p300 t.6 k3   78%  <-- real step
        p4000 t0 vs p1000 t.3     82%  <-- real step

==== Frontline 6x6 ====
    p8 t3 k6     ·   8%   0%   0%   0%   0%   field   2%
   p30 t2 k5   92%    ·   5%   2%   2%   0%   field  20%
  p100 t1 k4  100%  95%    ·  12%   5%   2%   field  43%
 p300 t.6 k3  100%  98%  88%    ·  20%  20%   field  65%
   p1000 t.3  100%  98%  95%  80%    ·  28%   field  80%
    p4000 t0  100% 100%  98%  80%  72%    ·   field  90%
  ladder steps (upper beats lower):
       p30 t2 k5 vs p8 t3 k6      92%  <-- real step
      p100 t1 k4 vs p30 t2 k5     95%  <-- real step
     p300 t.6 k3 vs p100 t1 k4    88%  <-- real step
       p1000 t.3 vs p300 t.6 k3   80%  <-- real step
        p4000 t0 vs p1000 t.3     72%  <-- real step
```
