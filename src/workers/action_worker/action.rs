use podcasts_data::{discovery::FoundPodcast, Episode, ShowId};

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
    missed: Vec<Episode>,
    itunes_top_charts: Vec<FoundPodcast>,
    fyyd_top_charts: Vec<FoundPodcast>,
}

impl HomeFeedData {
    pub fn new(
        unfinished: Vec<Episode>,
        missed: Vec<Episode>,
        itunes_top_charts: Vec<FoundPodcast>,
        fyyd_top_charts: Vec<FoundPodcast>,
    ) -> Self {
        Self {
            unfinished,
            missed,
            itunes_top_charts,
            fyyd_top_charts,
        }
    }

    pub fn get_unfineshed_episodes(&self) -> Vec<Episode> {
        self.unfinished.clone()
    }

    pub fn get_itunes_top_charts(&self) -> Vec<FoundPodcast> {
        self.itunes_top_charts.clone()
    }

    pub fn get_fyyd_top_charts(&self) -> Vec<FoundPodcast> {
        self.fyyd_top_charts.clone()
    }

    pub fn get_missed_episodes(&self) -> Vec<Episode> {
        self.missed.clone()
    }
}
