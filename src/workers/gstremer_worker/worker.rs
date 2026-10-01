use std::path::Path;

use gst::{CoreError, ResourceError, StreamError};
use gst_play::{Play, PlaySignalAdapter, PlayState};
use gtk::gio::File;
use podcasts_data::{EpisodeId, EpisodeModel, dbqueries};
use relm4::{
    Worker,
    gtk::{gio::prelude::FileExt, glib::MainContext},
};
use url::Url;

use crate::{
    config::{PKGDATADIR, VERSION},
    settings::GenaralSettings,
    util::{
        external_controls::ExternalControlsMode,
        gst_errors::{handel_gst_core_error, handel_gst_resource_error, handel_gst_stream_error},
        play_list::PlayList,
    },
    workers::{
        action_worker::service::MprisCommand,
        gstremer_worker::worker::GStreamerWorkerOutput::NotifyError,
    },
};

pub struct GStreamerWorker {
    pub(crate) player: Play,
    pub(crate) _player_signals: PlaySignalAdapter,
    pub(crate) current_player_state: PlayState,
    pub(crate) play_list: PlayList,
    pub(crate) current_duration_ms: u64,
    pub(crate) mpris_tx: async_channel::Sender<MprisCommand>,
}

#[derive(Debug, Clone)]
pub enum GStreamerWorkerInput {
    TogglePlayBack,
    PlayEpisode(EpisodeId),
    StateChanged(PlayState),
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
pub enum GStreamerWorkerOutput {
    PlayBackProgress(EpisodeId, f64, u64),
    NotifyError(String),
    StateChanged(gst_play::PlayState, EpisodeId),
    SetCurrentEpisode(EpisodeId),
    VolumeValue(f64),
    UpdatePlaylist(Vec<EpisodeId>, Option<usize>),
    Muted,
    UnMuted,
}

impl Worker for GStreamerWorker {
    type Init = ();
    type Input = GStreamerWorkerInput;
    type Output = GStreamerWorkerOutput;

