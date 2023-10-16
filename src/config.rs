use std::collections::HashMap;

use serde::Deserialize;
use url::Url;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub resources: HashMap<String, Resource>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub backend: Url,
    pub transforms: Vec<Transform>,
}

#[derive(Deserialize)]
pub enum Transform {
    #[serde(rename_all = "camelCase")]
    RenameColumn { from: String, to: String },
}
