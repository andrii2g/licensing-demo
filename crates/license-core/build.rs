fn main(){
    println!("cargo:rerun-if-env-changed=LICENSE_GUARD_TRUST_FILE");
    let data=match std::env::var("LICENSE_GUARD_TRUST_FILE"){
        Ok(path)=>{println!("cargo:rerun-if-changed={path}");std::fs::read_to_string(path).expect("read public trust file")},
        Err(_)=>"[]".into()
    };
    // Runtime parser also rejects these by value, even under a different key identifier.
    if std::env::var_os("CARGO_FEATURE_DEV").is_none(){
        for forbidden in ["TEST-ONLY","ebVWLo_mVPlAeLES6KmLp5AfhTrmlb7X4OORC60ElmQ","5_FioQvsVZr-oZXk3OhLaVaNXSywlj60RsBoXisX8vA"]{
            assert!(!data.contains(forbidden),"fixture trust forbidden in production");
        }
    }
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("trust.json"),data).unwrap();
}
