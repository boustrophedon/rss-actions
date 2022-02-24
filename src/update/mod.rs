use std::collections::HashMap;
use std::process::ExitStatus;

use anyhow::{anyhow, Result, Context};
use chrono::prelude::*;
use reqwest::blocking::Client;
use rss::Channel;

use crate::{Feed, Filter};
use crate::db::RSSActionsTx;
use crate::UpdateOutput;

/// Stdout, Stderr, ExitStatus
pub type ProcessOutput = (String, String, ExitStatus);

static RSSACTIONS_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);

/// A wrapper class containing a validated RSS Feed entry with all relevant necessary data.
#[derive(Debug)]
struct FeedEntry {
    title: String,
    // We don't need to parse it into an actual URL since we don't ever fetch the resource.
    link: String,
    pub_date: DateTime<Utc>,
}

impl FeedEntry {
    pub fn new(entry: &rss::Item) -> Result<FeedEntry> {
        if entry.title.is_none() {
            return Err(anyhow!("Entry title is missing."));
        }
        if entry.link.is_none() {
            return Err(anyhow!("Entry link is missing."));
        }
        if entry.pub_date.is_none() {
            return Err(anyhow!("Entry pub date is missing."));
        }

        let pub_date = entry.pub_date.as_ref().unwrap();
        let pub_date_res = DateTime::parse_from_rfc2822(&*pub_date);
        if let Err(err) = pub_date_res {
            return Err(err)
                .context("Entry pub date did not parse correctly.");
        }
        let pub_date = pub_date_res.unwrap().into();

        Ok(FeedEntry {
            title: entry.title.as_ref().unwrap().clone(),
            link: entry.link.as_ref().unwrap().clone(),
            pub_date,
        })
    }
}

pub fn update(tx: &mut RSSActionsTx) -> Result<UpdateOutput> {
    // TODO instead of fetching all feeds and then all filters, could do join in db. maybe faster
    // maybe not, doesn't really matter to be honest.
    let feeds = tx.fetch_feeds()?;
    let filters = tx.fetch_filters()?;
    if filters.is_empty() {
        return Ok(UpdateOutput {
            executed_feeds: Vec::new(),
            executed_filters: Vec::new(),
            successes: 0,
            failures: 0,
            updates: 0,
        });
    }
    let mut filters_map = join_feeds_and_filters(&feeds, filters);

    let mut output = UpdateOutput {
        executed_feeds: Vec::new(),
        executed_filters: Vec::new(),
        successes: 0,
        failures: 0,
        updates: 0,
    };

    // Download feeds

    let download_results = download_feeds(feeds);
    // If all downloads resulted in an error, network is probably down.
    if download_results.iter().all(|(_, res)| res.is_err()) {
        return Err(anyhow!("All RSS feed downloads failed. Is the network down?"));
    }

    // Parse relevant data from downloaded RSS feeds

    let mut feed_data = Vec::<(Feed, Vec<FeedEntry>)>::new();
    // Otherwise, report errors individually for each download and immediately fail all relevant
    // filters.
    for (feed, res) in download_results {
        if let Ok(channel) = res {
            let parsed_items_res: Vec<_> = channel.items().iter().map(FeedEntry::new).collect();

            // If any entries are missing data, fail the whole feed
            // TODO: is this really the best idea? maybe just ignore the ones with missing data
            if parsed_items_res.iter().any(|res| res.is_err()) {
                if let Some(filters) = filters_map.remove(&feed.alias) {
                    output.failures += filters.len();
                    output.executed_feeds.push(
                        (feed.clone(),
                        Err(anyhow!("{} entries in feed {} had data errors",
                            parsed_items_res.iter().filter(|res| res.is_err()).count(),
                            &feed.alias))));
                }
                continue;
            }
            // No errors, can unwrap all and sort by pub date
            else {
                let mut entries = parsed_items_res.into_iter().map(|res| res.unwrap())
                    .collect::<Vec<FeedEntry>>();
                entries.sort_by_key(|entry| entry.pub_date);

                output.executed_feeds.push((feed.clone(), Ok(())));
                feed_data.push((feed, entries));
            }
        }
        else if let Err(err) = res {
            if let Some(filters) = filters_map.remove(&feed.alias) {
                output.failures += filters.len();
                output.executed_feeds.push((feed.clone(), Err(err)));
            }
            else {
                unreachable!("The feed for the channel was missing in the feeds-filters map.")
            }
        }
    }

    // For each feed, for each filter, process the feed's entries with the filter

    for (feed, entries) in feed_data {
        let filters = match filters_map.get(&feed.alias) {
            Some(filters) => filters,
            None => {
                // If the feed failed to download or there are no filters for the feed, just
                // continue.
                continue;
            }
        };

        let results = process_filters(filters, &entries);
        for (filter, res) in results {
            if let Ok((updated_filter, was_updated, script_outputs)) = res {

                // Originally I was going to only update filters if the script executed
                // successfully on all entries matched by the filter, but that would mean that if a
                // scripted succeeded on one entry and failed on the next, every time update was
                // run the script would be re-run on that entry, forcing scripts to be idempotent
                // to work in all cases.
                tx.update_filter(&updated_filter)?;

                if was_updated { output.updates += 1; }
                output.executed_filters.push((filter.clone(), Ok(script_outputs)));
                output.successes += 1;
            }
            else if let Err(err) = res {
                output.executed_filters.push((filter.clone(), Err(err)));
                output.failures += 1;
            }
        }
    }

    Ok(output)
}

