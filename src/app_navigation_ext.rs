use gettextrs::gettext;
use gst_play::PlayState;
use podcasts_data::EpisodeId;
use relm4::{ComponentController, Controller};
use uuid::Uuid;

use crate::{
    pages::{
        downloads::{DownloadsPage, DownloadsPageInput},
        home::{HomePage, HomePageInput},
        new::{NewPage, NewPageInput},
        podcast::{PodcastPage, PodcastPageInput},
        recents::{RecentlyUpdatedPage, RecentlyUpdatedPageInput},
        search::{SearchPage, SearchPageInput},
        shows::{ShowsPage, ShowsPageInput},
    },
    workers::action_worker::worker::ActionResult,
};

#[derive(Debug)]
pub enum NavigationPage {
    Search,
    Home,
    New,
    Shows,
    Recents,
    Podcast,
    Downloads,
}

impl NavigationPage {
    pub fn from_name(name: &str) -> Self {
        let localized_name = gettext(name);

        if localized_name == "Search" || name == "Search" {
            Self::Search
        } else if localized_name == gettext("Home") || name == "Home" {
            Self::Home
        } else if localized_name == gettext("New") || name == "New" {
            Self::New
        } else if localized_name == "Shows" || name == "Shows" {
            Self::Shows
        } else if localized_name == "Podcast" || name == "Podcast" {
            Self::Podcast
        } else if localized_name == gettext("Downloaded") || name == "Downloaded" {
            Self::Downloads
        } else if localized_name == gettext("Recently updated") || name == "Recently updated" {
            Self::Recents
        } else {
            Self::Home
        }
    }

    pub fn to_key(&self) -> String {
        match self {
            Self::Search => "Search".to_string(),
            Self::Home => "Home".to_string(),
            Self::New => "New".to_string(),
            Self::Shows => "Shows".to_string(),
            Self::Podcast => "Podcast".to_string(),
            Self::Downloads => "Downloaded".to_string(),
            Self::Recents => "Recently updated".to_string(),
        }
    }
}

#[derive(Debug)]

pub enum PageController {
    Search(Controller<SearchPage>),
    Home(Controller<HomePage>),
    New(Controller<NewPage>),
    Shows(Controller<ShowsPage>),
    Podcast(Controller<PodcastPage>),
    Downloads(Controller<DownloadsPage>),
    Recents(Controller<RecentlyUpdatedPage>),
}

impl PageController {
    pub(crate) fn widget(&self) -> &adw::NavigationPage {
        match self {
            Self::Search(c) => c.widget(),
            Self::Home(c) => c.widget(),
            Self::New(c) => c.widget(),
            Self::Shows(c) => c.widget(),
            Self::Podcast(c) => c.widget(),
            Self::Downloads(c) => c.widget(),
            Self::Recents(c) => c.widget(),
        }
    }

    pub(crate) fn notify_download_finished(&self, episode_id: EpisodeId) {
        match self {
            Self::Search(c) => {
                c.emit(SearchPageInput::DownloadFinished(episode_id));
            }
            Self::Home(c) => {
                c.emit(HomePageInput::DownloadFinished(episode_id));
            }
            Self::New(c) => {
                c.emit(NewPageInput::DownloadFinished(episode_id));
            }
            Self::Shows(c) => {
                c.emit(ShowsPageInput::DownloadFinished(episode_id));
            }
            Self::Podcast(c) => {
                c.emit(PodcastPageInput::DownloadFinished(episode_id));
            }
            Self::Downloads(c) => {
                c.emit(DownloadsPageInput::DownloadFinished(episode_id));
            }
            Self::Recents(c) => {
                c.emit(RecentlyUpdatedPageInput::DownloadFinished(episode_id));
            }
        }
    }

    pub(crate) fn notify_download_started(&self, episode_id: EpisodeId) {
        match self {
            Self::Search(c) => {
                c.emit(SearchPageInput::DownloadStarted(episode_id));
            }
            Self::Home(c) => {
                c.emit(HomePageInput::DownloadStarted(episode_id));
            }
            Self::New(c) => {
                c.emit(NewPageInput::DownloadStarted(episode_id));
            }
            Self::Shows(c) => {
                c.emit(ShowsPageInput::DownloadStarted(episode_id));
            }
            Self::Podcast(c) => {
                c.emit(PodcastPageInput::DownloadStarted(episode_id));
            }
            Self::Downloads(c) => {
                c.emit(DownloadsPageInput::DownloadStarted(episode_id));
            }
            Self::Recents(c) => {
                c.emit(RecentlyUpdatedPageInput::DownloadStarted(episode_id));
            }
        }
    }

