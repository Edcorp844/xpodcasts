use std::path::Path;
use url::Url;

use gtk::gio::{File, prelude::FileExt};
use podcasts_data::{
    EpisodeCleanerModel, EpisodeId, EpisodeModel, FEED_MANAGER, dbqueries, nextcloud_sync,
    nextcloud_sync::{SyncError, SyncPolicy, SyncResult},
    utils::delete_local_content,
};
use relm4::{ComponentSender, Worker};

use crate::{
    action::Action,
    settings::GenaralSettings,
    util::{
        external_controls::ExternalControlsMode,
        gst_errors::{handel_gst_core_error, handel_gst_resource_error, handel_gst_stream_error},
        play_list::PlayList,
    },
};

#[derive(Debug, Clone)]
pub enum ActionWorkerInput {
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
pub enum ActionWorkerOutput {
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

#[derive(Debug)]
pub enum MprisCommand {
    ChangePlaybackState(gst_play::PlayState),
    UpdateMetadata {
        title: String,
        show_title: String,
        art_url: Option<String>,
    },
}

pub struct ActionWorker {
    pub(crate) syncing: bool,
    pub(crate) player: gst_play::Play,
    pub(crate) _player_signals: gst_play::PlaySignalAdapter,
    pub(crate) current_player_state: gst_play::PlayState,
    pub(crate) play_list: PlayList,
    pub(crate) current_duration_ms: u64,
    pub(crate) mpris_tx: async_channel::Sender<MprisCommand>,
}
impl Worker for ActionWorker {
    type Init = ();
    type Input = ActionWorkerInput;
    type Output = ActionWorkerOutput;

