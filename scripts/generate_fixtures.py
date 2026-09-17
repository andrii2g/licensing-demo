#!/usr/bin/env python3
"""Recreate PUBLIC test vectors. Never use these keys in a real deployment."""
from pathlib import Path
import base64, copy, hashlib, hmac, json
from datetime import datetime, timezone
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat

ROOT = Path(__file__).resolve().parents[1]
FIX = ROOT / "fixtures"
NOW = int(datetime(2027, 1, 17, tzinfo=timezone.utc).timestamp())
PRODUCT = "worker-suite"
INSTALL = "11111111-1111-4111-8111-111111111111"
KID = "TEST-ONLY-issuer-v1"
ISSUER_SEED = bytes(range(1, 33))
DEVICE_SEED = bytes(range(33, 65))
issuer = Ed25519PrivateKey.from_private_bytes(ISSUER_SEED)
device = Ed25519PrivateKey.from_private_bytes(DEVICE_SEED)

def b64(value): return base64.urlsafe_b64encode(value).rstrip(b"=").decode("ascii")
def compact(value): return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
def pub(key): return key.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)
def put(name, value):
    path = ROOT / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")

def sign(payload, kid=KID, typ="license-guard-lease-v1", key=issuer, raw=None):
    protected = b64(compact({"typ":typ, "alg":"Ed25519", "kid":kid}))
    encoded = b64(raw if raw is not None else compact(payload))
    message = b"license-guard/v1\n" + protected.encode() + b"." + encoded.encode()
    return {"protected":protected, "payload":encoded, "signature":b64(key.sign(message))}

machine = "0123456789abcdef0123456789abcdef"
uuid = "12345678-1234-4234-8234-123456789abc"
fingerprint_key = hashlib.sha256(("license-guard/fingerprint/v1/" + PRODUCT).encode()).digest()
def fingerprint(kind, value):
    return hmac.new(fingerprint_key, (kind+":"+value).encode(), hashlib.sha256).hexdigest()
mh, sh = fingerprint("machine-id", machine), fingerprint("system-uuid", uuid)
identity = {"schema_version":1, "installation_id":INSTALL, "installation_public_key":b64(pub(device))}
inventory = {
    "schema_version":1, "collected_at":NOW, "hostname":"vmware-demo-01",
    "network_interfaces":[{"name":"ens160","mac":"00:50:56:12:34:56","kind":"virtual"},
                          {"name":"ens192","mac":"02:11:22:33:44:55","kind":"virtual"}],
    "cpu":{"model":"Synthetic guest CPU","sockets":1,"cores":4,"logical_processors":8,"available_logical_processors":4},
    "os":{"id":"ubuntu","version":"24.04","kernel":"synthetic-6.x","architecture":"x86_64"},
    "virtualization":"vmware", "machine_id_hash":mh, "system_uuid_hash":sh
}
claims = {
    "schema_version":1,"issuer":"license-guard","license_id":"LIC-TEST-0001","customer_id":"customer-demo",
    "product":PRODUCT,"installation_id":INSTALL,"installation_public_key_sha256":hashlib.sha256(pub(device)).hexdigest(),
    "sequence":1,"issued_at":NOW,"not_before":NOW-120,
    "lease_valid_until":NOW+604800,"entitlement_expires_at":NOW+31536000,
    "mode":"renewable","features":["messaging","transcoding"],
    "binding":{"policy":"linux-host-v1","machine_id_hash":mh,"system_uuid_hash":sh},
    "max_logical_processors":None
}
context = {"now":NOW,"product":PRODUCT,"required_features":["messaging"],"identity":identity,
           "machine_id_hash":mh,"system_uuid_hash":sh,"logical_processors":8}
cases=[]
def case(name, expected, payload=None, envelope=None, context_changes=None, signature_valid=True):
    envelope = envelope if envelope is not None else sign(payload if payload is not None else claims)
    put("fixtures/"+name+".lic", envelope)
    cases.append({"file":name+".lic","expected":expected,"context_changes":context_changes or {},
                  "signature_valid_under_test_key":signature_valid})
def changed(**kw):
    result=copy.deepcopy(claims);result.update(kw);return result
