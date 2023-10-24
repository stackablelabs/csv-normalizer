use std::{borrow::Cow, char::TryFromCharError};

use crate::{config::Transform, http_error};

use super::AppState;

use axum::{
    extract::{Path, Query, State},
    http::HeaderValue,
    response::IntoResponse,
};
use csv::ByteRecord;
use hyper::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    StatusCode,
};
use serde::Deserialize;
use snafu::{OptionExt, ResultExt, Snafu};

static CONTENT_TYPE_CSV: HeaderValue = HeaderValue::from_static("text/csv");

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("no known resource named {resource:?}"))]
    NoResource { resource: String },
    #[snafu(display("failed to forward request to backend"))]
    BackendRequest { source: reqwest::Error },
    #[snafu(display("failed to retrieve requewst from backend"))]
    ReadResponse { source: reqwest::Error },
    #[snafu(display("invalid output delimiter requested"))]
    InvalidOutputDelimiter { source: TryFromCharError },
    #[snafu(display("failed to parse record"))]
    ParseRecord { source: csv::Error },
    #[snafu(display("failed to write record"))]
    WriteRecord { source: csv::Error },
    #[snafu(display("failed to flush records"))]
    FlushRecords { source: std::io::Error },
}
impl http_error::Error for Error {
    fn status_code(&self) -> hyper::StatusCode {
        match self {
            Error::NoResource { .. } => StatusCode::NOT_FOUND,
            Error::BackendRequest { .. } => StatusCode::BAD_GATEWAY,
            Error::ReadResponse { .. } => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Deserialize)]
pub struct Opts {
    #[serde(default = "Opts::default_delimiter")]
    delimiter: char,
}

impl Opts {
    fn default_delimiter() -> char {
        ','
    }
}

pub async fn get_resource(
    State(state): State<AppState>,
    Path(resource): Path<String>,
    Query(opts): Query<Opts>,
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
    let status = backend_response.status();
    let mut headers = backend_response.headers().clone();

    let in_bytes = backend_response.bytes().await.context(ReadResponseSnafu)?;
    // pass error responses through unchanged
    let output_bytes = if status.is_success() {
        headers.insert(CONTENT_TYPE, CONTENT_TYPE_CSV.clone());
        headers.remove(CONTENT_LENGTH);

        let mut transformers = resource_config
            .transforms
            .iter()
            .map(Transformer::from_transform)
            .collect::<Vec<_>>();

        let mut csv_r = csv::ReaderBuilder::new()
            .has_headers(false) // Each transformer manages its own headers
            .delimiter(resource_config.parser.field_separator as u8)
            .from_reader(in_bytes.as_ref());
        let mut csv_output_bytes = Vec::<u8>::new();
        let mut csv_w = csv::WriterBuilder::new()
            .delimiter(
                opts.delimiter
                    .try_into()
                    .context(InvalidOutputDelimiterSnafu)?,
            )
            .from_writer(&mut csv_output_bytes);
        let mut in_record = ByteRecord::new();
        while csv_r
            .read_byte_record(&mut in_record)
            .context(ParseRecordSnafu)?
        {
            let mut fields = in_record.iter().map(Cow::Borrowed).collect();
            for transformer in &mut transformers {
                transformer.process_record(&mut fields);
            }
            csv_w.write_record(&fields).context(WriteRecordSnafu)?;
        }
        csv_w.flush().context(FlushRecordsSnafu)?;
        drop(csv_w);
        csv_output_bytes.into_response()
    } else {
        in_bytes.into_response()
    };

    Ok((status, headers, output_bytes).into_response())
}

pub enum Transformer<'a> {
    RenameColumn {
        is_header_row: bool,
        from: &'a str,
        to: &'a str,
    },
}
impl<'a> Transformer<'a> {
    fn from_transform(transform: &'a Transform) -> Self {
        match transform {
            Transform::RenameColumn { from, to } => Self::RenameColumn {
                is_header_row: true,
                from,
                to,
            },
        }
    }

    fn process_record<'b>(&mut self, record: &mut Vec<Cow<'b, [u8]>>)
    where
        'a: 'b,
    {
        match self {
            Self::RenameColumn {
                is_header_row,
                from,
                to,
            } => {
                if *is_header_row {
                    for col in record {
                        if *col == from.as_bytes() {
                            *col = Cow::Borrowed(to.as_bytes());
                        }
                    }
                    *is_header_row = false;
                }
            }
        }
    }
}