/// Run filters' scripts on each entry that matches, and return filters with an updated
/// `last_updated` time if the filters' script successfully finishes, whether the filter was
/// actually updated, and the script's stdout and stderr.
#[allow(clippy::type_complexity)]
fn process_filters(filters: &[Filter], entries: &[FeedEntry])
        -> Vec<(Filter, Result<(Filter, bool, Vec<ProcessOutput>)>)> {

    filters.iter().map(|filter| {
        (filter.clone(), process_single_filter(filter, entries))
    })
    .collect()
}

/// Returns the filter with a possibly updated time, a bool indicating whether the filter was
/// updated, and the output of scripts run.
///
/// Currently the entire filter fails if the script fails on a single entry. This is because it's
/// easier but also because if we updated the filter's last_updated field there would be no way to
/// retry failed entries.
fn process_single_filter(filter: &Filter, entries: &[FeedEntry]) -> Result<(Filter, bool, Vec<ProcessOutput>)> {
    // The entries must be sorted by pub date for the most_recent_updated to be computed properly.
    assert!(entries.windows(2).all(|s| s[0].pub_date <= s[1].pub_date));

    let mut most_recent_updated = filter.last_updated;
    let mut script_outputs = Vec::new();
    for entry in entries {
        if filter.matches_keywords(&entry.title) &&
                (filter.last_updated.is_none() || filter.last_updated.unwrap() < entry.pub_date) {
            most_recent_updated = Some(entry.pub_date);

            let script_output = run_script(filter, entry)
                .with_context(|| format!("Script failed for filter on feed {}, keywords {}, script {}",
                        filter.alias, filter.keywords.join(", "), filter.script_path.to_string_lossy()))?;
            script_outputs.push(script_output);
        }
    }

    let was_updated = most_recent_updated != filter.last_updated;
    let updated_filter = {
        let mut updated_filter = filter.clone();
        if was_updated {
            if let Some(last_update) = most_recent_updated {
                updated_filter.update_time(last_update);
            }
        }
        updated_filter
    };
    Ok((updated_filter, was_updated, script_outputs))
}

/// Returns a pair of Strings (stdout, stderr) with the script's output if it succeeded, or an error message with the
/// script's output.
fn run_script(filter: &Filter, entry: &FeedEntry) -> Result<(String, String, ExitStatus)> {
    let process = std::process::Command::new(&filter.script_path)
        .env("RSSACTIONS_ENTRY_TITLE", entry.title.clone())
        .env("RSSACTIONS_ENTRY_URL", entry.link.clone())
        .env("RSSACTIONS_ENTRY_DATE", entry.pub_date.to_rfc2822())
        .output()?;

    let stdout = String::from_utf8_lossy(&process.stdout).into();
    let stderr = String::from_utf8_lossy(&process.stderr).into();

    if process.status.success() {
        Ok((stdout, stderr, process.status))
    }
    else {
        Err(anyhow!("Feed {} filter script {} failed", filter.alias, filter.script_path.to_string_lossy()))
            .with_context(|| format!("stdout: \n{}", stdout))
            .with_context(|| format!("stderr: \n{}", stderr))
    }
}

fn download_feeds(feeds: Vec<Feed>) -> Vec<(Feed, Result<Channel>)> {
    return feeds.into_iter()
        .map(|feed| {
            let res = download_single_feed(&feed);
            (feed, res)
        })
        .collect();
}

fn download_single_feed(feed: &Feed) -> Result<Channel> {
    let client = Client::builder()
        .user_agent(RSSACTIONS_USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build().unwrap();

    let response = client.get(feed.url.clone()).send()
        .with_context(|| format!("Failed to download {} rss feed from url {}", feed.alias, feed.url))?
        .bytes()
        .with_context(|| format!("Failed to download {} rss feed from url {}", feed.alias, feed.url))?;

    return Channel::read_from(&*response)
        .with_context(|| format!("Could not parse {} rss feed from url {}", feed.alias, feed.url));
}

/// This is just a join on feeds and filters where feed.alias = filter.alias.
/// We could do this at the database layer if we really wanted.
/// The output hashmap's key is the feed alias.
fn join_feeds_and_filters(feeds: &[Feed], filters: Vec<Filter>) -> HashMap<String, Vec<Filter>> {
    let mut filters_map: HashMap<String, Vec<Filter>> = HashMap::new();
    for feed in feeds {
        filters_map.insert(feed.alias.clone(), Vec::new());
    }
    for filter in filters.into_iter() {
        let alias = filter.alias.clone();
        match filters_map.get_mut(&alias) {
            Some(feed_filters) => feed_filters.push(filter),
            // This should never happen because the database would error first due to constraint
            // violations.
            None => unreachable!(format!("Missing feed {} for filter {:?}", alias, filter)),
        }
    }
    return filters_map;
}

// async fn download_feeds(feeds: &[Feed]) -> Vec<Result<Channel>> {
//     let client = reqwest::Client::builder()
//         .user_agent(RSSACTIONS_USER_AGENT)
//         .timeout(std::time::Duration::from_secs(30))
//         .build().unwrap();
//
//     let mut tasks: Vec<Pin<Box<dyn Future<Output = Result<(Feed, Bytes)>>>>> = Vec::new();
//     for feed in feeds {
//         tasks.push(
//             Box::pin(
//            client.get(feed.url).send()
//            .map(|resp_res| resp_res.context("Failed to download rss feed"))
//            .and_then(|resp| resp.bytes().map(|bytes| (feed, bytes)).into())
//            .boxed()
//             )
//         );
//     }
//     let results = futures::future::join_all(tasks).await
//         .into_iter()
//         .map(|bytes_res: Result<(Feed, Bytes)>| {
//             bytes_res.and_then(|(feed, bytes)| {
//                 Channel::read_from(&*bytes)
//                     .with_context(|| format!("Could not parse rss feed from url {}", feed.url))
//             })
//         })
//         .collect();
//
//
//     results
// }
