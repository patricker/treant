import type { JSX } from 'react';
import { gameById } from './games';

// Consistent flat icon set. All use `currentColor`, so they pick up the
// surrounding text color (white on the hero, dark on tiles/headers).
const GLYPHS: Record<string, JSX.Element> = {
  'connect-four': (
    // four discs on a diagonal
    <g fill="currentColor">
      <circle cx="6" cy="18" r="2.6" />
      <circle cx="11" cy="13.5" r="2.6" />
      <circle cx="16" cy="9" r="2.6" opacity="0.55" />
      <circle cx="21" cy="4.5" r="2.6" opacity="0.3" />
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
  trails: (
    <g stroke="currentColor" strokeWidth="2.2" fill="none" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 20v-6h6V8h6V4" />
      <circle cx="4" cy="20" r="1.8" fill="currentColor" stroke="none" />
      <circle cx="16" cy="4" r="1.8" fill="currentColor" stroke="none" />
    </g>
  ),
  reversi: (
    <g>
      <circle cx="12" cy="12" r="9.5" fill="none" stroke="currentColor" strokeWidth="2" />
      <path d="M12 2.5a9.5 9.5 0 0 1 0 19z" fill="currentColor" />
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
  '2048': (
    <g>
      <rect x="3.5" y="3.5" width="17" height="17" rx="3" fill="none" stroke="currentColor" strokeWidth="2" />
      <text x="12" y="16.5" textAnchor="middle" fontSize="9" fontWeight="800" fill="currentColor">
        2
      </text>
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
