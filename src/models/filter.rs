use std::os::unix::fs::MetadataExt;

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Filter {
    /// The feed alias this filter is associated with.
    pub alias: String,
    /// Keywords used to filter the titles of the feed entries.
    pub keywords: Vec<String>,
    /// The path to the script to execute on matching feed entries.
    pub script_path: PathBuf,
    /// The last time the filter was updated. If it has never been updated, it will be None.
    pub last_updated: Option<DateTime<Utc>>,
}

impl Filter {
    pub fn new(alias: &str, keywords: Vec<String>, script_path: PathBuf) -> Result<Filter> {
        if !script_path.is_file() {
            return Err(anyhow!("The filter's script path is not a file: {}", script_path.to_string_lossy()));
        }

        let script_metadata = std::fs::metadata(&script_path)
            .with_context(|| format!("Failed to read file metadata: {:?}", script_path))?;

        // Test if the executable bit is set on user, group, or other permissions for the file.
        if (script_metadata.mode() & 0o111) == 0 {
            return Err(anyhow!("The filter's script path is not executable: {}", script_path.to_string_lossy()));
        }

        // decided not to implement this because it's not really worth the effort but leaving it
        // commented for future reference.
        //
        // If the script is executable by the owner or (TODO) the group but we're not that user or
        // in that group (again, group TODO), report an error since we won't be able to execute the
        // script.
        // https://stackoverflow.com/questions/57951893/how-to-determine-the-effective-user-id-of-a-process-in-rust
        // use the `uid` of the current process to determine the current user
        // let current_process_metadata = std::fs::metadata("/proc/self")
        //     .with_context(|| format!("Failed to read file metadata: {:?}", script_path))?;

        Ok(Filter {
            alias: alias.into(),
            keywords,
            script_path,
            last_updated: None
        })
    }

    pub fn update_time(&mut self, update_time: DateTime<Utc>) {
        self.last_updated = Some(update_time);
    }
}
