mod test_utils;
use test_utils::*;

use rss_actions::{RSSActionCmd, ListFiltersCmd};
use rss_actions::{ConsoleOutput};

/// Displaying with no feeds shows a "no filters" message.
#[test]
fn display_no_filters() {
    let (_dir, cfg) = temp_config();

    let cmd = ListFiltersCmd;

    // execute list filters command with no feeds in db
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    // assert message
    let output = res.unwrap();
    assert_eq!(output.output(), vec!["No filters in database."]);
}

/// Test that trying to add a filter with an alias that doesn't exist shows an error message
#[test]
fn display_error_message_on_filter_with_nonexistant_alias() {
    let (_dir, cfg) = temp_config();

    // Add filter without adding feed. could also have used example_add_filter1() here.
    let cmd = example_add_filter_bad_feed_alias();
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Adding filter without adding feed succeeded incorrectly");

    let err = res.unwrap_err();
    assert!(err.to_string().contains("Couldn't find a feed with alias example_nonexistant."),
            "Incorrect error message: {:?}", err);
}

/// Test that two filters with the same feed do not error
#[test]
fn same_feed_different_keywords() {
    let (_dir, cfg) = temp_config();
    // add example feeds for filters to filter
    example_add_feed1().execute(&cfg).unwrap();


    // add first filter
    let cmd = example_add_filter1();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add filter command failed: {:?}", res.unwrap_err());

    // add second filter, doesn't fail
    let cmd = example_add_filter2();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add filter command failed: {:?}", res.unwrap_err());

    // list filters, two filters in db
    let cmd = ListFiltersCmd;
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert!(output.filters.len() == 2);
    assert_eq!(output.filters[0].alias, "example_1");
    assert_eq!(output.filters[0].keywords, ["test"]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "print_data");
    assert!(output.filters[0].last_updated.is_none());
    assert_eq!(output.filters[1].alias, "example_1");
    assert_eq!(output.filters[1].keywords, ["other_keyword", "test"]);
    assert_eq!(output.filters[1].script_path.get_file_name(), "print_data");
    assert!(output.filters[1].last_updated.is_none());
}


/// Test that two filters with the same feed and keywords but different scripts don't error
#[test]
fn same_feed_same_filters_different_script() {
    let (_dir, cfg) = temp_config();
    // add example feeds for filters to filter
    example_add_feed1().execute(&cfg).unwrap();


    // add first filter
    let cmd = example_add_filter1();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add filter command failed: {:?}", res.unwrap_err());

    // add second filter, doesn't fail
    let cmd = example_add_filter3();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add filter command failed: {:?}", res.unwrap_err());

    // list filters, two filters in db
    let cmd = ListFiltersCmd;
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());
    let output = res.unwrap();
    assert_eq!(output.filters[0].alias, "example_1");
    assert_eq!(output.filters[0].keywords, ["test"]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "print_data");
    assert!(output.filters[0].last_updated.is_none());
    assert_eq!(output.filters[1].alias, "example_1");
    assert_eq!(output.filters[1].keywords, ["test"]);
    assert_eq!(output.filters[1].script_path.get_file_name(), "false");
    assert!(output.filters[1].last_updated.is_none());
}

/// Test that adding two of the same exact filters fails
#[test]
fn same_filters_all_parameters_fails() {
    let (_dir, cfg) = temp_config();
    // add example feeds for filters to filter
    example_add_feed1().execute(&cfg).unwrap();

    
    // add first filter
    let cmd = example_add_filter1();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add filter command failed: {:?}", res.unwrap_err());

    // add first filter again and fail
    let cmd = example_add_filter1();
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Executing add filter succeeded but should have failed");

    // list filters, only first filter in db
    let cmd = ListFiltersCmd;
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.filters[0].alias, "example_1");
    assert_eq!(output.filters[0].keywords, ["test"]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "print_data");
    assert!(output.filters[0].last_updated.is_none());
}

/// Test that adding two of the same exact filters with keywords in different order fails
#[test]
fn same_filters_different_keyword_order_fails() {
    let (_dir, cfg) = temp_config();
    // add example feeds for filters to filter
    example_add_feed1().execute(&cfg).unwrap();


    // add first filter
    let cmd = example_add_filter2();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add filter command failed: {:?}", res.unwrap_err());

    // add second filter and fails
    let cmd = example_add_filter_same_keywords_different_order();
    let res = cmd.execute(&cfg);
    assert!(res.is_err(), "Executing second add filter command suceeded but it shouldn't have");

    let err = res.unwrap_err();
    assert!(err.to_string().contains("You can't add another filter with the same feed alias, keyword, and script path."),
            "Incorrect error message: {:?}", err);

    // list filters, only first filter in db
    let cmd = ListFiltersCmd;
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.filters[0].alias, "example_1");
    assert_eq!(output.filters[0].keywords, ["other_keyword", "test"]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "print_data");
    assert!(output.filters[0].last_updated.is_none());
}

/// Test that adding two filters with the same keywords but different feeds succeeds
#[test]
fn same_filters_different_feed_aliases() {
    let (_dir, cfg) = temp_config();
    // add example feeds for filters to filter
    example_add_feed1().execute(&cfg).unwrap();
    example_add_feed2().execute(&cfg).unwrap();


    // add first filter
    let cmd = example_add_filter1();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add filter command failed: {:?}", res.unwrap_err());

    // add second filter and fails
    let cmd = example_add_filter4();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing second add filter command failed: {:?}", res.unwrap_err());

    // list filters, both filters in db
    let cmd = ListFiltersCmd;
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list command failed: {:?}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output.filters[0].alias, "example_1");
    assert_eq!(output.filters[0].keywords, ["test"]);
    assert_eq!(output.filters[0].script_path.get_file_name(), "print_data");
    assert!(output.filters[0].last_updated.is_none());
    assert_eq!(output.filters[1].alias, "example_2_org");
    assert_eq!(output.filters[1].keywords, ["other_keyword", "test"]);
    assert_eq!(output.filters[1].script_path.get_file_name(), "print_data");
    assert!(output.filters[1].last_updated.is_none());
}
