import type { JSX } from 'react';
import { gameById } from './games';

// Consistent flat icon set. All use `currentColor`, so they pick up the
// surrounding text color (white on the hero, dark on tiles/headers).
const GLYPHS: Record<string, JSX.Element> = {
  climb: (
    // a stepped mountain with a flag planted on the summit
    <g stroke="currentColor" strokeWidth="1.8" fill="none" strokeLinejoin="round" strokeLinecap="round">
      <path d="M3 20h4v-4h4v-4h4v-4h2" />
      <path d="M17 8V3l4 2-4 2" fill="currentColor" stroke="none" />
      <path d="M17 3v9" />
      <path d="M3 20l4-4 4-4 4-4" opacity="0.4" />
    </g>
  ),
  'bagh-chal': (
    // three tiger claw-slashes raked across the board
    <g stroke="currentColor" fill="none" strokeWidth="2.4" strokeLinecap="round">
      <path d="M6 4c1.5 4 2 9 1.5 15" />
      <path d="M12 3.5c1.6 4.5 2.1 10 1.5 16.5" />
      <path d="M18 4c1.5 4 2 9 1.5 15" />
    </g>
  ),
  'len-choa': (
    // the Len Choa board: a triangle split by two breadth lines and a central
    // axis, with the tiger (filled dot) waiting on the apex
    <g stroke="currentColor" fill="none" strokeWidth="1.6" strokeLinejoin="round" strokeLinecap="round">
      <path d="M12 3 L21 21 L3 21 Z" />
      <path d="M12 3 L12 21" />
      <path d="M9 9 L15 9" />
      <path d="M6 15 L18 15" />
      <circle cx="12" cy="3" r="1.7" fill="currentColor" stroke="none" />
    </g>
  ),
  'nine-morris': (
    // three nested squares joined by midpoint spokes, with a mill of three dots
    <g stroke="currentColor" strokeWidth="1.6" fill="none" strokeLinejoin="round">
      <rect x="3" y="3" width="18" height="18" rx="0.5" />
      <rect x="7" y="7" width="10" height="10" rx="0.5" />
      <rect x="10.5" y="10.5" width="3" height="3" rx="0.5" opacity="0.6" />
      <path d="M12 3v4M12 17v4M3 12h4M17 12h4" opacity="0.7" />
      <g fill="currentColor" stroke="none">
        <circle cx="3" cy="3" r="1.7" />
        <circle cx="12" cy="3" r="1.7" />
        <circle cx="21" cy="3" r="1.7" />
      </g>
    </g>
  ),
  'lasker-morris': (
    // The morris rings, plus a man still in hand — Lasker's place-or-move twist.
    <g stroke="currentColor" strokeWidth="1.6" fill="none" strokeLinejoin="round">
      <rect x="3" y="3" width="14" height="14" rx="0.5" />
      <rect x="6.5" y="6.5" width="7" height="7" rx="0.5" />
      <path d="M10 3v3.5M10 13.5V17M3 10h3.5M13.5 10H17" opacity="0.7" />
      <g fill="currentColor" stroke="none">
        <circle cx="3" cy="3" r="1.6" />
        <circle cx="10" cy="3" r="1.6" />
        <circle cx="17" cy="3" r="1.6" />
        <circle cx="20.5" cy="20.5" r="2" />
      </g>
    </g>
  ),
  'connect-four': (
    // four discs on a diagonal
    <g fill="currentColor">
      <circle cx="6" cy="18" r="2.6" />
      <circle cx="11" cy="13.5" r="2.6" />
      <circle cx="16" cy="9" r="2.6" opacity="0.55" />
      <circle cx="21" cy="4.5" r="2.6" opacity="0.3" />
    </g>
  ),
  'pop-out': (
    // a column of discs with the bottom one dropping out (a downward arrow)
    <g fill="currentColor">
      <circle cx="8" cy="5" r="2.4" opacity="0.5" />
      <circle cx="8" cy="10.5" r="2.4" opacity="0.75" />
      <circle cx="8" cy="16" r="2.4" />
      <g stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
        <path d="M17 5v11M13 12l4 4 4-4" />
      </g>
    </g>
  ),
  'cylinder-four': (
    // a cylinder/tube with a line of discs wrapping around it
    <g stroke="currentColor" strokeWidth="1.8" fill="none">
      <ellipse cx="12" cy="5" rx="8" ry="2.6" />
      <path d="M4 5v14M20 5v14" />
      <path d="M4 19a8 2.6 0 0 0 16 0" />
      <g fill="currentColor" stroke="none">
        <circle cx="7" cy="12" r="1.8" />
        <circle cx="12" cy="12.6" r="1.8" />
        <circle cx="17" cy="12" r="1.8" />
      </g>
      <path d="M4 12.3a8 2.6 0 0 0 16 0" opacity="0.35" />
    </g>
  ),
  'tic-tac-toe': (
    <g stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round">
      <path d="M9 3v18M15 3v18M3 9h18M3 15h18" opacity="0.45" />
      <path d="M4.5 4.5l3 3M7.5 4.5l-3 3" />
      <circle cx="18" cy="18" r="2.6" />
    </g>
  ),
  shift: (
    <g stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="8" cy="8" r="3" fill="currentColor" stroke="none" />
      <path d="M13 16h7M17 13l3 3-3 3" />
      <circle cx="9" cy="17" r="2.4" opacity="0.4" fill="currentColor" stroke="none" />
    </g>
  ),
  quadline: (
    // a line of four dots morphing into a 2×2 square
    <g fill="currentColor">
      <circle cx="4" cy="5" r="1.9" />
      <circle cx="9.3" cy="5" r="1.9" />
      <circle cx="14.6" cy="5" r="1.9" />
      <circle cx="20" cy="5" r="1.9" />
      <circle cx="9" cy="14" r="1.9" opacity="0.5" />
      <circle cx="15" cy="14" r="1.9" opacity="0.5" />
      <circle cx="9" cy="20" r="1.9" />
      <circle cx="15" cy="20" r="1.9" />
    </g>
  ),
  'order-chaos': (
    <g stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round">
      <path d="M3.5 4l5 5M8.5 4l-5 5" />
      <circle cx="17" cy="17" r="4" />
      <path d="M3.5 15l5 5M8.5 15l-5 5" opacity="0.4" />
      <circle cx="17" cy="6.5" r="2.4" opacity="0.4" />
    </g>
  ),
  'no-tac-toe': (
    <g stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round">
      <circle cx="12" cy="12" r="9" />
      <path d="M8 8l8 8M16 8l-8 8" />
      <path d="M5.6 5.6l12.8 12.8" strokeWidth="2.4" />
    </g>
  ),
  'trap-three': (
    <g fill="currentColor">
      <circle cx="5" cy="17" r="2.4" />
      <circle cx="12" cy="17" r="2.4" />
      <circle cx="19" cy="17" r="2.4" />
      <path d="M12 3l3.4 6H8.6z" fill="none" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
      <rect x="11.2" y="5" width="1.6" height="2.2" rx="0.8" />
    </g>
  ),
  'square-up': (
    <g stroke="currentColor" strokeWidth="2" fill="none">
      <rect x="5" y="5" width="14" height="14" rx="1" opacity="0.4" />
      <g fill="currentColor" stroke="none">
        <circle cx="5" cy="5" r="2.4" />
        <circle cx="19" cy="5" r="2.4" />
        <circle cx="5" cy="19" r="2.4" />
        <circle cx="19" cy="19" r="2.4" />
      </g>
    </g>
  ),
  'connect-six': (
    <g fill="currentColor">
      {[3, 7, 11, 15, 19, 23].map((x, i) => (
        <circle key={i} cx={x - 1} cy="12" r="1.9" opacity={i >= 5 ? 0.4 : 1} />
      ))}
    </g>
  ),
  nim: (
    <g stroke="currentColor" strokeWidth="2.4" strokeLinecap="round">
      <path d="M5 19V8M10 19V5M15 19V9M20 19V6" />
    </g>
  ),
  mancala: (
    <g stroke="currentColor" strokeWidth="2" fill="none">
      <rect x="3" y="8" width="12" height="8" rx="4" />
      <ellipse cx="19.5" cy="12" rx="2.2" ry="5" />
      <g fill="currentColor" stroke="none">
        <circle cx="6.5" cy="12" r="1.3" />
        <circle cx="9.5" cy="10.5" r="1.3" />
        <circle cx="11.5" cy="13.5" r="1.3" />
      </g>
    </g>
  ),
  oware: (
    // a row of three houses, seeds inside, evoking counter-clockwise sowing
    <g stroke="currentColor" strokeWidth="1.8" fill="none">
      <rect x="2.5" y="8" width="19" height="8" rx="4" />
      <path d="M8.5 8v8M15.5 8v8" opacity="0.5" />
      <g fill="currentColor" stroke="none">
        <circle cx="5.5" cy="12" r="1.2" />
        <circle cx="11" cy="10.7" r="1.2" />
        <circle cx="12" cy="13.4" r="1.2" />
        <circle cx="17.2" cy="11" r="1.2" />
        <circle cx="18.6" cy="13.6" r="1.2" />
      </g>
    </g>
  ),
  gomoku: (
    <g fill="currentColor">
      {[0, 1, 2, 3, 4].map((i) => (
        <circle key={i} cx={4 + i * 4} cy={20 - i * 4} r="1.9" />
      ))}
      <circle cx="16" cy="16" r="1.9" opacity="0.4" />
      <circle cx="8" cy="8" r="1.9" opacity="0.4" />
    </g>
  ),
  'pinch-five': (
    <g fill="currentColor">
      {/* a pair being pinched between two brackets */}
      <circle cx="10.5" cy="8" r="2.1" opacity="0.4" />
      <circle cx="13.5" cy="8" r="2.1" opacity="0.4" />
      <g stroke="currentColor" strokeWidth="1.8" fill="none" strokeLinecap="round" strokeLinejoin="round">
        <path d="M5 5l2.5 3L5 11" />
        <path d="M19 5l-2.5 3L19 11" />
      </g>
      {/* five-in-a-row goal */}
      {[4, 8, 12, 16, 20].map((x, i) => (
        <circle key={i} cx={x} cy="18" r="1.7" />
      ))}
    </g>
  ),
  'dots-and-boxes': (
    <g fill="currentColor">
      <rect x="6" y="6" width="12" height="2" rx="1" />
      <rect x="6" y="16" width="12" height="2" rx="1" opacity="0.4" />
      <rect x="5" y="6" width="2" height="12" rx="1" />
      <rect x="17" y="6" width="2" height="12" rx="1" opacity="0.4" />
      {[5, 17].flatMap((x) => [6, 17].map((y) => <circle key={`${x}-${y}`} cx={x + 1} cy={y + 1} r="2" />))}
    </g>
  ),
  wythoff: (
    <g stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
      <path d="M19 19L6 6" />
      <path d="M6 11V6h5" />
      <circle cx="19" cy="19" r="2" fill="currentColor" stroke="none" />
    </g>
  ),
  chomp: (
    <g fill="currentColor">
      <path d="M12 3a9 9 0 1 0 8.5 6.1 3 3 0 0 1-4.2-4.2A9 9 0 0 0 12 3z" />
      <circle cx="9" cy="11" r="1.1" fill="var(--arc-bg, #fff)" />
      <circle cx="14" cy="14" r="1.1" fill="var(--arc-bg, #fff)" />
      <circle cx="11" cy="15.5" r="1" fill="var(--arc-bg, #fff)" />
    </g>
  ),
  'first-capture': (
    <g fill="currentColor">
      <circle cx="12" cy="12" r="2.6" opacity="0.4" />
      <circle cx="12" cy="4.5" r="2.2" />
      <circle cx="12" cy="19.5" r="2.2" />
      <circle cx="4.5" cy="12" r="2.2" />
      <circle cx="19.5" cy="12" r="2.2" />
    </g>
  ),
  trails: (
    <g stroke="currentColor" strokeWidth="2.2" fill="none" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 20v-6h6V8h6V4" />
      <circle cx="4" cy="20" r="1.8" fill="currentColor" stroke="none" />
      <circle cx="16" cy="4" r="1.8" fill="currentColor" stroke="none" />
    </g>
  ),
  joust: (
    // Trails with a knight's move — the L-leap (two-then-one) plus a jump arc.
    <g fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M6 20V10h6" />
      <path d="M6 20q3-11 12-13" strokeDasharray="1.6 2" opacity="0.55" />
      <g fill="currentColor" stroke="none">
        <circle cx="6" cy="20" r="1.9" />
        <circle cx="18" cy="7" r="1.9" />
      </g>
    </g>
  ),
  strand: (
    // A lone palm on a small island above a waterline — you've been marooned.
    <g stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
      <path d="M9 14v-3" />
      <path d="M9 11c-2.4-1.4-4.2-1-5 0M9 11c2.4-1.4 4.2-1 5 0M9 11c0-2 1.6-3.4 3.6-3.4M9 11c0-2-1.6-3.4-3.6-3.4" />
      <path d="M4 16.5c1.6 1.2 3.4-.8 5 .4 1.6-1.2 3.4.8 5-.4" />
      <path d="M3 20c1.8 1.3 3.8-.9 5.5.4 1.7-1.3 3.7.9 5.5-.4" opacity="0.55" />
    </g>
  ),
  slimetrail: (
    // A snail: a spiral shell over a body, trailing a dotted slime trail behind.
    <g fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 18h4" strokeDasharray="0.5 2.4" opacity="0.6" />
      <path d="M7 18h3c1.3 0 2-0.9 2-2" />
      <circle cx="14" cy="12" r="4.4" />
      <path d="M14 12.2a2.1 2.1 0 1 1 2-2.2" />
      <path d="M17.6 8.4l1.8-2M19.6 8.7l2-1.3" stroke="currentColor" />
    </g>
  ),
  reversi: (
    <g>
      <circle cx="12" cy="12" r="9.5" fill="none" stroke="currentColor" strokeWidth="2" />
      <path d="M12 2.5a9.5 9.5 0 0 1 0 19z" fill="currentColor" />
    </g>
  ),
  'anti-reversi': (
    // Reversi's disc with the fill flipped to the other half — the goal is inverted.
    <g>
      <circle cx="12" cy="12" r="9.5" fill="none" stroke="currentColor" strokeWidth="2" />
      <path d="M12 2.5a9.5 9.5 0 0 0 0 19z" fill="currentColor" />
    </g>
  ),
  clobber: (
    <g fill="currentColor">
      <circle cx="8" cy="12" r="4" opacity="0.4" />
      <circle cx="15" cy="12" r="5" />
      <g stroke="var(--arc-bg, #fff)" strokeWidth="1.6" strokeLinecap="round">
        <path d="M13 10l4 4M17 10l-4 4" />
      </g>
    </g>
  ),
  hex: (
    <g stroke="currentColor" strokeWidth="2" fill="none" strokeLinejoin="round">
      <path d="M12 3l7 4.5v9L12 21l-7-4.5v-9z" />
      <circle cx="12" cy="12" r="2.4" fill="currentColor" stroke="none" />
    </g>
  ),
  y: (
    // a triangle of dots — connect the three corners
    <g fill="currentColor">
      <path d="M12 3.5L20 20H4z" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" opacity="0.5" />
      <circle cx="12" cy="4.5" r="2" />
      <circle cx="5.5" cy="19" r="2" />
      <circle cx="18.5" cy="19" r="2" />
      <circle cx="8.6" cy="12" r="1.5" opacity="0.7" />
      <circle cx="15.4" cy="12" r="1.5" opacity="0.7" />
      <circle cx="12" cy="19" r="1.5" opacity="0.7" />
    </g>
  ),
  gale: (
    // an arch bridge with piers, deck, and suspension cables
    <g stroke="currentColor" strokeWidth="1.8" fill="none" strokeLinecap="round" strokeLinejoin="round">
      <path d="M2 15c4-8 16-8 20 0" />
      <path d="M3 15v4M21 15v4" />
      <path d="M2 19h20" />
      <path d="M7.5 12.2v6.8M12 10.6v8.4M16.5 12.2v6.8" opacity="0.6" />
    </g>
  ),
  frontline: (
    // two pawns advancing past a center line
    <g stroke="currentColor" strokeWidth="2" fill="none" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 12h18" opacity="0.4" />
      <path d="M8 16l-2 3h4z" fill="currentColor" />
      <circle cx="6" cy="13.5" r="1.8" fill="currentColor" stroke="none" />
      <path d="M16 8l-2-3h4z" fill="currentColor" transform="rotate(180 16 6.5)" />
      <circle cx="18" cy="10.5" r="1.8" fill="currentColor" stroke="none" />
      <path d="M6 19v-1M18 5v1" />
    </g>
  ),
  'pawn-duel': (
    // a single chess-pawn silhouette — pawns, and only pawns
    <g fill="currentColor">
      <circle cx="12" cy="6" r="3" />
      <path d="M9.6 9.5h4.8l-1 3.2h-2.8z" />
      <path d="M7.5 20c0-3.6 2.2-4.7 3.1-6.6h2.8c0.9 1.9 3.1 3 3.1 6.6z" />
      <rect x="6" y="19" width="12" height="2.6" rx="1.3" />
    </g>
  ),
  pig: (
    <g>
      <rect x="3.5" y="3.5" width="17" height="17" rx="4" fill="none" stroke="currentColor" strokeWidth="2" />
      <g fill="currentColor">
        <circle cx="8" cy="8" r="1.6" />
        <circle cx="16" cy="8" r="1.6" />
        <circle cx="12" cy="12" r="1.6" />
        <circle cx="8" cy="16" r="1.6" />
        <circle cx="16" cy="16" r="1.6" />
      </g>
    </g>
  ),
  'two-dice-pig': (
    // two overlapping dice
    <g>
      <rect x="2.5" y="7" width="12" height="12" rx="3" fill="none" stroke="currentColor" strokeWidth="1.8" />
      <g fill="currentColor">
        <circle cx="6" cy="10.5" r="1.2" />
        <circle cx="11" cy="15.5" r="1.2" />
      </g>
      <rect x="11" y="3" width="10.5" height="10.5" rx="2.6" fill="var(--arc-card, #1a1a1a)" stroke="currentColor" strokeWidth="1.8" />
      <g fill="currentColor">
        <circle cx="13.8" cy="5.8" r="1" />
        <circle cx="18.7" cy="10.7" r="1" />
        <circle cx="16.25" cy="8.25" r="1" />
      </g>
    </g>
  ),
  'big-pig': (
    // a pig snout
    <g stroke="currentColor" strokeWidth="1.8" fill="none">
      <ellipse cx="12" cy="12" rx="9" ry="7.5" />
      <ellipse cx="12" cy="13" rx="4" ry="3.2" />
      <g fill="currentColor" stroke="none">
        <circle cx="10.5" cy="13" r="0.9" />
        <circle cx="13.5" cy="13" r="0.9" />
        <circle cx="8.5" cy="7.5" r="0.9" />
        <circle cx="15.5" cy="7.5" r="0.9" />
      </g>
    </g>
  ),
  euclid: (
    <g fill="currentColor">
      <circle cx="12" cy="6" r="1.7" />
      <rect x="5" y="11" width="14" height="2" rx="1" />
      <circle cx="9" cy="18" r="1.5" />
      <circle cx="15" cy="18" r="1.5" />
    </g>
  ),
  'subtract-square': (
    <g fill="currentColor">
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="11.5" y="9.5" width="5" height="5" rx="1" opacity="0.7" />
      <rect x="18" y="6" width="3" height="3" rx="0.6" opacity="0.45" />
    </g>
  ),
  treblecross: (
    <g>
      <rect x="2.5" y="9" width="19" height="6" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.6" opacity="0.5" />
      <g stroke="currentColor" strokeWidth="2" strokeLinecap="round">
        <path d="M6 10.5l3 3M9 10.5l-3 3" />
        <path d="M10.5 10.5l3 3M13.5 10.5l-3 3" />
        <path d="M15 10.5l3 3M18 10.5l-3 3" />
      </g>
    </g>
  ),
  'toads-frogs': (
    // A strip with two creatures facing INWARD: a toad hopping right meets a
    // frog hopping left — the whole game is "which way does it face".
    <g fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2.5" y="9" width="19" height="6" rx="1.5" opacity="0.45" />
      {/* toad (left) faces right */}
      <circle cx="6.5" cy="12" r="1.9" fill="currentColor" stroke="none" />
      <path d="M8.4 10.4l2 1.6-2 1.6" />
      {/* frog (right) faces left */}
      <circle cx="17.5" cy="12" r="1.9" fill="currentColor" stroke="none" />
      <path d="M15.6 10.4l-2 1.6 2 1.6" />
    </g>
  ),
  'fox-hounds': (
    <g fill="currentColor">
      {/* one fox chip pursued by a row of hound chips */}
      <path d="M12 4l2.2 3.2L12 9.4 9.8 7.2z" />
      <g opacity="0.5">
        <circle cx="5" cy="18" r="2.2" />
        <circle cx="11" cy="18" r="2.2" />
        <circle cx="17" cy="18" r="2.2" />
      </g>
      <path d="M12 10v5" stroke="currentColor" strokeWidth="1.6" strokeDasharray="1.5 1.8" strokeLinecap="round" />
    </g>
  ),
  sim: (
    <g stroke="currentColor" fill="none" strokeWidth="1.6" strokeLinejoin="round" strokeLinecap="round">
      <path d="M12 3l7.8 4.5v9L12 21l-7.8-4.5v-9z" opacity="0.4" />
      <path d="M12 3l7.8 13.5H4.2z" strokeWidth="2" />
      <g fill="currentColor" stroke="none">
        <circle cx="12" cy="3" r="1.6" />
        <circle cx="19.8" cy="16.5" r="1.6" />
        <circle cx="4.2" cy="16.5" r="1.6" />
      </g>
    </g>
  ),
  col: (
    <g fill="currentColor">
      {/* two same-colour patches kept apart by a gap */}
      <rect x="3" y="3" width="8" height="8" rx="1.5" />
      <rect x="13" y="13" width="8" height="8" rx="1.5" />
      <rect x="13" y="3" width="8" height="8" rx="1.5" opacity="0.3" />
      <rect x="3" y="13" width="8" height="8" rx="1.5" opacity="0.3" />
    </g>
  ),
  snort: (
    // Col's two-patch motif, but the constraint is the ENEMY: a barrier keeps
    // your patch (filled) off the opponent's (outline).
    <g>
      <rect x="3" y="3" width="8" height="8" rx="1.5" fill="currentColor" />
      <rect x="13" y="13" width="8" height="8" rx="1.5" fill="none" stroke="currentColor" strokeWidth="2" />
      <path d="M4.5 19.5L19.5 4.5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" opacity="0.7" />
    </g>
  ),
  amazons: (
    <g stroke="currentColor" fill="none" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      {/* a crown (amazon) with an arrow flying off it */}
      <path d="M4 16l1-7 2.5 3L9.5 7l2 4.5L13.5 8l1 8z" fill="currentColor" stroke="none" />
      <rect x="4" y="16.5" width="10.5" height="2.2" rx="0.6" fill="currentColor" stroke="none" />
      <path d="M14 6l6 6M20 12v-4M20 12h-4" />
    </g>
  ),
  nogo: (
    <g>
      <circle cx="12" cy="12" r="5" fill="currentColor" opacity="0.45" />
      <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="2" />
      <path d="M5.6 5.6l12.8 12.8" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" />
    </g>
  ),
  konane: (
    <g fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
      {/* jump arc from left stone over the middle to the empty landing */}
      <path d="M5 14 Q12 2 19 14" opacity="0.7" />
      <circle cx="5" cy="15" r="2.6" fill="currentColor" stroke="none" />
      <circle cx="12" cy="15" r="2.6" fill="currentColor" stroke="none" opacity="0.4" />
      <circle cx="19" cy="15" r="2.6" />
    </g>
  ),
  domineering: (
    <g fill="currentColor">
      {/* vertical domino */}
      <rect x="4" y="3.5" width="7" height="14" rx="1.5" />
      <circle cx="7.5" cy="7" r="1" fill="var(--arc-bg, #fff)" />
      <circle cx="7.5" cy="14" r="1" fill="var(--arc-bg, #fff)" />
      {/* horizontal domino */}
      <rect x="6.5" y="13.5" width="14" height="7" rx="1.5" opacity="0.5" />
      <circle cx="10" cy="17" r="1" fill="var(--arc-bg, #fff)" />
      <circle cx="17" cy="17" r="1" fill="var(--arc-bg, #fff)" />
    </g>
  ),
  cram: (
    // Domineering's domino, but placeable either way by both players — a vertical
    // and a horizontal tile crossing, pips and all.
    <g fill="currentColor">
      <rect x="9" y="3.5" width="6" height="17" rx="1.5" opacity="0.55" />
      <rect x="3.5" y="9" width="17" height="6" rx="1.5" opacity="0.55" />
      <circle cx="12" cy="6.2" r="0.9" fill="var(--arc-bg, #fff)" />
      <circle cx="12" cy="17.8" r="0.9" fill="var(--arc-bg, #fff)" />
      <circle cx="6.2" cy="12" r="0.9" fill="var(--arc-bg, #fff)" />
      <circle cx="17.8" cy="12" r="0.9" fill="var(--arc-bg, #fff)" />
    </g>
  ),
  'mu-torere': (
    <g stroke="currentColor" strokeWidth="1.6" fill="none" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 4l5.66 2.34L20 12l-2.34 5.66L12 20l-5.66-2.34L4 12l2.34-5.66z" opacity="0.5" />
      {Array.from({ length: 8 }, (_, k) => {
        const a = ((k * 45 - 90) * Math.PI) / 180;
        return <line key={k} x1="12" y1="12" x2={12 + 8 * Math.cos(a)} y2={12 + 8 * Math.sin(a)} opacity="0.5" />;
      })}
      <g fill="currentColor" stroke="none">
        {Array.from({ length: 8 }, (_, k) => {
          const a = ((k * 45 - 90) * Math.PI) / 180;
          return <circle key={k} cx={12 + 8 * Math.cos(a)} cy={12 + 8 * Math.sin(a)} r={k < 4 ? 1.7 : 1.4} opacity={k < 4 ? 1 : 0.45} />;
        })}
        <circle cx="12" cy="12" r="2" />
      </g>
    </g>
  ),
  '2048': (
    <g>
      <rect x="3.5" y="3.5" width="17" height="17" rx="3" fill="none" stroke="currentColor" strokeWidth="2" />
      <text x="12" y="16.5" textAnchor="middle" fontSize="9" fontWeight="800" fill="currentColor">
        2
      </text>
    </g>
  ),
  'world-threes': (
    <g>
      <g fill="none" stroke="currentColor" strokeWidth="1.6">
        <circle cx="12" cy="12" r="9" />
        <ellipse cx="12" cy="12" rx="3.6" ry="9" />
        <line x1="3" y1="12" x2="21" y2="12" />
      </g>
      <g fill="currentColor" stroke="none">
        <circle cx="7" cy="12" r="1.7" />
        <circle cx="12" cy="12" r="1.7" />
        <circle cx="17" cy="12" r="1.7" />
      </g>
    </g>
  ),
};

export function GameIcon({ id, size = 22 }: { id: string; size?: number }): JSX.Element {
  const glyph = GLYPHS[id];
  if (!glyph) {
    // emoji fallback for any game without a custom glyph yet
    const def = gameById(id);
    return (
      <span style={{ fontSize: size * 0.9, lineHeight: 1 }} aria-hidden>
        {def?.icon ?? '🎲'}
      </span>
    );
  }
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      aria-hidden
      style={{ display: 'inline-block', verticalAlign: 'middle', flexShrink: 0 }}
    >
      {glyph}
    </svg>
  );
}
