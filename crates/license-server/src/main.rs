#![forbid(unsafe_code)]
use clap::Parser;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    config: std::path::PathBuf,
}
#[tokio::main]
async fn main() {
    if run().await.is_err() {
        eprintln!("SERVER_START_FAILED");
        std::process::exit(70);
    }
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let a = Args::parse();
    let c = license_server::config::Config::read(&a.config)?;
    let issuer = license_server::issuer::Issuer::load(
        c.issuer_key_id.clone(),
        std::path::Path::new(&c.issuer_key_path),
    )?;
    let db = license_server::repository::open(
        std::path::Path::new(&c.database_path),
        c.sqlite_busy_timeout_milliseconds,
    )?;
    let listener = tokio::net::TcpListener::bind(&c.listen).await?;
    println!("LICENSE_SERVER_READY {}", listener.local_addr()?);
    let app = license_server::routes::App::new(db, issuer, c);
    axum::serve(
        listener,
        license_server::routes::router(app)
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
