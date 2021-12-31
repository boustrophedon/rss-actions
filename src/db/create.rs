use anyhow::{Context, Result};

use crate::db::RSSActionsTx;

impl<'conn> RSSActionsTx<'conn> {
    /// Create the tables of the database.
    pub fn create_tables(&self) -> Result<()> {
        self.tx.execute(
            "CREATE TABLE feeds (
                id INTEGER PRIMARY KEY,
                url TEXT NOT NULL,
                alias TEXT NOT NULL UNIQUE
            )", []).context("failed to create feeds table")?;
        self.tx.execute(
            "CREATE TABLE filters (
                id INTEGER PRIMARY KEY,
                feed_id INTEGER NOT NULL,
                keywords TEXT NOT NULL,
                script_path TEXT NOT NULL,
                last_updated TEXT,
                FOREIGN KEY (feed_id) REFERENCES feeds(id),
                UNIQUE(feed_id,keywords,script_path)
            )", []).context("failed to create filters table")?;

        Ok(())
    }
}
