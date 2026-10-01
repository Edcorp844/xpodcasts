// extended_comment.rs
//
// Copyright 2025 nee <nee-git@patchouli.garden>
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

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::chapter_parser::Chapter;

/// Parses comment and writes updates into the chapters HashMap.
/// A chapter comment can either contain the title, or the timestamp.
/// So parsing of one chapter has to be done over multiple comment tags.
pub fn parse_extended_comment(
    chapters: &mut HashMap<String, Chapter>,
    comment: &str,
) -> Option<()> {
    //     "CHAPTER002NAME=Prologue"
    static RE_NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^CHAPTER([0-9]+)NAME=(.+)$").unwrap());
    let matches = RE_NAME.captures_iter(comment).next();
    if let Some(matches) = matches {
        let id = matches.get(1)?.as_str().to_string();
        let title = matches.get(2)?.as_str().to_string();
        let chapter = get_or_init(chapters, id.clone());
        chapter.id = id;
        chapter.title = title;
        return Some(());
    }

    //     "CHAPTER002=00:00:14.640"
    static RE_TIME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"CHAPTER([0-9]+)=([0-9]+):([0-9]+):([0-9\.]+)").unwrap());
    let matches = RE_TIME.captures_iter(comment).next();
    if let Some(matches) = matches {
        let id = matches.get(1)?.as_str().to_string();
        let hours = matches.get(2)?.as_str().parse::<i64>().ok()?;
        let minutes = matches.get(3)?.as_str().parse::<i64>().ok()?;
        let seconds = matches.get(4)?.as_str().parse::<f64>().ok()?;

        let chapter = get_or_init(chapters, id.clone());
        chapter.id = id;
        chapter.start = chrono::Duration::hours(hours)
            + chrono::Duration::minutes(minutes)
            + chrono::Duration::from_std(std::time::Duration::from_secs_f64(seconds)).ok()?;
        return Some(());
    }
    None
}

/// Gets an existing chapter from the hashmap, or initalizes a new one in it.
fn get_or_init(chapters: &mut HashMap<String, Chapter>, key: String) -> &mut Chapter {
    if chapters.contains_key(&key) {
        return chapters.get_mut(&key).unwrap();
    }
    chapters.insert(key.clone(), Chapter::default());
    chapters.get_mut(&key).unwrap()
}