    fn init(_init: Self::Init, sender: relm4::prelude::ComponentSender<Self>) -> Self {
        let player = gst_play::Play::default();
        let mut player_config = player.config();

        let user_agent = format!("{}/{}", PKGDATADIR, VERSION);
        player_config.set_user_agent(&user_agent);
        player_config.set_position_update_interval(250);

        if let Err(error) = player.set_config(player_config) {
            let _ = sender.output(NotifyError(error.to_string()));
            error!("Failed to apply GStreamer player configuration: {error}");
        }
        player.set_video_track_enabled(false);

        let player_signals = PlaySignalAdapter::new(&player);

        let duration_sender = sender.clone();
        player_signals.connect_duration_changed(move |_, duration| {
            if let Some(dur) = duration {
                let ms = dur.mseconds();
                trace!("GStreamer duration updated: {ms}ms");
                duration_sender.input(GStreamerWorkerInput::DurationChanged(ms));
            }
        });

        let position_sender = sender.clone();
        player_signals.connect_position_updated(move |_, position| {
            if let Some(pos) = position {
                let ms = pos.mseconds();
                trace!("GStreamer position updated: {ms}ms");
                position_sender.input(GStreamerWorkerInput::PositionChanged(ms));
            }
        });

        let sender_clone = sender.clone();
        player_signals.connect_end_of_stream(move |_play| {
            sender_clone.input(GStreamerWorkerInput::EpisodeEnded);
        });

        let error_sender = sender.clone();
        player_signals.connect_error(move |_player, error, _details| {
            let raw_error_msg = error.to_string();
            warn!("GStreamer error event received: {raw_error_msg}");

            let error_msg = if let Some(res_err) = error.kind::<ResourceError>() {
                handel_gst_resource_error(res_err)
            } else if let Some(stream_err) = error.kind::<StreamError>() {
                handel_gst_stream_error(stream_err)
            } else if let Some(core_err) = error.kind::<CoreError>() {
                handel_gst_core_error(core_err)
            } else if raw_error_msg.contains("souphttpsrc") || raw_error_msg.contains("reason error (-5)") {
                "Could not stream the podcast due to a network connection timeout or a bad server response.".to_string()
            } else {
                "An unexpected playback error occurred.".to_string()
            };

            error!("Sending playback error to UI: '{error_msg}'");

            if let Err(err) = error_sender.output(GStreamerWorkerOutput::NotifyError(error_msg)) {
                error!("Failed to output NotifyError message to UI worker receiver: {:?}", err);
            }
        });

        let state_sender = sender.clone();
        player_signals.connect_state_changed(move |_, state| {
            info!("GStreamer playback state changed: {state:?}");
            state_sender.input(GStreamerWorkerInput::StateChanged(state));
        });

        let (mpris_tx, mpris_rx) = async_channel::unbounded::<MprisCommand>();
        let loopback_sender = sender.clone();

        MainContext::default().spawn_local(async move {
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
							inner_sender.input(GStreamerWorkerInput::TogglePlayBack);
						});

						let next_sender = loopback_sender.clone();
						p.connect_next(move || {
							debug!("MPRIS Next action triggered");
							let mode = GenaralSettings::new().get_external_controls_mode();

							match mode {
								ExternalControlsMode::ForwardBack => {
									next_sender.input(GStreamerWorkerInput::SeekFoward);
								}
								ExternalControlsMode::NextPrevious => {
									next_sender.input(GStreamerWorkerInput::NextEpisode); // Match your actual enum variant
								}
							}
						});

						let prev_sender = loopback_sender.clone();
						p.connect_previous(move || {
							debug!("MPRIS Previous action triggered");
							let mode = GenaralSettings::new().get_external_controls_mode();

							match mode {
								ExternalControlsMode::ForwardBack => {
									prev_sender.input(GStreamerWorkerInput::SeekBackward);
								}
								ExternalControlsMode::NextPrevious => {
									prev_sender.input(GStreamerWorkerInput::PreviousEpisode);
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
            player,
            current_player_state: PlayState::Stopped,
            _player_signals: player_signals,
            current_duration_ms: 0,
            play_list: PlayList::new(),
            mpris_tx,
        }
    }

    fn update(&mut self, message: Self::Input, sender: relm4::prelude::ComponentSender<Self>) {
        match message {
            GStreamerWorkerInput::TogglePlayBack => match self.current_player_state {
                PlayState::Stopped | PlayState::Paused => {
                    self.player.play();
                }
                PlayState::Playing => {
                    self.player.pause();
                }
                _ => {}
            },
            GStreamerWorkerInput::StateChanged(play_state) => {
                self.current_player_state = play_state;

                let _ = self
                    .mpris_tx
                    .send_blocking(MprisCommand::ChangePlaybackState(play_state));
                if let Some(id) = self.play_list.current() {
                    let _ = sender.output(GStreamerWorkerOutput::StateChanged(play_state, id));
                }
            }
            GStreamerWorkerInput::SeekAudioPosition(fraction) => {
                if self.current_duration_ms > 0 {
                    let clamped = fraction.clamp(0.0, 1.0);
                    let seek_ms = (clamped * self.current_duration_ms as f64) as u64;
                    let seek_pos = gst::ClockTime::from_mseconds(seek_ms);

                    self.player.seek(seek_pos);
                }
            }
            GStreamerWorkerInput::DurationChanged(ms) => {
                self.current_duration_ms = ms;
            }
            GStreamerWorkerInput::PositionChanged(position_ms) => {
                if self.current_duration_ms > 0 {
                    let remaining_ms = self.current_duration_ms.saturating_sub(position_ms);
                    let remaining_secs = remaining_ms / 1000;

                    let ratio = position_ms as f64 / self.current_duration_ms as f64;
                    if !ratio.is_nan() && !ratio.is_infinite() {
                        let fraction = ratio.clamp(0.0, 1.0);
                        if let Some(id) = self.play_list.current() {
                            let _ = sender.output(GStreamerWorkerOutput::PlayBackProgress(
                                id,
                                fraction,
                                remaining_secs,
                            ));
                        }
                    }
                }
            }
            GStreamerWorkerInput::SetVolume(vol) => {
                if vol.is_nan() || vol.is_infinite() {
                    return;
                }

                self.player.set_volume(vol);
            }
            GStreamerWorkerInput::AddToPlaylist(episode_id) => {
                if self.play_list.current().is_some() {
                    self.play_list.push_back(episode_id);
                    let (ids, current_pos) = self.play_list.play_list();
                    let _ = sender.output(GStreamerWorkerOutput::UpdatePlaylist(ids, current_pos));
                } else {
                    sender.input(GStreamerWorkerInput::PlayEpisode(episode_id));
                }
            }
            GStreamerWorkerInput::SetPlayNext(episode_id) => {
                let (ids, current) = self.play_list.play_list();
                let mut ids = ids.clone();

                let Some(current) = current else {
                    sender.input(GStreamerWorkerInput::AddToPlaylist(episode_id));
                    return;
                };

                let mut current_index = current;

                if let Some(existing_pos) = ids.iter().position(|x| *x == episode_id) {
                    ids.remove(existing_pos);

                    if existing_pos < current_index {
                        current_index -= 1;
                    }
                }

                let insert_pos = (current_index + 1).min(ids.len());
                ids.insert(insert_pos, episode_id);

                if let Some(current_id) = ids.get(current_index).cloned() {
                    self.play_list.set_sequence(ids.clone(), &current_id);
                    let _ = sender.output(GStreamerWorkerOutput::UpdatePlaylist(
                        ids,
                        Some(current_index),
                    ));
                }
            }
            GStreamerWorkerInput::SetPlayNow(episode_id) => {
                sender.input(GStreamerWorkerInput::PlayEpisode(episode_id));
            }
            GStreamerWorkerInput::RemoveFromPlayList(episode_id) => {
                if let Some(current) = self.play_list.current() {
                    if current == episode_id {
                        if let Some(next_id) = self.play_list.next() {
                            sender.input(GStreamerWorkerInput::PlayEpisode(next_id));
                        }
                    }
                }
                self.play_list.remove(&episode_id);
            }
            GStreamerWorkerInput::GetVolume => {
                let volume = self.player.volume();
                let _ = sender.output(GStreamerWorkerOutput::VolumeValue(volume));
            }
            GStreamerWorkerInput::RequestMute => {
                self.player.set_mute(true);
                let _ = sender.output(GStreamerWorkerOutput::Muted);
            }
            GStreamerWorkerInput::RequestUnmute => {
                self.player.set_mute(false);
                let _ = sender.output(GStreamerWorkerOutput::UnMuted);
            }
            GStreamerWorkerInput::SeekFoward => {
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
            GStreamerWorkerInput::SeekBackward => {
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
            GStreamerWorkerInput::NextEpisode => {
                if let Some(next_id) = self.play_list.next() {
                    sender.input(GStreamerWorkerInput::PlayEpisode(next_id));
                }
            }
            GStreamerWorkerInput::PreviousEpisode => {
                if let Some(prev_id) = self.play_list.prev() {
                    sender.input(GStreamerWorkerInput::PlayEpisode(prev_id));
                }
            }
            GStreamerWorkerInput::EpisodeEnded => {
                if GenaralSettings::new().get_continuous_playback() {
                    if let Some(next_id) = self.play_list.next() {
                        sender.input(GStreamerWorkerInput::PlayEpisode(next_id));
                    }
                }
            }
            GStreamerWorkerInput::PlayEpisode(episode_id) => {
                if self.play_list.current() == Some(episode_id) {
                    sender.input(GStreamerWorkerInput::TogglePlayBack);
                } else {
                    self.play_list.set_current(episode_id);
                    let (ids, current_pos) = self.play_list.play_list();
                    let _ = sender.output(GStreamerWorkerOutput::UpdatePlaylist(ids, current_pos));

                    let _ = sender.output(GStreamerWorkerOutput::SetCurrentEpisode(episode_id));
                    match dbqueries::get_episode_from_id(episode_id) {
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
                            let _ = sender
                                .output(GStreamerWorkerOutput::NotifyError(error.to_string()));
                        }
                    };
                }
            }
        }
    }
}
