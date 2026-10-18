// fyyd.rs
//
// Copyright 2022-2024 nee <nee-git@patchouli.garden>
// Copyright 2026 Frost Edson <edsonchan6@gmail.com> (Modified for extended Fyyd metadata)
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

use crate::discovery::data::*;
use anyhow::Result;
use chrono::prelude::*;
use serde::Deserialize;
use url::Url;

// curl "https://api.fyyd.de/0.2/search/podcast?term=chapo&count=10"
#[derive(Deserialize, Debug)]
struct Response {
    data: Vec<Podcast>,
}

#[derive(Deserialize, Debug)]
struct Podcast {
    title: String,
    author: String,
    description: String,
    lastpub: Option<DateTime<Local>>,
    #[serde(rename = "xmlURL")]
    xml_url: UrlString, // the rss feed
    #[serde(rename = "smallImageURL")]
    small_image_url: UrlString, // 150px, next lower is thumbImageURL at 80px
    episode_count: i32,
}

impl From<Podcast> for FoundPodcast {
    fn from(p: Podcast) -> FoundPodcast {
        FoundPodcast {
            feed: p.xml_url,
            title: p.title,
            author: p.author,
            description: p.description,
            art: p.small_image_url,
            episode_count: Some(p.episode_count),
            last_publication: p.lastpub,

            // --- NEW FIELDS ---
            // fyyd does not provide these Apple-specific metadata fields,
            // so we default them to None. They will be populated if the 
            // same podcast is found via the Apple extension.
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
}

pub async fn search(query: &str, enabled: bool) -> Result<Vec<FoundPodcast>> {
    if !enabled {
        return Ok(vec![]);
    }
    let url = Url::parse_with_params(
        "https://api.fyyd.de/0.2/search/podcast?count=10",
        &[("term", query)],
    )?;
    let client = crate::downloader::client_builder().build()?;
    let result: Response = client.get(url).send().await?.json().await?;
    Ok(result.data.into_iter().map(|p| p.into()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to build a FoundPodcast with all new fields defaulted to None.
    // This prevents us from having to write `censored_name: None, ...` in every test fixture.
    fn make_test_podcast(
        feed: &str, title: &str, author: &str, description: &str, art: &str, 
        episode_count: i32, last_publication_str: &str
    ) -> FoundPodcast {
        FoundPodcast {
            feed: feed.to_string(),
            title: title.to_string(),
            author: author.to_string(),
            description: description.to_string(),
            art: art.to_string(),
            episode_count: Some(episode_count),
            last_publication: Some(chrono::DateTime::parse_from_rfc3339(last_publication_str).unwrap().with_timezone(&chrono::Local)),
            
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
    fn test_1_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/fyyd/search_1.json")?;
        let result: Response = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.data.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![
            make_test_podcast(
                "https://feeds.buzzsprout.com/1890340.rss",
                "The Deprogram",
                "JT, Hakim, and Yugopnik",
                "      What do an Iraqi, a Balkan Slav and a Texan have in common? A burning hatred for the system. Oh, and a podcast. Say no to eating out of the trash can of ideology. Join us on a journey exploring and critically assessing the perceived “normalcy” of late-stage capitalism. The only truly international, global, and anti-capitalist podcast you’ll find. SUPPORT US on PATREON: https://www.patreon.com/TheDeprogram FOLLOW US on Twitter @TheDeprogramPod    ",
                "https://img-1.fyyd.de/pd/small/7733270e232241983f50e3e88665b72cfa4f5.jpg",
                242,
                "2024-03-08T13:00:00+01:00"
            )
        ];
        assert_eq!(1, found.len());
        assert_eq!(expected, found);
        Ok(())
    }

    #[test]
    fn test_5_results() -> Result<()> {
        let input = std::fs::read_to_string("tests/fyyd/search_5.json")?;
        let result: Response = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.data.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![
            make_test_podcast(
                "https://feeds.acast.com/public/shows/7540dfb3-3c5f-43ae-91d3-aad22b2ede46",
                "Chapo",
                "VICE",
                "As Sinaloa cartel leader Joaquín “El Chapo” Guzmán goes on trial, VICE News explores his&nbsp;high-stakes case through the stories of people caught up in the drug war in the U.S. and Mexico. Hosted on Acast. See acast.com/privacy for more information.",
                "https://img-1.fyyd.de/pd/small/8374473344d689aada87d46655bb0d1e89028.jpg",
                17,
                "2019-02-22T23:19:05+01:00"
            ),
            make_test_podcast(
                "https://feeds.soundcloud.com/users/soundcloud:users:211911700/sounds.rss",
                "Chapo Trap House",
                "Chapo Trap House",
                "Podcast by Chapo Trap House",
                "https://img-1.fyyd.de/pd/small/58282c0c842fc16e73e519ff141e856b175f8.jpg",
                792,
                "2024-03-08T08:01:37+01:00"
            ),
            make_test_podcast(
                "https://www.omnycontent.com/d/playlist/e73c998e-6e60-432f-8610-ae210140c5b1/a6b57093-81fe-4401-a963-af2000fe4e3a/56b7926c-6bfd-4d8c-81f9-af2000ffad49/podcast.rss",
                "Surviving El Chapo: The Twins Who Brought Down A Drug Lord",
                "iHeartPodcasts",
                "Identical twins Jay and Pete Flores, who were once North America’s biggest drug traffickers and El Chapo’s right hand men, turned themselves into the U.S. government with the hopes of starting a new, safer life for their family. But after years of cooperating to get the world's most powerful drug kingpin behind bars, and finally gaining their freedom with a chance to start again, everything for the Flores family began to unravel. In Season 2 of Surviving El Chapo, hosts Curtis \"50 Cent\" Jackson and Charlie Webster hear Jay and Pete reveal for the first time what really happened during their turbulent 14-year prison journey and what it was like to come face-to-face in court with El Chapo. Plus, find out the shocking backstory to the prison sentence that the Flores wives are currently facing.\n\nHosted and executive produced by award-winning artist and producer Curtis \"50 Cent\" Jackson and critically acclaimed broadcast journalist and producer Charlie Webster. Brought to you by Lionsgate Sound as a world exclusive with iHeartPodcasts.",
                "https://img-1.fyyd.de/pd/small/80419996d728f3c44a832def2cffbae93e380.jpg",
                26,
                "2023-12-06T09:00:00+01:00"
            ),
            make_test_podcast(
                "https://feeds.buzzsprout.com/350771.rss",
                "People's History of Ideas Podcast",
                "Matthew Rothwell",
                "In this podcast, Matthew Rothwell, author of Transpacific Revolutionaries: The Chinese Revolution in Latin America, explores the global history of ideas related to rebellion and revolution. The main focus of this podcast for the near future will be on the history of the Chinese Revolution, going all the way back to its roots in the initial Chinese reactions to British imperialism during the Opium War of 1839-1842, and then following the development of the revolution and many of the ideas that were products of the revolution through to their transnational diffusion in the late 20th century.",
                "https://img-1.fyyd.de/pd/small/8442586e8539fdf80c9748cea30f43e638922.jpg",
                112,
                "2024-02-04T21:00:00+01:00"
            ),
            make_test_podcast(
                "https://badfaith.libsyn.com/rss",
                "Bad Faith",
                "Briahna Joy Gray & Virgil Texas",
                "America's only podcast. //\r\n\r\nwith Briahna Joy Gray, former National Press Secretary for Bernie Sanders' Presidential campaign //\r\n\r\nand Virgil Texas //\r\n\r\nSubscribe for exclusive premium episodes at patreon.com/badfaithpodcast /\r\n@badfaithpod / \r\nbadfaithpodcast at gmail dot com",
                "https://img-1.fyyd.de/pd/small/6132904ae864f41dbbb60eaf1054cbfcc6292.jpg",
                372,
                "2024-03-07T11:44:00+01:00"
            )
        ];
        assert_eq!(5, found.len());
        assert_eq!(expected, found);
        Ok(())
    }

    #[test]
    fn empty_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/fyyd/search_empty.json")?;
        let result: Response = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.data.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![];
        assert_eq!(expected, found);
        Ok(())
    }

    #[test]
    fn unicode_result() -> Result<()> {
        let input = std::fs::read_to_string("tests/fyyd/search_unicode.json")?;
        let result: Response = serde_json::from_str(&input)?;
        let found: Vec<FoundPodcast> = result.data.into_iter().map(|p| p.into()).collect();
        let expected: Vec<FoundPodcast> = vec![
            make_test_podcast(
                "https://feeds.fireside.fm/cornerspaeti/rss",
                "Corner Späti",
                "The Späti Boys",
                "Weekly discussions of a deteriorating world all from the comfort of your local smoke-filled Spätkauf.\nhttps://www.patreon.com/cornerspaeti\nhttps://www.operationglad.io/start\n",
                "https://img-1.fyyd.de/pd/small/77182e877ac679f9414148fab3dddb74782c2.jpg",
                348,
                "2024-03-07T10:00:00+01:00"
            )
        ];
        assert_eq!(expected, found);
        Ok(())
    }
}