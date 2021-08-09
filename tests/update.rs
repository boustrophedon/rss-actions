mod test_utils;
use test_utils::*;

use rss_actions::{ListFiltersCmd, UpdateCmd};
use rss_actions::{RSSActionCmd, ConsoleOutput};

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::prelude::*;
use chrono::{Utc, Duration};
use warp::Filter;

use url::Url;

use std::path::PathBuf;
use std::sync::mpsc::Sender;

// TODO (actually need to do this): add
// assert!(output.executed_feeds.iter().all(|res| res.is_ok()));
// assert!(output.filters.iter().all(|res| res.is_ok()));
// to all UpdateCmd output assertions (where appropriate, otherwise use iter::count and check err
// messages)

// TODO: use proptest to automate interleaving adding feeds and filters and checking updates
// there are over 1000 lines of tests here and there are still missing cases.
// eg adding a feed, adding a filter, doing an update, adding a feed, doing an update, and then
// adding a filter. or when a script fails one run and then succeeds the next run, that the
// filter's last_update is updated properly.

// TODO: the *Cmd execute methods return a data object that has the results, which also implement a
// trait for console output. originally they just returned the console output lines, which I then
// tested against directly.
//
// When I replaced them with an intermediate data object, the tests for updates got nicer but the
// tests for eg filter lists became more verbose but slightly clearer, and the asserts are more
// precise.
//
// The asserts currently check each field of the returned objects directly, rather than creating
// the objects (e.g. Filters) and checking for equality there. doing so would make the tests less
// verbose, but also less clear since using values directly in the constructor is not very
// enlightening
// ```
// let filter = Filter::new("local2", vec!["Example,"Entry], PathBuf::new("/bin/true"), timestamp1);
// assert_eq(filter, output.filters[0]);
// ```
// and at the same time having a custom test method is also not very clear
// `verify_filter(&filter, "local2", ["Example", "Entry"], true, timestamp1)`
//
// There's a section in either Kent Beck's TDD book or maybe Michael Feather's Working with Legacy
// Code about how they had to do some (similar to what I'm currently doing at the time of writing)
// annoying refactor and change a bunch of tests and they ended up just sitting down and doing the
// changes. I kind of think there isn't necessarily a better option, and the lesson is that
// changing existing APIs is annoying and you should try hard to get it right the first time. It's
// annoying for your clients as well!


// start a local warp server (per-test, in a new thread) on an unused port that serves example rss files and return
// a url to the location the files are being served from.

fn run_rss_files_server() -> Url {
    // blocking channel to get url from server url with local port from inside thread
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        tokio_runtime.block_on(_run_static_files_server(tx));
    });
    // get the url from the thread
    return rx.recv().unwrap();
}

fn run_rss_dynamic_server() -> Url {
    // blocking channel to get url from server url with local port from inside thread
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        tokio_runtime.block_on(_run_dynamic_server(tx));
    });
    // get the url from the thread
    return rx.recv().unwrap();
}

async fn _run_static_files_server(tx: Sender<Url>) {
    let mut test_rss_files_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_rss_files_dir.push("tests/test_rss_feed_files/");

    let route = warp::path("feeds").and(warp::fs::dir(test_rss_files_dir));

    let any_port_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (addr, server) = warp::serve(route).bind_ephemeral(any_port_addr);

    let port = addr.port();
    let url = Url::parse(&format!("http://127.0.0.1:{}/feeds/", port)).unwrap();

    tx.send(url).unwrap();
    server.await;
}

/// Creates a server that starts with one entry and adds one new one upon every access. The oldest
/// entries are first which helps us test that regardless of order we'll always collect the most
/// recent update.
async fn _run_dynamic_server(tx: Sender<Url>) {
    let counter = Arc::new(AtomicI64::new(0));
    let route = warp::path("feeds").and(warp::path("updates_every_access.rss")).and(warp::path::end())
        .map(move || {
            let count = counter.clone().fetch_add(1, Ordering::SeqCst);
            let rss_channel = make_feed(count);
            let rss_xml = rss_channel.to_string();
            warp::reply::with_header(rss_xml, "Content-Type", "application/rss+xml")
        });

    let any_port_addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let (addr, server) = warp::serve(route).bind_ephemeral(any_port_addr);

    let port = addr.port();
    let url = Url::parse(&format!("http://127.0.0.1:{}/feeds/", port)).unwrap();

    tx.send(url).unwrap();
    server.await;
}

fn make_feed(count: i64) -> rss::Channel {
    let mut items = Vec::new();
    for i in 0..(count+1) {
        let start_date = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);
        let offset = Duration::days(i);
        let date = start_date + offset;
        items.push(rss::ItemBuilder::default()
            .title(format!("Item {}", i))
            .link(format!("https://baddata.example.com/item/{}", i))
            .pub_date(date.to_rfc2822())
            .build().unwrap()
        )
    }
    rss::ChannelBuilder::default()
        .title("(non)Dynamic RSS title")
        .description("This is an example of a dynamically-generated RSS feed")
        .items(items)
        .build().unwrap()
}

