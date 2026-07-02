#!/usr/bin/env node
// Extracts the arcade's translation-key inventory and checks locale
// dictionaries against it.
//
// Keys are gettext-style (the English string is the key). They reach t() two
// ways: literal t('...')/tn(...) call sites, and GameDefinition data fields
// (name/blurb/preset labels/knob labels/playerLabels/rules) that render sites
// wrap in t(). This script collects both by scanning source text.
//
// Usage:
//   node scripts/i18n-keys.mjs           # report coverage per locale
//   node scripts/i18n-keys.mjs --write   # also write i18n/keys.json manifest
//   node scripts/i18n-keys.mjs --check   # exit 1 if any locale misses a key
import { readFileSync, readdirSync, writeFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const ARCADE = join(ROOT, 'src/components/arcade');
const LOCALES_DIR = join(ARCADE, 'i18n/locales');

const keys = new Set();

function* walk(dir) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) yield* walk(p);
    else if (/\.(tsx?|ts)$/.test(e.name)) yield p;
  }
}

// A quoted JS string literal (no template literals — keys must be static).
const STR = /'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)"/;
const unescape = (s) => s.replace(/\\(['"\\])/g, '$1');

function addMatches(src, re, groups = [1]) {
  for (const m of src.matchAll(re)) {
    for (const g of groups) {
      if (m[g] !== undefined) {
        const key = unescape(m[g]).trim();
        if (key) keys.add(key);
      }
    }
  }
}

for (const file of walk(ARCADE)) {
  const src = readFileSync(file, 'utf8');

  // 1. Literal t('...') / t("...") / translate('...') call sites (first argument only).
  addMatches(src, /\b(?:t|translate)\(\s*'((?:[^'\\]|\\.)*)'/g);
  addMatches(src, /\b(?:t|translate)\(\s*"((?:[^"\\]|\\.)*)"/g);
  // tn(n, 'singular', 'plural')
  addMatches(
    src,
    /\btn\(\s*[^,]+,\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")\s*,\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")/g,
    [1, 2, 3, 4],
  );

  // 2. GameDefinition data fields translated at render sites.
  addMatches(src, /\bname:\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")/g, [1, 2]);
  addMatches(src, /\bblurb:\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")/g, [1, 2]);
  addMatches(src, /\blabel:\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")/g, [1, 2]);
  for (const m of src.matchAll(/\bplayerLabels:\s*\[([^\]]*)\]/g)) {
    for (const s of m[1].matchAll(new RegExp(STR.source, 'g'))) {
      keys.add(unescape(s[1] ?? s[2]));
    }
  }
  // Long-form rules: `rules:` fields and the RULES map in rules.ts.
  addMatches(src, /\brules:\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")/g, [1, 2]);
  if (file.endsWith('rules.ts')) {
    addMatches(src, /:\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)")\s*,?\s*$/gm, [1, 2]);
  }
}

// Data fields that hold non-translatable tokens, collected by the broad
// `label:` regex above. Keep this list tight and explicit.
const NOT_KEYS = new Set(['X', 'O', '−', '+']);
for (const k of NOT_KEYS) keys.delete(k);

const manifest = [...keys].sort((a, b) => a.localeCompare(b));
console.log(`${manifest.length} distinct keys.`);

const args = new Set(process.argv.slice(2));
if (args.has('--write')) {
  writeFileSync(join(ARCADE, 'i18n/keys.json'), JSON.stringify(manifest, null, 2) + '\n');
  console.log('Wrote i18n/keys.json');
}

let failed = false;
if (existsSync(LOCALES_DIR)) {
  for (const f of readdirSync(LOCALES_DIR).filter((f) => f.endsWith('.json'))) {
    const dict = JSON.parse(readFileSync(join(LOCALES_DIR, f), 'utf8'));
    const have = new Set(Object.keys(dict));
    const missing = manifest.filter((k) => !have.has(k));
    const orphans = [...have].filter((k) => !keys.has(k));
    console.log(`${f}: ${have.size} entries, ${missing.length} missing, ${orphans.length} orphaned`);
    if (missing.length && args.has('--verbose')) missing.forEach((k) => console.log(`  missing: ${k}`));
    if (orphans.length && args.has('--verbose')) orphans.forEach((k) => console.log(`  orphan:  ${k}`));
    if (missing.length) failed = true;
  }
}
if (args.has('--check') && failed) process.exit(1);
