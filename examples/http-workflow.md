# Activation and renewal walkthrough

This walkthrough describes the implemented API. Run the isolated [local demo](../README.md#run-from-a-clean-checkout) with bash scripts/demo-local.sh to exercise it with synthetic inventory and development trust.

1. licensectl creates installation.json + device.key durably.
2. POST /v1/challenges with bearer activation credential:
~~~json
{"schema_version":1,"action":"activate","installation_id":"11111111-1111-4111-8111-111111111111","product":"worker-suite"}
~~~
3. Server returns challenge_id, random nonce and a five-minute expiry.
4. CLI builds the activate-request payload, including public key and collected inventory. It signs the exact envelope using the device key and request typ.
5. POST /v1/activations with the same activation credential and the signed envelope.
6. Server transaction validates nonce/slot/policy, inserts installation, signs lease, saves response and commits.
7. CLI verifies against independently installed issuer trust and live machine details, then installs license.lic atomically.
8. Worker validates locally through the native library.
9. Daily timer gets a renew challenge without a bearer token, signs a renew request with the stored device key, and calls /v1/installations/{id}/renew.
10. Lost response: retry the identical envelope; do not generate a fresh device identity.

Use [activate-request.json](../fixtures/activate-request.json) to inspect a signed synthetic request and [valid.lic](../fixtures/valid.lic) to inspect a test-signed response. Their keys are publicly known and unusable as production trust.

