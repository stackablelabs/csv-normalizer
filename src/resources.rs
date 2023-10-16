use crate::http_error;

use super::AppState;

use axum::{
    extract::{Path, State},
    http::HeaderValue,
    response::IntoResponse,
};
use hyper::{header::CONTENT_TYPE, StatusCode};
use snafu::{OptionExt, ResultExt, Snafu};

static CONTENT_TYPE_BINARY: HeaderValue = HeaderValue::from_static("application/octet-stream");
static CONTENT_TYPE_CSV: HeaderValue = HeaderValue::from_static("text/csv");

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("no known resource named {resource:?}"))]
    NoResource { resource: String },
    #[snafu(display("failed to forward request to backend"))]
    BackendRequest { source: reqwest::Error },
}
impl http_error::Error for Error {
    fn status_code(&self) -> hyper::StatusCode {
        match self {
            Error::NoResource { .. } => StatusCode::NOT_FOUND,
            Error::BackendRequest { .. } => StatusCode::BAD_GATEWAY,
        }
    }
}

pub async fn get_resource(
    State(state): State<AppState>,
    Path(resource): Path<String>,
) -> Result<axum::response::Response, http_error::JsonResponse<Error>> {
    let resource_config = state
        .config
        .resources
        .get(&resource)
        .context(NoResourceSnafu { resource })?;
    let backend_response = state
        .http
        .get(resource_config.backend.clone())
        .send()
        .await
        .context(BackendRequestSnafu)?;
    let mut headers = backend_response.headers().clone();
    match headers.entry(CONTENT_TYPE) {
        hyper::header::Entry::Occupied(mut entry) if entry.get() == CONTENT_TYPE_BINARY => {
            entry.insert(CONTENT_TYPE_CSV.clone());
        }
        hyper::header::Entry::Vacant(entry) => {
            entry.insert(CONTENT_TYPE_CSV.clone());
        }
        _ => {}
    }
    Ok((
        backend_response.status(),
        headers,
        backend_response.bytes().await.unwrap(),
    )
        .into_response())
}
