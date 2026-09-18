#!/usr/bin/env python3
"""Real loopback API -> CLI -> native verifier -> managed/AOT process. Synthetic host by default."""
import argparse, base64, json, os, pathlib, selectors, signal, socket, subprocess, tempfile, time, urllib.request
import http.client, http.server, threading, shutil

ROOT = pathlib.Path(__file__).resolve().parents[1]

def write(path, data, mode=0o600):
    if not isinstance(data, (str, bytes, bytearray)):
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
    ap.add_argument("--container-smoke",action="store_true")
    a=ap.parse_args()
    binaries=(ROOT/a.bin_dir).resolve()
    executable=(ROOT/a.worker).resolve()
    worker_command=(["dotnet",str(executable)] if executable.suffix==".dll" else [str(executable)])
    state=pathlib.Path(tempfile.mkdtemp(prefix="license-guard-demo-"))
    processes=[]
    server=None
    proxy=None
    container_name=None
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
        # Drop the first committed activation response; client must retry identical bytes.
        class Proxy(http.server.BaseHTTPRequestHandler):
            dropped=False
            mutations=[]
            def log_message(self,*args): pass
            def do_POST(self):
                body=self.rfile.read(int(self.headers.get("Content-Length","0")))
                upstream=http.client.HTTPConnection("127.0.0.1",port,timeout=20)
                headers={k:v for k,v in self.headers.items() if k.lower() not in ("host","connection")}
                try:
                    upstream.request("POST",self.path,body,headers)
                    response=upstream.getresponse();data=response.read();status=response.status;upstream.close()
                except OSError:
                    self.connection.close();return
                if self.path=="/v1/activations":
                    Proxy.mutations.append(body)
                    if status==200 and not Proxy.dropped:
                        Proxy.dropped=True
                        self.connection.shutdown(socket.SHUT_RDWR);self.connection.close();return
                self.send_response(status);self.send_header("Content-Type","application/json");self.send_header("Content-Length",str(len(data)));self.end_headers();self.wfile.write(data)
        proxy=http.server.ThreadingHTTPServer(("127.0.0.1",0),Proxy)
        threading.Thread(target=proxy.serve_forever,daemon=True).start()
        proxy_port=proxy.server_address[1]
        install=state/"installation";install.mkdir(mode=0o750)
        client_config=(ROOT/"examples/client.toml").read_text().replace("https://licenses.example.invalid",f"http://127.0.0.1:{proxy_port}").replace("/var/lib/license-guard",str(install))
        write(state/"client.toml",client_config)
        ctl=[binaries/"licensectl","--config",state/"client.toml"]
        inspected=json.loads(run([*ctl,"inspect","--json"],env))
        assert "machine_id" not in inspected
        activated=json.loads(run([*ctl,"activate","--token-stdin"],env,stdin=token))
        assert activated["valid"] and activated["sequence"]==1
        assert Proxy.dropped and len(Proxy.mutations)>=2 and Proxy.mutations[0]==Proxy.mutations[1]
        run([*ctl,"status","--json"],env)
        env.update(LICENSE_NATIVE_PATH=str(binaries/"liblicense_guard.so"),LICENSE_FILE=str(install/"license.lic"),LICENSE_IDENTITY_FILE=str(install/"installation.json"))
        for _ in range(2):
            p=Worker(worker_command,env);processes.append(p);p.until("JOB_STARTED")
        renewed=json.loads(run([*ctl,"renew"],env))
        assert renewed["valid"] and renewed["sequence"]==2
        for p in processes:
            text=p.finish(0,stop=True)
            assert "JOB_COMPLETED" in text
        processes.clear()
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
        for name,defines in [("bad-abi",["-DBAD_ABI"]),("bad-json",[])]:
            lib=state/(name+".so")
            subprocess.run(["cc","-shared","-fPIC",*defines,str(ROOT/"tests/ffi_bad.c"),"-o",str(lib)],check=True)
            p=subprocess.run(worker_command,env=dict(env,LICENSE_NATIVE_PATH=str(lib)),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=15)
            assert p.returncode==78 and b"JOB_STARTED" not in p.stdout,(name,p.stdout)
        wrong_arch=bytearray((binaries/"liblicense_guard.so").read_bytes());wrong_arch[18:20]=b"\xb7\x00"
        write(state/"wrong-arch.so",wrong_arch,0o700)
        p=subprocess.run(worker_command,env=dict(env,LICENSE_NATIVE_PATH=str(state/"wrong-arch.so")),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=15)
        assert p.returncode==78 and b"JOB_STARTED" not in p.stdout
        if a.container_smoke:
            assert not a.real_host and executable.suffix!=".dll"
            smoke=state/"runtime-image";smoke.mkdir(mode=0o700)
            for source,name in [(executable,"NativeAotWorker"),(binaries/"liblicense_guard.so","liblicense_guard.so"),(install/"license.lic","license.lic"),(install/"installation.json","installation.json"),(issuer/"trust.json","trust.json"),(state/"inventory.json","inventory.json")]:
                shutil.copy2(source,smoke/name)
            container_name="license-guard-smoke-"+str(os.getpid())
            command=["docker","create","--network","none","--name",container_name,"--user",f"{os.getuid()}:{os.getgid()}"]
            for key,value in {"LICENSE_NATIVE_PATH":"/demo/liblicense_guard.so","LICENSE_FILE":"/demo/license.lic","LICENSE_IDENTITY_FILE":"/demo/installation.json","LICENSE_GUARD_DEV_TRUST":"/demo/trust.json","LICENSE_GUARD_DEV_INVENTORY":"/demo/inventory.json"}.items():
                command += ["--env",f"{key}={value}"]
            command += ["ubuntu:24.04","sh","-c","test ! -d /usr/share/dotnet && ! command -v dotnet && exec /demo/NativeAotWorker"]
            subprocess.run(command,check=True,stdout=subprocess.DEVNULL)
            subprocess.run(["docker","cp","-a",str(smoke),container_name+":/demo"],check=True)
            p=Worker(["docker","start","--attach",container_name],env);processes.append(p);p.until("JOB_STARTED",timeout=30)
            subprocess.run(["docker","kill","--signal=TERM",container_name],check=True,stdout=subprocess.DEVNULL)
            p.finish(0)
            result=subprocess.check_output(["docker","inspect","--format","{{.State.ExitCode}}",container_name]).strip()
            assert result==b"0",result
            print("PASS: published Native AOT worker on stock Ubuntu 24.04 with no .NET runtime and no network")
        if a.extended:
            # Separate short-lease installation. Drives runtime expiry without modifying wall time.
            entitlement.update(license_id="LIC-short",lease_seconds=12)
            write(state/"short-entitlement.json",entitlement)
            short_token=run([binaries/"license-admin","--config",state/"server.toml","entitlement","create","--input",state/"short-entitlement.json"],env)
            short=state/"short";short.mkdir(mode=0o750)
            write(state/"short.toml",client_config.replace(str(install),str(short)))
            short_ctl=[binaries/"licensectl","--config",state/"short.toml"]
            run([*short_ctl,"activate","--token-stdin"],env,stdin=short_token)
            short_env=dict(env,LICENSE_FILE=str(short/"license.lic"),LICENSE_IDENTITY_FILE=str(short/"installation.json"))
            p=Worker(worker_command,short_env);processes.append(p);p.until("JOB_STARTED")
            p.until("JOB_STARTED Job=2")
            assert json.loads(run([*short_ctl,"renew"],env))["sequence"]==2
            p.until("LEASE_ACCEPTED Sequence=2",timeout=20)
            # Service outage leaves the renewed local file usable until its deadline.
            server.send_signal(signal.SIGINT);server.communicate(timeout=10);server=None
            text=p.finish(78,timeout=40);assert "JOB_COMPLETED" in text and "LICENSE_DENIED" in text
            p=subprocess.run(worker_command,env=short_env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=15)
            assert p.returncode==78 and b"JOB_STARTED" not in p.stdout
            before=(install/"license.lic").read_bytes()
            run([*ctl,"renew"],env,expected=75)
            assert (install/"license.lic").read_bytes()==before
            server=subprocess.Popen([str(binaries/"license-server"),"--config",str(state/"server.toml")],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
            for _ in range(100):
                try:
                    with urllib.request.urlopen(f"http://127.0.0.1:{port}/health/ready",timeout=1): break
                except OSError: time.sleep(0.05)
            run([*short_ctl,"status","--json"],env,expected=78)
        retired=json.loads(run([*ctl,"retire"],env))
        assert retired["reserved_until"]>=int(time.time())
        run([*ctl,"remove"],env)
        assert (install/"installation.json").exists() and not (install/"license.lic").exists()
        print("PASS: activation, inspect/status, two workers per installation, graceful SIGTERM, renewal, missing/tampered/product/feature/library startup denial, retirement and local removal")
        if a.extended: print("PASS: live higher-sequence renewal, server outage, runtime expiry/draining, expired startup, failed renewal preserves file")
        print(f"Target: Linux x86_64 glibc; synthetic inventory={not a.real_host}")
    finally:
        for p in processes:p.close()
        if proxy is not None: proxy.shutdown();proxy.server_close()
        if container_name is not None: subprocess.run(["docker","rm","-f",container_name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        if server is not None:
            server.send_signal(signal.SIGINT) if server.poll() is None else None
            try:server.communicate(timeout=5)
            except subprocess.TimeoutExpired:server.kill();server.communicate()
        if a.keep: print(f"Development state retained at {state}")
        else:
            shutil.rmtree(state)
if __name__=="__main__":main()
