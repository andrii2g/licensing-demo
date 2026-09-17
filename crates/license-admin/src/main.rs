#![forbid(unsafe_code)]
use clap::{Parser,Subcommand};
use std::path::PathBuf;
use license_core::*;
#[derive(Parser)]struct Args{#[arg(long,global=true)]config:Option<PathBuf>,#[command(subcommand)]command:Command}
#[derive(Subcommand)]enum Command{
    Entitlement{#[command(subcommand)]command:EntitlementCommand},
    Installation{#[command(subcommand)]command:InstallationCommand},
    Installations{#[command(subcommand)]command:ListCommand},
    Keygen{#[arg(long)]directory:PathBuf,#[arg(long)]kid:String},
}
#[derive(Subcommand)]enum EntitlementCommand{Create{#[arg(long)]input:PathBuf},Revoke{#[arg(long)]id:String}}
#[derive(Subcommand)]enum InstallationCommand{Retire{#[arg(long)]id:String}}
#[derive(Subcommand)]enum ListCommand{List{#[arg(long)]license_id:String}}
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(64);}}
fn run()->std::result::Result<(),Box<dyn std::error::Error>>{
    let args=Args::parse();
    if let Command::Keygen{directory,kid}=args.command{
        identifier(&kid)?;let d=license_store::SecureDir::create(&directory)?;let _lock=d.lock()?;
        match d.read("issuer.key",32,true){Err(Code::LicenseMissing)=>{},_=>return Err("KEY_ALREADY_EXISTS".into())}
        let seed=zeroize::Zeroizing::new(crypto::random::<32>()?);let key=ed25519_dalek::SigningKey::from_bytes(&seed);
        d.atomic("issuer.key",seed.as_slice(),true)?;
        d.atomic("trust.json",&serde_json::to_vec(&vec![(kid,envelope::encode(key.verifying_key().as_bytes()))])?,false)?;
        println!("KEY_CREATED");return Ok(());
    }
    let c=license_server::config::Config::read(&args.config.ok_or("CONFIG_REQUIRED")?)?;
    let dir=license_store::SecureDir::open(std::path::Path::new(&c.database_path).parent().ok_or("DATABASE_PATH")?)?;
    let _lock=dir.lock()?;
    let mut db=license_server::repository::open(std::path::Path::new(&c.database_path),c.sqlite_busy_timeout_milliseconds)?;
    match args.command{
        Command::Entitlement{command:EntitlementCommand::Create{input}}=>{
            let e:Entitlement=envelope::strict(&license_store::read_absolute(&input,16384,false)?,16384)?;
            println!("{}",license_server::repository::create(&mut db,&e,now())?);
        },
        Command::Entitlement{command:EntitlementCommand::Revoke{id}}=>license_server::repository::revoke(&mut db,&id,now())?,
        Command::Installation{command:InstallationCommand::Retire{id}}=>{
            let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let result=license_server::repository::retire(&tx,&id,now(),"admin")?;tx.commit()?;println!("{}",serde_json::to_string(&result)?);
        },
        Command::Installations{command:ListCommand::List{license_id}}=>{
            let mut query=db.prepare("SELECT installation_id,status,sequence,last_lease_valid_until,reserved_until,inventory_json FROM installations WHERE license_id=?1 ORDER BY installation_id")?;
            let rows=query.query_map([license_id],|r|Ok(serde_json::json!({"installation_id":r.get::<_,String>(0)?,"status":r.get::<_,String>(1)?,"sequence":r.get::<_,u64>(2)?,"lease_valid_until":r.get::<_,i64>(3)?,"reserved_until":r.get::<_,i64>(4)?,"inventory":serde_json::from_str::<serde_json::Value>(&r.get::<_,String>(5)?).unwrap_or_default()})))?;
            let rows=rows.collect::<std::result::Result<Vec<_>,_>>()?;println!("{}",serde_json::to_string(&rows)?);
        },
        Command::Keygen{..}=>unreachable!()
    }Ok(())
}
