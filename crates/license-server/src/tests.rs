use crate::{config::Config,issuer::Issuer,repository as repo,service,ApiError};
use license_core::*;
use ed25519_dalek::SigningKey;
use rusqlite::Connection;
fn config()->Config{toml::from_str(include_str!("../../../examples/server.toml")).unwrap()}
fn db()->Connection{let c=Connection::open_in_memory().unwrap();c.execute_batch(include_str!("../migrations/001_initial.sql")).unwrap();c}
fn entitlement(slots:u32)->Entitlement{let mut e:Entitlement=serde_json::from_str(include_str!("../../../examples/entitlement.json")).unwrap();e.max_installations=slots;e.expires_at=1800145000;e.lease_seconds=100;e}
fn inventory()->Inventory{serde_json::from_str(include_str!("../../../examples/inventory-vmware.json")).unwrap()}
fn identity(n:u8)->(Identity,SigningKey){let k=SigningKey::from_bytes(&[n;32]);(Identity{schema_version:1,installation_id:uuid::Uuid::new_v4().to_string(),installation_public_key:envelope::encode(k.verifying_key().as_bytes())},k)}
fn request(c:&mut Connection,i:&Identity,k:&SigningKey,a:Action,t:Option<&str>,now:i64)->Vec<u8>{
    let ch=service::challenge(c,&ChallengeRequest{schema_version:1,action:a,installation_id:i.installation_id.clone(),product:"worker-suite".into()},t,now,&config()).unwrap();
    let r=DeviceRequest{schema_version:1,action:a,operation_id:uuid::Uuid::new_v4().to_string(),challenge_id:ch.challenge_id,nonce:ch.nonce,installation_id:i.installation_id.clone(),product:"worker-suite".into(),installation_public_key:i.installation_public_key.clone(),inventory:if a==Action::Retire{None}else{Some(inventory())}};
    serde_json::to_vec(&crypto::sign(&r,REQUEST_TYPE,&i.installation_id,k).unwrap()).unwrap()
}
fn issue(c:&mut Connection,issuer:&Issuer,r:&[u8],a:Action,t:Option<&str>,now:i64)->std::result::Result<String,ApiError>{
    let e=envelope::Envelope::parse(r).unwrap();let req=DeviceRequest::parse(&e).unwrap();
    service::mutate(c,issuer,r,a,if a==Action::Activate{None}else{Some(&req.installation_id)},t,now,86400)
}
#[test]fn activation_renew_replay_retirement_boundary(){
    let now=1800144000;let mut c=db();let issuer=Issuer::new("dev-generated".into(),SigningKey::from_bytes(&crypto::random::<32>().unwrap())).unwrap();
    let token=repo::create(&mut c,&entitlement(1),now).unwrap();let(i,k)=identity(70);
    let request1=request(&mut c,&i,&k,Action::Activate,Some(&token),now);
    let response=issue(&mut c,&issuer,&request1,Action::Activate,Some(&token),now).unwrap();
    assert_eq!(response,issue(&mut c,&issuer,&request1,Action::Activate,Some(&token),now).unwrap());
    let inv=inventory();let context=Context{now,product:"worker-suite".into(),required_features:vec!["messaging".into()],identity:i.clone(),machine_id_hash:Some(inv.machine_id_hash),system_uuid_hash:inv.system_uuid_hash,logical_processors:inv.cpu.logical_processors};
    let trust=Trust::new(vec![(issuer.kid.clone(),issuer.public())]).unwrap();
    assert_eq!(verify(response.as_bytes(),&trust,&context).unwrap().claims.sequence,1);
    let e=envelope::Envelope::parse(&request1).unwrap();let mut mutated=DeviceRequest::parse(&e).unwrap();
    mutated.inventory.as_mut().unwrap().hostname="changed".into();
    let mutated=serde_json::to_vec(&crypto::sign(&mutated,REQUEST_TYPE,&i.installation_id,&k).unwrap()).unwrap();
    assert_eq!(issue(&mut c,&issuer,&mutated,Action::Activate,Some(&token),now).unwrap_err().code,"IDEMPOTENCY_CONFLICT");
    let mut replay=DeviceRequest::parse(&e).unwrap();replay.operation_id=uuid::Uuid::new_v4().to_string();
    let replay=serde_json::to_vec(&crypto::sign(&replay,REQUEST_TYPE,&i.installation_id,&k).unwrap()).unwrap();
    assert_eq!(issue(&mut c,&issuer,&replay,Action::Activate,Some(&token),now).unwrap_err().code,"CHALLENGE_INVALID");
    let renew=request(&mut c,&i,&k,Action::Renew,None,now+10);
    let response=issue(&mut c,&issuer,&renew,Action::Renew,None,now+10).unwrap();
    assert_eq!(policy::authenticate(response.as_bytes(),&trust).unwrap().claims.sequence,2);
    let retire=request(&mut c,&i,&k,Action::Retire,None,now+20);
    let retired=issue(&mut c,&issuer,&retire,Action::Retire,None,now+20).unwrap();
    assert_eq!(retired,issue(&mut c,&issuer,&retire,Action::Retire,None,now+20).unwrap());
    assert_eq!(issue(&mut c,&issuer,&renew,Action::Renew,None,now+20).unwrap_err().status,410);
    let(j,l)=identity(71);let replacement=request(&mut c,&j,&l,Action::Activate,Some(&token),now+20);
    assert_eq!(issue(&mut c,&issuer,&replacement,Action::Activate,Some(&token),now+20).unwrap_err().code,"ACTIVATION_LIMIT");
    assert!(issue(&mut c,&issuer,&replacement,Action::Activate,Some(&token),now+110).is_ok());
    assert_eq!(c.query_row("SELECT count(*) FROM installations",[],|r|r.get::<_,u32>(0)).unwrap(),2);
}
#[test]fn revoke_expire_and_scope_after_challenge(){
    let now=1800144000;let mut c=db();let issuer=Issuer::new("issuer".into(),SigningKey::from_bytes(&[72;32])).unwrap();
    let token=repo::create(&mut c,&entitlement(1),now).unwrap();let(i,k)=identity(73);
    let r=request(&mut c,&i,&k,Action::Activate,Some(&token),now);
    assert_eq!(issue(&mut c,&issuer,&r,Action::Renew,None,now).unwrap_err().status,400);
    assert_eq!(issue(&mut c,&issuer,&r,Action::Activate,Some(&token),now+300).unwrap_err().code,"CHALLENGE_INVALID");
    repo::revoke(&mut c,"LIC-2026-001234",now).unwrap();
    assert_eq!(issue(&mut c,&issuer,&r,Action::Activate,Some(&token),now).unwrap_err().status,401);
    assert_eq!(c.query_row("SELECT count(*) FROM installations",[],|r|r.get::<_,i64>(0)).unwrap(),0);
}
#[test]fn parallel_one_slot(){
    let d=tempfile::tempdir().unwrap();let path=d.path().join("race.db");let mut c=Connection::open(&path).unwrap();c.execute_batch(include_str!("../migrations/001_initial.sql")).unwrap();
    let now=1800144000;let token=repo::create(&mut c,&entitlement(1),now).unwrap();
    let requests=(80..82).map(|n|{let(i,k)=identity(n);request(&mut c,&i,&k,Action::Activate,Some(&token),now)}).collect::<Vec<_>>();
    let barrier=std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles=requests.into_iter().map(|r|{
        let p=path.clone();let t=token.clone();let b=barrier.clone();
        std::thread::spawn(move||{let mut c=Connection::open(p).unwrap();c.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
            let issuer=Issuer::new("issuer".into(),SigningKey::from_bytes(&[83;32])).unwrap();b.wait();issue(&mut c,&issuer,&r,Action::Activate,Some(&t),now)})
    }).collect::<Vec<_>>();
    let results=handles.into_iter().map(|h|h.join().unwrap()).collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r|r.is_ok()).count(),1);
    assert_eq!(results.iter().filter_map(|r|r.as_ref().err()).next().unwrap().code,"ACTIVATION_LIMIT");
}
#[tokio::test]async fn http_limits_and_shapes(){
    use tower::ServiceExt;use axum::{body::Body,http::Request};use http_body_util::BodyExt;
    let app=crate::routes::App::new(db(),Issuer::new("issuer".into(),SigningKey::from_bytes(&[84;32])).unwrap(),config());
    let router=crate::routes::router(app);
    let health=router.clone().oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap()).await.unwrap();assert_eq!(health.status(),200);
    for body in ["{\"schema_version\":1,\"schema_version\":1}".to_string(),"x".repeat(131073)]{
        let res=router.clone().oneshot(Request::builder().method("POST").uri("/v1/challenges").header("content-type","application/json").body(Body::from(body)).unwrap()).await.unwrap();
        assert_eq!(res.status(),400);let bytes=res.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema_version"],1);
    }
}
