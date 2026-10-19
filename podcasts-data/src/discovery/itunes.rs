// itunes.rs
//
// Copyright 2022-2024 nee <nee-git@patchouli.garden>
// Copyright 2026 Frost Edson <edsonchan6@gmail.com> (Modified for extended Apple metadata)
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

use crate::{
    dbqueries,
    discovery::{data::*, platform_repo::Repository},
};
use anyhow::Result;
use chrono::prelude::*;
use serde::Deserialize;
use url::Url;

// ==========================================
// 1. Data Structures
// ==========================================

#[derive(Deserialize)]
struct SearchResult {
    results: Vec<Podcast>,
}

#[derive(Deserialize)]
struct Podcast {
    #[serde(rename = "feedUrl")]
    feed_url: UrlString,
    #[serde(rename = "collectionName")]
    collection_name: String,
    #[serde(rename = "artistName")]
    artist_name: String,
    #[serde(rename = "releaseDate")]
    release_date: Option<DateTime<Local>>,
    #[serde(rename = "trackCount")]
    track_count: Option<i32>,
    #[serde(rename = "artworkUrl100")]
    artwork_url_100: UrlString,
    #[serde(rename = "artworkUrl600")]
    artwork_url_600: Option<UrlString>,
    #[serde(rename = "artworkUrl30")]
    artwork_url_30: Option<UrlString>,
    #[serde(rename = "artworkUrl60")]
    artwork_url_60: Option<UrlString>,

    #[serde(rename = "collectionCensoredName")]
    collection_censored_name: Option<String>,
    #[serde(rename = "primaryGenreName")]
    primary_genre_name: Option<String>,
    #[serde(rename = "genres", default)]
    genres: Option<Vec<String>>,
    #[serde(rename = "genreIds", default)]
    genre_ids: Option<Vec<String>>,
    #[serde(rename = "collectionExplicitness")]
    collection_explicitness: Option<String>,
    #[serde(rename = "contentAdvisoryRating")]
    content_advisory_rating: Option<String>,
    #[serde(rename = "collectionViewUrl")]
    collection_view_url: Option<String>,
    #[serde(rename = "trackTimeMillis")]
    track_time_millis: Option<i64>,
}

impl From<Podcast> for FoundPodcast {
    fn from(p: Podcast) -> FoundPodcast {
        let is_explicit = p
            .collection_explicitness
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("explicit"));

        FoundPodcast {
            feed: p.feed_url,
            title: p.collection_name,
            author: p.artist_name,
            description: "".to_string(),
            art: p.artwork_url_600.unwrap_or(p.artwork_url_100.clone()),
            episode_count: p.track_count,
            last_publication: p.release_date,

            censored_name: p.collection_censored_name,
            primary_genre: p.primary_genre_name,
            genres: p.genres,
            genre_ids: p.genre_ids,
            is_explicit,
            content_advisory_rating: p.content_advisory_rating,
            apple_url: p.collection_view_url,
            artwork_url_30: p.artwork_url_30,
            artwork_url_60: p.artwork_url_60,
            artwork_url_100: Some(p.artwork_url_100),
            track_time_millis: p.track_time_millis,
        }
    }
}

// ==========================================
// 2. The Itunes Object
// ==========================================

pub struct Itunes {
    base_url: String,
    enabled: bool,
}

impl Itunes {
    pub fn new() -> Self {
        let settings = dbqueries::get_discovery_settings();
        let itunes_on = *settings.get("itunes.apple.com").unwrap_or(&false);

        Self {
            base_url: "https://itunes.apple.com".to_string(),
            enabled: itunes_on,
        }
    }

    /// Look up a single Apple Podcasts show by its numeric ID.
    pub async fn lookup_by_id(&self, id: &str) -> Result<Option<FoundPodcast>> {
        if !self.enabled {
            return Ok(None);
        }

        let url = Url::parse_with_params(
            &format!("{}/lookup", self.base_url),
            &[("id", id), ("entity", "podcast")],
        )?;

        let client = crate::downloader::client_builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .build()?;

        let resp = client.get(url).send().await?;

        if !resp.status().is_success() {
            eprintln!("iTunes Lookup API returned status {}", resp.status());
            return Ok(None);
        }

        let response: SearchResult = match resp.json().await {
            Ok(json) => json,
            Err(err) => {
                eprintln!("Failed to parse lookup JSON: {}", err);
                return Ok(None);
            }
        };

        Ok(response.results.into_iter().next().map(FoundPodcast::from))
    }