#[test]
fn test_rss_file_server() {
    let base_url = run_rss_files_server();
    let example_feed_url = base_url.join("simple_feed.rss").unwrap();

    // Get the feed
    let res = reqwest::blocking::get(example_feed_url);
    assert!(res.is_ok(), "Error getting example feed: {:?}", res.unwrap_err());

    // Assert the text is somewhat correct
    let text = res.unwrap().text().unwrap();
    assert!(text.contains("This is an example of an RSS feed"), "Local rss feed server returns incorrect text:\n{}", text);

    // Assert that we can parse the xml
    let parsed_rss_res = rss::Channel::read_from(text.as_bytes());
    assert!(parsed_rss_res.is_ok(), "Couldn't parse example RSS feed: {:?}", parsed_rss_res.unwrap_err());

    let feed = parsed_rss_res.unwrap();
    assert_eq!(feed.copyright(), Some("2020 Example.com All rights reserved"));
    assert_eq!(feed.items().len(), 1);

    let item = feed.items()[0].clone();
    assert_eq!(item.title(), Some("Example entry"));
    assert_eq!(item.link(), Some("http://www.example.com/blog/post/1"));


    // make sure we can get another feed from the same server
    let example_feed_url = base_url.join("bad_feed.rss").unwrap();
    let res = reqwest::blocking::get(example_feed_url);
    assert!(res.is_ok(), "Error getting example feed: {:?}", res.unwrap_err());

    // Assert the non-rss text is there
    let text = res.unwrap().text().unwrap();
    assert_eq!(text, "This is not an RSS feed.\n");
}

#[test]
fn test_rss_dynamic_server() {
    let base_url = run_rss_dynamic_server();
    let example_feed_url = base_url.join("updates_every_access.rss").unwrap();

    // First access, one item
    {
    // Get the feed
    let res = reqwest::blocking::get(example_feed_url.clone());
    assert!(res.is_ok(), "Error getting example feed: {:?}", res.unwrap_err());

    // Assert the text is somewhat correct
    let text = res.unwrap().text().unwrap();
    assert!(text.contains("This is an example of a dynamically-generated RSS feed"),
        "Local rss feed server returns incorrect text:\n{}", text);

    // Assert that we can parse the xml
    let parsed_rss_res = rss::Channel::read_from(text.as_bytes());
    assert!(parsed_rss_res.is_ok(), "Couldn't parse example RSS feed: {:?}", parsed_rss_res.unwrap_err());

    let feed = parsed_rss_res.unwrap();
    assert_eq!(feed.title(), "(non)Dynamic RSS title");
    assert_eq!(feed.items().len(), 1);

    let item = feed.items()[0].clone();
    assert_eq!(item.title(), Some("Item 0"));
    }

    // Second access, two items
    {
    // Get the feed
    let res = reqwest::blocking::get(example_feed_url.clone());
    assert!(res.is_ok(), "Error getting example feed: {:?}", res.unwrap_err());

    // Assert the text is somewhat correct
    let text = res.unwrap().text().unwrap();
    assert!(text.contains("This is an example of a dynamically-generated RSS feed"),
        "Local rss feed server returns incorrect text:\n{}", text);

    // Assert that we can parse the xml
    let parsed_rss_res = rss::Channel::read_from(text.as_bytes());
    assert!(parsed_rss_res.is_ok(), "Couldn't parse example RSS feed: {:?}", parsed_rss_res.unwrap_err());

    let feed = parsed_rss_res.unwrap();
    assert_eq!(feed.title(), "(non)Dynamic RSS title");
    assert_eq!(feed.items().len(), 2);

    // ordered by date ascending!
    let item0 = feed.items()[0].clone();
    assert_eq!(item0.title(), Some("Item 0"));

    let item1 = feed.items()[1].clone();
    assert_eq!(item1.title(), Some("Item 1"));

    let date0 = DateTime::parse_from_rfc2822(item0.pub_date().unwrap()).unwrap();
    let date1 = DateTime::parse_from_rfc2822(item1.pub_date().unwrap()).unwrap();
    assert!(date1 > date0);
    }

    // Third access, three items
    {
    // Get the feed
    let res = reqwest::blocking::get(example_feed_url.clone());
    assert!(res.is_ok(), "Error getting example feed: {:?}", res.unwrap_err());

    // just check there are three items
    let text = res.unwrap().text().unwrap();
    let feed = rss::Channel::read_from(text.as_bytes()).unwrap();
    assert_eq!(feed.title(), "(non)Dynamic RSS title");
    assert_eq!(feed.items().len(), 3);
    }
}
/// Update with nothing in database says there's nothing to update as long as there are no filters.
#[test]
fn update_empty_db_no_updates() {
    let (_dir, cfg) = temp_config();

    let update_cmd = UpdateCmd;
    let res = update_cmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with empty database: {:?}", res.unwrap_err());
    assert_eq!(res.unwrap().output(), vec!["No filters in the database to update."]);

    // Add a feed but no filter, still print "Nothing to update."
    let feed_cmd = example_add_feed1();
    feed_cmd.execute(&cfg).unwrap();

    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with feed but no filter: {:?}", res.unwrap_err());
    assert_eq!(res.unwrap().output(), vec!["No filters in the database to update."]);
}

