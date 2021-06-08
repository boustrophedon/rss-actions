use crate::db::{RSSActionsDb};

mod ops;

#[test]
fn test_db_open() {
    let res = RSSActionsDb::open_in_memory();
    assert!(res.is_ok(), "failed to open db in memory: {}", res.unwrap_err());

    let mut db = res.unwrap();

    let res = db.transaction();
    assert!(res.is_ok(), "failed to begin db transaction: {}", res.unwrap_err());
}