    /// Fetch the Apple Podcasts top charts for a specific country.
    pub async fn get_top_charts(&self, country: &str, limit: u32) -> Result<Vec<FoundPodcast>> {
        if !self.enabled {
            return Ok(vec![]);
        }

        let limit = limit.clamp(1, 100);
        let url = format!(
            "https://rss.marketingtools.apple.com/api/v2/{}/podcasts/top/{}/podcasts.json",
            country, limit
        );

        let client = crate::downloader::client_builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .build()?;

        let resp = client.get(&url).send().await?;

        if !resp.status().is_success() {
            eprintln!(
                "Apple Charts API returned status {} for {}",
                resp.status(),
                url
            );
            return Ok(vec![]);
        }

        #[derive(Deserialize)]
        struct ChartsResponse {
            feed: ChartsFeed,
        }
        #[derive(Deserialize)]
        struct ChartsFeed {
            results: Vec<ChartEntry>,
        }
        #[derive(Deserialize)]
        struct ChartEntry {
            id: String,
        }

        let response: ChartsResponse = match resp.json().await {
            Ok(json) => json,
            Err(err) => {
                eprintln!("Failed to parse chart JSON: {}", err);
                return Ok(vec![]);
            }
        };

        let mut out = Vec::with_capacity(response.feed.results.len());

        for entry in response.feed.results {
            match self.lookup_by_id(&entry.id).await {
                Ok(Some(podcast)) => {
                    if !podcast.feed.is_empty() {
                        out.push(podcast);
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    eprintln!("lookup failed for {}: {}", entry.id, err);
                }
            }
            // Pacing for Apple API rate limits
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }

        Ok(out)
    }
}

// ==========================================
// 3. Repository Trait Implementation
// ==========================================

impl Repository for Itunes {
    async fn search(&self, query: &str) -> Result<Vec<FoundPodcast>> {
        if !self.enabled {
            return Ok(vec![]);
        }

        let url = Url::parse_with_params(
            &format!("{}/search?entity=podcast&limit=10", self.base_url),
            &[("term", query)],
        )?;

        let client = crate::downloader::client_builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .build()?;

        let resp = client.get(url).send().await?;

        if !resp.status().is_success() {
            eprintln!("iTunes Search API returned status {}", resp.status());
            return Ok(vec![]);
        }

        let result: SearchResult = resp.json().await?;
        Ok(result.results.into_iter().map(|p| p.into()).collect())
    }

    async fn get_top(&self, limit: i32, language: Option<&str>) -> Result<Vec<FoundPodcast>> {
        let country = language.unwrap_or("us");

        self.get_top_charts(country, limit as u32).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to construct a FoundPodcast with all optional Apple fields set to None.
    // This keeps the test fixtures clean and readable.
    fn make_test_podcast(
        feed: &str,
        title: &str,
        author: &str,
        art: &str,
        episode_count: i32,
        release_date: &str,
    ) -> FoundPodcast {
        FoundPodcast {
            feed: feed.to_string(),
            title: title.to_string(),
            author: author.to_string(),
            description: "".to_string(),
            art: art.to_string(),
            episode_count: Some(episode_count),
            last_publication: Some(
                chrono::DateTime::parse_from_rfc3339(release_date)
                    .unwrap()
                    .with_timezone(&chrono::Local),
            ),

            censored_name: None,
            primary_genre: None,
            genres: None,
            genre_ids: None,
            is_explicit: None,
            content_advisory_rating: None,
            apple_url: None,
            artwork_url_30: None,
            artwork_url_60: None,
            artwork_url_100: Some(art.to_string()),
            track_time_millis: None,
        }
    }

    #[test]
    fn test_1_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/itunes/search_1.txt")?;
        let result: SearchResult = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.results.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![make_test_podcast(
            "https://anchor.fm/s/19346b24/podcast/rss",
            "CushVlogs Audio - Matt Christman - Chapo Trap House",
            "Jackson Jacker",
            "https://is1-ssl.mzstatic.com/image/thumb/Podcasts124/v4/74/41/7c/74417c8a-151f-5090-5460-fe5e7ca0e671/mza_7528059777000521713.jpg/100x100bb.jpg",
            167,
            "2021-06-08T06:56:00+02:00",
        )];
        assert_eq!(1, found.len());
        assert_eq!(expected, found);
        Ok(())
    }