/// Update with a filter that hasn't been updated and no matching keywords in feed has no matches
/// and the filter is not updated.
#[test]
fn update_new_filter_but_no_matching_entries() {
    let (dir, cfg) = temp_config();
    let (script_path, log_path) = temp_log_data_script(dir.path());

    // set up test-specific server to serve rss feeds
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("simple_feed.rss").unwrap();


    // Add feed and filter
    let feed_cmd = example_add_feed_local1(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["xxx"], script_path);
    filter_cmd.execute(&cfg).unwrap();

    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update: {}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 0);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 1);
    assert_eq!(output.executed_filters.len(), 1);

    // Check that script was not run by checking for no log file
    assert!(!log_path.exists(), "Script was run on non-matching filter: {}", std::fs::read_to_string(&log_path).unwrap());

    // Check that filters are not updated
    let res = ListFiltersCmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.filters.len(), 1);
    assert!(output.filters[0].last_updated.is_none());
}

/// Test that with one un-updated filter, and a keyword-matching rss entry, the script is executed.
#[test]
fn more_recent_entry_updates_filter() {
    let (dir, cfg) = temp_config();
    let (script_path, log_path) = temp_log_data_script(dir.path());

    // set up test-specific server to serve rss feeds
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("two_entries.rss").unwrap();


    // Add feed and filter
    let feed_cmd = example_add_feed_local1(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["asthmatic", "NYC", "guestbook"], script_path);
    filter_cmd.execute(&cfg).unwrap();

    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with filter on matching entry: {}", res.unwrap_err());

    // Check that only one entry was matched
    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 1);
    assert_eq!(output.executed_filters.len(), 1);

    // Check that script was run once
    let script_output = std::fs::read_to_string(&log_path).unwrap();
    let expected_output = vec!["rss action script start",
    "title: Example entry NYC with random asthmatic words guestbook interspersed",
    "url: http://www.example.com/blog/post/2",
    "rss action script end\n"].join("\n");
    assert_eq!(script_output, expected_output);

    // Check that filter is updated in db
    let res = ListFiltersCmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    let timestamp = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0);
    assert_eq!(output.filters.len(), 1);
    assert_eq!(output.filters[0].last_updated.unwrap(), timestamp);
}

/// Make sure that when there are two new entries in a feed, the last_updated for that filter is the newest
/// of their pubdates.
#[test]
fn two_matching_entries_updates_last_updated_to_newest() {
    let (dir, cfg) = temp_config();
    let (script_path, log_path) = temp_log_data_script(dir.path());

    // set up test-specific server to serve rss feeds
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("two_entries.rss").unwrap();

    // Add feed and filter
    let feed_cmd = example_add_feed_local1(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    // "Example" is in the title of both entries
    let filter_cmd = example_add_filter_local1(vec!["Example"], script_path);
    filter_cmd.execute(&cfg).unwrap();

    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with filter on matching entry: {}", res.unwrap_err());

    // Check that the output is correct
    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 1);
    assert_eq!(output.executed_filters.len(), 1);

    // Check that script was run twice, and older entry was processed first
    let script_output = std::fs::read_to_string(&log_path).unwrap();
    let expected_output = vec!["rss action script start",
    "title: Pizza Example marshmallow entry with random listener words interspersed",
    "url: http://www.example.com/blog/post/1",
    "rss action script end",
    "rss action script start",
    "title: Example entry NYC with random asthmatic words guestbook interspersed",
    "url: http://www.example.com/blog/post/2",
    "rss action script end\n"].join("\n");
    assert_eq!(script_output, expected_output);

    // Check that filter is updated in db
    let res = ListFiltersCmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    let timestamp = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0);
    assert_eq!(output.filters.len(), 1);
    assert_eq!(output.filters[0].alias, "local1");
    assert_eq!(output.filters[0].keywords, ["Example",]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "data_script.sh");
    assert_eq!(output.filters[0].last_updated.unwrap(), timestamp);
}

/// Test that with one updated filter, and a keyword-matching rss entry, the script is not
/// executed. This is achieved by just running update twice and checking via the log file that the
/// script was only executed once.
#[test]
fn older_entry_does_not_update_filter() {
    let (dir, cfg) = temp_config();
    let (script_path, log_path) = temp_log_data_script(dir.path());

    // set up test-specific server to serve rss feeds
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("two_entries.rss").unwrap();


    // Add feed and filter
    let feed_cmd = example_add_feed_local1(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["asthmatic", "NYC", "guestbook"], script_path);
    filter_cmd.execute(&cfg).unwrap();

    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with filter matching entries: {}", res.unwrap_err());

    // Check that the output is correct
    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 1);
    assert_eq!(output.executed_filters.len(), 1);

    // Execute update again and get no updates
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with filter matching entries: {}", res.unwrap_err());
    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 0);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 1);
    assert_eq!(output.executed_filters.len(), 1);

    // Check that script was run once
    let expected_output = vec!["rss action script start",
    "title: Example entry NYC with random asthmatic words guestbook interspersed",
    "url: http://www.example.com/blog/post/2",
    "rss action script end\n"].join("\n");
    let script_output = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(script_output, expected_output);

    // Check that filter is updated in db
    let res = ListFiltersCmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    let timestamp = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0);
    assert_eq!(output.filters.len(), 1);
    assert_eq!(output.filters[0].alias, "local1");
    assert_eq!(output.filters[0].keywords, ["NYC", "asthmatic", "guestbook"]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "data_script.sh");
    assert_eq!(output.filters[0].last_updated.unwrap(), timestamp);
}

