#!/usr/bin/env python3
"""Standard-library consistency and integrity checks for repository assets.
The small schema checker covers only the keywords used by the repository schemas;
it is not a general JSON Schema implementation or a production parser.
"""
from pathlib import Path
import hashlib, json, re, sqlite3, subprocess, tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
CONTRACTS = ROOT / "contracts"
MIGRATION = "crates/license-server/migrations/001_initial.sql"

def strict_pairs(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("Duplicate JSON key: " + key)
        value[key] = item
    return value

def load_json(path):
    return json.loads(Path(path).read_text(encoding='utf-8'), object_pairs_hook=strict_pairs)

def require(condition, message):
    if not condition: raise ValueError(message)

def check_schema(data, schema, base=CONTRACTS):
    if "$ref" in schema:
        target = base / schema["$ref"]
        return check_schema(data, load_json(target), target.parent)
    if "anyOf" in schema:
        for option in schema["anyOf"]:
            try: check_schema(data, option, base); return
            except ValueError: pass
        raise ValueError("No anyOf variant matches")
    for option in schema.get("allOf", []):
        check_schema(data, option, base)
    if "if" in schema:
        try: check_schema(data, schema["if"], base); matched=True
        except ValueError: matched=False
        check_schema(data, schema.get("then" if matched else "else", {}), base)
    if "not" in schema:
        try: check_schema(data, schema["not"], base)
        except ValueError: pass
        else: raise ValueError("Forbidden schema matches")
    if "const" in schema:
        require(data == schema["const"] and type(data) is type(schema["const"]), "Wrong const")
    if "enum" in schema:
        require(data in schema["enum"], "Wrong enum")
    kind=schema.get("type")
    types={"object":lambda: isinstance(data,dict),"array":lambda:isinstance(data,list),
           "string":lambda:isinstance(data,str),"integer":lambda:type(data) is int,
           "boolean":lambda:type(data) is bool,"null":lambda:data is None}
    if kind:require(kind in types and types[kind](), "Wrong type: "+str(kind))
    if isinstance(data,dict):
        require(set(schema.get("required",[])) <= set(data), "Missing required property")
        props=schema.get("properties",{})
        if schema.get("additionalProperties") is False:
            require(set(data)<=set(props), "Unknown property")
        for key,item in data.items():
            if key in props:check_schema(item,props[key],base)
    elif isinstance(data,list):
        require(len(data)>=schema.get("minItems",0), "Too few items")
        require(len(data)<=schema.get("maxItems",10**9), "Too many items")
        if schema.get("uniqueItems"):
            require(len({json.dumps(x,sort_keys=True) for x in data})==len(data), "Duplicate items")
        if "items" in schema:
            for x in data:check_schema(x,schema["items"],base)
    elif isinstance(data,str):
        require(len(data)>=schema.get("minLength",0),"String too short")
        require(len(data)<=schema.get("maxLength",10**9),"String too long")
        if "pattern" in schema:require(re.search(schema["pattern"],data) is not None,"Pattern mismatch")
    elif type(data) is int:
        require(data>=schema.get("minimum",-10**100),"Below minimum")
        require(data<=schema.get("maximum",10**100),"Above maximum")

def walk_refs(value,base):
    if isinstance(value,dict):
        if "$ref" in value:
            require((base/value["$ref"]).is_file(), "Missing schema reference "+value["$ref"])
        for item in value.values():walk_refs(item,base)
    elif isinstance(value,list):
        for item in value:walk_refs(item,base)

def verify_integrity():
    info=load_json(ROOT/"tests/asset-integrity.json")
    require(info["format_version"]==1, "Unsupported integrity manifest")
    protected={p.relative_to(ROOT).as_posix() for p in CONTRACTS.rglob("*") if p.is_file()}
    protected.update(p.relative_to(ROOT).as_posix() for p in (ROOT/"fixtures").iterdir() if p.suffix in (".json", ".lic"))
    protected.add(MIGRATION)
    records=info["files"]
    names={record["path"] for record in records}
    require(len(records)==len(names), "Duplicate integrity manifest entry")
    require(names==protected, "Integrity manifest coverage mismatch")
    for record in records:
        data=(ROOT/record["path"]).read_bytes()
        require(len(data)==record["size_bytes"], "Size mismatch "+record["path"])
        require(hashlib.sha256(data).hexdigest()==record["sha256"], "Hash mismatch "+record["path"])


def validate():
    verify_integrity()
    paths=[p for directory in ("contracts","fixtures","examples") for p in (ROOT/directory).rglob("*")]
    json_files=[p for p in paths if p.is_file() and p.suffix in (".json",".lic")]
    for path in json_files:load_json(path)
    schemas=list(CONTRACTS.glob("*.schema.json"))
    for path in schemas:walk_refs(load_json(path),path.parent)
    pairs=[
        ("fixtures/base-claims.json","license-claims"),
        ("fixtures/installation.json","installation"),
        ("fixtures/activate-payload.json","activate-request"),
        ("fixtures/activate-request.json","envelope"),
        ("examples/inventory-vmware.json","inventory"),
        ("examples/inventory-physical.json","inventory"),
        ("examples/native-request.json","native-request"),
        ("examples/native-result-valid.json","native-result")]
    for name,schema in pairs:
        check_schema(load_json(ROOT/name),load_json(CONTRACTS/(schema+".schema.json")))
    for p in (ROOT/"fixtures").glob("*.lic"):
        check_schema(load_json(p),load_json(CONTRACTS/"envelope.schema.json"))
    project=ROOT/"samples/dotnet/NativeAotWorker/NativeAotWorker.csproj"
    xml=ET.parse(project).getroot()
    require(xml.findtext(".//TargetFramework")=="net10.0","Unexpected managed target")
    require(xml.findtext(".//PublishAot")=="true","Native AOT not enabled")
    conn=sqlite3.connect(":memory:")
    conn.executescript((ROOT/MIGRATION).read_text(encoding="utf-8"))
    require(conn.execute("PRAGMA integrity_check").fetchone()[0]=="ok","SQLite migration integrity")
    require(conn.execute("SELECT version FROM schema_migrations").fetchall()==[(1,)],"Migration missing")
    conn.close()
    for path in [ROOT/"samples/dotnet/publish-linux.sh", *sorted((ROOT/"scripts").glob("*.sh"))]:
        subprocess.run(["bash","-n",str(path)],check=True)
    for p in (ROOT/"scripts").glob("*.py"):
        compile(p.read_text(encoding="utf-8"),str(p),"exec")
    with tempfile.TemporaryDirectory() as tmp:
        c=Path(tmp)/"header.c"
        c.write_text('#include "license_guard.h"\nint main(void) { return 0; }\n')
        subprocess.run(["cc","-std=c11","-Wall","-Wextra","-Werror","-fsyntax-only","-I",str(CONTRACTS),str(c)],check=True)
    print("PASS: JSON/examples, schema references, C header, project XML, shell/Python syntax, deployed SQLite migration and required asset integrity.")
    print("JSON/lease files:",len(json_files),"schemas:",len(schemas))
    print("Static asset checks only; run bash scripts/check.sh for build, ABI and lifecycle gates.")

if __name__=="__main__":validate()

