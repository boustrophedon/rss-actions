use anyhow::Result;
use chrono::*;

use crate::db::{RSSActionsDb, RSSActionsTx};
use crate::models::Feed;
use crate::models::Filter;
use crate::config::Config;


pub enum RSSActionCmd {
    ListFeeds,
    ListFilters,
    AddFeed(Feed),
    /// Feed alias, list of filter keywords, script path
    AddFilter(Filter),
}

impl RSSActionCmd {
    pub fn execute(self, cfg: &Config) -> Result<Vec<String>> {
        let mut db = RSSActionsDb::open(&cfg.db_path)?;
        let tx = db.transaction()?;

        let result = match self {
            RSSActionCmd::ListFeeds => RSSActionCmd::list_feeds(&tx),
            RSSActionCmd::ListFilters => RSSActionCmd::list_filters(&tx),
            RSSActionCmd::AddFeed(feed) => RSSActionCmd::add_feed(&tx, feed),
            RSSActionCmd::AddFilter(filter) =>
                RSSActionCmd::add_filter(&tx, filter),
        };

        if result.is_ok() {
            tx.commit()?;
        }

        result
    }

    fn list_feeds(tx: &RSSActionsTx) -> Result<Vec<String>> {
        let results = tx.fetch_feeds()?;
       
        if results.is_empty() {
            return Ok(vec!["No feeds in database.".into()]);
        }

        let mut output: Vec<String> = Vec::new();
        output.push("Current feeds:".into());
        output.push("".into());

        for feed in results {
            output.push(format!("{}\t{}", feed.alias, feed.url));
        }

        Ok(output)
    }

    fn add_feed(tx: &RSSActionsTx, feed: Feed) -> Result<Vec<String>> {
        tx.store_feed(&feed.alias, &feed.url)?;
    
        Ok(vec![format!("Successfully added feed {}", feed.alias)])
    }

    fn list_filters(tx: &RSSActionsTx) -> Result<Vec<String>> {
        let results = tx.fetch_filters()?;

        if results.is_empty() {
            return Ok(vec!["No filters in database.".into()]);
        }

        let mut output: Vec<String> = Vec::new();
        output.push("Current filters:".into());
        output.push("".into());

        for filter in results {
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

        Ok(output)
    }

    fn add_filter(tx: &RSSActionsTx, filter: Filter)
         -> Result<Vec<String>> {
        tx.store_filter(&filter)?;

        Ok(vec![format!("Successfully added filter on feed {}", filter.alias),
                format!("Keywords: {}", filter.keywords.join(", "))])
    }
}
