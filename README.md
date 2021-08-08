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

This is because both in personal projects and at work I've been somewhat annoyed at writing unit tests for accessors (whether over the network or from a database) and then writing those same exact tests when you're exposing a read/write/update endpoint for the same data.

In previous projects, writing both kinds of tests has been somewhat useful for exposing idiosyncrasies of the API, e.g. when using proptest to test getters and setters for an sqlite database, I learned you can't have null bytes in strings.

I think in general the lesson is that each *layer* or *api* within your codebase should be well-defined and have "integration" tests written against it. Here I want "integration" tests to mean "your API is internally and behaviorally consistent". For example, if you have

From this viewpoint I think you might want to view "unit" tests as just tests that test against a faked or mocked endpoint. However "integration" tests would not, in contrast, be tests that test against a "real" endpoint, but instead are tests that test **the functionality of the API as a whole, whether internal or external.**

For example, it doesn't make sense to "unit" test a database or network service accessor. If you are testing against an in-memory sqlite database but you're testing that you can write to and read back from it (i.e. testing that your foreign keys and CHECKs, serializers and deserializers are correct), this is an integration test.

A corollary of this is that it doesn't make sense to "unit test" purely computational APIs, e.g. a convex hull solver. I'm not really sure that I like this, so I kind of want to change my definition of a unit test to just *a test that doesn't test your external API*. If you had a function that, e.g. takes a line and a point and tells you whether the point is to the left of the line, you could consider that a single-function internal API if you wanted, but all of your tests for it would still be *unit tests*.

There are two dimensions that I've kind of mixed up here: testing aginst "real" vs "fake" data, and testing an api for "consistency" vs testing a method for "correctness". I feel that they're related somehow though, but at this point I kind of want to delete my rambling here and move it to a blog post.

It seems like everyone has their own idea of what unit, integration, end-to-end, behavioral, etc. tests are, but this project helped me figure out what I *want* them to mean a little bit better. I'm still kind of confused though.


So far the only bug that wasn't caught was that in my tests I was making a custom Config with a temporary database directory, and so I missed the case where the default directory (~/.local/share/rss-actions/) did not exist. This might have been caught by inspecting code coverage. In a previous project I had explicit tests for this that actually manipulated the $HOME environment variable and used a global lock so that the tests for it couldn't run in parallel (but the rest of the test suite could).
