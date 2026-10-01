use std::collections::HashMap;

use anyhow::Result;
use xpodcasts::chapter_parser::{Chapter, extended_comment::parse_extended_comment};

#[test]
    #[rustfmt::skip]
    fn test_comment_newz() -> Result<()> {
        let comments = [
            "CHAPTER001=00:00:00.000",
            "CHAPTER001NAME=Intro",
            "CHAPTER002=00:00:14.640",
            "CHAPTER002NAME=Prologue",
            "CHAPTER003=00:03:47.967",
            "CHAPTER003NAME=US president accepts reality of election. Kind of.",
            "CHAPTER004=00:15:18.630",
            "CHAPTER004NAME=What damage can trump still cause",
            "CHAPTER005=00:22:11.746",
            "CHAPTER005NAME=Biden starts choosing his cabinet and announcing his policies",
            "CHAPTER006=00:33:43.575",
            "CHAPTER006NAME=Georgia Senate Runoffs",
            "CHAPTER007=00:45:02.492",
            "CHAPTER007NAME=Supremes make a covid-religion ruling where conservative judges show their stripes",
            "CHAPTER008=00:49:22.818",
            "CHAPTER008NAME=Countries struggles with consipiracy theories around Covid",
            "CHAPTER009=00:58:12.247",
            "CHAPTER009NAME=Poland and Hungary block EU's Gender Action Plan",
            "CHAPTER010=01:07:00.960",
            "CHAPTER010NAME=Brexit in the end stage",
            "CHAPTER011=01:16:29.643",
            "CHAPTER011NAME=Epilog",
            "CHAPTER012=01:17:06.308",
            "CHAPTER012NAME=Bonus Track",
        ];
        let expected_chapters = [
            Chapter {id: "001".to_string(),
                     title: "Intro".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(0, 0).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "002".to_string(),
                     title: "Prologue".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(14, 640000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "003".to_string(),
                     title: "US president accepts reality of election. Kind of.".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(227, 967000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "004".to_string(),
                     title: "What damage can trump still cause".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(918, 630000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "005".to_string(),
                     title: "Biden starts choosing his cabinet and announcing his policies".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(1331, 746000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "006".to_string(),
                     title: "Georgia Senate Runoffs".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(2023, 575000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "007".to_string(),
                     title: "Supremes make a covid-religion ruling where conservative judges show their stripes".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(2702, 492000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "008".to_string(),
                     title: "Countries struggles with consipiracy theories around Covid".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(2962, 818000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "009".to_string(),
                     title: "Poland and Hungary block EU's Gender Action Plan".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(3492, 247000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "010".to_string(),
                     title: "Brexit in the end stage".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(4020, 960000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "011".to_string(),
                     title: "Epilog".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(4589, 643000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            },
            Chapter {id: "012".to_string(),
                     title: "Bonus Track".to_string(),
                     description: "".to_string(),
                     start: chrono::Duration::new(4626, 308000000).unwrap(),
                     end: chrono::Duration::new(0, 0).unwrap(),
            }
        ];
        let mut chapters_map: HashMap<String, Chapter> = HashMap::new();
        for s in comments {
            parse_extended_comment(&mut chapters_map, &s);
        }

        let mut result: Vec<_> = chapters_map.into_values().collect();
        result.sort_by_key(|c| c.id.clone());

        assert_eq!(expected_chapters.len(), result.len());

        let mut i = 0;
        for chapter in result {
            assert_eq!(expected_chapters[i], chapter);
            i = i+1;
        }
        Ok(())
    }
