use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::routing::get;
use clap::Parser;
use config::Config;
use snafu::{ResultExt, Snafu};

mod config;
mod http_error;
mod resources;

#[derive(clap::Parser)]
struct Args {
    #[clap(long, env)]
    addr: SocketAddr,
    #[clap(long, env)]
    config: PathBuf,
}

#[derive(Snafu, Debug)]
enum RunError {
    ReadConfig { source: std::io::Error },
    ParseConfig { source: serde_yaml::Error },
    StartServer { source: hyper::Error },
}

#[derive(Clone)]
pub struct AppState {
    config: Arc<Config>,
    http: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<(), RunError> {
    let args = Args::parse();
    let config = Arc::<Config>::new(
        serde_yaml::from_slice(
            &tokio::fs::read(args.config)
                .await
                .context(ReadConfigSnafu)?,
        )
        .context(ParseConfigSnafu)?,
    );

    let app = axum::Router::new()
        .route("/resource/:resource", get(resources::get_resource))
        .with_state(AppState {
            config,
            http: reqwest::Client::new(),
        });
    axum::Server::bind(&args.addr)
        .serve(app.into_make_service())
        .await
        .context(StartServerSnafu)?;

    Ok(())
}
