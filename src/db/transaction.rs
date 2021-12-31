use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::named_params;
use url::Url;

use crate::db::{RSSActionsTx};
use crate::models::Feed;
use crate::models::Filter;

struct FilterId(pub usize);

/// Sort the filters list and then join with two "unit separator" (code 1F) ascii characters into a
/// single string to serialize in the database.
fn encode_filter_keywords(keywords: &[String]) -> String {
    let mut sorted_keywords: Vec<String> = keywords.to_vec();
    sorted_keywords.sort();

    sorted_keywords.into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\x1F")
}

/// Deserialize from `encode_filter_keywords`.
fn decode_filter_keywords(keywords_packed: &str) -> Vec<String> {
    keywords_packed
        .split('\x1F')
        .filter(|s| !s.is_empty())
        .map(|s| s.into()).collect()
}


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
        let keywords = encode_filter_keywords(&filter.keywords);

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
        Ok(self.fetch_filters_with_ids()?
            .into_iter().map(|(_db_id, filter)| filter)
            .collect())
    }

    fn fetch_filters_with_ids(&self) -> Result<Vec<(FilterId, Filter)>> {
        let mut stmt = self.tx.prepare(
            "SELECT filters.id, feeds.alias, filters.keywords, filters.script_path, filters.last_updated
             FROM filters
             LEFT JOIN feeds
             ON filters.feed_id = feeds.id
             ORDER BY filters.last_updated DESC")?;

        return stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))
            .context("Failed to fetch filters from db")?
            .map(|res| {
                let (filter_id, alias, keywords, script_path, last_updated): (usize, String, String, String, Option<DateTime<Utc>>) =
                     res.context("Failed to read feed from db")?;

                let keywords = decode_filter_keywords(&keywords);
                let script_path = PathBuf::from(script_path);
                Ok((FilterId(filter_id), Filter {
                    alias,
                    keywords,
                    script_path,
                    last_updated
                }))
            }).collect();

    }

    /// Update filter last_updated keyed on alias, keywords, and script path
    pub fn update_filter(&mut self, filter: &Filter) -> Result<()> {
        let keywords = encode_filter_keywords(&filter.keywords);

        let sp = self.tx.savepoint()?;
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

    // TODO return the filter deleted as read from the database.
    // TODO check whether the feed exists and return a different error in that case
    pub fn delete_filter(&mut self, alias: &str, keywords: &[String]) -> Result<()> {
        let filters = self.fetch_filters_with_ids()?;

        // NOTE: originally I wanted to do this in the db with a `LIKE %XkeyX%XwordX%` kind of
        // query but that doesn't work because you can't put a % in a parameter and have it act as
        // a wildcard (because then users could insert them). So the options are either insert the
        // parameter into the query string with normal string formatting and enable sql injections
        // or do it like this.
        //
        // Another option is generating the query string with a bunch of
        // ```
        // keywords LIKE %Xkeyword1X%
        // OR keywords LIKE %Xkeyword2X%
        // OR ...
        // ```
        //
        // but that's more complicated and error-prone and it's a wash whether it's actually faster
        // to do that (it probably is but you'd want to measure at that point).

        let mut matching_filters = Vec::new();
        for (id, filter) in filters {
            // all user-given keywords are in the filter we're checking
            let all_keywords_match = keywords.iter().all(|k| filter.keywords.contains(k));

            if filter.alias == alias && all_keywords_match {
                matching_filters.push((id, filter));
            }
        }

        if matching_filters.len() == 0 {
            return Err(anyhow!("No filters matching `{}` on the feed `{}` were found in the database.",
                    &keywords.join(","), &alias));
        }
        else if matching_filters.len() > 1 {
            return Err(anyhow!("Multiple filters matching `{}` on the feed `{}` were found in the database.",
                    &keywords.join(","), &alias));
        }

        let single_matching_filter = matching_filters.pop().expect("checked for 0 and >1");

        self.tx.execute(
            "DELETE FROM filters
            WHERE
                id = :filter_id",
            named_params!{":filter_id": &single_matching_filter.0.0})
            // .with_context(|| format!("Failed to filter alias {} keywords {:?}",
            //         &alias, &keywords));
            .with_context(|| format!("A database error occurred deleting filter with keywords {:?} on feed {}",
                    &keywords, &alias))
            .map(|_| ())
    }

    pub fn delete_feed(&mut self, alias: &str) -> Result<()> {
        self.tx.pragma_update(None, "foreign_keys", &true)
            .context("failed to enable foreign keys pragma")?;
        let res = self.tx.execute(
            "DELETE FROM feeds
            WHERE
                alias = :alias",
            named_params!{":alias": &alias,})
            .with_context(|| format!("Failed to delete feed {} in db",
                    &alias));

        // Add custom error messages for certain errors.
        match res {
            Ok(count) => {
                if count == 0 {
                    return Err(anyhow!("No feed was found to delete that matches name `{}`", alias));
                }
                else if count > 1 {
                    return Err(anyhow!("More than one feed was found when trying to delete `{}`", alias));
                }
            }
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("A database error occurred deleting feed `{}`", alias));
            }
        }

        Ok(())
    }
}
