#!/usr/bin/env bash
set -euo pipefail
sample_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$sample_dir/../.." && pwd)"
rid="${1:-linux-x64}"
case "$rid" in
  linux-x64|linux-arm64) ;;
  *) echo "Supported script RIDs: linux-x64, linux-arm64 (arm64 needs its own validation)" >&2; exit 64 ;;
esac
dotnet publish "$sample_dir/NativeAotWorker/NativeAotWorker.csproj" \
  --configuration Release --runtime "$rid" --self-contained true \
  -p:PublishAot=true -p:StripSymbols=true \
  --output "$repo_dir/artifacts/native-worker/$rid"
echo "Published native executable; install the matching Rust .so separately."