case("valid","VALID")
case("expired","LICENSE_EXPIRED",changed(issued_at=NOW-604900,not_before=NOW-605000,lease_valid_until=NOW-1))
case("not-yet-valid","NOT_YET_VALID",changed(issued_at=NOW+60,not_before=NOW+60))
case("future-issued","CLOCK_SUSPECT",changed(issued_at=NOW+121))
case("wrong-product","PRODUCT_MISMATCH",changed(product="different-product"))
case("missing-feature","FEATURE_MISSING",changed(features=["transcoding"]))
case("wrong-installation","INSTALLATION_MISMATCH",changed(installation_id="22222222-2222-4222-8222-222222222222"))
binding=copy.deepcopy(claims["binding"]);binding["machine_id_hash"]="0"*64
case("wrong-machine","MACHINE_MISMATCH",changed(binding=binding))
case("capacity-exceeded","CAPACITY_EXCEEDED",changed(max_logical_processors=4))
case("capacity-unknown","CAPACITY_UNAVAILABLE",changed(max_logical_processors=8),context_changes={"logical_processors":None})
case("missing-dmi","IDENTITY_UNAVAILABLE",context_changes={"system_uuid_hash":None})
case("offline","VALID",changed(mode="manual_offline",lease_valid_until=claims["entitlement_expires_at"]))
case("at-expiry","LICENSE_EXPIRED",context_changes={"now":NOW+604800})
case("unknown-key","UNKNOWN_KEY",envelope=sign(claims,kid="unknown-issuer"))
case("unknown-field","LICENSE_MALFORMED",changed(unexpected=True))
case("unsupported-header","UNSUPPORTED_FORMAT",envelope=sign(claims,typ="license-guard-request-v1"))
duplicate=compact(claims)[:-1]+b',"product":"worker-suite"}'
case("duplicate-key","LICENSE_MALFORMED",envelope=sign(claims,raw=duplicate))
bad=sign(claims);sig=bytearray(base64.urlsafe_b64decode(bad["signature"]+"=="));sig[0]^=1;bad["signature"]=b64(sig)
case("bad-signature","INVALID_SIGNATURE",envelope=bad,signature_valid=False)
bad=sign(claims);bad["payload"]=b64(compact(changed(lease_valid_until=NOW+999999)))
case("tampered-expiry","INVALID_SIGNATURE",envelope=bad,signature_valid=False)
case("invalid-chronology","LICENSE_MALFORMED",changed(not_before=NOW+1,issued_at=NOW))

request={"schema_version":1,"action":"activate","operation_id":"33333333-3333-4333-8333-333333333333",
         "challenge_id":"44444444-4444-4444-8444-444444444444","nonce":b64(bytes(range(32))),
         "installation_id":INSTALL,"product":PRODUCT,"installation_public_key":b64(pub(device)),"inventory":inventory}
put("fixtures/activate-payload.json",request)
put("fixtures/activate-request.json",sign(request,kid=INSTALL,typ="license-guard-request-v1",key=device))
put("fixtures/installation.json",identity)
put("fixtures/base-claims.json",claims)
put("examples/inventory-vmware.json",inventory)
physical=copy.deepcopy(inventory);physical["hostname"]="hardware-demo-01";physical["virtualization"]="physical"
physical["network_interfaces"]=[{"name":"enp1s0","mac":"00:11:22:33:44:55","kind":"physical"}]
put("examples/inventory-physical.json",physical)
put("fixtures/TEST-ONLY-keys.json",{"warning":"PUBLIC deterministic test keys. NEVER trust or deploy in production.",
    "kid":KID,"issuer_seed_hex":ISSUER_SEED.hex(),"issuer_public_key":b64(pub(issuer)),
    "device_seed_hex":DEVICE_SEED.hex(),"device_public_key":b64(pub(device))})
put("fixtures/fingerprint-vector.json",{"product":PRODUCT,"normalized_machine_id":machine,
    "normalized_system_uuid":uuid,"public_domain_key_hex":fingerprint_key.hex(),
    "machine_id_hash":mh,"system_uuid_hash":sh})
put("fixtures/manifest.json",{"schema_version":1,"fixed_utc":"2027-01-17T00:00:00Z","base_context":context,"cases":cases})
env=sign(claims)
digest=hashlib.sha256((env["protected"]+"."+env["payload"]+"."+env["signature"]).encode()).hexdigest()
put("examples/native-request.json",{"schema_version":1,"license_path":"/var/lib/license-guard/license.lic",
    "identity_path":"/var/lib/license-guard/installation.json","product":PRODUCT,"required_features":["messaging"]})
put("examples/native-result-valid.json",{"schema_version":1,"valid":True,"code":"VALID","checked_at":NOW,
    "license_id":claims["license_id"],"installation_id":INSTALL,"sequence":1,
    "lease_valid_until":claims["lease_valid_until"],"entitlement_expires_at":claims["entitlement_expires_at"],
    "features":claims["features"],"lease_digest":digest})
print("Generated",len(cases),"lease vectors; fixed UTC seconds =",NOW)

