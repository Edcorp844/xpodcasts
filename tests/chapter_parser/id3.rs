use std::io::BufReader;

use anyhow::Result;
use xpodcasts::chapter_parser::{Chapter, id3::*};

#[test]
#[rustfmt::skip]
fn test_id3_newrustacean() -> Result<()> {
        // Examples taken from https://newrustacean.com/feed.xml Episode

        // USLT - Unsynced lyrics Tag
        let id3_bin: &[u8] = &[ 85, 83, 76, 84, 0, 0, 0, 132, 0, 0, 1, 101, 110, 103, 255, 254, 0, 0, 255, 254, 87, 0, 65, 0, 83, 0, 73, 0, 44, 0, 32, 0, 96, 0, 79, 0, 112, 0, 116, 0, 105, 0, 111, 0, 110, 0, 58, 0, 58, 0, 99, 0, 111, 0, 112, 0, 105, 0, 101, 0, 100, 0, 96, 0, 44, 0, 32, 0, 97, 0, 110, 0, 100, 0, 32, 0, 116, 0, 104, 0, 101, 0, 32, 0, 102, 0, 117, 0, 116, 0, 117, 0, 114, 0, 101, 0, 32, 0, 111, 0, 102, 0, 32, 0, 97, 0, 115, 0, 121, 0, 110, 0, 99, 0, 47, 0, 97, 0, 119, 0, 97, 0, 105, 0, 116, 0, 32, 0, 115, 0, 121, 0, 110, 0, 116, 0, 97, 0, 120, 0, 33, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin));
        assert!(chapter.is_err());

        // CTOC - Chapter Table of Contents
        let id3_bin: &[u8] = &[ 67, 84, 79, 67, 0, 0, 0, 41, 0, 0, 116, 111, 99, 0, 3, 7, 99, 104, 112, 48, 0, 99, 104, 112, 49, 0, 99, 104, 112, 50, 0, 99, 104, 112, 51, 0, 99, 104, 112, 52, 0, 99, 104, 112, 53, 0, 99, 104, 112, 54, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin));
        assert!(chapter.is_err());

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 44, 0, 0, 99, 104, 112, 48, 0, 0, 0, 0, 0, 0, 0, 89, 206, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 13, 0, 0, 1, 255, 254, 73, 0, 110, 0, 116, 0, 114, 0, 111, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp0".to_string(),
                title: "\u{feff}Intro".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(0, 0).unwrap(),
                end: chrono::Duration::new(22, 990000000).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 113, 0, 0, 99, 104, 112, 49, 0, 0, 0, 89, 206, 0, 1, 1, 208, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 33, 0, 0, 1, 255, 254, 83, 0, 112, 0, 111, 0, 110, 0, 115, 0, 111, 0, 114, 0, 58, 0, 32, 0, 80, 0, 97, 0, 114, 0, 105, 0, 116, 0, 121, 0, 87, 88, 88, 88, 0, 0, 0, 39, 0, 0, 0, 99, 104, 97, 112, 116, 101, 114, 32, 117, 114, 108, 0, 104, 116, 116, 112, 115, 58, 47, 47, 119, 119, 119, 46, 112, 97, 114, 105, 116, 121, 46, 105, 111, 47, 106, 111, 98, 115, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp1".to_string(),
                title: "\u{feff}Sponsor: Parity".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(22, 990000000).unwrap(),
                end: chrono::Duration::new(66, 0).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 68, 0, 0, 99, 104, 112, 50, 0, 0, 1, 1, 208, 0, 3, 59, 240, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 37, 0, 0, 1, 255, 254, 52, 0, 32, 0, 121, 0, 101, 0, 97, 0, 114, 0, 115, 0, 32, 0, 115, 0, 105, 0, 110, 0, 99, 0, 101, 0, 32, 0, 49, 0, 46, 0, 48, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp2".to_string(),
                title: "\u{feff}4 years since 1.0".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(66, 0).unwrap(),
                end: chrono::Duration::new(211, 952000000).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 52, 0, 0, 99, 104, 112, 51, 0, 0, 3, 59, 240, 0, 7, 122, 16, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 21, 0, 0, 1, 255, 254, 82, 0, 117, 0, 115, 0, 116, 0, 32, 0, 49, 0, 46, 0, 51, 0, 53, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp3".to_string(),
                title: "\u{feff}Rust 1.35".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(211, 952000000).unwrap(),
                end: chrono::Duration::new(490, 0).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 90, 0, 0, 99, 104, 112, 52, 0, 0, 7, 122, 16, 0, 15, 35, 0, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 59, 0, 0, 1, 255, 254, 70, 0, 105, 0, 110, 0, 97, 0, 108, 0, 32, 0, 96, 0, 97, 0, 115, 0, 121, 0, 110, 0, 99, 0, 96, 0, 47, 0, 96, 0, 97, 0, 119, 0, 97, 0, 105, 0, 116, 0, 96, 0, 32, 0, 115, 0, 121, 0, 110, 0, 116, 0, 97, 0, 120, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp4".to_string(),
                title: "\u{feff}Final `async`/`await` syntax".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(490, 0).unwrap(),
                end: chrono::Duration::new(992, 0).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 121, 0, 0, 99, 104, 112, 53, 0, 0, 15, 35, 0, 0, 15, 235, 140, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 35, 0, 0, 1, 255, 254, 80, 0, 97, 0, 116, 0, 114, 0, 101, 0, 111, 0, 110, 0, 32, 0, 83, 0, 112, 0, 111, 0, 110, 0, 115, 0, 111, 0, 114, 0, 115, 0, 87, 88, 88, 88, 0, 0, 0, 45, 0, 0, 0, 99, 104, 97, 112, 116, 101, 114, 32, 117, 114, 108, 0, 104, 116, 116, 112, 115, 58, 47, 47, 112, 97, 116, 114, 101, 111, 110, 46, 99, 111, 109, 47, 110, 101, 119, 114, 117, 115, 116, 97, 99, 101, 97, 110, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp5".to_string(),
                title: "\u{feff}Patreon Sponsors".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(992, 0).unwrap(),
                end: chrono::Duration::new(1043, 340000000).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 52, 0, 0, 99, 104, 112, 54, 0, 0, 15, 235, 140, 0, 16, 193, 16, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 21, 0, 0, 1, 255, 254, 83, 0, 104, 0, 111, 0, 119, 0, 32, 0, 105, 0, 110, 0, 102, 0, 111, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp6".to_string(),
                title: "\u{feff}Show info".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(1043, 340000000).unwrap(),
                end: chrono::Duration::new(1098, 0).unwrap(),
            },
            chapter
        );

        Ok(())
 }
