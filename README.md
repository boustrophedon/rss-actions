[![build-test](https://github.com/boustrophedon/rss-actions/actions/workflows/build-test.yaml/badge.svg)](https://github.com/boustrophedon/rss-actions/actions/workflows/build-test.yaml) [![Coverage Status](https://coveralls.io/repos/github/boustrophedon/rss-actions/badge.svg?branch=github_ci)](https://coveralls.io/github/boustrophedon/rss-actions?branch=github_ci)

# About rss-actions

This is a Rust library and command-line program for tracking RSS feeds and automatically running scripts when they update.

Feeds are added with a url and alias used to refer to that feed. You can then associate multiple filters with a feed, which can be used to match certain entries in the feed based on keywords in the title of the entry. A filter also has a script associated with it which is run on each entry that's matched by the filter's keywords.

To recap, a feed has:

- A name, called an alias
- A url

and a filter has:

- The feed it's associated with, referenced by alias
- A list of keywords used to match against every entry in the associated feed.
- A script to run on matching entries
- A timestamp marking the last time the filter was matched (not the newest-seen entry of the feed!)

Each feed is only downloaded once, to prevent accidentally hitting rate limits when you have many filters on a single feed.

Data is stored in a local sqlite database.

# Usage

Add feeds with

```
rss-actions add feed <alias> <url>
```

Add filters with

```
rss-actions add filter <alias> <path-to-script> [keywords...]
```

and then add a crontab entry or systemd timer file that calls

```
rss-actions update
```

To list feeds and filters you can use `rss-actions list feeds` and `rss-actions list filters` respectively.

## Usage and deployment notes
Note that if you want the update to run as a different user than the one you ran the commands with, you'll have to copy the config file from `$XDG_CONFIG_DIR/rss-actions/` and sqlite db from `$XDG_DATA_DIR/rss-actions/` to the corresponding directories in the other user's home directory, or change the configuration file to point to the correct location for the database. Also make sure your scripts have the correct locations and are accessible.

# Other stuff

While writing this program, I experimented with only writing "integration" tests, i.e. only in the top-level tests/ directory.

So far the only bugs that weren't caught were:

- In my tests I was making a custom Config with a temporary database directory, and so I missed the case where the default directory (~/.local/share/rss-actions/) did not exist. This might have been caught by inspecting code coverage. In a previous project I had explicit tests for this that actually manipulated the $HOME environment variable and used a global lock so that the tests for it couldn't run in parallel (but the rest of the test suite could).

- keyword matching was supposed to be case-insensitive but none of my tests tested that
