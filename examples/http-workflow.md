# Activation and renewal walkthrough

Values here describe a future implemented API. No real server is contacted by this kit.

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

Use fixtures/activate-request.json to inspect a real signed synthetic request and fixtures/valid.lic to inspect a real test-signed response. Their keys are publicly known and unusable as production trust.

