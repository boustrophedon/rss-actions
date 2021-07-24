// Allow dead code because each test includes this file separately and may not use all functions.
#![allow(dead_code)]

use rss_actions::*;

use tempfile::{TempDir, tempdir};
use url::Url;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};


/// This method just makes it easy to get the filename when testing, when we know the path is just
/// going to be plain ascii.
pub trait GetFileName {
    fn get_file_name(&self) -> String;
}
impl GetFileName for PathBuf {
    fn get_file_name(&self) -> String {
        self.file_name().unwrap().to_string_lossy().to_string()
    }
}


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

//// Example no-parameter commands
//// We could write these inline but if they need to change it's nice to uniformly have them behind
//// convenience functions.

//// Example Feeds

pub fn example_add_feed1() -> AddFeedCmd {
    AddFeedCmd(
        Feed::new(url::Url::parse("https://example.com/feed.rss").unwrap(), "example_1").unwrap()
    )
}

pub fn example_add_feed2() -> AddFeedCmd {
    AddFeedCmd(
        Feed::new(url::Url::parse("https://example.org/feed2.rss").unwrap(), "example_2_org").unwrap()
    )
}

/// Add a feed with url pointing to a local server. The name doesn't actually matter and the URL
/// doesn't have to be a local server, that's just what it's being used for.
pub fn example_add_feed_local1(url: Url) -> AddFeedCmd {
    AddFeedCmd(
        Feed::new(url, "local1").unwrap()
    )
}

pub fn example_add_feed_local2(url: Url) -> AddFeedCmd {
    AddFeedCmd(
        Feed::new(url, "local2").unwrap()
    )
}

pub fn example_add_feed_local3(url: Url) -> AddFeedCmd {
    AddFeedCmd(
        Feed::new(url, "local3").unwrap()
    )
}

//// Example Filters

/// Example filter with empty filter keywords
pub fn example_add_filter_empty() -> AddFilterCmd {
    AddFilterCmd(
        Filter::new("example_1", vec![], example_script_path2()).unwrap()
    )
}

pub fn example_add_filter1() -> AddFilterCmd {
    AddFilterCmd(
        Filter::new("example_1", to_strings(vec!["test"]), example_script_path1()).unwrap()
    )
}

/// Same feed as filter1 but with different filter keywords
pub fn example_add_filter2() -> AddFilterCmd {
    AddFilterCmd(
        Filter::new("example_1", to_strings(vec!["test", "other_keyword"]), example_script_path1()).unwrap()
    )
}

/// Same feed and keywords as filter1 but different script path
pub fn example_add_filter3() -> AddFilterCmd {
    AddFilterCmd(
        Filter::new("example_1", to_strings(vec!["test"]), example_script_path2()).unwrap()
    )
}

/// filter with different feed than filters1,2,3
pub fn example_add_filter4() -> AddFilterCmd {
    AddFilterCmd(
        Filter::new("example_2_org", to_strings(vec!["test", "other_keyword"]), example_script_path1()).unwrap()
    )
}

/// filter with non-existant feed
pub fn example_add_filter_bad_feed_alias() -> AddFilterCmd {
    AddFilterCmd(
        Filter::new("example_nonexistant", to_strings(vec!["fake"]), example_script_path2()).unwrap()
    )
}

/// Same feed and filters and script path as filter 2 but with filters in different order
pub fn example_add_filter_same_keywords_different_order() -> AddFilterCmd {
    AddFilterCmd(
        Filter::new("example_1", to_strings(vec!["other_keyword", "test"]), example_script_path1()).unwrap()
    )
}

/// filters using local server feed alias 1,2,3
pub fn example_add_filter_local1(strings: Vec<&str>, script_path: PathBuf) -> AddFilterCmd {
    AddFilterCmd(
        Filter::new("local1", to_strings(strings), script_path).unwrap()
    )
}
pub fn example_add_filter_local2(strings: Vec<&str>, script_path: PathBuf) -> AddFilterCmd {
    AddFilterCmd(
        Filter::new("local2", to_strings(strings), script_path).unwrap()
    )
}
pub fn example_add_filter_local3(strings: Vec<&str>, script_path: PathBuf) -> AddFilterCmd {
    AddFilterCmd(
        Filter::new("local3", to_strings(strings), script_path).unwrap()
    )
}

//// Utility functions

fn to_strings(strs: Vec<&str>) -> Vec<String> {
    strs.iter().map(|s| s.to_string()).collect()
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

/// A script that outputs data about the relevant enviroment variables passed into it when run
/// during `rss-actions update`.
static SCRIPT_TEMPLATE: &str =
"#!/bin/bash

OUTPUT={output_file}
# this line redirects stdout and stderr to the log file for the entire program
# see https://github.com/koalaman/shellcheck/wiki/SC2129 and
# https://mywiki.wooledge.org/BashFAQ/014
exec >> $OUTPUT 2>&1

echo rss action script start
echo title: $RSSACTIONS_ENTRY_TITLE
echo url: $RSSACTIONS_ENTRY_URL
echo rss action script end
";

/// The script writes the data out to a file for verification that it's executing and the data
/// received by the script is correct.
///
/// The script is written to `exec_dir/data_script.sh` and the log is written to
/// `exec_dir/test_script_log.txt`.
pub fn temp_log_data_script(exec_dir: &Path) -> (PathBuf, PathBuf) {
    let script_path = exec_dir.join("data_script.sh");
    let log_path = exec_dir.join("test_script_log.txt");

    // populate logfile (in test temp directory) file path into script
    // and write script to script file
    let script_contents = SCRIPT_TEMPLATE.replace("{output_file}", &log_path.to_string_lossy());
    std::fs::write(&script_path, script_contents).unwrap();

    // set permissions on script file
    let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script_path, perms).unwrap();


    (script_path, log_path)
}
