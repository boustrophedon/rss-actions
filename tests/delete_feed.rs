mod test_utils;
use test_utils::*;

use rss_actions::{RSSActionCmd, DeleteFeedCmd, ListFeedsCmd};
use rss_actions::ConsoleOutput;


// success
// - delete with feed and no filters
// - delete with feed and filter on other feed
// - add filter, fail to delete feed, delete filter, delete feed
//
// failure
// - delete with nonexistant feed
// - delete with filter on feed
//   - delete with multiple filters on feed

#[test]
/// Delete a non-existant feed.
fn delete_non_existant_feed_err() {
    let (_dir, cfg) = temp_config();

    let cmd = DeleteFeedCmd("example_1".into());
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Deleting nonexistant feed succeeded mistakenly");
    assert_eq!(res.unwrap_err().to_string(),
        "No feed was found to delete that matches name `example_1`");

    // check no feeds in db
    let output = ListFeedsCmd.execute(&cfg).unwrap();
    assert_eq!(output.feeds.len(), 0);
}

#[test]
/// Delete with filter on feed
fn delete_filter_on_feed_err() {
    let (_dir, cfg) = temp_config();

    example_add_feed1().execute(&cfg).unwrap();
    example_add_filter1().execute(&cfg).unwrap();


    let cmd = DeleteFeedCmd("example_1".into());
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Deleting feed with filter succeeded mistakenly");

    let err = res.unwrap_err();
    let errchain: String = err.chain().map(|cause| cause.to_string()).collect();
    assert!(errchain.contains("FOREIGN KEY constraint failed"));

    // check feed is still there
    let output = ListFeedsCmd.execute(&cfg).unwrap();
    assert_eq!(output.feeds.len(), 1);


    // still fails with two filters
    example_add_filter2().execute(&cfg).unwrap();

    let cmd = DeleteFeedCmd("example_1".into());
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Deleting feed with filter succeeded mistakenly");

    let err = res.unwrap_err();
    let errchain: String = err.chain().map(|cause| cause.to_string()).collect();
    assert!(errchain.contains("FOREIGN KEY constraint failed"));

    // check feed is still there
    let output = ListFeedsCmd.execute(&cfg).unwrap();
    assert_eq!(output.feeds.len(), 1);
}


#[test]
/// Delete a feed normally.
fn delete_feed_suceeds_normally() {
    let (_dir, cfg) = temp_config();

    example_add_feed1().execute(&cfg).unwrap();

    let cmd = DeleteFeedCmd("example_1".into());
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "deleting feed failed: {:?}", res.unwrap_err());
    let output = res.unwrap();
    assert_eq!(output.0, "example_1");
    assert_eq!(output.output(), ["Successfully deleted feed example_1"]);

    // check no feeds
    let output = ListFeedsCmd.execute(&cfg).unwrap();
    assert_eq!(output.feeds.len(), 0);
}

#[test]
/// Delete with feed and filter on other feed
fn delete_feed_with_filter_on_other_feed() {
    let (_dir, cfg) = temp_config();

    example_add_feed1().execute(&cfg).unwrap();
    example_add_feed2().execute(&cfg).unwrap();
    example_add_filter4().execute(&cfg).unwrap();


    let cmd = DeleteFeedCmd("example_1".into());
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "deleting feed failed: {:?}", res.unwrap_err());
    let output = res.unwrap();
    assert_eq!(output.0, "example_1");
    assert_eq!(output.output(), ["Successfully deleted feed example_1"]);

    // check 1 feed remaining
    let output = ListFeedsCmd.execute(&cfg).unwrap();
    assert_eq!(output.feeds.len(), 1);
}

#[test]
/// Try to delete with filter, fail, remove filter, delete feed and succeed.
fn delete_feed_suceeds_after_deleting_filter() {
    let (_dir, cfg) = temp_config();

    example_add_feed1().execute(&cfg).unwrap();
    example_add_filter1().execute(&cfg).unwrap();

    let cmd = DeleteFeedCmd("example_1".into());
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Deleting feed with filter succeeded mistakenly");

    let err = res.unwrap_err();
    let errchain: String = err.chain().map(|cause| cause.to_string()).collect();
    assert!(errchain.contains("FOREIGN KEY constraint failed"));

    // check still 1 feed
    let output = ListFeedsCmd.execute(&cfg).unwrap();
    assert_eq!(output.feeds.len(), 1);

    // delete filter, then delete feed and succeed
    example_delete_filter1().execute(&cfg).unwrap();

    let cmd = DeleteFeedCmd("example_1".into());
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "deleting feed failed: {:?}", res.unwrap_err());
    let output = res.unwrap();
    assert_eq!(output.0, "example_1");
    assert_eq!(output.output(), ["Successfully deleted feed example_1"]);

    // check 0 feeds remaining
    let output = ListFeedsCmd.execute(&cfg).unwrap();
    assert_eq!(output.feeds.len(), 0);

}

