use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Filter {
    /// The feed alias this filter is associated with
    pub alias: String,
    /// Keywords used to filter the titles of the feed entries
    pub keywords: Vec<String>,
    /// The path to the script to execute on matching feed entries
    pub script_path: PathBuf,
    /// The last time the filter was updated. If it has never been updated, it will be None.
    pub last_updated: Option<DateTime<Utc>>,
}
