#!/usr/bin/env bash
set -euo pipefail
sample_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$sample_dir/../.." && pwd)"
rid="${1:-linux-x64}"
[[ "$rid" == linux-x64 ]] || { echo "Only linux-x64 is currently validated." >&2; exit 64; }
if [[ -x "$repo_dir/.dev-tools/dotnet/dotnet" ]]; then export PATH="$repo_dir/.dev-tools/dotnet:$PATH"; fi
dotnet publish "$sample_dir/NativeAotWorker/NativeAotWorker.csproj" \
  --configuration Release --runtime "$rid" --self-contained true \
  -p:PublishAot=true -p:StripSymbols=true \
  --output "$repo_dir/artifacts/native-worker/$rid"
