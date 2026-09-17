#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
export DOTNET_CLI_TELEMETRY_OPTOUT=1
if [[ -x .dev-tools/dotnet/dotnet ]]; then export PATH="$PWD/.dev-tools/dotnet:$PATH"; fi
cargo build --locked --workspace --features licensectl/dev,license-ffi/dev,license-server/dev,license-admin/dev --target-dir target/dev
dotnet build samples/dotnet/NativeAotWorker -c Release -r linux-x64 -p:PublishAot=false -o artifacts/managed-worker --nologo
python3 scripts/local_flow.py --bin-dir target/dev/debug --worker artifacts/managed-worker/NativeAotWorker.dll "$@"
