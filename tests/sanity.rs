use rss_actions::*;

mod test_utils;

/// Sanity check that data is being written to disk.
///
/// Open test config and db (in temp dir), write to it, and then *close and reopen the db
/// and config* and read back the data.
#[test]
fn test_config_database_sanity() {
    let (dir, cfg) = test_utils::temp_config();

    // Add example feed
    let cmd = test_utils::example_add_feed1();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing add feed command failed: {}", res.unwrap_err());


    // Reopen cfg
    let mut cfg_path = dir.path().to_path_buf();
    cfg_path.push("config.toml");
    let cfg = Config::open(Some(cfg_path.as_path())).unwrap();

    // Read back example feed we wrote
    let cmd = test_utils::example_list_feeds();
    let res = cmd.execute(&cfg);
    assert!(res.is_ok(), "Executing list feeds command failed: {}", res.unwrap_err());

    let output = res.unwrap();
    assert_eq!(output, vec!["Current feeds:", "", "example_1\thttps://example.com/feed.rss"]);
}
