use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::routing::get;
use clap::Parser;
use config::Config;
use hyper::server::conn::AddrIncoming;
use hyper_rustls::TlsAcceptor;
use snafu::{OptionExt, ResultExt, Snafu};

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
    #[snafu(display("unable to read TLS certificate"))]
    ReadTlsCert { source: std::io::Error },
    #[snafu(display("unable to parse TLS certificate"))]
    ParseTlsCert { source: std::io::Error },
    #[snafu(display("unable to read TLS key"))]
    ReadTlsKey { source: std::io::Error },
    #[snafu(display("unable to parse TLS key"))]
    ParseTlsKey { source: std::io::Error },
    #[snafu(display("TLS keyfile contained no private key"))]
    NoTlsKey,
    #[snafu(display("TLS keyfile contained no valid private key record"))]
    InvalidRecordType,
    #[snafu(display("unable to initialize TLS acceptor"))]
    InitTlsAcceptor { source: rustls::Error },
    #[snafu(display("unable to bind socket"))]
    BindServer { source: hyper::Error },
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
            config: config.clone(),
            http: reqwest::Client::new(),
        });
    let incoming = AddrIncoming::bind(&args.addr).context(BindServerSnafu)?;
    if let Some(tls) = &config.tls {
        axum::Server::builder(load_tls(tls, incoming).await?)
            .serve(app.into_make_service())
            .await
    } else {
        axum::Server::builder(incoming)
            .serve(app.into_make_service())
            .await
    }
    .context(StartServerSnafu)?;

    Ok(())
}

async fn load_tls(tls: &config::Tls, incoming: AddrIncoming) -> Result<TlsAcceptor, RunError> {
    let certs = tokio::fs::read(&tls.cert).await.context(ReadTlsCertSnafu)?;
    let certs = rustls_pemfile::certs(&mut certs.as_ref()).context(ParseTlsCertSnafu)?;
    let certs = certs.into_iter().map(rustls::Certificate).collect();

    let keys = tokio::fs::read(&tls.private_key)
        .await
        .context(ReadTlsKeySnafu)?;
    let keys = rustls_pemfile::read_one(&mut keys.as_ref()).context(ParseTlsKeySnafu)?;
    let key = rustls::PrivateKey(match keys.context(NoTlsKeySnafu)? {
        rustls_pemfile::Item::RSAKey(key) => key,
        rustls_pemfile::Item::PKCS8Key(key) => key,
        rustls_pemfile::Item::ECKey(key) => key,
        _ => return InvalidRecordTypeSnafu.fail(),
    });
    // let keys = rustls_pemfile::rsa_private_keys(&mut keys.as_ref()).context(ParseTlsKeySnafu)?;
    // let key = rustls::PrivateKey(keys.into_iter().next().context(NoTlsKeySnafu)?);

    Ok(TlsAcceptor::builder()
        .with_single_cert(certs, key)
        .context(InitTlsAcceptorSnafu)?
        .with_all_versions_alpn()
        .with_incoming(incoming))
}
