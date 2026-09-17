#!/usr/bin/env python3
"""Independent Python Ed25519 verification and fixed policy-vector checks.
Test-only: does not implement production file handling, ABI, networking or trust.
"""
import base64, copy, hashlib, hmac, json, re
from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
from validate_kit import ROOT, CONTRACTS, check_schema, load_json, require, strict_pairs

def b64(value):
    require(bool(re.fullmatch(r"[A-Za-z0-9_-]+",value)),"Bad base64url")
    result=base64.urlsafe_b64decode(value+"="*((-len(value))%4))
    require(base64.urlsafe_b64encode(result).rstrip(b"=").decode()==value,"Noncanonical base64")
    return result

def payload(envelope):
    return json.loads(b64(envelope["payload"]),object_pairs_hook=strict_pairs)

def verify_signature(envelope,pub):
    message=b"license-guard/v1\n"+envelope["protected"].encode()+b"."+envelope["payload"].encode()
    try:pub.verify(b64(envelope["signature"]),message);return True
    except InvalidSignature:return False

def evaluate(env,context,keys):
    try:
        check_schema(env,load_json(CONTRACTS/"envelope.schema.json"))
        header=json.loads(b64(env["protected"]),object_pairs_hook=strict_pairs)
        check_schema(header,load_json(CONTRACTS/"protected.schema.json"))
    except (ValueError,KeyError):return "LICENSE_MALFORMED"
    if header["typ"]!="license-guard-lease-v1":return "UNSUPPORTED_FORMAT"
    if header["kid"]!=keys["kid"]:return "UNKNOWN_KEY"
    public=Ed25519PublicKey.from_public_bytes(b64(keys["issuer_public_key"]))
    if not verify_signature(env,public):return "INVALID_SIGNATURE"
    try:
        c=payload(env)
        check_schema(c,load_json(CONTRACTS/"license-claims.schema.json"))
        require(c["not_before"]<=c["issued_at"]<c["lease_valid_until"]<=c["entitlement_expires_at"],"Bad chronology")
    except (ValueError,KeyError):return "LICENSE_MALFORMED"
    if c["product"]!=context["product"]:return "PRODUCT_MISMATCH"
    if not set(context["required_features"])<=set(c["features"]):return "FEATURE_MISSING"
    identity=context["identity"]
    if c["installation_id"]!=identity["installation_id"] or c["installation_public_key_sha256"]!=hashlib.sha256(b64(identity["installation_public_key"])).hexdigest():
        return "INSTALLATION_MISMATCH"
    now=context["now"]
    if now<c["not_before"]:return "NOT_YET_VALID"
    if c["issued_at"]>now+120:return "CLOCK_SUSPECT"
    if now>=c["lease_valid_until"]:return "LICENSE_EXPIRED"
    binding=c["binding"]
    required=["machine_id_hash"]+(["system_uuid_hash"] if binding["policy"]=="linux-host-v1" else [])
    if any(context.get(k) is None for k in required):return "IDENTITY_UNAVAILABLE"
    if any(context[k]!=binding[k] for k in required):return "MACHINE_MISMATCH"
    limit=c["max_logical_processors"]
    if limit is not None:
        if context["logical_processors"] is None:return "CAPACITY_UNAVAILABLE"
        if context["logical_processors"]>limit:return "CAPACITY_EXCEEDED"
    return "VALID"

def main():
    manifest=load_json(ROOT/"fixtures/manifest.json")
    keys=load_json(ROOT/"fixtures/TEST-ONLY-keys.json")
    public=Ed25519PublicKey.from_public_bytes(b64(keys["issuer_public_key"]))
    for case in manifest["cases"]:
        envelope=load_json(ROOT/"fixtures"/case["file"])
        actual_signature=verify_signature(envelope,public)
        require(actual_signature==case["signature_valid_under_test_key"],"Crypto mismatch: "+case["file"])
        context=copy.deepcopy(manifest["base_context"]);context.update(case["context_changes"])
        actual=evaluate(envelope,context,keys)
        require(actual==case["expected"],case["file"]+": expected "+case["expected"]+", got "+actual)
    vector=load_json(ROOT/"fixtures/fingerprint-vector.json")
    k=hashlib.sha256(("license-guard/fingerprint/v1/"+vector["product"]).encode()).digest()
    require(k.hex()==vector["public_domain_key_hex"],"Fingerprint domain key")
    for kind,raw,expected in [("machine-id","normalized_machine_id","machine_id_hash"),
                              ("system-uuid","normalized_system_uuid","system_uuid_hash")]:
        digest=hmac.new(k,(kind+":"+vector[raw]).encode(),hashlib.sha256).hexdigest()
        require(digest==vector[expected],"Fingerprint mismatch")
    request=load_json(ROOT/"fixtures/activate-request.json")
    require(verify_signature(request,Ed25519PublicKey.from_public_bytes(b64(keys["device_public_key"]))),"Device signature")
    check_schema(payload(request),load_json(CONTRACTS/"activate-request.schema.json"))
    require(payload(request)==load_json(ROOT/"fixtures/activate-payload.json"),"Request payload mismatch")
    print("PASS:",len(manifest["cases"]),"lease vectors, device request signature, and fingerprint vectors.")
    print("Test-only trust; no production Rust/.NET validation is claimed.")

if __name__=="__main__":main()