/// If the feed has new items update is run, and we run update again, only the new items
/// are processed.
#[test]
fn update_filter_with_new_entries_only_processes_new_items() {
    let (dir, cfg) = temp_config();
    let (script_path, log_path) = temp_log_data_script(dir.path());

    // set up test-specific server to serve rss feeds dynamically
    let base_url = run_rss_dynamic_server();
    let feed_url = base_url.join("updates_every_access.rss").unwrap();

    // Add feed and filter
    let feed_cmd = example_add_feed_local1(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["Item"], script_path);
    filter_cmd.execute(&cfg).unwrap();

    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with filter matching entries: {}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 1);
    assert_eq!(output.executed_filters.len(), 1);

    // Check that script was run 1 times via log
    let expected_output = vec!["rss action script start",
    "title: Item 0",
    "url: https://baddata.example.com/item/0",
    "rss action script end\n"].join("\n");
    let script_output = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(script_output, expected_output);

    // Check that the output of list filters has the correct last_updated date for the filter
    let output = ListFiltersCmd.execute(&cfg).unwrap();

    let timestamp = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);
    assert_eq!(output.filters.len(), 1);
    assert_eq!(output.filters[0].alias, "local1");
    assert_eq!(output.filters[0].keywords, ["Item"]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "data_script.sh");
    assert_eq!(output.filters[0].last_updated.unwrap(), timestamp);

    // Now run update again and check that only new item was processed

    // Delete the previous-update's log file. NB we could also check that the output is the
    // previous value and the new value, but then we're slightly vulnerable to the issue of what
    // happens if the script overwrites the file instead of appending.
    std::fs::remove_file(&log_path).unwrap();

    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with filter matching entries: {}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 1);
    assert_eq!(output.executed_filters.len(), 1);

    // Check that script was run 1 time with the newer item
    let expected_output = vec!["rss action script start",
    "title: Item 1",
    "url: https://baddata.example.com/item/1",
    "rss action script end\n"].join("\n");
    let script_output = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(script_output, expected_output);

    // Check that the output of list filters has the correct last_updated date for the filter
    let output = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp = Utc.ymd(2000, 1, 2).and_hms(0, 0, 0);
    assert_eq!(output.filters.len(), 1);
    assert_eq!(output.filters[0].alias, "local1");
    assert_eq!(output.filters[0].keywords, ["Item"]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "data_script.sh");
    assert_eq!(output.filters[0].last_updated.unwrap(), timestamp);
}

/// With two filters on two feeds, check that if one updates and the other doesn't, the
/// last_updated times are correct.
#[test]
fn update_one_feed_new_and_one_not_updated() {
    let (_dir, cfg) = temp_config();

    // set up test-specific server to serve rss feeds dynamically
    let base_url = run_rss_dynamic_server();
    let feed_url = base_url.join("updates_every_access.rss").unwrap();

    // Add feed and filter
    let feed_cmd = example_add_feed_local1(feed_url);
    feed_cmd.execute(&cfg).unwrap();


    // set up test-specific server to serve static rss feeds
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("simple_feed.rss").unwrap();

    let feed_cmd = example_add_feed_local2(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["Item"], PathBuf::from("/bin/true"));
    filter_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local2(vec!["Example", "entry"], PathBuf::from("/bin/true"));
    filter_cmd.execute(&cfg).unwrap();

    // Execute update with filters
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update: {}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 2);
    assert_eq!(output.updates, 2);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 2);
    assert_eq!(output.executed_filters.len(), 2);

    // List filters and check that both updated
    let output = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp1 = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0);
    let timestamp2 = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);

    assert_eq!(output.filters.len(), 2);
    assert_eq!(output.filters[0].alias, "local2");
    assert_eq!(output.filters[0].keywords, ["Example", "entry"]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "true");
    assert_eq!(output.filters[0].last_updated.unwrap(), timestamp1);
    assert_eq!(output.filters[1].alias, "local1");
    assert_eq!(output.filters[1].keywords, ["Item"]);
    assert_eq!(output.filters[1].script_path.get_file_name(), "true");
    assert_eq!(output.filters[1].last_updated.unwrap(), timestamp2);

    // Execute update again and see that only one filter was updated
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update: {}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 2);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 2);
    assert_eq!(output.executed_filters.len(), 2);

    // List filters and check that 1 updated and 1 not updated
    let output = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp1 = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0);
    let timestamp2 = Utc.ymd(2000, 1, 2).and_hms(0, 0, 0);

    assert_eq!(output.filters.len(), 2);
    assert_eq!(output.filters[0].alias, "local2");
    assert_eq!(output.filters[0].keywords, ["Example", "entry"]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "true");
    assert_eq!(output.filters[0].last_updated.unwrap(), timestamp1);
    assert_eq!(output.filters[1].alias, "local1");
    assert_eq!(output.filters[1].keywords, ["Item"]);
    assert_eq!(output.filters[1].script_path.get_file_name(), "true");
    assert_eq!(output.filters[1].last_updated.unwrap(), timestamp2);
}