    #[test]
    fn test_10_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/itunes/search_10.txt")?;
        let result: SearchResult = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.results.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![
            make_test_podcast(
                "https://feeds.acast.com/public/shows/7540dfb3-3c5f-43ae-91d3-aad22b2ede46",
                "Chapo",
                "VICE",
                "https://is1-ssl.mzstatic.com/image/thumb/Podcasts116/v4/58/9b/8d/589b8d72-13aa-55cb-463f-ad41096ca9ea/mza_560982554884614556.jpg/100x100bb.jpg",
                17,
                "2023-01-25T06:01:00+01:00",
            ),
            make_test_podcast(
                "https://feeds.soundcloud.com/users/soundcloud:users:211911700/sounds.rss",
                "Chapo Trap House",
                "Chapo Trap House",
                "https://is1-ssl.mzstatic.com/image/thumb/Podcasts18/v4/37/d9/a0/37d9a0b4-64f8-70d4-722e-b3a8772f3424/mza_5335192259855381405.jpg/100x100bb.jpg",
                507,
                "2024-03-05T21:55:00+01:00",
            ),
            make_test_podcast(
                "https://feeds.megaphone.fm/WMHY2717952910",
                "El Chapo: Dos rostros de un capo Podcast",
                "CNN en Español",
                "https://is1-ssl.mzstatic.com/image/thumb/Podcasts122/v4/31/e9/bb/31e9bb57-31f8-447d-e619-bf94e2ee42af/mza_7685287012963567517.jpg/100x100bb.jpg",
                7,
                "2020-07-15T11:10:00+02:00",
            ),
            make_test_podcast(
                "https://anchor.fm/s/19346b24/podcast/rss",
                "CushVlogs Audio - Matt Christman - Chapo Trap House",
                "Jackson Jacker",
                "https://is1-ssl.mzstatic.com/image/thumb/Podcasts124/v4/74/41/7c/74417c8a-151f-5090-5460-fe5e7ca0e671/mza_7528059777000521713.jpg/100x100bb.jpg",
                167,
                "2021-06-08T06:56:00+02:00",
            ),
            make_test_podcast(
                "https://audioboom.com/channels/4905580.rss",
                "‘El Chapo’: ¿héroe o villano?",
                "Univision",
                "https://is1-ssl.mzstatic.com/image/thumb/Podcasts116/v4/57/34/8e/57348e38-6c9a-7579-8269-42e61786ae3b/mza_17242328811639650011.jpg/100x100bb.jpg",
                4,
                "2017-05-23T02:59:00+02:00",
            ),
            make_test_podcast(
                "https://anchor.fm/s/5b423b40/podcast/rss",
                "Chapo",
                "Jack Gold",
                "https://is1-ssl.mzstatic.com/image/thumb/Podcasts125/v4/43/67/d3/4367d309-707e-6b7e-390a-202565c1d530/mza_9950196886709505935.jpg/100x100bb.jpg",
                1,
                "2021-05-15T07:32:00+02:00",
            ),
            make_test_podcast(
                "https://anchor.fm/s/1e56647c/podcast/rss",
                "Chapo",
                "Mijo 713",
                "https://is1-ssl.mzstatic.com/image/thumb/Podcasts123/v4/b5/7c/41/b57c410f-8b2f-d9ce-df76-44b5947ea261/mza_17707437198336862.jpg/100x100bb.jpg",
                1,
                "2020-04-25T21:46:00+02:00",
            ),
            make_test_podcast(
                "https://www.omnycontent.com/d/playlist/e73c998e-6e60-432f-8610-ae210140c5b1/a6b57093-81fe-4401-a963-af2000fe4e3a/56b7926c-6bfd-4d8c-81f9-af2000ffad49/podcast.rss",
                "Surviving El Chapo: The Twins Who Brought Down A Drug Lord",
                "iHeartPodcasts",
                "https://is1-ssl.mzstatic.com/image/thumb/Podcasts116/v4/aa/90/0f/aa900f15-7a67-e498-7b8c-a8efc9a667b5/mza_18039635476170983620.jpg/100x100bb.jpg",
                26,
                "2023-12-06T09:00:00+01:00",
            ),
            make_test_podcast(
                "https://anchor.fm/s/3a15ad8/podcast/rss",
                "CHAPOS Corner",
                "Chapo",
                "https://is1-ssl.mzstatic.com/image/thumb/Podcasts125/v4/a5/08/da/a508da5c-09c7-bc4d-3c2d-938bf406b36b/mza_14501171467539691063.jpg/100x100bb.jpg",
                704,
                "2022-10-19T02:24:00+02:00",
            ),
            make_test_podcast(
                "https://anchor.fm/s/5d83aab0/podcast/rss",
                "EL CHAPO",
                "Bernardo",
                "https://is1-ssl.mzstatic.com/image/thumb/Podcasts115/v4/df/3b/2c/df3b2cbc-a8ac-315b-d5db-56d055690dfc/mza_8387652467730745876.jpg/100x100bb.jpg",
                10,
                "2021-06-22T23:50:00+02:00",
            ),
        ];
        assert_eq!(10, found.len());
        assert_eq!(expected, found);
        Ok(())
    }

    #[test]
    fn empty_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/itunes/search_empty.txt")?;
        let result: SearchResult = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.results.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![];
        assert_eq!(expected, found);
        Ok(())
    }

    #[test]
    fn unicode_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/itunes/search_unicode.txt")?;
        let result: SearchResult = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.results.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![make_test_podcast(
            "https://feeds.fireside.fm/cornerspaeti/rss",
            "Corner Späti",
            "The Späti Boys",
            "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg",
            348,
            "2024-03-07T10:00:00+01:00",
        )];
        assert_eq!(expected, found);
        Ok(())
    }
}
