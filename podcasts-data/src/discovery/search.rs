// platform.rs
//
// Copyright 2022-2026 nee <nee-git@patchouli.garden>
// Copyright 2026 Frost Edson <frost@example.com> (Modified for extended metadata support)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::dbqueries;
use crate::discovery::data::*;
use crate::discovery::fyyd::FYYD;
use crate::discovery::itunes::Itunes;
use crate::discovery::platform_repo::Repository;
use anyhow::Result;
use thiserror::Error;
use tokio::join;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Network Error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Other Error: {0}")]
    AnyhowError(#[from] anyhow::Error),
    #[error("No Search Platform was selected.")]
    NoSearchPlatformsSelected,
}

/// Sends a http search to all platforms that are active in the settings.
/// It joins all results into a Vector and tries to filter out duplicates.
/// Results are sorted as they are returned by the Search platforms.
pub async fn search(query: &str) -> Result<Vec<FoundPodcast>, SearchError> {
    // This looks like it could be abstracted more,
    // but traits with async fns are impossible to deal with.
    let settings = dbqueries::get_discovery_settings();

    let fyyd_on = *settings.get("fyyd.de").unwrap_or(&false);
    let itunes_on = *settings.get("itunes.apple.com").unwrap_or(&false);

    if !fyyd_on && !itunes_on {
        return Err(SearchError::NoSearchPlatformsSelected);
    }

    let fyyd = FYYD::new();
    let itunes = Itunes::new();
    let (fyyd, itunes) = join!(fyyd.search(query), itunes.search(query));

    let fyyd: Vec<FoundPodcast> = fyyd.map_err(|e| error!("fyyd {e}")).unwrap_or_default();
    let itunes: Vec<FoundPodcast> = itunes.map_err(|e| error!("itunes {e}")).unwrap_or_default();

    trace!("combining {fyyd:#?} with {itunes:#?}");
    let mut merged = fyyd;
    merge_results(&mut merged, itunes);
    Ok(merged)
}

fn merge_results(merged: &mut Vec<FoundPodcast>, other_results: Vec<FoundPodcast>) {
    for p in other_results.into_iter() {
        if let Some(existing) = merged.iter_mut().find(|p2| p.eq(*p2)) {
            existing.combine(p);
        } else {
            merged.push(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to build a FoundPodcast with all new optional metadata fields defaulted.
    // This keeps test fixtures clean and isolates the test logic from future struct changes.
    fn make_test_podcast(
        feed: &str,
        title: &str,
        author: &str,
        description: &str,
        art: &str,
        episode_count: Option<i32>,
        last_publication: Option<chrono::DateTime<chrono::Local>>,
    ) -> FoundPodcast {
        FoundPodcast {
            feed: feed.to_string(),
            title: title.to_string(),
            author: author.to_string(),
            description: description.to_string(),
            art: art.to_string(),
            episode_count,
            last_publication,

            censored_name: None,
            primary_genre: None,
            genres: None,
            genre_ids: None,
            is_explicit: None,
            content_advisory_rating: None,
            apple_url: None,
            artwork_url_30: None,
            artwork_url_60: None,
            artwork_url_100: None,
            track_time_millis: None,
        }
    }

    #[test]
    fn merge() -> Result<()> {
        let itunes = vec![make_test_podcast(
            "https://feeds.fireside.fm/cornerspaeti/rss",
            "Corner Späti",
            "The Späti Boys",
            "",
            "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg",
            Some(347),
            Some(
                chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
            ),
        )];

        let fyyd = vec![make_test_podcast(
            "https://feeds.fireside.fm/cornerspaeti/rss",
            "Corner Späti",
            "The Späti Boys",
            "Weekly discussions of a deteriorating world all from the comfort of your local smoke-filled Spätkauf.\nhttps://www.patreon.com/cornerspaeti\nhttps://www.operationglad.io/start\n",
            "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg",
            Some(348),
            Some(
                chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
            ),
        )];

        let expected: Vec<FoundPodcast> = vec![make_test_podcast(
            "https://feeds.fireside.fm/cornerspaeti/rss",
            "Corner Späti",
            "The Späti Boys",
            "Weekly discussions of a deteriorating world all from the comfort of your local smoke-filled Spätkauf.\nhttps://www.patreon.com/cornerspaeti\nhttps://www.operationglad.io/start\n",
            "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg",
            Some(348),
            Some(
                chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
            ),
        )];

        let itunes2 = itunes.clone();
        let mut merged = itunes;
        merge_results(&mut merged, fyyd);
        assert_eq!(expected, merged);
        merge_results(&mut merged, itunes2);
        assert_eq!(expected, merged);
        Ok(())
    }

    #[test]
    fn merge_nones() -> Result<()> {
        let itunes = vec![make_test_podcast(
            "https://feeds.fireside.fm/cornerspaeti/rss",
            "Corner Späti",
            "The Späti Boys",
            "",
            "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg",
            None,
            None,
        )];

        let fyyd = vec![make_test_podcast(
            "https://feeds.fireside.fm/cornerspaeti/rss",
            "Corner Späti",
            "The Späti Boys",
            "Weekly discussions of a deteriorating world all from the comfort of your local smoke-filled Spätkauf.\nhttps://www.patreon.com/cornerspaeti\nhttps://www.operationglad.io/start\n",
            "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg",
            Some(348),
            Some(
                chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
            ),
        )];

        let expected: Vec<FoundPodcast> = vec![make_test_podcast(
            "https://feeds.fireside.fm/cornerspaeti/rss",
            "Corner Späti",
            "The Späti Boys",
            "Weekly discussions of a deteriorating world all from the comfort of your local smoke-filled Spätkauf.\nhttps://www.patreon.com/cornerspaeti\nhttps://www.operationglad.io/start\n",
            "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg",
            Some(348),
            Some(
                chrono::DateTime::parse_from_rfc3339("2024-03-07T10:00:00+01:00")
                    .unwrap()
                    .with_timezone(&chrono::Local),
            ),
        )];

        let itunes2 = itunes.clone();
        let mut merged = itunes;
        merge_results(&mut merged, fyyd);
        assert_eq!(expected, merged);
        merge_results(&mut merged, itunes2);
        assert_eq!(expected, merged);
        Ok(())
    }
}