/// With three feeds and some filters each, check the correct updates are made and that
/// list-filters lists the correct last_updated times.
///
/// TODO it's annoying and it would bloat the line count even further so i'm leaving these as
/// testing the ConsoleOutput for the filters list output rather than checking each individually.
///
/// I should just write a function to take in the parameters and do the asserts but I don't really
/// like the way that those kinds of tests look, e.g.:
///
/// `verify_filter(&filter, "local2", ["Example", "Entry"], true, timestamp1)`
///
/// but at the same time that's really not that different than
///
/// `
/// let filter = Filter::new("local2", vec!["Example,"Entry], PathBuf::new("/bin/true"), timestamp1);
/// assert_eq(filter, output.filters[0]);
/// `
#[test]
fn update_multiple_filters_with_multiple_feeds() {
    let (dir, cfg) = temp_config();
    let (script_path, log_path) = temp_log_data_script(dir.path());

    // Feed 1

    // set up test-specific server to serve static rss feeds
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("simple_feed.rss").unwrap();

    // Add static feed and filter
    example_add_feed_local1(feed_url)
        .execute(&cfg).unwrap();

    // Will not match
    example_add_filter_local1(vec!["Example", "xxx"], script_path.clone())
        .execute(&cfg).unwrap();
    // Will match
    example_add_filter_local1(vec!["Example"], script_path.clone())
        .execute(&cfg).unwrap();

    // Feed 2

    let feed_url = base_url.join("two_entries.rss").unwrap();
    example_add_feed_local2(feed_url)
        .execute(&cfg).unwrap();

    // Feed 3

    // set up test-specific server to serve rss feeds dynamically
    let base_url = run_rss_dynamic_server();
    let feed_url = base_url.join("updates_every_access.rss").unwrap();

    // Add static feed and filter
    example_add_feed_local3(feed_url)
        .execute(&cfg).unwrap();

    // Will only match after the third update
    example_add_filter_local3(vec!["Item", "2"], script_path.clone())
        .execute(&cfg).unwrap();
    // Will match on the first update
    example_add_filter_local3(vec!["Item", "0"], script_path.clone())
        .execute(&cfg).unwrap();

    // Update
    // feed 3 and feed 1 match and have updated time

    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Failed to execute update {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 4);
    assert_eq!(output.updates, 2);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 3);
    assert_eq!(output.executed_filters.len(), 4);

    let script_output = std::fs::read_to_string(&log_path).unwrap();
    assert!(script_output.matches("rss action script start").count() == 2, "script output:\n{}", script_output);
    assert!(script_output.matches("title: Item 0").count() == 1, "script output:\n{}", script_output);
    assert!(script_output.matches("title: Example entry").count() == 1, "script output:\n{}", script_output);

    std::fs::remove_file(&log_path).unwrap();

    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp1: DateTime<Local> = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0).into();
    let timestamp2: DateTime<Local> = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0).into();

    let filter_line1 = ["local1", "Example", "data_script.sh", &timestamp1.to_string()].join("\t");
    let filter_line2 = ["local3", "0, Item", "data_script.sh", &timestamp2.to_string()].join("\t");
    let filter_line3 = ["local1", "Example, xxx", "data_script.sh", "Never updated"].join("\t");
    let filter_line4 = ["local3", "2, Item", "data_script.sh", "Never updated"].join("\t");
    assert_eq!(message.output(),
        vec!["Current filters:", "", &filter_line1, &filter_line2, &filter_line3, &filter_line4]);
    // Update
    // no updates

    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Failed to execute update {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 4);
    assert_eq!(output.updates, 0);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 3);
    assert_eq!(output.executed_filters.len(), 4);


    assert!(!log_path.exists(), "Script was run on filter with no updates: {}", std::fs::read_to_string(&log_path).unwrap());
    std::fs::remove_file(&log_path).unwrap_err(); // should be not found

    // same as previous, no updates
    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp1: DateTime<Local> = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0).into();
    let timestamp2: DateTime<Local> = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0).into();

    let filter_line1 = ["local1", "Example", "data_script.sh", &timestamp1.to_string()].join("\t");
    let filter_line2 = ["local3", "0, Item", "data_script.sh", &timestamp2.to_string()].join("\t");
    let filter_line3 = ["local1", "Example, xxx", "data_script.sh", "Never updated"].join("\t");
    let filter_line4 = ["local3", "2, Item", "data_script.sh", "Never updated"].join("\t");
    assert_eq!(message.output(),
        vec!["Current filters:", "", &filter_line1, &filter_line2, &filter_line3, &filter_line4]);

    // Update
    // Feed 3 match and has updated time
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Failed to execute update {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 4);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 3);
    assert_eq!(output.executed_filters.len(), 4);

    let script_output = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(script_output.matches("rss action script start").count(), 1, "script output:\n{}", script_output);
    assert_eq!(script_output.matches("title: Item 2").count(), 1, "script output:\n{}", script_output);
    assert_eq!(script_output.matches("\n").count(), 4, "script output:\n{}", script_output); // lazy way to count lines
    std::fs::remove_file(&log_path).unwrap();

    // dynamic filter watching for the third entry is updated, static and dynamic watching for
    // second are not.
    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp1: DateTime<Local> = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0).into();
    let timestamp2: DateTime<Local> = Utc.ymd(2000, 1, 3).and_hms(0, 0, 0).into();
    let timestamp3: DateTime<Local> = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0).into();

    let filter_line1 = ["local1", "Example", "data_script.sh", &timestamp1.to_string()].join("\t");
    let filter_line2 = ["local3", "2, Item", "data_script.sh", &timestamp2.to_string()].join("\t");
    let filter_line3 = ["local3", "0, Item", "data_script.sh", &timestamp3.to_string()].join("\t");
    let filter_line4 = ["local1", "Example, xxx", "data_script.sh", "Never updated"].join("\t");
    assert_eq!(message.output(),
        vec!["Current filters:", "", &filter_line1, &filter_line2, &filter_line3, &filter_line4,]);
    // Add new filter
    // Update
    // New feed 2 matches

    example_add_filter_local2(vec!["interspersed"], script_path.clone())
        .execute(&cfg).unwrap();

    // new filter is present and not updated
    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp1: DateTime<Local> = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0).into();
    let timestamp2: DateTime<Local> = Utc.ymd(2000, 1, 3).and_hms(0, 0, 0).into();
    let timestamp3: DateTime<Local> = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0).into();

    let filter_line1 = ["local1", "Example", "data_script.sh", &timestamp1.to_string()].join("\t");
    let filter_line2 = ["local3", "2, Item", "data_script.sh", &timestamp2.to_string()].join("\t");
    let filter_line3 = ["local3", "0, Item", "data_script.sh", &timestamp3.to_string()].join("\t");
    let filter_line4 = ["local1", "Example, xxx", "data_script.sh", "Never updated"].join("\t");
    let filter_line5 = ["local2", "interspersed", "data_script.sh", "Never updated"].join("\t");
    assert_eq!(message.output(),
        vec!["Current filters:", "", &filter_line1, &filter_line2, &filter_line3, &filter_line4, &filter_line5]);

    // run update and only new filter is updated
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Failed to execute update {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 5);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 3);
    assert_eq!(output.executed_filters.len(), 5);

    let script_output = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(script_output.matches("rss action script start").count(), 2, "script output:\n{}", script_output);
    assert_eq!(script_output.matches("title: Example entry NYC with random asthmatic words guestbook interspersed").count(), 1,
        "script output:\n{}", script_output);
    assert_eq!(script_output.matches("title: Pizza Example marshmallow entry with random listener words interspersed").count(), 1,
        "script output:\n{}", script_output);
    assert_eq!(script_output.matches("\n").count(), 8, "script output:\n{}", script_output); // lazy way to count lines
    std::fs::remove_file(&log_path).unwrap();

    // New filter is updated, old ones are not
    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let filter_line1 = ["local1", "Example", "data_script.sh", &timestamp1.to_string()].join("\t");
    let filter_line2 = ["local2", "interspersed", "data_script.sh", &timestamp1.to_string()].join("\t");
    let filter_line3 = ["local3", "2, Item", "data_script.sh", &timestamp2.to_string()].join("\t");
    let filter_line4 = ["local3", "0, Item", "data_script.sh", &timestamp3.to_string()].join("\t");
    let filter_line5 = ["local1", "Example, xxx", "data_script.sh", "Never updated"].join("\t");
    let expected = vec!["Current filters:", "", &filter_line1, &filter_line2, &filter_line3, &filter_line4, &filter_line5];
    assert_eq!(message.output(), expected,
        "\n---output:\n{}\n\n\n---expected:\n{}", message.output().join("\n"), expected.join("\n"));
    // Update
    // No matches
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Failed to execute update {:?}", res.unwrap_err());
    let output = res.unwrap();
    assert_eq!(output.successes, 5);
    assert_eq!(output.updates, 0);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 3);
    assert_eq!(output.executed_filters.len(), 5);

    // no log
    assert!(!log_path.exists(), "Script was run on filter with no updates: {}", std::fs::read_to_string(&log_path).unwrap());
    std::fs::remove_file(&log_path).unwrap_err(); // should be not found

    // same as previous
    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let filter_line1 = ["local1", "Example", "data_script.sh", &timestamp1.to_string()].join("\t");
    let filter_line2 = ["local2", "interspersed", "data_script.sh", &timestamp1.to_string()].join("\t");
    let filter_line3 = ["local3", "2, Item", "data_script.sh", &timestamp2.to_string()].join("\t");
    let filter_line4 = ["local3", "0, Item", "data_script.sh", &timestamp3.to_string()].join("\t");
    let filter_line5 = ["local1", "Example, xxx", "data_script.sh", "Never updated"].join("\t");
    let expected = vec!["Current filters:", "", &filter_line1, &filter_line2, &filter_line3, &filter_line4, &filter_line5];
    assert_eq!(message.output(), expected,
        "\n---output:\n{}\n\n\n---expected:\n{}", message.output().join("\n"), expected.join("\n"));
    // Update
}