    fn init(_init: Self::Init, sender: ComponentSender<Self>) -> Self {
        let player = gst_play::Play::default();
        let mut config = player.config();

        const USER_AGENT: &str = "XPodcasts/1.0";
        config.set_user_agent(USER_AGENT);
        config.set_position_update_interval(250);

        if let Err(err) = player.set_config(config) {
            error!("Failed to apply GStreamer player configuration: {err}");
        }
        player.set_video_track_enabled(false);

        let player_signals = gst_play::PlaySignalAdapter::new(&player);

        let duration_sender = sender.clone();
        player_signals.connect_duration_changed(move |_, duration| {
            if let Some(dur) = duration {
                let ms = dur.mseconds();
                trace!("GStreamer duration updated: {ms}ms");
                duration_sender.input(ActionWorkerInput::DurationChanged(ms));
            }
        });

        let position_sender = sender.clone();
        player_signals.connect_position_updated(move |_, position| {
            if let Some(pos) = position {
                let ms = pos.mseconds();
                trace!("GStreamer position updated: {ms}ms");
                position_sender.input(ActionWorkerInput::PositionChanged(ms));
            }
        });

        let sender_clone = sender.clone();
        player_signals.connect_end_of_stream(move |_play| {
            sender_clone.input(ActionWorkerInput::EpisodeEnded);
        });

        let error_sender = sender.clone();
        player_signals.connect_error(move |_player, error, _details| {
            let raw_error_msg = error.to_string();
            warn!("GStreamer error event received: {raw_error_msg}");

            let error_msg = if let Some(res_err) = error.kind::<gst::ResourceError>() {
                handel_gst_resource_error(res_err)
            } else if let Some(stream_err) = error.kind::<gst::StreamError>() {
                handel_gst_stream_error(stream_err)
            } else if let Some(core_err) = error.kind::<gst::CoreError>() {
                handel_gst_core_error(core_err)
            } else if raw_error_msg.contains("souphttpsrc") || raw_error_msg.contains("reason error (-5)") {
                "Could not stream the podcast due to a network connection timeout or a bad server response.".to_string()
            } else {
                "An unexpected playback error occurred.".to_string()
            };

            error!("Sending playback error to UI: '{error_msg}'");

            // Always forward the error message back out to the UI
            if let Err(err) = error_sender.output(ActionWorkerOutput::NotifyError(error_msg)) {
                error!("Failed to output NotifyError message to UI worker receiver: {:?}", err);
            }
        });

        let state_sender = sender.clone();
        player_signals.connect_state_changed(move |_, state| {
            info!("GStreamer playback state changed: {state:?}");
            state_sender.input(ActionWorkerInput::StateChanged(state));
        });

        let (mpris_tx, mpris_rx) = async_channel::unbounded::<MprisCommand>();
        let loopback_sender = sender.clone();

        relm4::gtk::glib::MainContext::default().spawn_local(async move {
            struct GlobalMpris {
                player: std::sync::Arc<mpris_player::MprisPlayer>,
            }

            thread_local! {
                static INSTANCE: std::cell::RefCell<Option<GlobalMpris>> = const { std::cell::RefCell::new(None) };
            }

            while let Ok(cmd) = mpris_rx.recv().await {
                INSTANCE.with(|cell| {
                    let mut cell = cell.borrow_mut();
                    let state = cell.get_or_insert_with(|| {
                        info!("Initializing MPRIS D-Bus interface");
                        let p = mpris_player::MprisPlayer::new(
                            "org.mpris.MediaPlayer2.ZoePodcastApp".to_string(),
                            "XPodcasts".to_string(),
                            "".to_string(),
                        );

                        let inner_sender = loopback_sender.clone();
						p.connect_play_pause(move || {
							debug!("MPRIS Play/Pause action triggered");
							inner_sender.input(ActionWorkerInput::TogglePlayBack);
						});

						// Capture settings to pass into the Next handler
						let next_sender = loopback_sender.clone();
						p.connect_next(move || {
							debug!("MPRIS Next action triggered");
							let mode = GenaralSettings::new().get_external_controls_mode();

							match mode {
								ExternalControlsMode::ForwardBack => {
									next_sender.input(ActionWorkerInput::SeekFoward);
								}
								ExternalControlsMode::NextPrevious => {
									next_sender.input(ActionWorkerInput::NextEpisode); // Match your actual enum variant
								}
							}
						});

						// Capture settings to pass into the Previous handler
						let prev_sender = loopback_sender.clone();
						p.connect_previous(move || {
							debug!("MPRIS Previous action triggered");
							let mode = GenaralSettings::new().get_external_controls_mode();

							match mode {
								ExternalControlsMode::ForwardBack => {
									prev_sender.input(ActionWorkerInput::SeekBackward);
								}
								ExternalControlsMode::NextPrevious => {
									prev_sender.input(ActionWorkerInput::PreviousEpisode);
								}
							}
						});

                        GlobalMpris { player: p }
                    });

                    let player_ref = &state.player;

                    match cmd {
                        MprisCommand::ChangePlaybackState(state) => {
                            let status = match state {
                                gst_play::PlayState::Playing => mpris_player::PlaybackStatus::Playing,
                                gst_play::PlayState::Paused => mpris_player::PlaybackStatus::Paused,
                                _ => mpris_player::PlaybackStatus::Stopped,
                            };
                            debug!("Updating MPRIS status to: {status:?}");
                            player_ref.set_playback_status(status);
                        }
                        MprisCommand::UpdateMetadata { title, show_title, art_url } => {
                            info!("Updating MPRIS metadata: title='{title}', show='{show_title}'");
                            let mut metadata = mpris_player::Metadata::new();
                            metadata.title = Some(title);
                            metadata.artist = Some(vec![show_title]);

                            if let Some(ref remote_url) = art_url {
                                if !remote_url.is_empty() {
                                    let mut cache_path = adw::glib::user_cache_dir();
                                    cache_path.push("xpodcasts");
                                    cache_path.push("covers");

                                    let glib_url_bytes = adw::glib::Bytes::from(remote_url.as_bytes());
                                    if let Some(hashed_name) = adw::glib::compute_checksum_for_bytes(
                                        adw::glib::ChecksumType::Sha256,
                                        &glib_url_bytes,
                                    ) {
                                        let local_disk_file = cache_path.join(hashed_name.as_str());

                                        if local_disk_file.exists() {
                                            let uri_string = adw::gio::File::for_path(local_disk_file).uri();
                                            debug!("Found local cached cover art for MPRIS: {uri_string}");
                                            metadata.art_url = Some(uri_string.to_string());
                                        } else {
                                            debug!("Using remote cover art URL for MPRIS: {remote_url}");
                                            metadata.art_url = Some(remote_url.clone());
                                        }
                                    }
                                }
                            }

                            player_ref.set_metadata(metadata);
                        }
                    }
                });
            }
            info!("MPRIS command listener channel closed");
        });

        Self {
            syncing: false,
            player,
            current_player_state: gst_play::PlayState::Stopped,
            _player_signals: player_signals,
            current_duration_ms: 0,
            play_list: PlayList::new(),
            mpris_tx,
        }
    }

