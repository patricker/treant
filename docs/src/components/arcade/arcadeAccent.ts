import { useEffect, useState } from 'react';

// The arcade ships in a single CRT-neon look; the only user choice is the accent
// colour. Each accent is a near-black theme with one bright hue. The board
// renderers keep their own (--arc-*) palette — only the launcher + shell chrome
// react to the accent. Persisted per browser; defaults to Phosphor green.
export const ACCENTS = [
  { id: 'green', label: 'Phosphor', color: '#39ff7a' },
  { id: 'amber', label: 'Amber', color: '#ffb000' },
  { id: 'cyan', label: 'Cyan', color: '#2fd4ff' },
  { id: 'magenta', label: 'Magenta', color: '#ff2e88' },
  { id: 'stark', label: 'Stark', color: '#ffffff' },
] as const;

export type AccentId = (typeof ACCENTS)[number]['id'];

const KEY = 'treant-arcade-accent';
const DEFAULT: AccentId = 'green';
const isAccent = (v: unknown): v is AccentId => ACCENTS.some((a) => a.id === v);

/**
 * Accent state for the arcade, persisted to localStorage. Returns the current
 * accent and a setter that also mirrors the choice onto `document.body` (via the
 * `data-arcade-accent` attribute) so global CSS can paint a full-viewport
 * backdrop behind the centred arcade column.
 */
export function useArcadeAccent(): [AccentId, (a: AccentId) => void] {
  const [accent, setAccent] = useState<AccentId>(DEFAULT);

  // Read the saved choice on mount (client-only; the arcade renders inside
  // <BrowserOnly>, so localStorage is available).
  useEffect(() => {
    try {
      const saved = localStorage.getItem(KEY);
      if (isAccent(saved)) setAccent(saved);
    } catch {
      /* private mode / blocked storage — fall back to the default */
    }
  }, []);

  // Keep <body> in sync so the backdrop tracks the accent, and clean up on leave.
  useEffect(() => {
    document.body.dataset.arcadeAccent = accent;
    return () => {
      delete document.body.dataset.arcadeAccent;
    };
  }, [accent]);

  const set = (a: AccentId) => {
    setAccent(a);
    try {
      localStorage.setItem(KEY, a);
    } catch {
      /* ignore persistence failures */
    }
  };

  return [accent, set];
}
