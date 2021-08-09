use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::named_params;
use url::Url;

use crate::db::{RSSActionsTx};
use crate::models::Feed;
use crate::models::Filter;


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

    }

    pub fn store_filter(&self, filter: &Filter) -> Result<()> {
        // Sort the filters list and then join with the "unit separator" ascii character into a
        // single string to serialize in the database.
        let mut sorted_keywords: Vec<String> = filter.keywords.to_vec();
        sorted_keywords.sort();

        let keywords = sorted_keywords.join("\x1F");
        let res = self.tx.execute(
            "INSERT INTO filters
             (feed_id, keywords, script_path, last_updated) VALUES
             ((SELECT id FROM feeds WHERE feeds.alias = :alias),
              :keywords, :script_path, :last_updated)",
            named_params!{":alias": &filter.alias, ":keywords": keywords,
                    ":script_path": &filter.script_path.to_string_lossy(), ":last_updated": &filter.last_updated})
            .with_context(|| format!("Failed to insert filter {:?} {:?} {:?} into db", &filter.alias, &keywords, &filter.script_path))
            .map(|_| ()); // ignore returned number of rows modified

        // TODO this is a hack but the alternative is to do another select and check explicitly
        // for each error message or perhaps check the rusqlite error type.
        // Add custom error messages for certain errors.
        if let Err(err) = res {
            // Check whether the constraint failed via the feed id select
            if err.chain().any(|e| e.to_string() == "NOT NULL constraint failed: filters.feed_id") {
                return Err(err).with_context(|| format!("Couldn't find a feed with alias {}.", filter.alias));
            }
            else if err.chain().any(|e| e.to_string() == "UNIQUE constraint failed: filters.feed_id, filters.keywords, filters.script_path") {
                return Err(err).context("You can't add another filter with the same feed alias, keyword, and script path.");
            }
            else {
                return Err(err);
            }
        }
        // otherwise return result as normal
        return res;
    }

    pub fn fetch_filters(&self) -> Result<Vec<Filter>> {
        let mut stmt = self.tx.prepare(
            "SELECT feeds.alias, filters.keywords, filters.script_path, filters.last_updated
             FROM filters
             LEFT JOIN feeds
             ON filters.feed_id = feeds.id
             ORDER BY filters.last_updated DESC")?;

        return stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .context("Failed to fetch filters from db")?
            .map(|res| {
                let (alias, keywords, script_path, last_updated): (String, String, String, Option<DateTime<Utc>>) =
                     res.context("Failed to read feed from db")?;

                let keywords: Vec<String> = keywords.split('\x1F').map(|s| s.into()).collect();
                let script_path = PathBuf::from(script_path);
                Ok(Filter {
                    alias,
                    keywords,
                    script_path,
                    last_updated
                })
            }).collect();

    }

    /// Update filter last_updated keyed on alias, keywords, and script path
    pub fn update_filter(&mut self, filter: &Filter) -> Result<()> {
        // Sort the filters list and then join with the "unit separator" ascii character into a
        // single string to serialize in the database.
        let mut sorted_keywords: Vec<String> = filter.keywords.to_vec();
        sorted_keywords.sort();

        let sp = self.tx.savepoint()?;
        let keywords = sorted_keywords.join("\x1F");
        let res = sp.execute(
            "UPDATE filters
            SET last_updated = :last_updated
            WHERE
                feed_id = (SELECT id FROM feeds WHERE feeds.alias = :alias) AND
                keywords = :keywords AND
                script_path = :script_path",
            named_params!{":alias": &filter.alias, ":keywords": keywords,
                    ":script_path": &filter.script_path.to_string_lossy(), ":last_updated": &filter.last_updated})
            .with_context(|| format!("Failed to update filter {:?} {:?} {:?} in db with new time {:?}",
                    &filter.alias, &keywords, &filter.script_path, &filter.last_updated));

        // Add custom error messages for certain errors.
        match res {
            Ok(count) => {
                if count == 0 {
                    return Err(anyhow!("No filter was found to update that matches {:?}", filter));
                }
                else if count > 1 {
                    return Err(anyhow!("More than one filter was updated when updating {:?}", filter));
                }
            }
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("A database error occurred updating a filter {:?}", filter));
            }
        }

        // If no error, commit subtransaction and return result as normal, ignoring count
        sp.commit()
            .with_context(|| format!("Failed to commit subtransaction savepoint updating filter {:?}", filter))?;
        return res.map(|_| ());
    }
}