    fn update(&mut self, input: Self::Input, sender: ComponentSender<Self>) {
        match input {
            ActionWorkerInput::Execute(action) => self.execute(action, sender.clone()),
            ActionWorkerInput::SyncFinished => {
                self.syncing = false;
            }
            ActionWorkerInput::Subscirbe(feed) => {
                relm4::tokio::spawn(async move {
                    Self::subscribe(sender, feed).await;
                });
            }
            ActionWorkerInput::DownloadEpisode(episode) => {
                relm4::spawn(async move {
                    ActionWorker::download_podcast_episode(sender, episode).await;
                });
            }
            ActionWorkerInput::CancelDownload(id) => {
                Self::cancel_download(id, &sender);
            }

            ActionWorkerInput::StateChanged(state) => {
                self.current_player_state = state;

                let _ = self
                    .mpris_tx
                    .send_blocking(MprisCommand::ChangePlaybackState(state));
                if let Some(id) = self.play_list.current() {
                    let _ = sender.output(ActionWorkerOutput::StateChanged(state, id));
                }
            }

            ActionWorkerInput::TogglePlayBack => match self.current_player_state {
                gst_play::PlayState::Stopped | gst_play::PlayState::Paused => {
                    self.player.play();
                }
                gst_play::PlayState::Playing => {
                    self.player.pause();
                }
                _ => {}
            },
            ActionWorkerInput::PositionChanged(position_ms) => {
                if self.current_duration_ms > 0 {
                    let remaining_ms = self.current_duration_ms.saturating_sub(position_ms);
                    let remaining_secs = remaining_ms / 1000;

                    let ratio = position_ms as f64 / self.current_duration_ms as f64;
                    if !ratio.is_nan() && !ratio.is_infinite() {
                        let fraction = ratio.clamp(0.0, 1.0);
                        if let Some(id) = self.play_list.current() {
                            let _ = sender.output(ActionWorkerOutput::PlayBackProgress(
                                id,
                                fraction,
                                remaining_secs,
                            ));
                        }
                    }
                }
            }
            ActionWorkerInput::DurationChanged(ms) => {
                self.current_duration_ms = ms;
            }
            ActionWorkerInput::SeekAudioPosition(pos_fraction) => {
                if self.current_duration_ms > 0 {
                    let clamped = pos_fraction.clamp(0.0, 1.0);
                    let seek_ms = (clamped * self.current_duration_ms as f64) as u64;
                    let seek_pos = gst::ClockTime::from_mseconds(seek_ms);

                    self.player.seek(seek_pos);
                }
            }
            ActionWorkerInput::SetVolume(vol) => {
                if vol.is_nan() || vol.is_infinite() {
                    return;
                }

                self.player.set_volume(vol);
            }
            ActionWorkerInput::RequestMute => {}
            ActionWorkerInput::RequestUnmute => {}
            ActionWorkerInput::GetVolume => {
                let volume = self.player.volume();
                let _ = sender.output(ActionWorkerOutput::VolumeValue(volume));
            }
            ActionWorkerInput::SeekFoward => {
                if let (Some(current_pos), Some(total_duration)) =
                    (self.player.position(), self.player.duration())
                {
                    let forward_duration = gst::ClockTime::from_seconds(
                        GenaralSettings::new().get_skip_foward_seconds() as u64,
                    );

                    let target_pos = std::cmp::min(current_pos + forward_duration, total_duration);

                    self.player.seek(target_pos);
                }
            }

            ActionWorkerInput::SeekBackward => {
                if let Some(current_pos) = self.player.position() {
                    let backward_duration = gst::ClockTime::from_seconds(
                        GenaralSettings::new().get_skip_backward_seconds() as u64,
                    );

                    let target_pos = if current_pos > backward_duration {
                        current_pos - backward_duration
                    } else {
                        gst::ClockTime::ZERO
                    };

                    self.player.seek(target_pos);
                }
            }

            ActionWorkerInput::DeleteEpisode(episode_id) => {
                match dbqueries::get_episode_from_id(episode_id) {
                    Ok(ep) => match delete_local_content(&mut EpisodeCleanerModel::from(ep)) {
                        Ok(_) => {
                            let _ = sender.output(ActionWorkerOutput::EpisodeDeleted(episode_id));
                        }
                        Err(error) => {
                            let _ =
                                sender.output(ActionWorkerOutput::NotifyError(error.to_string()));
                        }
                    },
                    Err(error) => {
                        let _ = sender.output(ActionWorkerOutput::NotifyError(error.to_string()));
                    }
                }
            }
            ActionWorkerInput::NextEpisode => {
                if let Some(next_id) = self.play_list.next() {
                    sender.input(ActionWorkerInput::Execute(Action::TogglePlay(next_id)));
                }
            }

            ActionWorkerInput::PreviousEpisode => {
                if let Some(prev_id) = self.play_list.prev() {
                    sender.input(ActionWorkerInput::Execute(Action::TogglePlay(prev_id)));
                }
            }
            ActionWorkerInput::EpisodeEnded => {
                if GenaralSettings::new().get_continuous_playback() {
                    if let Some(next_id) = self.play_list.next() {
                        sender.input(ActionWorkerInput::Execute(Action::TogglePlay(next_id)));
                    }
                }
            }
            ActionWorkerInput::AddToPlaylist(episode_id) => {
                if self.play_list.current().is_some() {
                    self.play_list.push_back(episode_id);
                    let (ids, current_pos) = self.play_list.play_list();
                    let _ = sender.output(ActionWorkerOutput::UpdatePlaylist(ids, current_pos));
                } else {
                    sender.input(ActionWorkerInput::Execute(Action::TogglePlay(episode_id)));
                }
            }
            ActionWorkerInput::SetPlayNext(episode_id) => {
                let (ids, current) = self.play_list.play_list();
                let mut ids = ids.clone();

                let Some(current) = current else {
                    sender.input(ActionWorkerInput::AddToPlaylist(episode_id));
                    return;
                };

                let mut current_index = current;

                if let Some(existing_pos) = ids.iter().position(|x| *x == episode_id) {
                    ids.remove(existing_pos);

                    // Adjust `current_index` if the removed item was before it.
                    if existing_pos < current_index {
                        current_index -= 1;
                    }
                    // If existing_pos == current_index, current_index still correctly
                    // points at whatever slid into that slot (the episode right after
                    // the one that was removed), which is fine since we're re-deriving
                    // the starting id from ids[current_index] below anyway.
                }

                let insert_pos = (current_index + 1).min(ids.len());
                ids.insert(insert_pos, episode_id);

                // set_sequence needs the *id* of the current episode, not its index,
                // since ids may have shifted around it.
                if let Some(current_id) = ids.get(current_index).cloned() {
                    self.play_list.set_sequence(ids.clone(), &current_id);
                    let _ =
                        sender.output(ActionWorkerOutput::UpdatePlaylist(ids, Some(current_index)));
                }
            }
            ActionWorkerInput::SetPlayNow(episode_id) => {
                sender.input(ActionWorkerInput::Execute(Action::TogglePlay(episode_id)));
            }
            ActionWorkerInput::RemoveFromPlayList(episode_id) => {
                if let Some(current) = self.play_list.current() {
                    if current == episode_id {
                        if let Some(next_id) = self.play_list.next() {
                            sender.input(ActionWorkerInput::Execute(Action::TogglePlay(next_id)));
                        } else {
                            //.input(ActionWorkerInput::TogglePlayBack);
                            print!("current {:?}: now {:?}", current, episode_id)
                        }
                    }
                }
                self.play_list.remove(&episode_id);
            }
        }
    }
}

impl ActionWorker {
    fn execute(&mut self, action: Action, sender: ComponentSender<Self>) {
        match action {
            Action::RefreshEpisode(id) => self.refresh_episode(id, sender.clone()),

            Action::RefreshAllViews => {
                let _ = sender.output(ActionWorkerOutput::RefreshAllViews);
            }

            Action::RefreshWidgetIfSame(id) => {
                let _ = sender.output(ActionWorkerOutput::Forward(Action::RefreshWidgetIfSame(id)));
            }

            Action::MarkAsPlayed(played, id) => self.mark_as_played(played, id, sender),

            Action::CopyUrl(id) => self.copy_url(id, sender.clone()),

            Action::QuickSyncNextcloud => self.quick_sync_nextcloud(sender),

            Action::FeedRefreshed(id) => {
                let _ = sender.output(ActionWorkerOutput::Forward(Action::FeedRefreshed(id)));
            }

            Action::TogglePlay(id) => {
                if self.play_list.current() == Some(id) {
                    sender.input(ActionWorkerInput::TogglePlayBack);
                } else {
                    self.play_list.set_current(id);
                    let (ids, current_pos) = self.play_list.play_list();
                    let _ = sender.output(ActionWorkerOutput::UpdatePlaylist(ids, current_pos));

                    let _ = sender.output(ActionWorkerOutput::SetCurrentEpisode(id));
                    match dbqueries::get_episode_from_id(id) {
                        Ok(episode) => {
                            let uri = if let Some(path) = episode.local_uri() {
                                if Path::new(path).exists() {
                                    let file_uri = File::for_path(path).uri();
                                    Some(file_uri.to_string())
                                } else {
                                    None
                                }
                            } else {
                                episode.uri().map(|u| u.to_string())
                            };

                            if let Some(stream_url) = uri {
                                self.current_duration_ms = 0;
                                self.player.set_uri(Some(&stream_url));
                                self.player.play();
                            }

                            let title = episode.title().to_string();
                            let mut show_title = "Unknown Podcast".to_string();

                            if let Ok(show) =
                                dbqueries::get_podcast_cover_from_id(episode.show_id())
                            {
                                show_title = show.title().to_string();
                            }

                            let art_url = episode.image_uri().and_then(|uri_str| {
                                let s = uri_str;

                                // 1. If it's already a HTTP/HTTPS or file URL, keep it
                                if s.starts_with("http://")
                                    || s.starts_with("https://")
                                    || s.starts_with("file://")
                                {
                                    Some(s.to_string())
                                }
                                // 2. If it's a local filesystem path, convert it to a valid file:// URI
                                else {
                                    Url::from_file_path(s).ok().map(|u| u.to_string())
                                }
                            });

                            let _ = self.mpris_tx.try_send(MprisCommand::UpdateMetadata {
                                title,
                                show_title,
                                art_url,
                            });
                        }
                        Err(error) => {
                            let _ =
                                sender.output(ActionWorkerOutput::NotifyError(error.to_string()));
                        }
                    };
                }
            }
            Action::Pause => {
                self.player.pause();
            }
            Action::Play => {
                self.player.play();
            }

            other @ (Action::RefreshShowsView
            | Action::RefreshEpisodesView
            | Action::ReplaceWidget(_)
            | Action::GoToEpisodeDescription(_, _)
            | Action::GoToShow(_)
            | Action::GoToFoundPodcasts(_)
            | Action::GoToChaptersPage(_, _)
            | Action::ChaptersAvailable(_, _)
            | Action::CopiedUrlNotification
            | Action::MarkAllPlayerNotification(_)
            | Action::RemoveShow(_)
            | Action::ErrorNotification(_)
            | Action::StartUpdating
            | Action::StopUpdating
            | Action::InitEpisode(_)
            | Action::InitEpisodeAt(_, _)
            | Action::UpdateCover(_)
            | Action::EmptyState
            | Action::PopulatedState
            | Action::RaiseWindow
            | Action::InhibitSuspend
            | Action::UninhibitSuspend) => {
                let _ = sender.output(ActionWorkerOutput::Forward(other));
            }
        }
    }

