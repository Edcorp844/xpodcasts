use chrono::{DateTime, Local, TimeZone};
use podcasts_data::{discovery::FoundPodcast, downloader, errors::DataError};
use serde::Deserialize;
use url::Url;

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

// ---------- Lookup response (same schema as the search API) ----------

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
///
/// `country` is a lowercase ISO 3166-1 alpha-2 code (e.g. "ug", "us", "gb").
/// `limit` is clamped to 1..=100 by Apple.
pub async fn top_charts(country: &str, limit: u32) -> Result<Vec<FoundPodcast>, DataError> {
    let limit = limit.clamp(1, 100);

    let url = format!(
        "https://rss.marketingtools.apple.com/api/v2/{}/podcasts/top/{}/podcasts.json",
        country, limit
    );

    let client = downloader::client_builder().build()?;
    let response: ChartsResponse = client.get(&url).send().await?.json().await?;

    let mut out = Vec::with_capacity(response.feed.results.len());

    for entry in response.feed.results {
        match lookup_by_id(&entry.id).await {
            Ok(Some(podcast)) => {
                // Skip entries with no feed URL — can't subscribe to those.
                if !podcast.feed.is_empty() {
                    out.push(podcast);
                }
            }
            Ok(None) => {
                // Delisted or unavailable; skip.
            }
            Err(err) => {
                // Log and continue; one failure shouldn't kill the chart.
                eprintln!("lookup failed for {}: {}", entry.id, err);
            }
        }

        // Pace requests to stay under Apple's ~20/min rate limit.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }

    Ok(out)
}

/// Look up a single Apple Podcasts show by its numeric ID.
///
/// Returns `Ok(None)` if the ID isn't found.
pub async fn lookup_by_id(id: &str) -> Result<Option<FoundPodcast>, DataError> {
    let url = Url::parse_with_params(
        "https://itunes.apple.com/lookup",
        &[("id", id), ("entity", "podcast")],
    )?;

    let client = downloader::client_builder().build()?;
    let response: LookupResponse = client.get(url).send().await?.json().await?;

    Ok(response.results.into_iter().next().map(FoundPodcast::from))
}
