#!/usr/bin/env python3
"""Test a local clone of a source-only snapshot, without touching the user's Git state."""
import os, pathlib, shutil, subprocess, tempfile
root=pathlib.Path(__file__).resolve().parents[1]
env=os.environ.copy()
sdk=root/".dev-tools/dotnet"
if (sdk/"dotnet").exists(): env["PATH"]=str(sdk)+os.pathsep+env["PATH"]
with tempfile.TemporaryDirectory(prefix="license-guard-checkout-") as temporary:
    base=pathlib.Path(temporary)
    seed=base/"source";seed.mkdir()
    paths=subprocess.check_output(["git","ls-files","--cached","--others","--exclude-standard","-z"],cwd=root).split(b"\0")
    for encoded in sorted(set(paths)):
        if not encoded:continue
        relative=pathlib.Path(os.fsdecode(encoded))
        source=root/relative
        if not source.is_file() or any(part in (".git",".agents",".codex") for part in relative.parts):continue
        target=seed/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,target)
    def git(*args,cwd=seed):
        return subprocess.run(["git",*args],cwd=cwd,env=env,check=True)
    git("init","-q","-b","main")
    git("config","core.autocrlf","false")
    git("add",".")
    git("-c","user.name=License Guard Test","-c","user.email=local-test@example.invalid","commit","--no-gpg-sign","-qm","Source snapshot for local clean-checkout verification")
    git("clone","--quiet",str(seed),str(base/"checkout"),cwd=base)
    subprocess.run(["bash","scripts/demo-local.sh"],cwd=base/"checkout",env=env,check=True)
    print("PASS: demo from a fresh local Git clone, no generated files or SDK directories copied")
