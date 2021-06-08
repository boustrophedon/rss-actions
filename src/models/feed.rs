use url::Url;

#[derive(Debug, Clone)]
pub struct Feed {
    /// The feed's URL.
    pub url: Url,
    /// The user-chosen alias for the feed.
    pub alias: String,
}

impl Feed {
    pub fn new(url: Url, alias: &str) -> Feed {
        Feed {
            url,
            alias: alias.into(),
        }
    }
}
