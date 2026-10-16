use podcasts_data::{Episode, ShowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchActionResultCategory {
    Podcast,
    Library,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Subscribe(String),
    Search(String, SearchActionResultCategory),
    FetchHomeFeed,
    FetchDownloads,
    FetchShows,
    FetchRecents,
    FetchShowEpisodes(ShowId),
    QuickSyncNextcloud,
    RefreshAllViews,
}

#[derive(Debug, Clone)]
pub struct HomeFeedData {
    unfinished: Vec<Episode>,
}

impl HomeFeedData {
    pub fn new(unfinished: Vec<Episode>) -> Self {
        Self { unfinished }
    }

    pub fn get_unfineshedEpisodes(&self) -> Vec<Episode> {
        self.unfinished.clone()
    }
}
