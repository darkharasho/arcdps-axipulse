#!/usr/bin/env bash
# Downloads the latest GW2EICLI.zip into vendor/ for bundling into the DLL.
# Re-run when bumping the bundled EI version.
set -euo pipefail

OUT="vendor/GW2EICLI.zip"
URL=$(curl -fsSL https://api.github.com/repos/baaron4/GW2-Elite-Insights-Parser/releases/latest \
    | grep -oE 'https://[^"]+GW2EICLI\.zip' | sort -u | head -1)

if [[ -z "$URL" ]]; then
    echo "could not resolve GW2EICLI.zip download URL" >&2
    exit 1
fi

echo "downloading $URL"
mkdir -p vendor
curl -fL --progress-bar -o "$OUT.tmp" "$URL"
mv "$OUT.tmp" "$OUT"
ls -lh "$OUT"
