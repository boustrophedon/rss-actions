// i'm trying out just writing integration tests and not db level tests because it ends up just
// being a proxy for the db anyway

use std::path::PathBuf;

use crate::db::RSSActionsDb;

use crate::models::{Feed, Filter};


/// Start a test transaction with a new in memory database
fn make_test_db() -> RSSActionsDb {
    RSSActionsDb::open_in_memory().unwrap()
}

// I think these qualify sufficiently as integration tests that I don't feel like I'm cheating what
// I said. I do think they could be made top-level integration tests but it's definitely easier to
// test here.

#[test]
/// Test that adding a filter, updating, and fetching succeeds
fn add_update_fetch_filter_succeeds() {
    let mut db = make_test_db();
    let mut tx = db.transaction().unwrap();

    let feed = Feed::new(url::Url::parse("http://example.com/").unwrap(), "test_example").unwrap();
    let mut filter = Filter::new("test_example", vec!["a".into(), "b".into()],
        PathBuf::from("/bin/false")).unwrap();

    tx.store_feed(&feed.alias, &feed.url).unwrap();
    tx.store_filter(&filter).unwrap();

    // Update filter check for success
    filter.update_time(chrono::Utc::now());
    let res = tx.update_filter(&filter);
    assert!(res.is_ok(), "Error updating filter during update: {:?}", res.unwrap_err());

    // Fetch filters and check filter is updated
    let res = tx.fetch_filters();
    assert!(res.is_ok(), "Error fetching updated filter: {:?}", res.unwrap_err());
    let filters = res.unwrap();
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], filter);
}

#[test]
/// Test that adding a filter, updating it, and then trying to update a bad (i.e. non-existant)
/// filter fails, but the entire transaction succeeds.
fn add_update_update_bad_fetch_succeeds() {
    let mut db = make_test_db();
    let mut tx = db.transaction().unwrap();

    let feed = Feed::new(url::Url::parse("http://example.com/").unwrap(), "test_example").unwrap();
    let mut filter = Filter::new("test_example", vec!["a".into(), "b".into()],
        PathBuf::from("/bin/false")).unwrap();
    let mut bad_filter = Filter::new("test_example", vec!["b".into()],
        PathBuf::from("/bin/false")).unwrap();

    tx.store_feed(&feed.alias, &feed.url).unwrap();
    tx.store_filter(&filter).unwrap();

    // Update filter check for success
    filter.update_time(chrono::Utc::now());
    let res = tx.update_filter(&filter);
    assert!(res.is_ok(), "Error updating filter during update: {:?}", res.unwrap_err());

    // Update bad filter, check for failure
    bad_filter.update_time(chrono::Utc::now());
    let res = tx.update_filter(&bad_filter);
    assert!(res.is_err(), "Updating bad filter during update succeeded erroneously");
    let err = res.unwrap_err();
    assert!(err.to_string().contains("No filter was found to update that matches"),
        "Incorrect error message with bad filter update:\n{:?}", err);

    // Fetch filters and check good filter is still updated
    let res = tx.fetch_filters();
    assert!(res.is_ok(), "Error fetching updated filter: {:?}", res.unwrap_err());
    let filters = res.unwrap();
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0], filter);
}
