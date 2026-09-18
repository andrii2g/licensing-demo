#!/usr/bin/env python3
"""Verify a production-feature package using a temporary DEVELOPMENT issuer in an isolated container."""
import base64, hashlib, json, os, pathlib, selectors, signal, subprocess, sys, tempfile, time
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat

root=pathlib.Path(__file__).resolve().parents[1]
archive=pathlib.Path(sys.argv[1]).resolve()
seed=pathlib.Path(sys.argv[2]).read_bytes()
kid=sys.argv[3]
encode=lambda b:base64.urlsafe_b64encode(b).rstrip(b"=").decode()
device=Ed25519PrivateKey.generate().public_key().public_bytes(Encoding.Raw,PublicFormat.Raw)
import uuid
identity={"schema_version":1,"installation_id":str(uuid.uuid4()),"installation_public_key":encode(device)}
claims=json.loads((root/"fixtures/base-claims.json").read_text())
now=int(time.time())
claims.update(installation_id=identity["installation_id"],installation_public_key_sha256=hashlib.sha256(device).hexdigest(),
              sequence=1,issued_at=now,not_before=now-120,lease_valid_until=now+120,entitlement_expires_at=now+120,
              mode="manual_offline",max_logical_processors=None)
claims["binding"]["policy"]="linux-machine-v1"
claims["binding"]["system_uuid_hash"]=None
header=encode(json.dumps({"typ":"license-guard-lease-v1","alg":"Ed25519","kid":kid},separators=(",",":")).encode())
payload=encode(json.dumps(claims,separators=(",",":")).encode())
signature=encode(Ed25519PrivateKey.from_private_bytes(seed).sign(("license-guard/v1\n"+header+"."+payload).encode()))
envelope={"protected":header,"payload":payload,"signature":signature}
name="license-guard-package-smoke-"+str(os.getpid())
with tempfile.TemporaryDirectory(prefix="license-guard-package-smoke-") as temporary:
    path=pathlib.Path(temporary)
    (path/"license.lic").write_text(json.dumps(envelope))
    (path/"installation.json").write_text(json.dumps(identity))
    script="""set -eu
tar -xzf /client.tar.gz -C /
mkdir /trusted
cp /input/license.lic /input/installation.json /trusted/
chgrp 65534 /trusted /trusted/license.lic /trusted/installation.json
chmod 750 /trusted
chmod 640 /trusted/license.lic /trusted/installation.json
printf '%s\\n' 0123456789abcdef0123456789abcdef > /etc/machine-id
test ! -d /usr/share/dotnet
! command -v dotnet
export LICENSE_FILE=/trusted/license.lic
export LICENSE_IDENTITY_FILE=/trusted/installation.json
exec setpriv --reuid=65534 --regid=65534 --clear-groups /opt/license-guard/worker/NativeAotWorker
"""
    try:
        subprocess.run(["docker","create","--network","none","--name",name,"ubuntu:24.04","sh","-c",script],check=True,stdout=subprocess.DEVNULL)
        subprocess.run(["docker","cp",str(archive),name+":/client.tar.gz"],check=True)
        subprocess.run(["docker","cp",str(path),name+":/input"],check=True)
        proc=subprocess.Popen(["docker","start","--attach",name],stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        selector=selectors.DefaultSelector();selector.register(proc.stdout,selectors.EVENT_READ)
        output=b"";deadline=time.monotonic()+45
        while b"JOB_STARTED" not in output and time.monotonic()<deadline:
            for key,_ in selector.select(0.25):
                part=os.read(key.fd,4096)
                if not part:raise AssertionError(output.decode(errors="replace"))
                output+=part
        assert b"JOB_STARTED" in output,output
        subprocess.run(["docker","kill","--signal=TERM",name],check=True,stdout=subprocess.DEVNULL)
        tail,_=proc.communicate(timeout=40);output+=tail
        assert subprocess.check_output(["docker","inspect","--format","{{.State.ExitCode}}",name]).strip()==b"0",output
        print("PASS: production-feature client archive, live synthetic Linux machine-id, unprivileged AOT worker, no .NET runtime/network, SIGTERM exit 0")
    finally:
        subprocess.run(["docker","rm","-f",name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
