use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Filter {
    /// The feed alias this filter is associated with
    pub alias: String,
    /// Space-separated keywords to filter the feed by
    pub keywords: String,
    /// The path to the script to execute on matching feed entries
    pub script_path: PathBuf,
    /// The last time the filter was updated. If it has not been updated, it will be None.
    pub last_updated: Option<DateTime<Utc>>,
}
