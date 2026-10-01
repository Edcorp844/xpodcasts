use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use podcasts_data::{
    EpisodeCleanerModel, EpisodeId, EpisodeWidgetModel,
    dbqueries::{self},
    downloader::{DownloadProgress, get_episode},
    errors::DownloadError,
    utils::{delete_local_content, get_download_dir},
};

use relm4::Worker;
use tokio::time::interval;

use crate::workers::database_worker::download_ext::{
    ACTIVE_DOWNLOADS, ActiveProgress, EpisodeDownloadExt, Progress,
};

pub struct DatabaseWoker {}

#[derive(Debug, Clone)]
pub enum DatabaseWokerInput {
    DownloadEpisode(EpisodeId),
    CancelDownload(EpisodeId),
    DeleteEpisode(EpisodeId),
}

#[derive(Debug, Clone)]
pub enum DatabaseWokerOutput {
    NotifyError(String),
    DownloadStarted(EpisodeId),
    DownloadProgress {
        episode_id: EpisodeId,
        fraction: f64,
    },
    DownloadFinished(EpisodeId),
    DownloadCancelled(EpisodeId),
    EpisodeDeleted(EpisodeId),
}

impl Worker for DatabaseWoker {
    type Init = ();
    type Input = DatabaseWokerInput;
    type Output = DatabaseWokerOutput;

    fn init(_init: Self::Init, _sender: relm4::prelude::ComponentSender<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, message: Self::Input, sender: relm4::prelude::ComponentSender<Self>) {
        match message {
            DatabaseWokerInput::DownloadEpisode(episode_id) => {
                relm4::spawn(async move {
                    DatabaseWoker::download_episode(sender, episode_id).await;
                });
            }
            DatabaseWokerInput::CancelDownload(episode_id) => {
                Self::cancel_download(episode_id, sender)
            }
            DatabaseWokerInput::DeleteEpisode(episode_id) => {
                match dbqueries::get_episode_from_id(episode_id) {
                    Ok(ep) => match delete_local_content(&mut EpisodeCleanerModel::from(ep)) {
                        Ok(_) => {
                            let _ = sender.output(DatabaseWokerOutput::EpisodeDeleted(episode_id));
                        }
                        Err(error) => {
                            let _ =
                                sender.output(DatabaseWokerOutput::NotifyError(error.to_string()));
                        }
                    },
                    Err(error) => {
                        let _ = sender.output(DatabaseWokerOutput::NotifyError(error.to_string()));
                    }
                }
            }
        }
    }
}

impl EpisodeDownloadExt for DatabaseWoker {
    async fn download_episode(
        sender: relm4::prelude::ComponentSender<DatabaseWoker>,
        episode_id: EpisodeId,
    ) {
        let mut episode: EpisodeWidgetModel =
            match dbqueries::get_episode_widget_from_id(episode_id) {
                Ok(ep) => ep,
                Err(e) => {
                    sender
                        .output(DatabaseWokerOutput::NotifyError(format!(
                            "Download failed: {e}"
                        )))
                        .ok();
                    return;
                }
            };

        let download_dir = match dbqueries::get_podcast_from_id(episode.show_id()) {
            Ok(pd) => match get_download_dir(pd.title()) {
                Ok(dir) => dir,
                Err(e) => {
                    sender
                        .output(DatabaseWokerOutput::NotifyError(format!(
                            "Download failed: {e}"
                        )))
                        .ok();
                    return;
                }
            },
            Err(e) => {
                sender
                    .output(DatabaseWokerOutput::NotifyError(format!(
                        "Download failed: {e}"
                    )))
                    .ok();
                return;
            }
        };

        let prog: ActiveProgress = Arc::new(Mutex::new(Progress::default()));
        match ACTIVE_DOWNLOADS.write() {
            Ok(mut guard) => {
                guard.insert(episode_id, prog.clone());
            }
            Err(err) => {
                sender
                    .output(DatabaseWokerOutput::NotifyError(format!(
                        "ActiveDownloads: {err}."
                    )))
                    .ok();
                return;
            }
        }

        sender
            .output(DatabaseWokerOutput::DownloadStarted(episode_id))
            .ok();

        let poll_prog = prog.clone();
        let poll_sender = sender.clone();

        let progress_ticker = relm4::spawn(async move {
            let mut ticker = interval(Duration::from_millis(250));
            loop {
                ticker.tick().await;
                let fraction = match poll_prog.lock() {
                    Ok(p) => p.get_fraction(),
                    Err(_) => break,
                };

                if fraction.is_nan() {
                    continue;
                }

                if poll_sender
                    .output(DatabaseWokerOutput::DownloadProgress {
                        episode_id,
                        fraction,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        // Run the actual download block
        let download_result = get_episode(&mut episode, download_dir.as_str(), Some(prog)).await;

        // Stop the progress ticker immediately after the download future completes
        progress_ticker.abort();

        // Explicitly push a final 1.0 fraction update to the UI so the progress bar fills up
        sender
            .output(DatabaseWokerOutput::DownloadProgress {
                episode_id,
                fraction: 1.0,
            })
            .ok();

        // Handle results cleanly
        match download_result {
            Ok(_) => {
                if let Ok(episode_widget) = dbqueries::get_episode_widget_from_id(episode_id) {
                    let is_downloaded = episode_widget.local_uri().is_some();

                    if is_downloaded {
                        println!("This episode is downloaded!");
                    } else {
                        println!("This episode is not downloaded yet.");
                    }
                }

                sender
                    .output(DatabaseWokerOutput::DownloadFinished(episode_id))
                    .ok();
            }
            Err(DownloadError::DownloadCancelled) => {
                sender
                    .output(DatabaseWokerOutput::DownloadCancelled(episode_id))
                    .ok();
            }
            Err(e) => {
                sender
                    .output(DatabaseWokerOutput::NotifyError(format!(
                        "Download failed: {e}"
                    )))
                    .ok();
            }
        }

        if let Ok(mut m) = ACTIVE_DOWNLOADS.write() {
            let progress = m.remove(&episode_id);
            debug!("Removed: {:?}", progress);
        }
    }

    fn cancel_download(
        episode_id: EpisodeId,
        sender: relm4::prelude::ComponentSender<DatabaseWoker>,
    ) {
        let guard = match ACTIVE_DOWNLOADS.read() {
            Ok(g) => g,
            Err(_) => return,
        };

        if let Some(prog) = guard.get(&episode_id) {
            if let Ok(mut p) = prog.lock() {
                p.cancel();
                return;
            }
        }

        sender
            .output(DatabaseWokerOutput::NotifyError(
                "No active download to cancel".into(),
            ))
            .ok();
    }
}
