use std::sync::Arc;

use podcasts_data::{Episode, EpisodeId, EpisodeModel, Show, ShowId, discovery::FoundPodcast};

use crate::chapter_parser::Chapter;


#[derive(Debug, Clone)]
pub enum Action {
    RefreshAllViews,
    RefreshEpisodesView,
    RefreshEpisode(EpisodeId),
    RefreshShowsView,
    ReplaceWidget(Arc<Show>),
    RefreshWidgetIfSame(ShowId),
    GoToEpisodeDescription(Arc<Show>, Arc<Episode>),
    GoToShow(Arc<Show>),
    GoToFoundPodcasts(Arc<Vec<FoundPodcast>>),
    GoToChaptersPage(EpisodeId, Vec<Chapter>),
    ChaptersAvailable(EpisodeId, Vec<Chapter>),
    CopiedUrlNotification,
    CopyUrl(EpisodeId),
    MarkAllPlayerNotification(Arc<Show>),
    MarkAsPlayed(bool, EpisodeId),
    FeedRefreshed(u64),
    StartUpdating,
    QuickSyncNextcloud,
    StopUpdating,
    RemoveShow(Arc<Show>),
    ErrorNotification(String),
    InitEpisode(EpisodeId),
    InitEpisodeAt(EpisodeId, i32),
    TogglePlay(EpisodeId),
    UpdateCover(ShowId),
    EmptyState,
    PopulatedState,
    RaiseWindow,
    InhibitSuspend,
    UninhibitSuspend,
    Pause,
    Play
}