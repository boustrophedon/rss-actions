// Allow dead code because each test includes this file separately and may not use all functions.
#![allow(dead_code)]

use rss_actions::*;

use tempfile::{TempDir, tempdir};

use std::path::PathBuf;

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

pub fn example_list_filters() -> RSSActionCmd {
    RSSActionCmd::ListFilters
}

pub fn example_add_feed1() -> RSSActionCmd {
    RSSActionCmd::AddFeed(
        Feed::new(url::Url::parse("https://example.com/feed.rss").unwrap(), "example_1").unwrap()
    )
}

pub fn example_add_feed2() -> RSSActionCmd {
    RSSActionCmd::AddFeed(
        Feed::new(url::Url::parse("https://example.org/feed2.rss").unwrap(), "example_2_org").unwrap()
    )
}

pub fn example_script_path1() -> PathBuf {
    let mut script_path = std::path::PathBuf::new();
    script_path.push(env!("CARGO_MANIFEST_DIR"));
    script_path.push("tests");
    script_path.push("scripts");
    script_path.push("print_data");

    script_path
}

pub fn example_script_path2() -> PathBuf {
    let mut script_path = std::path::PathBuf::new();
    script_path.push("/bin/false");

    script_path
}

fn to_strings(strs: Vec<&str>) -> Vec<String> {
    strs.iter().map(|s| s.to_string()).collect()
}

/// Example filter with empty filter keywords
pub fn example_add_filter_empty() -> RSSActionCmd {
    RSSActionCmd::AddFilter(
        Filter::new("example_1", vec![], example_script_path2()).unwrap()
    )
}

pub fn example_add_filter1() -> RSSActionCmd {
    RSSActionCmd::AddFilter(
        Filter::new("example_1", to_strings(vec!["test"]), example_script_path1()).unwrap()
    )
}

/// Same feed as filter1 but with different filter keywords
pub fn example_add_filter2() -> RSSActionCmd {
    RSSActionCmd::AddFilter(
        Filter::new("example_1", to_strings(vec!["test", "other_keyword"]), example_script_path1()).unwrap()
    )
}

/// Same feed and keywords as filter1 but different script path
pub fn example_add_filter3() -> RSSActionCmd {
    RSSActionCmd::AddFilter(
        Filter::new("example_1", to_strings(vec!["test"]), example_script_path2()).unwrap()
    )
}

/// different feed than filters1,2,3
pub fn example_add_filter4() -> RSSActionCmd {
    RSSActionCmd::AddFilter(
        Filter::new("example_2_org", to_strings(vec!["test", "other_keyword"]), example_script_path1()).unwrap()
    )
}

/// non-existant feed
pub fn example_add_filter_bad_feed_alias() -> RSSActionCmd {
    RSSActionCmd::AddFilter(
        Filter::new("example_nonexistant", to_strings(vec!["fake"]), example_script_path2()).unwrap()
    )
}

/// Same feed and filters and script path as filter 2 but with filters in different order
pub fn example_add_filter_same_keywords_different_order() -> RSSActionCmd {
    RSSActionCmd::AddFilter(
        Filter::new("example_1", to_strings(vec!["other_keyword", "test"]), example_script_path1()).unwrap()
    )
}
