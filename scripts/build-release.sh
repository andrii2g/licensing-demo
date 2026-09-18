#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
[[ "$(uname -m)" == x86_64 && "$(uname -s)" == Linux ]] || { echo "Only Linux x86_64 glibc is validated." >&2; exit 64; }
trust_file="${LICENSE_GUARD_TRUST_FILE:?Set an absolute path to your public issuer trust JSON}"
[[ "$trust_file" == /* && -f "$trust_file" ]] || { echo "Public trust file must exist at an absolute path." >&2; exit 64; }
# No release artifacts until all gates, including real C#/AOT, have passed.
(unset LICENSE_GUARD_TRUST_FILE; bash scripts/check.sh)
export LICENSE_GUARD_TRUST_FILE="$trust_file"
cargo run --locked --release -p license-core --example check_trust --target-dir target/release-build
cargo build --locked --release --workspace --no-default-features --target-dir target/release-build
bash samples/dotnet/publish-linux.sh linux-x64
package_root="$(mktemp -d "$PWD/artifacts/package.XXXXXX")"
trap 'rm -rf -- "$package_root"' EXIT
client="$package_root/client"
server="$package_root/server"
install -Dm755 target/release-build/release/licensectl "$client/usr/bin/licensectl"
install -Dm644 target/release-build/release/liblicense_guard.so "$client/usr/lib/license-guard/liblicense_guard.so"
install -Dm755 artifacts/native-worker/linux-x64/NativeAotWorker "$client/opt/license-guard/worker/NativeAotWorker"
for file in deploy/systemd/license-guard-worker.service deploy/systemd/license-guard-renew.service deploy/systemd/license-guard-renew.timer; do
  install -Dm644 "$file" "$client/usr/lib/systemd/system/$(basename "$file")"
done
install -Dm644 deploy/tmpfiles.d/license-guard.conf "$client/usr/lib/tmpfiles.d/license-guard.conf"
install -Dm640 examples/client.toml "$client/etc/license-guard/client.toml.example"
install -Dm755 target/release-build/release/license-server "$server/usr/bin/license-server"
install -Dm755 target/release-build/release/license-admin "$server/usr/bin/license-admin"
install -Dm600 examples/server.toml "$server/etc/license-guard-server/server.toml.example"
install -Dm644 deploy/systemd/license-guard-server.service "$server/usr/lib/systemd/system/license-guard-server.service"
python3 scripts/dependency_report.py "$package_root/dependencies.cdx.json"
for part in client server; do
  install -Dm644 README.md "$package_root/$part/usr/share/doc/license-guard/README.md"
  install -Dm644 "$package_root/dependencies.cdx.json" "$package_root/$part/usr/share/doc/license-guard/dependencies.cdx.json"
  tar --sort=name --owner=0 --group=0 --numeric-owner -C "$package_root/$part" -czf "artifacts/license-guard-$part-linux-x64-glibc.tar.gz" .
done
if tar -tzf artifacts/license-guard-client-linux-x64-glibc.tar.gz | grep -E '/(license-server|license-admin|issuer.key|device.key|fixtures)(/|$)'; then
  echo "Server/private material in client package" >&2; exit 1
fi
if strings target/release-build/release/liblicense_guard.so | grep 'LICENSE_GUARD_DEV_'; then
  echo "Development override in release" >&2; exit 1
fi
(cd artifacts && sha256sum license-guard-{client,server}-linux-x64-glibc.tar.gz > SHA256SUMS)
echo "Packages: artifacts/license-guard-{client,server}-linux-x64-glibc.tar.gz"
