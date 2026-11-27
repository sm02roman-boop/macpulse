#!/usr/bin/env bash
# Builds MacPulse.app (universal) and packages it as dist/MacPulse.dmg. Run on a Mac.
set -euo pipefail
cd "$(dirname "$0")/.."

./scripts/build_app.sh

STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
cp -R dist/MacPulse.app "$STAGE/"
ln -s /Applications "$STAGE/Applications"

rm -f dist/MacPulse.dmg
hdiutil create -volname "MacPulse" -srcfolder "$STAGE" -ov -format UDZO dist/MacPulse.dmg
echo "Built dist/MacPulse.dmg  (open it and drag MacPulse into Applications)"
