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
    #[snafu(display("unable to read config file"))]
    ReadConfig { source: std::io::Error },
    #[snafu(display("unable to parse config file"))]
    ParseConfig { source: serde_yaml::Error },
    #[snafu(display("unable to start HTTP server"))]
    StartServer { source: hyper::Error },
}

#[derive(Clone)]
pub struct AppState {
    config: Arc<Config>,
    http: reqwest::Client,
}

#[snafu::report]
#[tokio::main]
async fn main() -> Result<(), RunError> {
    let args = Args::parse();
    let config = Arc::<Config>::new(
        serde_yaml::with::singleton_map_recursive::deserialize(
            serde_yaml::Deserializer::from_slice(
                &tokio::fs::read(args.config)
                    .await
                    .context(ReadConfigSnafu)?,
            ),
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