/// Check that if a feed only partially matches a filter, the filter does not match and the script
/// is not run.
#[test]
fn update_new_filter_with_partially_matching_entries() {
    let (dir, cfg) = temp_config();
    let (script_path, log_path) = temp_log_data_script(dir.path());

    // set up test-specific server to serve rss feeds
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("simple_feed.rss").unwrap();


    // Add feed and filter
    let feed_cmd = example_add_feed_local1(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["Example", "xxx"], script_path);
    filter_cmd.execute(&cfg).unwrap();

    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update: {}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 0);
    assert_eq!(output.failures, 0);
    assert_eq!(output.executed_feeds.len(), 1);
    assert_eq!(output.executed_filters.len(), 1);

    // Check that script was not run by checking for no log file
    assert!(!log_path.exists(), "Script was run on non-matching filter: {}", std::fs::read_to_string(&log_path).unwrap());
}

/// Run update with one server not responding and one server that is fine, make sure filter on okay
/// server is updated and filter on bad server is not updated.
///
/// Has #[ignore] because it will wait a full 30 seconds for the request to time out.
/// Use `cargo test -- --ignored` to run.
#[test]
#[ignore]
fn update_with_one_feed_failing_succeeds() {
    let (dir, cfg) = temp_config();
    let (script_path, log_path) = temp_log_data_script(dir.path());

    // first feed, server does not exist. see comment inside "network failing" message test below.
    let feed_cmd = example_add_feed_local1(url::Url::parse("http://169.254.0.1/doesNotExist.rss").unwrap());
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["xxx"], script_path.clone());
    filter_cmd.execute(&cfg).unwrap();

    // feed two with working server
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("simple_feed.rss").unwrap();

    let feed_cmd = example_add_feed_local2(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local2(vec!["Example", "entry"], script_path);
    filter_cmd.execute(&cfg).unwrap();

    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with one failing server: {}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 1);
    assert_eq!(output.executed_feeds.len(), 2);
    assert_eq!(output.executed_filters.len(), 1);
    assert_eq!(output.executed_feeds[0].1.as_ref().unwrap_err().to_string(),
               "Failed to download local1 rss feed from url http://169.254.0.1/doesNotExist.rss");

    // Check that script was run once
    let expected_output = vec!["rss action script start",
    "title: Example entry",
    "url: http://www.example.com/blog/post/1",
    "rss action script end\n"].join("\n");
    let script_output = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(script_output, expected_output);

    // List filters and check that 1 updated and 1 not updated
    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp: DateTime<Local> = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0).into();

    let filter_line1 = ["local2", "Example, entry", "data_script.sh", &timestamp.to_string()].join("\t");
    let filter_line2 = ["local1", "xxx", "data_script.sh", "Never updated"].join("\t");
    assert_eq!(message.output(),
        vec!["Current filters:", "", &filter_line1, &filter_line2]);
}

