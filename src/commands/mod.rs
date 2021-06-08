use anyhow::Result;
use crate::db::{RSSActionsDb, RSSActionsTx};
use crate::models::Feed;
use crate::config::Config;

pub enum RSSActionCmd {
    ListFeeds,
    AddFeed(Feed),
}

impl RSSActionCmd {
    pub fn execute(self, cfg: &Config) -> Result<Vec<String>> {
        let mut db = RSSActionsDb::open(&cfg.db_path)?;
        let tx = db.transaction()?;

        let result = match self {
            RSSActionCmd::ListFeeds => RSSActionCmd::list(&tx),
            RSSActionCmd::AddFeed(feed) => RSSActionCmd::add(&tx, feed),
        };

        if result.is_ok() {
            tx.commit()?;
        }

        result
    }

    fn list(tx: &RSSActionsTx) -> Result<Vec<String>> {
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

    fn add(tx: &RSSActionsTx, feed: Feed) -> Result<Vec<String>> {
        tx.store_feed(&feed.alias, &feed.url)?;
    
        Ok(vec![format!("Successfully added feed {}", feed.alias)])
    }
}
