use anyhow::Result;
use structopt::StructOpt;
use url::Url;

use std::path::{Path, PathBuf};

use crate::Config;
use crate::{ConsoleOutput, RSSActionCmd};
use crate::{Feed, Filter};

// NB This is basically an adaptor that takes f: A->B and g: B->C
// (where A is the input, B is the output, and C is the Vec<String>)
// and fixes up the types
//
// if we tried to just box the original RSSActionCmd (below, in `to_cmd()`) it would fail because
// of the associated type

// I'm still not 100% sure about this design vs doing the execution inside the `match` block in
// `to_cmd` (and renaming it `execute_cmd` and just returning the output), since it still
// forces us to take `&self` in the traits instead of `self` in exchange for being able to have
// separate `to_cmd` and `execute` steps.

pub trait RSSActionCLICmd {
    /// Executes the command as in `execute` and just returns the output from the `ConsoleOutput`
    /// trait i.e. a list of strings.
    fn execute_console(&self, cfg: &Config) -> Result<Vec<String>>;
}

impl<T: RSSActionCmd> RSSActionCLICmd for T
    where <T as RSSActionCmd>::CmdOutput: ConsoleOutput {
    fn execute_console(&self, cfg: &Config) -> Result<Vec<String>> {
        let output = self.execute(cfg)?;
        Ok(ConsoleOutput::output(&output))
    }
}


#[derive(Debug, StructOpt)]
pub struct RSSActionsArgs {
    #[structopt(subcommand)]
    cmd: SubArg,

    #[structopt(short = "c", long = "config")]
    /// Override the default configuration directory.
    config_dir: Option<String>,
}

#[derive(Debug, StructOpt)]
enum SubArg {
    #[structopt(name = "add")]
    /// Add a feed or filter to the database
    Add(AddArg),

    #[structopt(name = "delete")]
    /// Add a feed or filter to the database
    Delete(DeleteArg),

    #[structopt(name = "list")]
    /// Display feeds or filters
    List(ListArg),

    #[structopt(name = "update")]
    /// Run update, downloading feeds and matching against filters, running scripts that match
    Update,
}

// -- Add

#[derive(Debug, StructOpt)]
struct AddArg {
    /// Add a feed or filter to the database.
    #[structopt(subcommand)]
    pub cmd: AddSubArg,
}

#[derive(Debug, StructOpt)]
enum AddSubArg {
    #[structopt(name = "feed")]
    /// Add a feed to the database
    Feed(AddFeed),

    #[structopt(name = "filter")]
    /// Add a filter to the database
    Filter(AddFilter)
}

#[derive(Debug, StructOpt)]
struct AddFeed {
    /// The name used to refer to this feed
    pub alias: String,
    /// The url for this feed
    pub url: String,
}

#[derive(Debug, StructOpt)]
struct AddFilter {
    /// The alias of the feed to filter
    pub alias: String,
    /// The path to the script to run on the matched entries
    pub script_path: String,
    /// The keywords to filter the entries with
    pub keywords: Vec<String>,
}

// -- Delete

#[derive(Debug, StructOpt)]
struct DeleteArg {
    /// Delete a feed or filter from the database.
    #[structopt(subcommand)]
    pub cmd: DeleteSubArg,
}

#[derive(Debug, StructOpt)]
enum DeleteSubArg {
    #[structopt(name = "feed")]
    /// Remove a feed from the database
    Feed(DeleteFeed),

    #[structopt(name = "filter")]
    /// Remove a filter from the database
    Filter(DeleteFilter)
}

#[derive(Debug, StructOpt)]
struct DeleteFeed {
    /// The name used to refer to the feed to be deleted
    pub alias: String,
}

#[derive(Debug, StructOpt)]
struct DeleteFilter {
    /// The alias of the feed the filter to be deleted is on
    pub alias: String,
    /// The keywords to filter the entries with. Not all keywords from the filter need to be
    /// present, but enough to uniquely identify the filter are required.
    pub keywords: Vec<String>,
}

// -- List args
//
#[derive(Debug, StructOpt)]
struct ListArg {
    /// Add a feed or filter to the database.
    #[structopt(subcommand)]
    pub cmd: ListSubArg,
}

#[derive(Debug, StructOpt)]
enum ListSubArg {
    #[structopt(name = "feeds")]
    /// Display all feeds in the database
    Feeds,

    #[structopt(name = "filters")]
    /// Display all filters in the database
    Filters
}

impl RSSActionsArgs {
    pub fn get_cfg_dir(&self) -> Option<&Path> {
        self.config_dir.as_ref().map(Path::new)
    }

    pub fn to_cmd(self) -> Result<Box<dyn RSSActionCLICmd>> {
        let cmd: Box<dyn RSSActionCLICmd> = match self.cmd {
            SubArg::Add(add_args) => {
                match add_args.cmd {
                    AddSubArg::Feed(feed_args) => {
                        let url = Url::parse(&feed_args.url)?;
                        let feed = Feed::new(url, &feed_args.alias)?;
                        Box::new(crate::commands::AddFeedCmd(feed))
                    },
                    AddSubArg::Filter(filter_args) => {
                        let path = PathBuf::from(filter_args.script_path);
                        let filter = Filter::new(&filter_args.alias, filter_args.keywords, path)?;
                        Box::new(crate::commands::AddFilterCmd(filter))
                    }
                }
            }
            SubArg::Delete(delete_args) => {
                match delete_args.cmd {
                    DeleteSubArg::Feed(feed_args) => {
                        Box::new(crate::commands::DeleteFeedCmd(feed_args.alias))
                    },
                    DeleteSubArg::Filter(filter_args) => {
                        Box::new(crate::commands::DeleteFilterCmd(filter_args.alias, filter_args.keywords))
                    }
                }
            }
            SubArg::List(list_args) => {
                match list_args.cmd {
                    ListSubArg::Feeds => Box::new(crate::commands::ListFeedsCmd),
                    ListSubArg::Filters => Box::new(crate::commands::ListFiltersCmd),
                }
            },
            SubArg::Update => {
                Box::new(crate::commands::UpdateCmd)
            }
        };

        Ok(cmd)
    }
}
