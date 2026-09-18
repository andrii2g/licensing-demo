#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
if [[ -x .dev-tools/dotnet/dotnet ]]; then export PATH="$PWD/.dev-tools/dotnet:$PATH"; fi
export PYTHONDONTWRITEBYTECODE=1
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
python3 scripts/validate_kit.py
python3 scripts/verify_fixtures.py
dotnet run --project samples/dotnet/NativeAotWorker.Tests -c Release
bash scripts/demo-local.sh --extended
bash scripts/test-ffi.sh
bash scripts/test-aot.sh
bash scripts/test-runtime-image.sh
bash scripts/test-production.sh
if command -v cargo-audit >/dev/null; then cargo audit
elif [[ -x .dev-tools/audit/bin/cargo-audit ]]; then .dev-tools/audit/bin/cargo-audit audit
else echo "Install cargo-audit 0.22.2 before release." >&2; exit 1; fi
dotnet list samples/dotnet/NativeAotWorker package --vulnerable --include-transitive
