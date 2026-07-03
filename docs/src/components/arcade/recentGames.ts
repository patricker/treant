// "Recently played" memory for the launcher, persisted per browser like the
// accent/language choices. Most-recent-first, capped so the row stays one line.
const KEY = 'treant-arcade-recent';
const MAX = 6;

export function recentGames(): string[] {
  try {
    const raw = localStorage.getItem(KEY);
    const list = raw ? JSON.parse(raw) : [];
    return Array.isArray(list) ? list.filter((x) => typeof x === 'string').slice(0, MAX) : [];
  } catch {
    return [];
  }
}

export function recordRecentGame(id: string): void {
  try {
    const next = [id, ...recentGames().filter((g) => g !== id)].slice(0, MAX);
    localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    /* private mode — recents just don't persist */
  }
}
