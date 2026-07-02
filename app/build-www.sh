#!/usr/bin/env bash
# Build the docs site in app-mode and stage it as the Capacitor web root.
#
# ARCADE_APP_BUILD=1 tells docusaurus.config.ts to drop Google Analytics (gtag)
# and the PWA service worker, producing an offline, telemetry-free bundle fit
# for an app-store binary. The result is copied to ./www (Capacitor's webDir).
#
# The tail of this script is a hard gate: if any analytics or service-worker
# artifact survives into www/, it fails loudly rather than shipping a build that
# would make the store's "no data collected" declaration false.
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(cd "$APP_DIR/../docs" && pwd)"
WWW_DIR="$APP_DIR/www"

echo "==> Building docs site (ARCADE_APP_BUILD=1) in $DOCS_DIR"
( cd "$DOCS_DIR" && ARCADE_APP_BUILD=1 npm run build )

echo "==> Staging build → $WWW_DIR"
rm -rf "$WWW_DIR"
cp -r "$DOCS_DIR/build" "$WWW_DIR"

echo "==> Verifying app build is stripped"
fail=0

if find "$WWW_DIR" -name 'sw.js' -o -name 'sw-*.js' | grep -q .; then
  echo "ERROR: service worker file found in www/ — plugin-pwa was not stripped" >&2
  find "$WWW_DIR" -name 'sw.js' -o -name 'sw-*.js' >&2
  fail=1
fi

if grep -rIl --include='*.js' --include='*.html' 'googletagmanager\|gtag/js\|G-GP4CNHKDF0' "$WWW_DIR" >/dev/null 2>&1; then
  echo "ERROR: Google Analytics references found in www/ — gtag was not stripped" >&2
  grep -rIl --include='*.js' --include='*.html' 'googletagmanager\|gtag/js\|G-GP4CNHKDF0' "$WWW_DIR" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "==> Build FAILED verification — refusing to ship www/" >&2
  exit 1
fi

echo "==> OK: www/ is analytics-free and has no service worker"
echo "    Next: npx cap sync"