    fn refresh_episode(&self, id: EpisodeId, sender: ComponentSender<Self>) {
        match dbqueries::get_episode_widget_from_id(id) {
            Ok(ep) => {
                let _ = sender.output(ActionWorkerOutput::EpisodeReady(ep));
            }
            Err(e) => {
                let _ = sender.output(ActionWorkerOutput::NotifyError(format!(
                    "failed to fetch episode for description refresh: {e}"
                )));
            }
        }
    }

    fn mark_as_played(&self, played: bool, id: EpisodeId, sender: ComponentSender<Self>) {
        let mut ep = match dbqueries::get_episode_widget_from_id(id) {
            Ok(ep) => ep,
            Err(e) => {
                let _ = sender.output(ActionWorkerOutput::NotifyError(format!(
                    "failed to fetch episode to mark played: {e}"
                )));
                return;
            }
        };

        let result = if played {
            ep.set_played_now()
        } else {
            ep.set_unplayed()
        };
        if let Err(e) = result {
            let _ = sender.output(ActionWorkerOutput::NotifyError(format!(
                "failed to update played state: {e}"
            )));
            return;
        }

        let _ = sender.output(ActionWorkerOutput::EpisodeReady(ep.clone()));

        sender.input(ActionWorkerInput::Execute(Action::QuickSyncNextcloud));
        sender.input(ActionWorkerInput::Execute(Action::RefreshEpisode(ep.id())));
    }

