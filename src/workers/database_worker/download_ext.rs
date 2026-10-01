use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex, RwLock},
};

use podcasts_data::{EpisodeId, downloader::DownloadProgress};
use relm4::ComponentSender;

use crate::workers::database_worker::worker::DatabaseWoker;

pub(crate) trait EpisodeDownloadExt {
    async fn download_episode(sender: ComponentSender<DatabaseWoker>, episode_id: EpisodeId);
    /// Ask an in-flight download to cancel. No-op (with a notification) if
    /// nothing is currently downloading for this episode.
    fn cancel_download(episode_id: EpisodeId, sender: ComponentSender<DatabaseWoker>);
}

#[derive(Debug, Default)]
pub(crate) struct Progress {
    total_bytes: u64,
    downloaded_bytes: u64,
    cancel: bool,
}

// ---------------------------------------------------------------------
// Progress tracking — corrected to prevent division by zero (NaN) errors.
// ---------------------------------------------------------------------

pub(crate) type ActiveProgress = Arc<Mutex<Progress>>;
pub(crate) type DownloadProgressLock = Arc<RwLock<HashMap<EpisodeId, ActiveProgress>>>;

pub(crate) static ACTIVE_DOWNLOADS: LazyLock<DownloadProgressLock> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

impl Progress {
    pub(crate) fn get_fraction(&self) -> f64 {
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
