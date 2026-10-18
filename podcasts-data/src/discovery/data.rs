// data.rs
//
// Copyright 2022-2024 nee <nee-git@patchouli.garden>
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

use chrono::prelude::*;
use url::Url;

pub(crate) type UrlString = String;

#[derive(Clone, Debug)]
pub struct FoundPodcast {
    // --- Core Fields (Required by UI) ---
    pub feed: UrlString,
    pub title: String,
    pub author: String,
    pub description: String,
    pub art: UrlString, // High-res (600x600)
    pub episode_count: Option<i32>,
    pub last_publication: Option<DateTime<Local>>,

    // --- Enhanced Apple Metadata (Optional for fyyd) ---
    pub censored_name: Option<String>,
    pub primary_genre: Option<String>,
    pub genres: Option<Vec<String>>,
    pub genre_ids: Option<Vec<String>>,

    // --- Explicitness & Advisory ---
    pub is_explicit: Option<bool>,
    pub content_advisory_rating: Option<String>,

    // --- Links & Small Art ---
    pub apple_url: Option<String>,
    pub artwork_url_30: Option<String>,
    pub artwork_url_60: Option<String>,
    pub artwork_url_100: Option<String>,

    // --- Duration ---
    pub track_time_millis: Option<i64>,
}

/// checks for rougly the same feed url
impl PartialEq for FoundPodcast {
    fn eq(&self, other: &Self) -> bool {
        let a = Url::parse(&self.feed);
        let b = Url::parse(&other.feed);

        if let (Ok(a), Ok(b)) = (a, b) {
            a.path().trim_end_matches('/') == b.path().trim_end_matches('/')
                && a.host() == b.host()
                && a.query() == b.query()
        } else {
            self.feed == other.feed
        }
    }
}

impl FoundPodcast {
    /// use the longer description / bigger episode number
    pub(crate) fn combine(&mut self, other: FoundPodcast) {
        if other.episode_count.unwrap_or_default() > self.episode_count.unwrap_or_default() {
            self.episode_count = other.episode_count;
        }
        if other.description.len() > self.description.len() {
            self.description = other.description;
        }
    }
}

pub const ALL_PLATFORM_IDS: [&str; 2] = ["fyyd.de", "itunes.apple.com"];
