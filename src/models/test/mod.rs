use crate::models::{Feed, Filter};

use std::path::PathBuf;

#[test]
fn feed_alias_must_be_nonempty() {
    let res = Feed::new(url::Url::parse("https://example.org").unwrap(), "");
    assert!(res.is_err(), "Feed with empty alias did not error");

    let err = res.unwrap_err();
    assert!(err.to_string().contains("A feed's alias must not be empty."));
}

#[test]
fn filter_new_time_is_none_and_update_is_some() {
    let res = Filter::new("example_feed", Vec::new(), PathBuf::from("/bin/false"));
    assert!(res.is_ok(), "Filter::new failed: {:?}", res.unwrap_err());

    let filter = res.unwrap();
    assert!(filter.last_updated.is_none(), "New filter had Some in last_updated field: {:?}", filter.last_updated);
}

#[test]
fn filter_new_fails_on_nonfile() {
    let res = Filter::new("example_feed", Vec::new(), PathBuf::from("/usr/"));
    assert!(res.is_err(), "Filter::new accepted script path with non-file");

    let err = res.unwrap_err();
    assert!(err.to_string().contains("The filter's script path is not a file: /usr"),
            "Filter error message was incorrect: {}", err);
}

#[test]
fn filter_new_fails_on_nonexecutable_file() {
    let res = Filter::new("example_feed", Vec::new(), PathBuf::from(format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR"))));
    assert!(res.is_err(), "Filter::new accepted script path with non-executable file");

    let err = res.unwrap_err();
    assert!(err.to_string().contains("The filter's script path is not executable"),
            "Filter error message was incorrect: {}", err);
}

#[test]
fn filter_new_succeeds_on_executable_file() {
    let res = Filter::new("example_feed", Vec::new(), PathBuf::from("/bin/false"));
    assert!(res.is_ok(), "Creating new filter failed: {:?}", res.unwrap_err());
}
