use anyhow::{anyhow, Result};
use url::Url;

#[derive(Debug, Clone)]
pub struct Feed {
    /// The feed's URL.
    pub url: Url,
    /// The user-chosen alias for the feed. Must not be empty.
    pub alias: String,
}

impl Feed {
    pub fn new(url: Url, alias: &str) -> Result<Feed> {
        if alias.is_empty() {
            return Err(anyhow!("A feed's alias must not be empty. {}", url));
        }
        Ok(Feed {
            url,
            alias: alias.into(),
        })
    }
}
