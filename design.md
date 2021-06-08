# Goal

Store a set of rss feed URLs, and filters on those feeds, such that when the update command is run:

- The RSS feeds are fetched from their URLs
- For each feed, for each entry that is new since the stored last update:
  - The entry is matched against the filters stored in the database
  - Entries that match have the filter's corresponding action executed
- The last update field is updated in the database

## Data

- RSS feeds
  - URL
  - last update
  - Feed alias name
- Filters for feed
  - Feed id
  - Filter string
  - Path to file to execute when filter is matched

## Operations

- Add feed
  - Input data: url, alias
- Add filter
  - Input data: feed alias, filter string, path to action executable
- Update
  - Runs update
  - Dry run?
