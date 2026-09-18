#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
cargo build --locked --workspace --features licensectl/dev,license-ffi/dev,license-server/dev,license-admin/dev --target-dir target/dev
bash samples/dotnet/publish-linux.sh linux-x64
python3 scripts/local_flow.py --bin-dir target/dev/debug --worker artifacts/native-worker/linux-x64/NativeAotWorker --extended
