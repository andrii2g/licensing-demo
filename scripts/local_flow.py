#!/usr/bin/env python3
"""Real loopback API -> CLI -> native verifier -> managed/AOT process. Synthetic host by default."""
import argparse, base64, json, os, pathlib, selectors, signal, socket, subprocess, tempfile, time, urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]

def write(path, data, mode=0o600):
    if not isinstance(data, (str, bytes)):
        data = json.dumps(data)
    path.write_bytes(data.encode() if isinstance(data, str) else data)
    path.chmod(mode)

def run(args, env, expected=0, stdin=None):
    p = subprocess.run([str(x) for x in args], input=stdin, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)
    if p.returncode != expected:
        raise AssertionError(f"{pathlib.Path(str(args[0])).name}: exit {p.returncode}, expected {expected}; {p.stderr.decode()}")
    return p.stdout

class Worker:
    def __init__(self, command, env):
        self.p = subprocess.Popen(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.lines = []
        self.pending = b""
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.p.stdout, selectors.EVENT_READ)
    def until(self, needle, timeout=20):
        end=time.monotonic()+timeout
        while time.monotonic()<end:
            if any(needle in s for s in self.lines): return
            for key,_ in self.selector.select(min(0.25,max(0,end-time.monotonic()))):
                data=os.read(key.fd,4096)
                if not data:
                    raise AssertionError(f"worker exited before {needle}: {self.lines}")
                self.pending+=data
                while b"\n" in self.pending:
                    line,self.pending=self.pending.split(b"\n",1)
                    self.lines.append(line.decode(errors="replace"))
        raise AssertionError(f"timeout waiting for {needle}: {self.lines}")
    def finish(self, expected, stop=False, timeout=40):
        if stop: self.p.send_signal(signal.SIGTERM)
        out,_=self.p.communicate(timeout=timeout)
        text="\n".join(self.lines)+"\n"+self.pending.decode(errors="replace")+out.decode(errors="replace")
        assert self.p.returncode==expected, (self.p.returncode,expected,text)
        return text
    def close(self):
        if self.p.poll() is None:
            self.p.kill()
            self.p.wait()

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument("--bin-dir",default="target/dev/debug")
    ap.add_argument("--worker",required=True)
    ap.add_argument("--real-host",action="store_true",help="Explicit opt-in staging collection; default is synthetic")
    ap.add_argument("--keep",action="store_true")
    ap.add_argument("--extended",action="store_true")
    a=ap.parse_args()
    binaries=(ROOT/a.bin_dir).resolve()
    executable=(ROOT/a.worker).resolve()
    worker_command=(["dotnet",str(executable)] if executable.suffix==".dll" else [str(executable)])
    state=pathlib.Path(tempfile.mkdtemp(prefix="license-guard-demo-"))
    processes=[]
    server=None
    env=os.environ.copy()
    try:
        issuer=state/"issuer";issuer.mkdir(mode=0o700)
        run([binaries/"license-admin","keygen","--directory",issuer,"--kid","dev-generated"],env)
        env["LICENSE_GUARD_DEV_TRUST"]=str(issuer/"trust.json")
        if not a.real_host:
            inventory=json.loads((ROOT/"examples/inventory-vmware.json").read_text())
            write(state/"inventory.json",inventory)
            env["LICENSE_GUARD_DEV_INVENTORY"]=str(state/"inventory.json")
        sock=socket.socket();sock.bind(("127.0.0.1",0));port=sock.getsockname()[1];sock.close()
        server_config=(ROOT/"examples/server.toml").read_text().replace("127.0.0.1:8080",f"127.0.0.1:{port}").replace("/var/lib/license-guard-server/licenses.db",str(issuer/"licenses.db")).replace("/etc/license-guard-server/issuer.key",str(issuer/"issuer.key")).replace("issuer-2026-01","dev-generated")
        server_config=server_config.replace("requests_per_ip_per_minute = 60","requests_per_ip_per_minute = 600").replace("requests_per_installation_per_minute = 20","requests_per_installation_per_minute = 200")
        write(state/"server.toml",server_config)
        entitlement=json.loads((ROOT/"examples/entitlement.json").read_text())
        entitlement.update(expires_at=int(time.time())+3600,lease_seconds=120,max_installations=1)
        write(state/"entitlement.json",entitlement)
        token=run([binaries/"license-admin","--config",state/"server.toml","entitlement","create","--input",state/"entitlement.json"],env)
        server=subprocess.Popen([str(binaries/"license-server"),"--config",str(state/"server.toml")],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        for _ in range(100):
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/health/ready",timeout=1) as response:
                    if response.status==200: break
            except OSError:
                if server.poll() is not None: raise AssertionError(server.communicate())
                time.sleep(0.05)
        else: raise AssertionError("server readiness timeout")
        install=state/"installation";install.mkdir(mode=0o750)
        client_config=(ROOT/"examples/client.toml").read_text().replace("https://licenses.example.invalid",f"http://127.0.0.1:{port}").replace("/var/lib/license-guard",str(install))
        write(state/"client.toml",client_config)
        ctl=[binaries/"licensectl","--config",state/"client.toml"]
        inspected=json.loads(run([*ctl,"inspect","--json"],env))
        assert "machine_id" not in inspected
        activated=json.loads(run([*ctl,"activate","--token-stdin"],env,stdin=token))
        assert activated["valid"] and activated["sequence"]==1
        run([*ctl,"status","--json"],env)
        env.update(LICENSE_NATIVE_PATH=str(binaries/"liblicense_guard.so"),LICENSE_FILE=str(install/"license.lic"),LICENSE_IDENTITY_FILE=str(install/"installation.json"))
        for _ in range(2):
            p=Worker(worker_command,env);processes.append(p);p.until("JOB_STARTED")
        for p in processes:
            text=p.finish(0,stop=True)
            assert "JOB_COMPLETED" in text
        processes.clear()
        renewed=json.loads(run([*ctl,"renew"],env))
        assert renewed["valid"] and renewed["sequence"]==2
        valid=(install/"license.lic").read_bytes()
        for name,mutation in [
            ("missing",None),
            ("tampered",b'{"protected":"a","payload":"b","signature":"c"}'),
        ]:
            if mutation is None: (install/"license.lic").unlink()
            else: write(install/"license.lic",mutation,0o640)
            p=subprocess.run(worker_command,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=15)
            assert p.returncode==78 and b"JOB_STARTED" not in p.stdout,(name,p.returncode,p.stdout)
            write(install/"license.lic",valid,0o640)
        wrong=env.copy();wrong["LICENSE_PRODUCT"]="wrong-product"
        for changed in [wrong,dict(env,LICENSE_NATIVE_PATH=str(state/"missing.so")),dict(env,LICENSE_FEATURE="not-entitled")]:
            p=subprocess.run(worker_command,env=changed,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=15)
            assert p.returncode==78 and b"JOB_STARTED" not in p.stdout,p.stdout
        if a.extended:
            # Separate short-lease installation. Drives runtime expiry without modifying wall time.
            entitlement.update(license_id="LIC-short",lease_seconds=6)
            write(state/"short-entitlement.json",entitlement)
            short_token=run([binaries/"license-admin","--config",state/"server.toml","entitlement","create","--input",state/"short-entitlement.json"],env)
            short=state/"short";short.mkdir(mode=0o750)
            write(state/"short.toml",client_config.replace(str(install),str(short)))
            short_ctl=[binaries/"licensectl","--config",state/"short.toml"]
            run([*short_ctl,"activate","--token-stdin"],env,stdin=short_token)
            short_env=dict(env,LICENSE_FILE=str(short/"license.lic"),LICENSE_IDENTITY_FILE=str(short/"installation.json"))
            p=Worker(worker_command,short_env);processes.append(p);p.until("JOB_STARTED")
            text=p.finish(78,timeout=40);assert "JOB_COMPLETED" in text and "LICENSE_DENIED" in text
            run([*short_ctl,"status","--json"],env,expected=78)
        retired=json.loads(run([*ctl,"retire"],env))
        assert retired["reserved_until"]>=int(time.time())
        run([*ctl,"remove"],env)
        assert (install/"installation.json").exists() and not (install/"license.lic").exists()
        print("PASS: activation, inspect/status, two workers per installation, graceful SIGTERM, renewal, missing/tampered/product/feature/library startup denial, retirement and local removal")
        if a.extended: print("PASS: runtime expiry closes admission, drains work and exits 78")
        print(f"Target: Linux x86_64 glibc; synthetic inventory={not a.real_host}")
    finally:
        for p in processes:p.close()
        if server is not None:
            server.send_signal(signal.SIGINT) if server.poll() is None else None
            try:server.communicate(timeout=5)
            except subprocess.TimeoutExpired:server.kill();server.communicate()
        if a.keep: print(f"Development state retained at {state}")
        else:
            import shutil
            shutil.rmtree(state)
if __name__=="__main__":main()
