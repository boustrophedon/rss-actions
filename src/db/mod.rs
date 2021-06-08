use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, Transaction};

mod create;
mod transaction;

/// A DB connection. Opens connection to local sqlite database.
#[derive(Debug)]
pub struct RSSActionsDb {
    connection: Connection,
}

/// A DB transaction. Has all DB CRUD operations as methods.
#[derive(Debug)]
pub struct RSSActionsTx<'conn> {
    tx: Transaction<'conn>,
}

impl RSSActionsDb {
    pub fn transaction(&mut self) -> Result<RSSActionsTx> {
        let transaction = self.connection.transaction()?;
        let tx = RSSActionsTx {
            tx: transaction,
        };

        Ok(tx)
    }

    pub fn open(db_path: &Path) -> Result<RSSActionsDb> {
        // check for existing db before calling sqlite's open because it creates the file.
        let existing_db = db_path.is_file();
        let connection = Connection::open(db_path)?;

        let mut db = RSSActionsDb {
            connection
        };

        // If the db is new, create the tables.
        if !existing_db {
            let create_tx = db.transaction()?;
            create_tx.create_tables()
                .context("failed to create db tables in memory")?;
            create_tx.commit()?;
        }


        Ok(db)
    }

    #[cfg(test)]
    /// Open an in-memory db and create tables. cfg(test) only.
    pub fn open_in_memory() -> Result<RSSActionsDb> {
        let connection = Connection::open_in_memory()?;
        let mut db = RSSActionsDb {
            connection,
        };

        let create_tx = db.transaction()?;
        create_tx.create_tables()
            .context("failed to create db tables in memory")?;

        create_tx.commit()?;
        Ok(db)
    }
}

impl<'conn> RSSActionsTx<'conn> {
    pub fn commit(self) -> Result<()> {
        let tx = self.tx;
        tx.commit()
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
#[macro_use]
pub(crate) mod tests;
