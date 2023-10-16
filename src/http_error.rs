// Copied from https://github.com/stackabletech/opa-operator/blob/c74b6d9efedc8d5489ed4354cbb4687d4c914cac/rust/user-info-fetcher/src/http_error.rs
// NOTE TO SELF: break out into library?

use axum::{response::IntoResponse, Json};
use hyper::StatusCode;
use serde::Serialize;

pub trait Error: std::error::Error {
    fn status_code(&self) -> StatusCode;
}

pub struct JsonResponse<E> {
    error: E,
}

impl<E> From<E> for JsonResponse<E> {
    fn from(error: E) -> Self {
        Self { error }
    }
}

impl<E: Error> IntoResponse for JsonResponse<E> {
    fn into_response(self) -> axum::response::Response {
        (
            self.error.status_code(),
            Json(Container {
                error: Payload {
                    message: self.error.to_string(),
                    causes: std::iter::successors(self.error.source(), |err| err.source())
                        .map(|err| err.to_string())
                        .collect(),
                },
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Container {
    error: Payload,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    message: String,
    causes: Vec<String>,
}
