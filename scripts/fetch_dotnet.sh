#!/usr/bin/env bash
# Downloads the latest .NET 8 win-x64 runtime ZIP into vendor/ for
# bundling into the DLL. Re-run when bumping the bundled runtime.
set -euo pipefail

OUT="vendor/dotnet-runtime-win-x64.zip"
META="https://builds.dotnet.microsoft.com/dotnet/release-metadata/8.0/releases.json"

URL=$(curl -fsSL "$META" \
    | python3 -c "
import json, sys
d = json.load(sys.stdin)
for f in d['releases'][0]['runtime']['files']:
    if f['rid'] == 'win-x64' and f['name'].endswith('.zip'):
        print(f['url']); break
")

if [[ -z "$URL" ]]; then
    echo "could not resolve .NET runtime URL from $META" >&2
    exit 1
fi

echo "downloading $URL"
mkdir -p vendor
curl -fL --progress-bar -o "$OUT.tmp" "$URL"
mv "$OUT.tmp" "$OUT"
ls -lh "$OUT"
