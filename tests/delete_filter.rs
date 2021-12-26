mod test_utils;
use test_utils::*;

use rss_actions::{RSSActionCmd, DeleteFilterCmd, ListFiltersCmd};
use rss_actions::ConsoleOutput;

// error:
// - delete from non-existant feed
// - delete with feed but non-existant filter
// - delete with multiple matching filters on the same feed, check nothing deleted
// - delete with matching filter on other feed but non on the selected feed, check error and not
//   deleted
//
// success
// - add filter, delete filter
// - delete filter with same filter on other feed, other feed's filter is not deleted
// - delete filter with empty keywords
// - delete filter with other nonmatching filter, other filter is not deleted
// - delete filter with filters given out of order, filter is still deleted

#[test]
/// Delete a non-existant filter from a non-existant feed.
fn delete_non_existant_feed_err() {
    // TODO because we scan the filters outside the db, the error in this is not different from not
    // finding a matching filter, which is a bit annoying if you're actually mistyping the alias
    // name rather than the filter keywords. maybe add a hint to the error message?

    let (_dir, cfg) = temp_config();

    let cmd = example_delete_filter1();
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Deleting nonexistant filter with nonexistant feed succeeded mistakenly");
    assert_eq!(res.unwrap_err().to_string(), "No filters matching `test` on the feed `example_1` were found in the database.");
}

#[test]
/// Delete a non-existant filter from an existing feed
fn delete_non_existant_filter_err() {
    let (_dir, cfg) = temp_config();

    // add feed, don't add filter
    example_add_feed1().execute(&cfg).unwrap();

    let cmd = example_delete_filter1();
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Deleting nonexistant filter with existing feed succeeded mistakenly");
    assert_eq!(res.unwrap_err().to_string(), "No filters matching `test` on the feed `example_1` were found in the database.");
}

#[test]
/// Matching failures:
///   - keyword substring match
///   - keyword missing
/// in all cases, should error with no matched filters and db should not be changed.
fn delete_filter_matching_failures_err() {
    let (_dir, cfg) = temp_config();

    // add feed, add filter with keywords "test, other_keyword"
    example_add_feed1().execute(&cfg).unwrap();
    example_add_filter2().execute(&cfg).unwrap();

    let keyword_lists = [
        vec!["est","other_keyword"],
        vec!["est",],
        vec!["uwu"]
    ];

    for keyword_list in keyword_lists {
        // delete partially-matching filter, check command failed
        let cmd = DeleteFilterCmd::new("example_1", &keyword_list);
        let res = cmd.execute(&cfg);
        assert!(res.is_err(), "Deleting nonexistant filter with existing feed succeeded mistakenly");
        assert_eq!(res.unwrap_err().to_string(),
            format!("No filters matching `{}` on the feed `example_1` were found in the database.",
                keyword_list.join(",")));

        // check filters still in db
        let output = ListFiltersCmd.execute(&cfg).unwrap();
        assert_eq!(output.filters.len(), 1);
    }
}

#[test]
/// If more than one filter matches, fail to prevent accidental unintended deletions. TODO add
/// --override or --force-all or --delete-all or separate delete-all command. Or --path and match
/// on path if you legitimately want multiple filters with the same keywords on the same feed.
fn delete_multiple_matches_fails() {
    let (_dir, cfg) = temp_config();

    // add feed, add filters with keywords both matching "test"
    example_add_feed1().execute(&cfg).unwrap();
    example_add_filter1().execute(&cfg).unwrap();
    example_add_filter2().execute(&cfg).unwrap();

    let cmd = DeleteFilterCmd::new("example_1", &["test"]);
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Deleting multiple filters matching the same keyword succeeded mistakenly");
    assert_eq!(res.unwrap_err().to_string(), "Multiple filters matching `test` on the feed `example_1` were found in the database.");

    // check filters still in db
    let output = ListFiltersCmd.execute(&cfg).unwrap();
    assert_eq!(output.filters.len(), 2);
}