    fn copy_url(&self, id: EpisodeId, sender: ComponentSender<Self>) {
        match dbqueries::get_episode_from_id(id)
            .ok()
            .and_then(|e| e.local_uri().map(|s| s.to_string()))
        {
            Some(uri) => {
                let _ = sender.output(ActionWorkerOutput::UriReady(uri));
            }
            None => {
                let _ = sender.output(ActionWorkerOutput::NotifyError(
                    "no URL available for that episode".to_string(),
                ));
            }
        }
    }

    fn quick_sync_nextcloud(&mut self, sender: ComponentSender<Self>) {
        if self.syncing {
            return;
        }
        self.syncing = true;

        let _ = sender.output(ActionWorkerOutput::SetUpdatingState(true));

        crate::RUNTIME.spawn(async move {
            let result = nextcloud_sync::sync(SyncPolicy::CancelOnMissingEpisodes).await;

            match result {
                Ok(SyncResult::Done {
                    episode_updates_downloaded,
                    subscription_updates_downloaded,
                }) => {
                    if episode_updates_downloaded > 0 || subscription_updates_downloaded > 0 {
                        let _ = sender.output(ActionWorkerOutput::Forward(Action::RefreshAllViews));
                    }
                }
                Ok(SyncResult::Skipped) => {}
                Err(SyncError::DownloadedUpdateForEpisodeNotInDb) => {
                    let errors = FEED_MANAGER.full_refresh().await;
                    let errors = FEED_MANAGER.retry_errors_full(errors).await;
                    let _ = FEED_MANAGER.retry_errors_full(errors).await;

                    match nextcloud_sync::sync(SyncPolicy::IgnoreMissingEpisodes).await {
                        Ok(_) => {
                            let _ =
                                sender.output(ActionWorkerOutput::Forward(Action::RefreshAllViews));
                        }
                        Err(e) => {
                            let _ = sender.output(ActionWorkerOutput::NotifyError(format!(
                                "Sync failed {e}"
                            )));
                        }
                    }
                }
                Err(e) => {
                    let _ =
                        sender.output(ActionWorkerOutput::NotifyError(format!("Sync failed {e}")));
                }
            }

            let _ = sender.output(ActionWorkerOutput::SetUpdatingState(false));

            sender.input(ActionWorkerInput::SyncFinished);
        });
    }

    fn handle_hardware_keys(&self) {
        let player_handle = self.player.clone();

        // Spawn a listener thread to catch OS/Hardware playback requests
        std::thread::spawn(move || {
            // Blocks waiting for system D-Bus commands from headphone hooks
            // event_loop.run_once() or similar depending on chosen executor
        });
    }
}
