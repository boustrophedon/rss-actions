// Allow dead code because each test includes this file separately and may not use all functions.
#![allow(dead_code)]

use rss_actions::*;

use tempfile::{TempDir, tempdir};

/// Create a test config with the database in a temporary directory. We return the TempDir because
/// it is deleted when it is dropped.
///
/// Kind of don't like this because I'm duplicating the stuff in the actual Config methods wrt
/// default locations but only kind of because it also simulates using a non-default config file
/// location and a non-default db file location.
///
/// Should just make separate Config unit tests that set the home directory and call new() and
/// check the file is in the right place.
pub fn temp_config() -> (TempDir, Config) {
    let test_dir = tempdir().expect("temporary directory could not be created");

    // don't use into_path because it would cause test_dir to not be deleted on drop
    let mut db_path = test_dir.path().to_path_buf();
    db_path.push("rss-actions-test.db");

    let cfg = Config {
        db_path,
    };

    let mut cfg_path = test_dir.path().to_path_buf();
    cfg_path.push("config.toml");
    cfg.write_out(&cfg_path).unwrap();

    (test_dir, cfg)
}

pub fn example_list_feeds() -> RSSActionCmd {
    RSSActionCmd::ListFeeds
}

pub fn example_add_feed1() -> RSSActionCmd {
    RSSActionCmd::AddFeed(
        Feed::new(url::Url::parse("https://example.com/feed.rss").unwrap(), "example_1")
    )
}

pub fn example_add_feed2() -> RSSActionCmd {
    RSSActionCmd::AddFeed(
        Feed::new(url::Url::parse("https://example.org/feed2.rss").unwrap(), "example_2_org")
    )
}
