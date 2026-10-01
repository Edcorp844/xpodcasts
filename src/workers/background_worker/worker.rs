use podcasts_data::EpisodeId;
use relm4::Worker;

use crate::action::Action;

pub struct BackgroundWorker {}

#[derive(Debug, Clone)]
pub enum BackgroundWorkerInput {
    Subscirbe(String),
    Execute(Action),
    SyncFinished,
    TogglePlayBack,
    StateChanged(gst_play::PlayState),
    DownloadEpisode(EpisodeId),
    CancelDownload(EpisodeId),
    DeleteEpisode(EpisodeId),
    SeekAudioPosition(f64),
    DurationChanged(u64),
    PositionChanged(u64),
    SetVolume(f64),
    AddToPlaylist(EpisodeId),
    SetPlayNext(EpisodeId),
    SetPlayNow(EpisodeId),
    RemoveFromPlayList(EpisodeId),
    GetVolume,
    RequestMute,
    RequestUnmute,
    SeekFoward,
    SeekBackward,
    NextEpisode,
    PreviousEpisode,
    EpisodeEnded,
}

#[derive(Debug, Clone)]
pub enum BackgroundWorkerOutput {
    Forward(Action),
    SetUpdatingState(bool),
    EpisodeReady(podcasts_data::EpisodeWidgetModel),
    PlayBackProgress(EpisodeId, f64, u64),
    UriReady(String),
    NotifyError(String),
    SyncFinished,
    StateChanged(gst_play::PlayState, EpisodeId),
    SetCurrentEpisode(EpisodeId),
    RefreshAllViews,
    DownloadStarted(EpisodeId),
    DownloadProgress { id: EpisodeId, fraction: f64 },
    DownloadFinished(EpisodeId),
    DownloadCancelled(EpisodeId),
    ErrorNotification(String),
    RefreshEpisode(EpisodeId),
    EpisodeDeleted(EpisodeId),
    VolumeValue(f64),
    UpdatePlaylist(Vec<EpisodeId>, Option<usize>),
}

impl Worker for BackgroundWorker {
    type Init = ();
    type Input = BackgroundWorkerInput;
    type Output = BackgroundWorkerOutput;

    fn init(init: Self::Init, sender: relm4::prelude::ComponentSender<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, message: Self::Input, sender: relm4::prelude::ComponentSender<Self>) {}
}
