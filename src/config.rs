use std::{collections::HashMap, path::PathBuf};

use derivative::Derivative;
use serde::Deserialize;
use url::Url;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub tls: Option<Tls>,
    pub resources: HashMap<String, Resource>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tls {
    pub cert: PathBuf,
    pub private_key: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub backend: Url,
    #[serde(default)]
    pub parser: CsvParser,
    #[serde(default)]
    pub transforms: Vec<Transform>,
}

#[derive(Deserialize, Derivative)]
#[derivative(Default)]
#[serde(rename_all = "camelCase")]
pub struct CsvParser {
    #[serde(default = "default_field_separator")]
    #[derivative(Default(value = "default_field_separator()"))]
    pub field_separator: char,
}

#[derive(Deserialize)]
pub enum Transform {
    #[serde(rename_all = "camelCase")]
    RenameColumn { from: String, to: String },
}

fn default_field_separator() -> char {
    ';'
}
