#!/usr/bin/env python3
"""Emit a CycloneDX 1.5 component inventory from committed dependency locks."""
import json, pathlib, sys, tomllib
root=pathlib.Path(__file__).resolve().parents[1]
components=[]
for package in tomllib.loads((root/"Cargo.lock").read_text())["package"]:
    item={"type":"library","name":package["name"],"version":package["version"],
          "purl":f"pkg:cargo/{package['name']}@{package['version']}"}
    if "checksum" in package:
        item["hashes"]=[{"alg":"SHA-256","content":package["checksum"]}]
    components.append(item)
nugets=set()
for lock in (root/"samples/dotnet").rglob("packages.lock.json"):
    for framework,dependencies in json.loads(lock.read_text())["dependencies"].items():
        for name,package in dependencies.items():
            if "resolved" in package:nugets.add((name,package["resolved"]))
for name,version in sorted(nugets):
    components.append({"type":"library","name":name,"version":version,"purl":f"pkg:nuget/{name}@{version}"})
out={"bomFormat":"CycloneDX","specVersion":"1.5","version":1,"components":components}
pathlib.Path(sys.argv[1]).write_text(json.dumps(out,indent=2)+"\n")
