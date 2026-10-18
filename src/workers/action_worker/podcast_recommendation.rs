use chrono::{DateTime, Local, TimeZone};
use gtk::glib;
use podcasts_data::{discovery::FoundPodcast, errors::DataError};
use serde::Deserialize;
use url::Url;

// ---------- Custom Client ----------

/// Builds a client specifically for Apple's APIs.
/// We use a standard browser User-Agent to bypass Apple's WAF, 
/// which frequently blocks default Rust clients with 502/504 errors.
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

    #[serde(rename = "releaseDate")]
    pub release_date: Option<String>,

    #[serde(rename = "trackCount")]
    pub track_count: Option<i32>,

    #[serde(rename = "primaryGenreName")]
    pub primary_genre_name: Option<String>,

    pub country: Option<String>,
}

// ---------- Conversion ----------

impl From<LookupEntry> for FoundPodcast {
    fn from(entry: LookupEntry) -> Self {
        let feed = entry.feed_url.unwrap_or_default();

        let title = entry
            .collection_name
            .or(entry.track_name)
            .unwrap_or_default();

        let author = entry.artist_name.unwrap_or_default();

        let art = entry
            .artwork_url600
            .or(entry.artwork_url100)
            .unwrap_or_default();

        let last_publication = entry
            .release_date
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Local));

        FoundPodcast {
            feed,
            title,
            author,
            description: String::new(),
            art,
            episode_count: entry.track_count,
            last_publication,
        }
    }
}

// ---------- Client methods ----------

/// Fetch the Apple Podcasts top chart for a country.
pub async fn top_charts(country: &str, limit: u32) -> Result<Vec<FoundPodcast>, DataError> {
    let limit = limit.clamp(1, 100);

    let url = format!(
        "https://rss.marketingtools.apple.com/api/v2/{}/podcasts/top/{}/podcasts.json",
        country, limit
    );

    // Use our custom Apple client
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

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }

    Ok(out)
}

/// Look up a single Apple Podcasts show by its numeric ID.
pub async fn lookup_by_id(id: &str) -> Result<Option<FoundPodcast>, DataError> {
    let url = Url::parse_with_params(
        "https://itunes.apple.com/lookup",
        &[("id", id), ("entity", "podcast")],
    )?;

    // Use our custom Apple client here too
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