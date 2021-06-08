use anyhow::{Context, Result};
//use chrono::{DateTime, Utc};
use rusqlite::named_params;
use url::Url;

use crate::db::{RSSActionsTx};
use crate::models::Feed;
//use crate::models::Filter;


impl<'conn> RSSActionsTx<'conn> {
    pub fn store_feed(&self, alias: &str, url: &Url) -> Result<()> {
        self.tx.execute(
            "INSERT INTO feeds
              (url, alias) VALUES (:url, :alias)",
            named_params!{":url": url, ":alias": alias})
            .with_context(|| format!("Failed to insert feed {} {} into db", alias, url))
            .map(|_| ()) // ignore returned number of rows modified
    }

    pub fn fetch_feeds(&self) -> Result<Vec<Feed>> {
        let mut stmt = self.tx.prepare("SELECT url, alias FROM feeds")?;
        //let rows: Vec<Result<Feed>> = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        
        return stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .context("Failed to fetch feeds from db")?
            .map(|res| {
                let (url, alias): (String, String) = res.context("Failed to read feed from db")?;
                let url = Url::parse(&url)
                    .with_context(|| format!("Failed to parse feed {} url from database", alias))?;
                Ok(Feed {
                    url,
                    alias
                })
            }).collect();

        //rows.into_iter().collect()
    }

    // pub fn store_filter(&self, filter: &Filter) -> Result<()> {
    //     self.tx.execute(
    //         "INSERT INTO filters
    //           (alias) VALUES (:url, :alias, :last_updated)",
    //         named_params!{":url": feed.url, ":alias": feed.alias, ":last_updated": feed.last_updated})
    //         .with_context(|| format!("Failed to insert feed {:?} into db", &feed))
    //         .map(|_| ()) // ignore returned number of rows modified
    // }
}
