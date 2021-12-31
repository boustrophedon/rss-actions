use anyhow::Result;
use chrono::*;

use crate::{Feed, Filter};
use crate::update::ProcessOutput; // (String, String, ExitStatus)

#[derive(Debug)]
pub struct ListFeedsOutput {
    pub feeds: Vec<Feed>,
}

#[derive(Debug)]
pub struct ListFiltersOutput {
    pub filters: Vec<Filter>,
}

#[derive(Debug)]
pub struct AddFeedOutput(pub Feed);

#[derive(Debug)]
pub struct AddFilterOutput(pub Filter);

#[derive(Debug)]
pub struct DeleteFilterOutput(pub String, pub Vec<String>);

#[derive(Debug)]
pub struct DeleteFeedOutput(pub String);

#[derive(Debug)]
pub struct UpdateOutput {
    /// Feeds that fail to download or parse have their filters skipped but are reported with Errors.
    pub executed_feeds: Vec<(Feed, Result<()>)>,
    /// Filters with scripts that fail to execute on at least one of the feed's entries are
    /// reported with Errors.
    pub executed_filters: Vec<(Filter, Result<Vec<ProcessOutput>>)>,
    pub successes: usize,
    pub failures: usize,
    pub updates: usize,
}

pub trait ConsoleOutput {
    fn output(&self) -> Vec<String>;
}

impl ConsoleOutput for ListFeedsOutput {
    fn output(&self) -> Vec<String> {
        let feeds = &self.feeds;

        if feeds.is_empty() {
            return vec!["No feeds in database.".into()];
        }

        let mut output: Vec<String> = vec![
        "Current feeds:".into(),
        "".into()];

        for feed in feeds {
            output.push(format!("{}\t{}", feed.alias, feed.url));
        }

        output
    }
}

impl ConsoleOutput for ListFiltersOutput {
    fn output(&self) -> Vec<String> {
        if self.filters.is_empty() {
            return vec!["No filters in database.".into()];
        }

        let mut output: Vec<String> = vec![
            "Current filters:".into(),
            "".into()];

        for filter in &self.filters {
            let last_updated = match filter.last_updated {
                Some(utc_dt) => {
                    let local_dt: DateTime<Local> = utc_dt.into();
                    local_dt.to_string()
                }
                None => { "Never updated".into() }
            };

            let keywords = filter.keywords.join(", ");
            let script = filter.script_path.file_name().map_or("".into(), |s| s.to_string_lossy());

            output.push(format!("{}\t{}\t{}\t{}", filter.alias, keywords, script, last_updated));
        }

        output
    }
}

impl ConsoleOutput for AddFeedOutput {
    fn output(&self) -> Vec<String> {
        vec![format!("Successfully added feed {}", self.0.alias)]
    }
}

impl ConsoleOutput for AddFilterOutput {
    fn output(&self) -> Vec<String> {
        let filter = &self.0;
        vec![format!("Successfully added filter on feed {}", filter.alias),
             format!("Keywords: {}", filter.keywords.join(", "))]
    }
}

impl ConsoleOutput for UpdateOutput {
    fn output(&self) -> Vec<String> {
        if self.executed_filters.is_empty() {
            return vec!["No filters in the database to update.".into()];
        }
        let mut output = Vec::new();

        output.push(format!("{} filters processed successfully.", self.successes));
        output.push(format!("{} filters updated.", self.updates));
        output.push(format!("{} filters failed to process.", self.failures));

        let mut errors = Vec::new();
        for (_, res) in &self.executed_feeds {
            if let Err(err) = res {
                //errors.push(err.to_string());
                errors.push(format!("{:?}", err));
            }
        }
        for (_, res) in &self.executed_filters {
            if let Err(err) = res {
                //errors.push(err.to_string());
                errors.push(format!("{:?}", err));
            }
        }

        output.extend(errors);

        output
    }
}

impl ConsoleOutput for DeleteFilterOutput {
    fn output(&self) -> Vec<String> {
        let alias = &self.0;
        let keywords = &self.1;
        vec![format!("Successfully deleted filter on feed {}", alias),
             format!("Keywords: {}", keywords.join(", "))]
    }
}

impl ConsoleOutput for DeleteFeedOutput {
    fn output(&self) -> Vec<String> {
        let alias = &self.0;
        vec![format!("Successfully deleted feed {}", alias)]
    }
}
