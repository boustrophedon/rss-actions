use anyhow::Result;

use crate::db::{RSSActionsDb, RSSActionsTx};
use crate::config::Config;

pub mod inputs;
pub mod outputs;
pub use inputs::*;
pub use outputs::*;


pub trait RSSActionCmd {
    type CmdOutput;

    /// Executes the command, opening the database and returning the output details object of the
    /// executed command.
    fn execute(&self, cfg: &Config) -> Result<Self::CmdOutput> {
        let mut db = RSSActionsDb::open(&cfg.db_path)?;
        let mut tx = db.transaction()?;

        let result = self.action(&mut tx);

        if result.is_ok() {
            tx.commit()?;
        }

        result
    }

    fn action(&self, tx: &mut RSSActionsTx) -> Result<Self::CmdOutput>;
}

impl RSSActionCmd for ListFeedsCmd {
    type CmdOutput = ListFeedsOutput;
    fn action(&self, tx: &mut RSSActionsTx) -> Result<ListFeedsOutput> {
        let feeds = tx.fetch_feeds()?;

        Ok(ListFeedsOutput { feeds })
    }
}

impl RSSActionCmd for AddFeedCmd {
    type CmdOutput = AddFeedOutput;
    fn action(&self, tx: &mut RSSActionsTx) -> Result<AddFeedOutput> {
        let feed = &self.0;
        tx.store_feed(&feed.alias, &feed.url)?;

        Ok(AddFeedOutput(feed.clone()))
    }
}

impl RSSActionCmd for ListFiltersCmd {
    type CmdOutput = ListFiltersOutput;
    fn action(&self, tx: &mut RSSActionsTx) -> Result<ListFiltersOutput> {
        let filters = tx.fetch_filters()?;

        Ok(ListFiltersOutput { filters })
    }
}

impl RSSActionCmd for AddFilterCmd {
    type CmdOutput = AddFilterOutput;
    fn action(&self, tx: &mut RSSActionsTx) -> Result<AddFilterOutput> {
        let filter = &self.0;
        tx.store_filter(filter)?;

        Ok(AddFilterOutput(filter.clone()))
    }
}

impl RSSActionCmd for UpdateCmd {
    type CmdOutput = UpdateOutput;
    fn action(&self, tx: &mut RSSActionsTx) -> Result<UpdateOutput> {
        crate::update::update(tx)
    }
}