/// Run update with one server that returns good data and one server that returns non-rss data and
/// check that good filter is updated and bad filter is not updated.
#[test]
fn update_with_one_feed_bad_data_succeeds() {
    let (dir, cfg) = temp_config();
    let (script_path, log_path) = temp_log_data_script(dir.path());

    // first feed, server serves bad data
    let base_url = run_rss_files_server();
    let bad_feed_url = base_url.join("bad_feed.rss").unwrap();
    let feed_cmd = example_add_feed_local1(bad_feed_url.clone());
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["xxx"], script_path.clone());
    filter_cmd.execute(&cfg).unwrap();

    // feed two with working server
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("simple_feed.rss").unwrap();

    let feed_cmd = example_add_feed_local2(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local2(vec!["Example", "entry"], script_path);
    filter_cmd.execute(&cfg).unwrap();


    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with one failing server: {}", res.unwrap_err());

    let err_msg = format!("Could not parse local1 rss feed from url {}", bad_feed_url.to_string());
    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 1);
    assert_eq!(output.executed_feeds.len(), 2);
    assert_eq!(output.executed_filters.len(), 1);
    assert!(output.executed_feeds[0].1.as_ref().unwrap_err().to_string()
        .contains(&err_msg));

    // Check that script was run once
    let expected_output = vec!["rss action script start",
    "title: Example entry",
    "url: http://www.example.com/blog/post/1",
    "rss action script end\n"].join("\n");
    let script_output = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(script_output, expected_output);

    // List filters and check that 1 updated and 1 not updated
    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp: DateTime<Local> = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0).into();

    let filter_line1 = ["local2", "Example, entry", "data_script.sh", &timestamp.to_string()].join("\t");
    let filter_line2 = ["local1", "xxx", "data_script.sh", "Never updated"].join("\t");
    assert_eq!(message.output(),
        vec!["Current filters:", "", &filter_line1, &filter_line2]);
}

