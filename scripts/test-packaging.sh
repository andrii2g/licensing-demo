#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
if [[ -x .dev-tools/dotnet/dotnet ]]; then export PATH="$PWD/.dev-tools/dotnet:$PATH"; fi
mkdir -p artifacts
test_state="$(mktemp -d /tmp/license-guard-packaging.XXXXXX)"
trap 'rm -rf -- "$test_state"' EXIT
cargo build --locked -p license-admin --features dev --target-dir target/dev
target/dev/debug/license-admin keygen --directory "$test_state/issuer" --kid packaging-test
export LICENSE_GUARD_TRUST_FILE="$test_state/issuer/trust.json"
export LICENSE_GUARD_PACKAGE_PREFIX=DEV-TRUST-TEST
bash scripts/build-release.sh > artifacts/release-test.log 2>&1
python3 scripts/package-smoke.py artifacts/DEV-TRUST-TEST-client-linux-x64-glibc.tar.gz "$test_state/issuer/issuer.key" packaging-test >> artifacts/release-test.log 2>&1
echo 'PASS: full gates and isolated development-trust production-feature package smoke test; see artifacts/release-test.log'
