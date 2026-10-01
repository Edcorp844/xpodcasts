use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::Duration;

use log::debug;
use podcasts_data::EpisodeWidgetModel;
use podcasts_data::downloader::get_episode;
use podcasts_data::errors::DownloadError;
use relm4::ComponentSender;
use tokio::time::interval;

use podcasts_data::{EpisodeId, dbqueries, downloader::DownloadProgress, utils::get_download_dir};

use crate::workers::action_worker::service::{ActionWorker, ActionWorkerOutput};

// ---------------------------------------------------------------------
// Progress tracking — corrected to prevent division by zero (NaN) errors.
// ---------------------------------------------------------------------

pub(crate) type ActiveProgress = Arc<Mutex<Progress>>;
pub(crate) type DownloadProgressLock = Arc<RwLock<HashMap<EpisodeId, ActiveProgress>>>;

pub(crate) static ACTIVE_DOWNLOADS: LazyLock<DownloadProgressLock> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

#[derive(Debug, Default)]
pub(crate) struct Progress {
    total_bytes: u64,
    downloaded_bytes: u64,
    cancel: bool,
}

impl Progress {
    pub(crate) fn get_fraction(&self) -> f64 {
        // FIX: Prevent 0 / 0 division by zero leading to NaN at start of download
        if self.total_bytes == 0 {
            return 0.0;
        }

        let ratio = self.downloaded_bytes as f64 / self.total_bytes as f64;

        if ratio >= 1.0 {
            return 1.0;
        }
        ratio
    }
}

impl DownloadProgress for Progress {
    fn get_downloaded(&self) -> u64 {
        self.downloaded_bytes
    }

    fn set_downloaded(&mut self, downloaded: u64) {
        self.downloaded_bytes = downloaded
    }

    fn set_size(&mut self, bytes: u64) {
        self.total_bytes = bytes;
    }

    fn get_size(&self) -> u64 {
        self.total_bytes
    }

    fn should_cancel(&self) -> bool {
        self.cancel
    }

    fn cancel(&mut self) {
        self.cancel = true;
    }
}

impl ActionWorker {
    pub async fn download_podcast_episode(sender: ComponentSender<Self>, id: EpisodeId) {
        let mut episode: EpisodeWidgetModel = match dbqueries::get_episode_widget_from_id(id) {
            Ok(ep) => ep,
            Err(e) => {
                sender
                    .output(ActionWorkerOutput::ErrorNotification(
		                format!(
		                    "Download failed: {e}"
		                ))
                    )
                    .ok();
                return;
            }
        };

        let download_dir = match dbqueries::get_podcast_from_id(episode.show_id()) {
            Ok(pd) => match get_download_dir(pd.title()) {
                Ok(dir) => dir,
                Err(e) => {
                    sender
                        .output(ActionWorkerOutput::ErrorNotification(format!(
                            "Download failed: {e}"
                        )))
                        .ok();
                    return;
                }
            },
            Err(e) => {
                sender
                    .output(ActionWorkerOutput::ErrorNotification(format!(
                        "Download failed: {e}"
                    )))
                    .ok();
                return;
            }
        };

        let prog: ActiveProgress = Arc::new(Mutex::new(Progress::default()));
        match ACTIVE_DOWNLOADS.write() {
            Ok(mut guard) => {
                guard.insert(id, prog.clone());
            }
            Err(err) => {
                sender
                    .output(ActionWorkerOutput::ErrorNotification(format!(
                        "ActiveDownloads: {err}."
                    )))
                    .ok();
                return;
            }
        }

        sender.output(ActionWorkerOutput::DownloadStarted(id)).ok();

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

                // FIX: Guard clause to prevent sending NaN downstream if anything goes wrong
                if fraction.is_nan() {
                    continue;
                }

                if poll_sender
                    .output(ActionWorkerOutput::DownloadProgress { id, fraction })
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
            .output(ActionWorkerOutput::DownloadProgress { id, fraction: 1.0 })
            .ok();

        // Handle results cleanly
        match download_result {
            Ok(_) => {
                if let Ok(episode_widget) = dbqueries::get_episode_widget_from_id(id) {
                    let is_downloaded = episode_widget.local_uri().is_some();

                    if is_downloaded {
                        println!("This episode is downloaded!");
                    } else {
                        println!("This episode is not downloaded yet.");
                    }
                }

                sender.output(ActionWorkerOutput::DownloadFinished(id)).ok();
            }
            Err(DownloadError::DownloadCancelled) => {
                sender
                    .output(ActionWorkerOutput::DownloadCancelled(id))
                    .ok();
            }
            Err(e) => {
                sender
                    .output(ActionWorkerOutput::ErrorNotification(format!(
                        "Download failed: {e}"
                    )))
                    .ok();
            }
        }

        if let Ok(mut m) = ACTIVE_DOWNLOADS.write() {
            let progress = m.remove(&id);
            debug!("Removed: {:?}", progress);
        }

        sender.output(ActionWorkerOutput::RefreshEpisode(id)).ok();
    }

    /// Ask an in-flight download to cancel. No-op (with a notification) if
    /// nothing is currently downloading for this episode.
    pub fn cancel_download(id: EpisodeId, sender: &ComponentSender<Self>) {
        let guard = match ACTIVE_DOWNLOADS.read() {
            Ok(g) => g,
            Err(_) => return,
        };

        if let Some(prog) = guard.get(&id) {
            if let Ok(mut p) = prog.lock() {
                p.cancel();
                return;
            }
        }

        sender
            .output(ActionWorkerOutput::ErrorNotification(
                "No active download to cancel".into(),
            ))
            .ok();
    }
}