/// If all feeds fail to download, probably network issue. Show a message for that.
///
/// Ignored because it will wait a full 30 seconds for the requests to time out.
/// Run with `cargo test -- --ignored`
#[test]
#[ignore]
fn all_feeds_fail_download_network_message() {
    let (_dir, cfg) = temp_config();

    // Add feeds with urls pointing to link local addresses which are very very very unlikely (and
    // according to wikipedia are forbidden by spec) to be routed to
    let feed_cmd = example_add_feed_local1(url::Url::parse("http://169.254.0.1/doesNotExist.rss").unwrap());
    feed_cmd.execute(&cfg).unwrap();
    let feed_cmd = example_add_feed_local2(url::Url::parse("http://169.254.0.2/alsoFake.rss").unwrap());
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["xxx"], PathBuf::from("/bin/false"));
    filter_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local2(vec!["Nothing"], PathBuf::from("/bin/false"));
    filter_cmd.execute(&cfg).unwrap();

    // Execute update and check for "network error" message
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_err(), "Update succeeded with all servers failing: {:?}", res.unwrap());
    assert_eq!(res.unwrap_err().to_string(),
        "All RSS feed downloads failed. Is the network down?");

    // List filters and check that it says not updated
    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let filter_line1 = ["local1", "xxx", "false", "Never updated"].join("\t");
    let filter_line2 = ["local2", "Nothing", "false", "Never updated"].join("\t");
    assert_eq!(message.output(),
        vec!["Current filters:", "", &filter_line1, &filter_line2]);
}

#[test]
/// If a feed has items that are missing titles, urls, or published dates, make sure that the
/// filter's script is not run and its last_updated is not changed.
fn feed_with_missing_data_does_not_update_filter() {
    let (dir, cfg) = temp_config();
    let (script_path, log_path) = temp_log_data_script(dir.path());

    // first feed, server serves feed with missing rss entry data
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("missing_data.rss").unwrap();
    let feed_cmd = example_add_feed_local1(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["pubdate", "missing"], script_path.clone());
    filter_cmd.execute(&cfg).unwrap();

    // feed two with working server
    let base_url = run_rss_files_server();
    let feed_url = base_url.join("simple_feed.rss").unwrap();

    let feed_cmd = example_add_feed_local2(feed_url);
    feed_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local2(vec!["Example", "entry"], script_path);
    filter_cmd.execute(&cfg).unwrap();


    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with one feed with missing data: {}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 1);
    assert_eq!(output.executed_feeds.len(), 2);
    assert_eq!(output.executed_filters.len(), 1);
    assert_eq!(output.executed_feeds[0].1.as_ref().unwrap_err().to_string(),
               "1 entries in feed local1 had data errors");

    // Check that script was run once
    let expected_output = vec!["rss action script start",
    "title: Example entry",
    "url: http://www.example.com/blog/post/1",
    "rss action script end\n"].join("\n");
    let script_output = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(script_output, expected_output);

    // List filters and check that it says not updated
    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp: DateTime<Local> = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0).into();

    let filter_line1 = ["local2", "Example, entry", "data_script.sh", &timestamp.to_string()].join("\t");
    let filter_line2 = ["local1", "missing, pubdate", "data_script.sh", "Never updated"].join("\t");
    assert_eq!(message.output(), ["Current filters:", "", &filter_line1, &filter_line2]);
}

#[test]
/// If a filter's script exits with a non-zero exit code, make sure the filter's last_updated is
/// not changed.
fn feed_with_failing_script_does_not_update_filter() {
    let (_dir, cfg) = temp_config();
    let script_path = PathBuf::from("/bin/false");

    let base_url = run_rss_files_server();
    let feed_url = base_url.join("two_entries.rss").unwrap();
    example_add_feed_local1(feed_url).execute(&cfg).unwrap();

    // two filters, both match but first filter's script will always fails
    let filter_cmd = example_add_filter_local1(vec!["Example", "entry"], script_path.clone());
    filter_cmd.execute(&cfg).unwrap();

    let filter_cmd = example_add_filter_local1(vec!["entry"], PathBuf::from("/bin/true"));
    filter_cmd.execute(&cfg).unwrap();

    // Execute update with filter
    let res = UpdateCmd.execute(&cfg);
    assert!(res.is_ok(), "Error running update with failing script: {}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.successes, 1);
    assert_eq!(output.updates, 1);
    assert_eq!(output.failures, 1);
    assert_eq!(output.executed_feeds.len(), 1);
    assert_eq!(output.executed_filters.len(), 2);
    assert_eq!(output.executed_filters[0].1.as_ref().unwrap_err().to_string(),
            "Script failed for filter on feed local1, keywords Example, entry, script /bin/false");


    // List filters and check that it says not updated
    let message = ListFiltersCmd.execute(&cfg).unwrap();
    let timestamp: DateTime<Local> = Utc.ymd(2009, 9, 6).and_hms(16, 20, 0).into();

    let filter_line1 = ["local1", "entry", "true", &timestamp.to_string()].join("\t");
    let filter_line2 = ["local1", "Example, entry", "false", "Never updated"].join("\t");
    assert_eq!(message.output(), ["Current filters:", "", &filter_line1, &filter_line2]);
}
