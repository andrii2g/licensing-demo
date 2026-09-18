#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
python3 scripts/local_flow.py --bin-dir target/dev/debug --worker artifacts/native-worker/linux-x64/NativeAotWorker --container-smoke --extended
