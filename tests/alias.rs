mod test_utils;
use test_utils::*;

/// Displaying no feeds shows a "no feeds" message.
#[test]
fn display_no_feeds() {
    let (_dir, cfg) = temp_config();

    let cmd = example_list_feeds();

    // execute list feeds command with no feeds in db
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    // assert message is "No feeds in database."
    let message = res.unwrap();
    assert_eq!(message, vec!["No feeds in database."]);
}

/// Add one feed and display it
#[test]
fn add_and_display_feed() {
    let (_dir, cfg) = temp_config();

    // Add example feed
    let cmd = example_add_feed1();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add feed command failed: {:?}", res.unwrap_err());

    // Execute list feeds command with 1 feed in db
    let cmd = example_list_feeds();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    // Check feed is listed
    let message = res.unwrap();
    assert_eq!(message, vec!["Current feeds:", "", "example_1\thttps://example.com/feed.rss"]);
}

/// Add multiple feeds and display them, based on addition order
#[test]
fn add_and_display_feeds() {
    let (_dir, cfg) = temp_config();

    // Add first example feed
    let cmd = example_add_feed1();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add feed command failed: {:?}", res.unwrap_err());

    // Add second example feed
    let cmd = example_add_feed2();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add feed command failed: {:?}", res.unwrap_err());

    // Execute list feeds command with 1 feed in db
    let cmd = example_list_feeds();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    // Check feed is listed
    let message = res.unwrap();
    assert_eq!(message, vec!["Current feeds:", "", "example_1\thttps://example.com/feed.rss", "example_2_org\thttps://example.org/feed2.rss"]);
}

/// Add two of the same feed and get an error
#[test]
fn add_duplicate_feed_and_error() {
    let (_dir, cfg) = temp_config();

    // Add first example feed
    let cmd = example_add_feed1();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add feed command failed: {:?}", res.unwrap_err());

    // Add first example feed again
    let cmd = example_add_feed1();
    let res = cmd.execute(&cfg);

    // Second, duplicate, addition should fail
    assert!(res.is_err(), "Adding duplicate feed didn't fail");
    let err = res.unwrap_err();
    let errchain: String = err.chain().map(|cause| cause.to_string()).collect();
    assert!(errchain.contains("UNIQUE constraint failed: feeds.alias"));
}

// /// Add feed with non-url and get an error
// /// We can't actually write this test because the cmd interface takes a feed with an already-parsed
// /// URL, and we can't force the URL crate to make a bad url as far as I can tell.
// ///
// /// This would have to be tested at the db layer level or at the input level
// #[test]
// fn add_bad_url_feed_and_error() {
//     let (_dir, cfg) = temp_config();
// 
//     // Add first example feed
//     let cmd = RSSActionCmd::AddFeed(
//         Feed::new(??? can't do this);
//     let res = cmd.execute(&cfg);
// 
//     // Second, duplicate, addition should fail
//     assert!(res.is_err(), "Adding duplicate feed didn't fail");
//     let err = res.unwrap_err();
//     assert!(err.to_string().contains("test"));
// }
