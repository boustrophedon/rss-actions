use anyhow::{Context, Result};

use crate::db::RSSActionsTx;

impl<'conn> RSSActionsTx<'conn> {
    /// Create the tables of the database.
    pub fn create_tables(&self) -> Result<()> {
        self.tx.execute("PRAGMA foreign_keys = ON;", [])
            .context("failed to enable foreign keys pragma")?;
        self.tx.execute(
            "CREATE TABLE feeds (
                id INTEGER PRIMARY KEY,
                url TEXT NOT NULL,
                alias TEXT NOT NULL UNIQUE
            )", []).context("failed to create feeds table")?;
        self.tx.execute(
            "CREATE TABLE filters (
                url_id INTEGER NOT NULL,
                filter TEXT NOT NULL,
                last_updated TEXT,
                action_path TEXT NOT NULL,
                FOREIGN KEY (url_id) REFERENCES feeds(id)
            )", []).context("failed to create filters table")?;

        Ok(())
    }
}
