#![forbid(unsafe_code)]
mod activation;
mod client;
mod config;
mod renewal;
use clap::{Parser, Subcommand};
use license_core::*;
use std::{io::BufRead, path::PathBuf};
#[derive(Parser)]
#[command(version, about = "Installation-scoped Linux license management")]
struct Args {
    #[arg(long, global = true, default_value = "/etc/license-guard/client.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Inspect {
        #[arg(long)]
        json: bool,
    },
    Activate {
        #[arg(long)]
        token_stdin: bool,
    },
    Renew {
        #[arg(long)]
        scheduled: bool,
    },
    Status {
        #[arg(long)]
        json: bool,
    },
    Verify {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Retire,
    Remove,
}
#[derive(Debug)]
pub struct Error {
    pub exit: i32,
    pub code: String,
}
impl Error {
    pub fn new(exit: i32, code: &str) -> Self {
        Self {
            exit,
            code: code.into(),
        }
    }
    pub fn config() -> Self {
        Self::new(64, "CONFIG_ERROR")
    }
}
impl From<Code> for Error {
    fn from(c: Code) -> Self {
        Self::new(
            if c == Code::InternalError {
                70
            } else if c == Code::IoError {
                75
            } else {
                78
            },
            &c.to_string(),
        )
    }
}
pub type Result<T> = std::result::Result<T, Error>;
fn main() {
    let args = Args::try_parse().unwrap_or_else(|e| {
        let _ = e.print();
        std::process::exit(if e.use_stderr() { 64 } else { 0 })
    });
    if let Err(e) = run(args) {
        eprintln!("{}", e.code);
        std::process::exit(e.exit);
    }
}
pub fn trust() -> Result<Trust> {
    #[cfg(feature = "dev")]
    if let Some(p) = std::env::var_os("LICENSE_GUARD_DEV_TRUST") {
        return Ok(Trust::new(envelope::strict(
            &license_store::read_absolute(&PathBuf::from(p), 16384, false)?,
            16384,
        )?)?);
    }
    Ok(Trust::compiled()?)
}
pub fn context(c: &config::Config, identity: Identity) -> Result<Context> {
    let now = now();
    let inventory = license_host::collect(&c.product, now)?;
    Ok(Context {
        now,
        product: c.product.clone(),
        required_features: vec![],
        identity,
        machine_id_hash: Some(inventory.machine_id_hash),
        system_uuid_hash: inventory.system_uuid_hash,
        logical_processors: inventory.cpu.logical_processors,
    })
}
fn status(c: &config::Config, file: Option<PathBuf>, json: bool) -> Result<()> {
    let now = now();
    let result = (|| {
        let dir = license_store::SecureDir::open(&c.state_dir)?;
        let identity: Identity =
            envelope::object(&dir.read("installation.json", 4096, false)?, 4096)?;
        let bytes = if let Some(p) = file {
            license_store::read_absolute(&p, 65536, false)?
        } else {
            dir.read("license.lic", 65536, false)?
        };
        let context = context(c, identity)?;
        Ok::<_, Error>(verify(&bytes, &trust()?, &context)?)
    })();
    let output = match result {
        Ok(v) => ValidationResult::accepted(&v, now),
        Err(e) => ValidationResult::denied(
            serde_json::from_value(serde_json::Value::String(e.code)).unwrap_or(Code::IoError),
            now,
        ),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&output).map_err(|_| Error::new(70, "SERIALIZATION"))?
        );
    } else {
        println!("{} valid={}", output.code, output.valid);
    }
    if output.valid {
        Ok(())
    } else {
        Err(output.code.into())
    }
}
fn run(args: Args) -> Result<()> {
    let c = config::Config::read(&args.config)?;
    match args.command {
        Command::Inspect { json: _ } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&license_host::collect(&c.product, now())?)
                    .map_err(|_| Error::config())?
            );
            Ok(())
        }
        Command::Status { json } => status(&c, None, json),
        Command::Verify { file, json } => status(&c, Some(file), json),
        command => {
            let dir = license_store::SecureDir::create(&c.state_dir)?;
            let _lock = dir.lock()?;
            match command {
                Command::Activate { token_stdin } => {
                    let token = if token_stdin {
                        let mut bytes = Vec::new();
                        std::io::stdin()
                            .lock()
                            .take(46)
                            .read_until(b'\n', &mut bytes)
                            .map_err(|_| Error::config())?;
                        if bytes.last() == Some(&b'\n') {
                            bytes.pop();
                            if bytes.last() == Some(&b'\r') {
                                bytes.pop();
                            }
                        }
                        String::from_utf8(bytes).map_err(|_| Error::config())?
                    } else {
                        rpassword::prompt_password("Activation token: ")
                            .map_err(|_| Error::config())?
                    };
                    let token = zeroize::Zeroizing::new(token);
                    envelope::fixed::<32>(&token)?;
                    activation::execute(&c, &dir, Action::Activate, Some(&token))
                }
                Command::Renew { scheduled } => renewal::renew(&c, &dir, scheduled),
                Command::Retire => activation::execute(&c, &dir, Action::Retire, None),
                Command::Remove => {
                    dir.remove("license.lic")?;
                    println!("LOCAL_LICENSE_REMOVED; server reservation unchanged");
                    Ok(())
                }
                _ => unreachable!(),
            }
        }
    }
}
use std::io::Read;
