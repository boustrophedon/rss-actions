use crate::db::RSSActionsDb;

use crate::models::Filter;


/// Start a test transaction with a new in memory database
fn make_test_db() -> RSSActionsDb {
    RSSActionsDb::open_in_memory().unwrap()
}

fn example_filter() -> Filter {
    let mut script_path = std::path::PathBuf::new();
    script_path.push(env!("CARGO_MANIFEST_DIR"));
    script_path.push("test");
    script_path.push("scripts");
    script_path.push("print_data");

    return Filter {
        alias: "example".into(),
        keywords: "test Arch Linux".into(),
        script_path: script_path,
        last_updated: None,
    };
}
// #[test]
// fn test_db_add_filter() {
//     let mut db = make_test_db();
//     let tx = db.transaction().unwrap();
// 
//     let feed = example_feed();
//     let filter = example_filter();
//     tx.store_feed(&feed).unwrap();
// 
//     let res = tx.add_filter(&example_filter);
//     assert!(res.is_ok(), "adding filter failed {:?}", res.unwrap_err());
// }
