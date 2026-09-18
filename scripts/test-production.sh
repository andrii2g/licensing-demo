#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
unset LICENSE_GUARD_TRUST_FILE
cargo build --locked -p license-ffi -p licensectl --target-dir target/production
mkdir -p artifacts/tests
cc -std=c11 -Wall -Wextra -Werror tests/production_trust.c -Ltarget/production/debug -llicense_guard -Wl,-rpath,/check -o artifacts/tests/production-trust
if strings target/production/debug/liblicense_guard.so | grep -q 'LICENSE_GUARD_DEV_'; then echo "Development override in production artifact" >&2; exit 1; fi
container="license-guard-production-$$"
trap 'docker rm -f "$container" >/dev/null 2>&1 || true' EXIT
docker create --network none --name "$container" \
  --env LICENSE_GUARD_DEV_TRUST=/fixtures/TEST-ONLY-keys.json \
  --env LICENSE_GUARD_DEV_INVENTORY=/fixtures/manifest.json \
  ubuntu:24.04 sh /test-production.sh >/dev/null
docker cp fixtures "$container:/fixtures"
docker cp tests/production-container.sh "$container:/test-production.sh"
docker cp target/production/debug/liblicense_guard.so "$container:/liblicense_guard.so"
docker cp artifacts/tests/production-trust "$container:/production-trust"
docker start --attach "$container"
[[ "$(docker inspect --format '{{.State.ExitCode}}' "$container")" == 0 ]]
docker rm "$container" >/dev/null
test_dir="$(mktemp -d /tmp/license-guard-trust-test.XXXXXX)"
trap 'rm -rf -- "$test_dir"' EXIT
python3 - "$test_dir/trust.json" <<'PY'
import json,sys
from pathlib import Path
key=json.loads(Path("fixtures/TEST-ONLY-keys.json").read_text())["issuer_public_key"]
Path(sys.argv[1]).write_text(json.dumps([["renamed-fixture",key]]))
PY
if LICENSE_GUARD_TRUST_FILE="$test_dir/trust.json" cargo check --locked -p license-ffi --target-dir target/rejected-trust >"$test_dir/build.log" 2>&1; then
  echo "Fixture key was accepted by production build" >&2; exit 1
fi
grep -q "fixture trust forbidden in production" "$test_dir/build.log"
echo "PASS: production build refuses renamed fixture public key"
