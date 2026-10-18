
use chrono::{DateTime, Local, TimeZone};
use gtk::glib;
use podcasts_data::{discovery::FoundPodcast, errors::DataError};
use serde::Deserialize;
use url::Url;

// ---------- Custom Client ----------

fn apple_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

// ---------- Charts response ----------

#[derive(Debug, Deserialize)]
pub struct ChartsResponse {
    pub feed: ChartsFeed,
}

#[derive(Debug, Deserialize)]
pub struct ChartsFeed {
    pub country: String,
    pub results: Vec<ChartEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ChartEntry {
    pub id: String,
    pub name: String,
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "artworkUrl100")]
    pub artwork_url100: String,
    pub url: String,
    #[serde(default)]
    pub genres: Vec<ChartGenre>,
}

#[derive(Debug, Deserialize)]
pub struct ChartGenre {
    #[serde(rename = "genreId")]
    pub genre_id: String,
    pub name: String,
}

// ---------- Lookup response ----------

#[derive(Debug, Deserialize)]
pub struct LookupResponse {
    #[serde(rename = "resultCount")]
    pub result_count: u32,
    pub results: Vec<LookupEntry>,
}

#[derive(Debug, Deserialize)]
pub struct LookupEntry {
    #[serde(rename = "wrapperType")]
    pub wrapper_type: Option<String>,
    pub kind: Option<String>,

    #[serde(rename = "collectionId")]
    pub collection_id: Option<u64>,
    #[serde(rename = "trackId")]
    pub track_id: Option<u64>,

    #[serde(rename = "artistName")]
    pub artist_name: Option<String>,
    #[serde(rename = "collectionName")]
    pub collection_name: Option<String>,
    #[serde(rename = "trackName")]
    pub track_name: Option<String>,

    #[serde(rename = "feedUrl")]
    pub feed_url: Option<String>,

    #[serde(rename = "artworkUrl600")]
    pub artwork_url600: Option<String>,
    #[serde(rename = "artworkUrl100")]
    pub artwork_url100: Option<String>,
    #[serde(rename = "artworkUrl30")]
    pub artwork_url30: Option<String>,
    #[serde(rename = "artworkUrl60")]
    pub artwork_url60: Option<String>,

    #[serde(rename = "releaseDate")]
    pub release_date: Option<String>,

    #[serde(rename = "trackCount")]
    pub track_count: Option<i32>,

    #[serde(rename = "primaryGenreName")]
    pub primary_genre_name: Option<String>,
    #[serde(rename = "genres", default)]
    pub genres: Option<Vec<String>>,
    #[serde(rename = "genreIds", default)]
    pub genre_ids: Option<Vec<String>>,

    #[serde(rename = "collectionCensoredName")]
    pub collection_censored_name: Option<String>,
    #[serde(rename = "collectionExplicitness")]
    pub collection_explicitness: Option<String>,
    #[serde(rename = "contentAdvisoryRating")]
    pub content_advisory_rating: Option<String>,
    #[serde(rename = "collectionViewUrl")]
    pub collection_view_url: Option<String>,
    #[serde(rename = "trackTimeMillis")]
    pub track_time_millis: Option<i64>,

    pub country: Option<String>,
}

// ---------- Conversion ----------

impl From<LookupEntry> for FoundPodcast {
    fn from(entry: LookupEntry) -> Self {
        let feed = entry.feed_url.unwrap_or_default();
        let title = entry.collection_name.or(entry.track_name).unwrap_or_default();
        let author = entry.artist_name.unwrap_or_default();
        let art = entry.artwork_url600.clone().or(entry.artwork_url100.clone()).unwrap_or_default();

        let last_publication = entry
            .release_date
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Local));

        let is_explicit = entry.collection_explicitness
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("explicit"));

        FoundPodcast {
            feed,
            title,
            author,
            description: String::new(),
            art,
            episode_count: entry.track_count,
            last_publication,
            
            censored_name: entry.collection_censored_name,
            primary_genre: entry.primary_genre_name,
            genres: entry.genres,
            genre_ids: entry.genre_ids,
            is_explicit,
            content_advisory_rating: entry.content_advisory_rating,
            apple_url: entry.collection_view_url,
            artwork_url_30: entry.artwork_url30,
            artwork_url_60: entry.artwork_url60,
            artwork_url_100: entry.artwork_url100,
            track_time_millis: entry.track_time_millis,
        }
    }
}

// ---------- Client methods ----------

pub async fn top_charts(country: &str, limit: u32) -> Result<Vec<FoundPodcast>, DataError> {
    let limit = limit.clamp(1, 100);

    let url = format!(
        "https://rss.marketingtools.apple.com/api/v2/{}/podcasts/top/{}/podcasts.json",
        country, limit
    );

    let client = apple_client();
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        eprintln!("Apple Charts API returned status {} for {}", resp.status(), url);
        return Ok(Vec::new());
    }

    let response: ChartsResponse = match resp.json().await {
        Ok(json) => json,
        Err(err) => {
            eprintln!("Failed to parse chart JSON: {}", err);
            return Ok(Vec::new());
        }
    };

    let mut out = Vec::with_capacity(response.feed.results.len());

    for entry in response.feed.results {
        match lookup_by_id(&entry.id).await {
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
        // Pacing for Apple API
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }

    Ok(out)
}

pub async fn lookup_by_id(id: &str) -> Result<Option<FoundPodcast>, DataError> {
    let url = Url::parse_with_params(
        "https://itunes.apple.com/lookup",
        &[("id", id), ("entity", "podcast")],
    )?;

    println!("url: {}", url.as_str());

    let client = apple_client();
    let resp = client.get(url).send().await?;

    if !resp.status().is_success() {
        eprintln!("iTunes Lookup API returned status {}", resp.status());
        return Ok(None);
    }

    let response: LookupResponse = match resp.json().await {
        Ok(json) => json,
        Err(err) => {
            eprintln!("Failed to parse lookup JSON: {}", err);
            return Ok(None);
        }
    };

    Ok(response.results.into_iter().next().map(FoundPodcast::from))
}

pub fn current_country() -> String {
    let env_vars = ["LANG", "LC_ALL", "LC_MESSAGES", "LANGUAGE"];

    for var in env_vars {
        if let Ok(val) = std::env::var(var) {
            let base = val.split('.').next().unwrap_or(&val);
            let base = base.split('@').next().unwrap_or(base);

            if let Some((_, region)) = base.split_once('_') {
                if region.len() == 2 && region.chars().all(|c| c.is_ascii_alphabetic()) {
                    return region.to_lowercase();
                }
            }
        }
    }

    for lang in glib::language_names() {
        let s = lang.to_string();
        let base = s.split('.').next().unwrap_or(&s);
        if let Some((_, region)) = base.split_once('_') {
            if region.len() == 2 && region.chars().all(|c| c.is_ascii_alphabetic()) {
                return region.to_lowercase();
            }
        }
    }

    "us".to_string()
}
