use std::collections::HashMap;

use serde::Deserialize;
use url::Url;

#[derive(Deserialize)]
pub struct Config {
    pub resources: HashMap<String, Resource>,
}

#[derive(Deserialize)]
pub struct Resource {
    pub backend: Url,
}
