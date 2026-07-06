// Every game id registered in games/index.ts must appear in a Launcher
// category — there is no fallback bucket; missing ids vanish from the shelf.
// The one exception: a VARIANT CHILD (a GameDefinition that declares
// `variantOf: '<parent id>'`) is reached through its parent's family expansion,
// so it may legitimately be absent from CATEGORIES. Such a child must instead
// point at a real, categorized, NON-variant parent (no variantOf chains).
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
// id -> parent id for every child that declares variantOf. Convention: the
// `variantOf` line sits immediately after its `id` line in the game file.
const variantOf = new Map();
for (const f of readdirSync(gamesDir)) {
  if (!f.endsWith('.tsx')) continue;
  const src = readFileSync(join(gamesDir, f), 'utf8');
  for (const m of src.matchAll(/^\s*id:\s*'([a-z0-9-]+)'/gm)) ids.push(m[1]);
  for (const m of src.matchAll(
    /^\s*id:\s*'([a-z0-9-]+)',\s*\n\s*variantOf:\s*'([a-z0-9-]+)',/gm,
  )) {
    variantOf.set(m[1], m[2]);
  }
}
const idSet = new Set(ids);

const problems = [];
for (const id of ids) {
  const parent = variantOf.get(id);
  if (parent === undefined) {
    // A normal (non-variant) game must be categorized.
    if (!categorized.has(id)) {
      problems.push(`'${id}' is in no Launcher category and declares no variantOf`);
    }
  } else {
    // A variant child must point at a real, categorized, non-variant parent.
    if (!idSet.has(parent)) {
      problems.push(`'${id}' has variantOf '${parent}', which is not a known game id`);
    } else if (!categorized.has(parent)) {
      problems.push(`'${id}' has variantOf '${parent}', which is itself in no category`);
    } else if (variantOf.has(parent)) {
      problems.push(`'${id}' has variantOf '${parent}', which is itself a variant (no chains)`);
    }
  }
}
if (problems.length) {
  console.error(`check-categories: ${problems.length} problem(s):`);
  for (const p of problems) console.error(`  - ${p}`);
  process.exit(1);
}

// Every game id must also have a monochrome GLYPHS entry in icons.tsx — without
// one it falls back to a colour emoji, which sticks out on the monochrome shelf.
const icons = readFileSync(join(root, 'src/components/arcade/icons.tsx'), 'utf8');
const glyphBlock = icons.match(/const GLYPHS[\s\S]*?\n\};/)?.[0] ?? '';
const glyphed = new Set([...glyphBlock.matchAll(/^ {2}(?:'([a-z0-9-]+)'|([a-z0-9]+)):\s*\(/gm)].map((m) => m[1] ?? m[2]));
const noGlyph = ids.filter((id) => !glyphed.has(id));
if (noGlyph.length) {
  console.error(`check-categories: no GLYPHS entry in icons.tsx (would fall back to emoji): ${noGlyph.join(', ')}`);
  process.exit(1);
}

console.log(
  `check-categories: ${ids.length} game ids all glyphed and either categorized or variant of a categorized parent (${variantOf.size} variant children).`,
);
