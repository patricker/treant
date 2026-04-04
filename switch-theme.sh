#!/bin/bash
set -e
if [ -z "$1" ]; then
  echo "Usage: ./switch-theme.sh <number>"
  echo "Available themes:"
  ls docs/src/css/themes/ | sed 's/^/  /'
  exit 1
fi
THEME_NUM=$(printf "%02d" "$1")
THEME_FILE=$(ls docs/src/css/themes/theme-${THEME_NUM}-*.css 2>/dev/null | head -1)
if [ -z "$THEME_FILE" ]; then
  echo "Theme $1 not found. Available:"
  ls docs/src/css/themes/
  exit 1
fi
cp "$THEME_FILE" docs/src/css/custom.css
echo "Switched to: $(basename "$THEME_FILE")"