#[test]
/// Delete a filter that matches another feed's filter, make sure it errors and the other feed's
/// filter is still there.
fn delete_nonexistant_filter_matching_other_feed() {
    let (_dir, cfg) = temp_config();

    // add feeds, add filter
    example_add_feed1().execute(&cfg).unwrap();
    example_add_feed2().execute(&cfg).unwrap();
    example_add_filter4().execute(&cfg).unwrap();

    // delete on wrong feed, get error no matches
    let cmd = DeleteFilterCmd::new("example_1", &["test"]);
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Deleting filter on the wrong feed succeeds mistakenly");
    assert_eq!(res.unwrap_err().to_string(), "No filters matching `test` on the feed `example_1` were found in the database.");

    // check filters still in db
    let output = ListFiltersCmd.execute(&cfg).unwrap();
    assert_eq!(output.filters.len(), 1);
}

#[test]
/// Adding and deleting a filter normally.
fn delete_succeeds_normally() {
    let (_dir, cfg) = temp_config();

    // add feeds, add filter
    example_add_feed1().execute(&cfg).unwrap();
    example_add_filter2().execute(&cfg).unwrap();

    ListFiltersCmd.execute(&cfg).unwrap();

    // delete filter
    let cmd = DeleteFilterCmd::new("example_1", &["test"]);
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Deleting filter failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.0, "example_1");
    assert_eq!(output.1, ["test"]);
    assert_eq!(output.output(), ["Successfully deleted filter on feed example_1", "Keywords: test"]);


    // check no filters
    let output = ListFiltersCmd.execute(&cfg).unwrap();
    assert_eq!(output.filters.len(), 0);
}

#[test]
/// With multiple filters on the same feed, make sure only the matching filter is deleted.
fn delete_filter_only_deletes_matching() {
    let (_dir, cfg) = temp_config();

    // add feeds, add filters
    example_add_feed1().execute(&cfg).unwrap();
    example_add_feed2().execute(&cfg).unwrap();
    example_add_filter2().execute(&cfg).unwrap();
    example_add_filter5().execute(&cfg).unwrap();

    // delete filter
    let cmd = DeleteFilterCmd::new("example_1", &["keyword"]);
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Deleting filter failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.0, "example_1");
    assert_eq!(output.1, ["keyword"]);
    assert_eq!(output.output(), ["Successfully deleted filter on feed example_1", "Keywords: keyword"]);


    // check 1 filter remaining
    let output = ListFiltersCmd.execute(&cfg).unwrap();
    assert_eq!(output.filters.len(), 1);
}

#[test]
/// With two different feeds and two filters with the same keywords, check that deleting a filter
/// from one feed doesn't delete the filter on another.
fn delete_existing_filter_matches_other_feed() {
    let (_dir, cfg) = temp_config();

    // add feed, add filters with same keywords but on different feeds
    example_add_feed1().execute(&cfg).unwrap();
    example_add_feed2().execute(&cfg).unwrap();
    example_add_filter1().execute(&cfg).unwrap();
    example_add_filter4().execute(&cfg).unwrap();

    // delete filter
    let cmd = DeleteFilterCmd::new("example_2_org", &["test"]);
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Deleting filter failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.0, "example_2_org");
    assert_eq!(output.1, ["test"]);
    assert_eq!(output.output(), ["Successfully deleted filter on feed example_2_org", "Keywords: test"]);


    // check 1 filter remaining (from other feed)
    let output = ListFiltersCmd.execute(&cfg).unwrap();
    assert_eq!(output.filters.len(), 1);
}

#[test]
/// With filter keywords given out of order, make sure the filter is still deleted.
fn delete_filters_out_of_order() {
    let (_dir, cfg) = temp_config();

    // add feeds, add filter
    example_add_feed1().execute(&cfg).unwrap();
    example_add_filter2().execute(&cfg).unwrap();

    // delete filter
    // note that filters are out of order in add command as well, should probably test both ways
    let cmd = DeleteFilterCmd::new("example_1", &["test", "other_keyword"]);
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Deleting filter failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.0, "example_1");
    // TODO user is only shown the keywords given in the command, rather than what we actually read
    // from the db.
    assert_eq!(output.1, ["test", "other_keyword"]);
    assert_eq!(output.output(), ["Successfully deleted filter on feed example_1", "Keywords: test, other_keyword"]);


    // check no filters
    let output = ListFiltersCmd.execute(&cfg).unwrap();
    assert_eq!(output.filters.len(), 0);
}
