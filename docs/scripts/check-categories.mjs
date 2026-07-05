// Every game id registered in games/index.ts must appear in a Launcher
// category — there is no fallback bucket; missing ids vanish from the shelf.
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const launcher = readFileSync(join(root, 'src/components/arcade/Launcher.tsx'), 'utf8');
const catBlock = launcher.match(/const CATEGORIES[\s\S]*?\n\];/)?.[0] ?? '';
const categorized = new Set([...catBlock.matchAll(/'([a-z0-9-]+)'/g)].map((m) => m[1]));

const gamesDir = join(root, 'src/components/arcade/games');
const { readdirSync } = await import('node:fs');
const ids = [];
for (const f of readdirSync(gamesDir)) {
  if (!f.endsWith('.tsx')) continue;
  const src = readFileSync(join(gamesDir, f), 'utf8');
  for (const m of src.matchAll(/^\s*id:\s*'([a-z0-9-]+)'/gm)) ids.push(m[1]);
}
const missing = ids.filter((id) => !categorized.has(id));
if (missing.length) {
  console.error(`check-categories: not in any Launcher category: ${missing.join(', ')}`);
  process.exit(1);
}
console.log(`check-categories: ${ids.length} game ids all categorized.`);