    pub(crate) fn notify_download_progress(&self, episode_id: EpisodeId, fraction: f64) {
        match self {
            Self::Search(c) => {
                c.emit(SearchPageInput::DownloadProgress(episode_id, fraction));
            }
            Self::Home(c) => {
                c.emit(HomePageInput::DownloadProgress(episode_id, fraction));
            }
            Self::New(c) => {
                c.emit(NewPageInput::DownloadProgress(episode_id, fraction));
            }
            Self::Shows(c) => {
                c.emit(ShowsPageInput::DownloadProgress(episode_id, fraction));
            }
            Self::Podcast(c) => {
                c.emit(PodcastPageInput::DownloadProgress(episode_id, fraction));
            }
            Self::Downloads(c) => {
                c.emit(DownloadsPageInput::DownloadProgress(episode_id, fraction));
            }

            Self::Recents(c) => {
                c.emit(RecentlyUpdatedPageInput::DownloadProgress(
                    episode_id, fraction,
                ));
            }
        }
    }

    pub(crate) fn notify_playing_state(&self, episode_id: EpisodeId, state: PlayState) {
        match self {
            Self::Search(c) => {
                c.emit(SearchPageInput::ChangePlayBackState(state, episode_id));
            }
            Self::Home(c) => {
                c.emit(HomePageInput::ChangePlayBackState(state, episode_id));
            }
            Self::New(c) => {
                c.emit(NewPageInput::ChangePlayBackState(state, episode_id));
            }
            Self::Shows(c) => {
                c.emit(ShowsPageInput::ChangePlayBackState(state, episode_id));
            }
            Self::Podcast(c) => {
                c.emit(PodcastPageInput::ChangePlayBackState(state, episode_id));
            }
            Self::Downloads(c) => {
                c.emit(DownloadsPageInput::ChangePlayBackState(state, episode_id));
            }
            Self::Recents(c) => {
                c.emit(RecentlyUpdatedPageInput::ChangePlayBackState(
                    state, episode_id,
                ));
            }
        }
    }

    pub(crate) fn notify_playback_progress(
        &self,
        episode_id: EpisodeId,
        fraction: f64,
        remaining_sec: u64,
    ) {
        match self {
            Self::Search(c) => {
                c.emit(SearchPageInput::PlayBackProgress(
                    episode_id,
                    fraction,
                    remaining_sec,
                ));
            }
            Self::Home(c) => {
                c.emit(HomePageInput::PlayBackProgress(
                    episode_id,
                    fraction,
                    remaining_sec,
                ));
            }
            Self::New(c) => {
                c.emit(NewPageInput::PlayBackProgress(
                    episode_id,
                    fraction,
                    remaining_sec,
                ));
            }
            Self::Shows(c) => {
                c.emit(ShowsPageInput::PlayBackProgress(
                    episode_id,
                    fraction,
                    remaining_sec,
                ));
            }
            Self::Podcast(c) => {
                c.emit(PodcastPageInput::PlayBackProgress(
                    episode_id,
                    fraction,
                    remaining_sec,
                ));
            }
            Self::Downloads(c) => {
                c.emit(DownloadsPageInput::PlayBackProgress(
                    episode_id,
                    fraction,
                    remaining_sec,
                ));
            }
            Self::Recents(c) => {
                c.emit(RecentlyUpdatedPageInput::PlayBackProgress(
                    episode_id,
                    fraction,
                    remaining_sec,
                ));
            }
        }
    }

    pub(crate) fn notify_current_episode(&self, episode_id: EpisodeId) {
        match self {
            Self::Search(c) => {
                c.emit(SearchPageInput::ChangeEpisodeTo(episode_id));
            }
            Self::Home(c) => {
                c.emit(HomePageInput::ChangeEpisodeTo(episode_id));
            }
            Self::New(c) => {
                c.emit(NewPageInput::ChangeEpisodeTo(episode_id));
            }
            Self::Shows(c) => {
                c.emit(ShowsPageInput::ChangeEpisodeTo(episode_id));
            }
            Self::Podcast(c) => {
                c.emit(PodcastPageInput::ChangeEpisodeTo(episode_id));
            }
            Self::Downloads(c) => {
                c.emit(DownloadsPageInput::ChangeEpisodeTo(episode_id));
            }
            Self::Recents(c) => {
                c.emit(RecentlyUpdatedPageInput::ChangeEpisodeTo(episode_id));
            }
        }
    }

    pub(crate) fn notify_episode_deleted(&self, episode_id: EpisodeId) {
        match self {
            Self::Downloads(c) => {
                c.emit(DownloadsPageInput::EpisodeDeleted(episode_id));
            }
            _ => {}
        }
    }

    pub(crate) fn notify_action_finished(&self, id: Uuid, result: ActionResult) {
        match self {
            Self::Search(c) => {
                c.emit(SearchPageInput::ActionFinished(id, result));
            }
            Self::Home(c) => {
                c.emit(HomePageInput::ActionFinished(id, result));
            }
            Self::New(c) => {
                c.emit(NewPageInput::ActionFinished(id, result));
            }
            Self::Shows(c) => {
                c.emit(ShowsPageInput::ActionFinished(id, result));
            }
            Self::Podcast(c) => {
                c.emit(PodcastPageInput::ActionFinished(id, result));
            }
            Self::Downloads(c) => {
                c.emit(DownloadsPageInput::ActionFinished(id, result));
            }
            Self::Recents(c) => {
                c.emit(RecentlyUpdatedPageInput::ActionFinished(id, result));
            }
        }
    }
}
