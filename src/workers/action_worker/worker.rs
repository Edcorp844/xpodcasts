use std::{
    any::Any,
    collections::{BTreeMap, HashMap},
    sync::{Arc, LazyLock},
};

use podcasts_data::{
    Episode, FEED_MANAGER, ShowId, Source,
    dbqueries::{self, ShowFilter},
    nextcloud_sync::{self, SyncError, SyncPolicy, SyncResult},
};
use relm4::{ComponentSender, Worker};
use uuid::Uuid;

use crate::pages::recents::TimeBucket;

/// Type-erased, shareable result payload. Different `Action` variants can
/// return completely different shapes (String, Vec<Show>, Result<T, E>,
/// etc). The caller downcasts back to whatever concrete type it expects,
/// since it's the one that knows what `Action` it originally submitted.
pub type ActionResult = Arc<dyn Any + Send + Sync>;

/// Fixed, process-wide ids for "singleton" background actions. Computed
/// exactly once on first access and identical everywhere in the app —
/// any file can reference these directly without needing a live
/// `ActionWorker` instance.
pub static SUBSCRIBE_ACTION_ID: LazyLock<Uuid> = LazyLock::new(Uuid::new_v4);
pub static QUICK_SYNC_NEXTCLOUD_ID: LazyLock<Uuid> = LazyLock::new(Uuid::new_v4);
pub static REFRESH_ALL_ID: LazyLock<Uuid> = LazyLock::new(Uuid::new_v4);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Subscribe(String),
    FetchShows,
    FetchRecents,
    FetchShowEpisodes(ShowId),
    QuickSyncNextcloud,
    RefreshAllViews,
}

pub struct ActionWorker {
    pending_actions: HashMap<Uuid, Action>,
}

#[derive(Clone)]
pub enum ActionWorkerInput {
    Execute(Uuid, Action),
    TaskFinished(Uuid, ActionResult),
}

impl std::fmt::Debug for ActionWorkerInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Execute(id, action) => f.debug_tuple("Execute").field(id).field(action).finish(),
            Self::TaskFinished(id, _) => f
                .debug_tuple("TaskFinished")
                .field(id)
                .field(&"<dyn Any>")
                .finish(),
        }
    }
}

#[derive(Clone)]
pub enum ActionWorkerOutput {
    RefreshAllPages,
    NotifyError(String),
    ActionStarted(Uuid),
    ActionCompleted(Uuid, ActionResult),
    ActionsCompleted,
}

impl std::fmt::Debug for ActionWorkerOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RefreshAllPages => write!(f, "RefreshAllPages"),
            Self::NotifyError(e) => f.debug_tuple("NotifyError").field(e).finish(),
            Self::ActionStarted(id) => f.debug_tuple("ActionStarted").field(id).finish(),
            Self::ActionCompleted(id, _) => f
                .debug_tuple("ActionCompleted")
                .field(id)
                .field(&"<dyn Any>")
                .finish(),
            Self::ActionsCompleted => write!(f, "ActionsCompleted"),
        }
    }
}

impl Worker for ActionWorker {
    type Init = ();
    type Input = ActionWorkerInput;
    type Output = ActionWorkerOutput;

    fn init(_init: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self {
            pending_actions: HashMap::new(),
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ActionWorkerInput::Execute(id, action) => {
                // For singleton-style global ids, skip if that exact task
                // is already in flight, so callers can't accidentally spawn
                // duplicates of e.g. the nextcloud sync.
                if self.pending_actions.contains_key(&id) {
                    return;
                }

                self.pending_actions.insert(id, action.clone());
                let _ = sender.output(ActionWorkerOutput::ActionStarted(id));

                let sender_clone = sender.clone();
                relm4::tokio::spawn(async move {
                    Self::execute(id, action, sender_clone).await;
                });
            }
            ActionWorkerInput::TaskFinished(id, result) => {
                if self.pending_actions.remove(&id).is_some() {
                    let _ = sender.output(ActionWorkerOutput::ActionCompleted(id, result));
                }
                if self.pending_actions.is_empty() {
                    let _ = sender.output(ActionWorkerOutput::ActionsCompleted);
                }
            }
        }
    }
}

impl ActionWorker {
    async fn execute(id: Uuid, action: Action, sender: ComponentSender<Self>) {
        match action {
            Action::Subscribe(feed) => {
                let mut error_source = None;

                // Result carried back to the caller: Ok(feed) on success,
                // Err(message) on failure.
                let outcome: Result<String, String> = async {
                    let source = dbqueries::get_source_from_uri(&feed)
                        .or_else(|_| Source::from_url(&feed))
                        .map_err(|e| e.to_string())?;
                    error_source = Some(source.clone());
                    info!("Subscribing to {feed}");
                    let _ = FEED_MANAGER.refresh(vec![source]).await;

                    if let Err(e) = podcasts_data::sync::Show::store_by_uri(
                        feed.to_string(),
                        podcasts_data::sync::ShowAction::Added,
                    ) {
                        error!("Failed store subscription for sync {e}");
                        let _ = sender.output(ActionWorkerOutput::NotifyError(format!(
                            "Failed store subscription for sync {e}",
                        )));
                    }

                    sender.input(ActionWorkerInput::Execute(
                        *QUICK_SYNC_NEXTCLOUD_ID,
                        Action::QuickSyncNextcloud,
                    ));
                    sender.input(ActionWorkerInput::Execute(
                        *REFRESH_ALL_ID,
                        Action::RefreshAllViews,
                    ));

                    Ok(feed.clone())
                }
                .await;

                if let Err(ref e) = outcome {
                    error!("Failed to subscribe: {feed} {e}");

                    if let Some(error_source) = error_source {
                        // only unsub if no Show was imported from the source.
                        if dbqueries::get_podcast_from_source_id(error_source.id()).is_err() {
                            if let Err(remove_err) = dbqueries::remove_source(&error_source) {
                                error!("failed to remove failed source! {remove_err} {feed}");
                            } else {
                                info!("auto removed source that failed to import {feed}");
                            }
                        }
                    }

                    let _ = sender.output(ActionWorkerOutput::NotifyError(format!(
                        "Failed to subscribe to feed: {feed}"
                    )));
                }

                let boxed: ActionResult = Arc::new(outcome);
                sender.input(ActionWorkerInput::TaskFinished(id, boxed));
            }
            Action::QuickSyncNextcloud => {
                let result = nextcloud_sync::sync(SyncPolicy::CancelOnMissingEpisodes).await;

                // Result carried back to the caller: Ok(true/false depending
                // on whether anything actually updated), Err(message) on failure.
                let outcome: Result<bool, String> = match result {
                    Ok(SyncResult::Done {
                        episode_updates_downloaded,
                        subscription_updates_downloaded,
                    }) => {
                        let updated =
                            episode_updates_downloaded > 0 || subscription_updates_downloaded > 0;
                        if updated {
                            let _ = sender.output(ActionWorkerOutput::RefreshAllPages);
                        }
                        Ok(updated)
                    }
                    Ok(SyncResult::Skipped) => Ok(false),
                    Err(SyncError::DownloadedUpdateForEpisodeNotInDb) => {
                        let errors = FEED_MANAGER.full_refresh().await;
                        let errors = FEED_MANAGER.retry_errors_full(errors).await;
                        let _ = FEED_MANAGER.retry_errors_full(errors).await;

                        match nextcloud_sync::sync(SyncPolicy::IgnoreMissingEpisodes).await {
                            Ok(_) => {
                                let _ = sender.output(ActionWorkerOutput::RefreshAllPages);
                                Ok(true)
                            }
                            Err(e) => {
                                let msg = format!("Sync failed {e}");
                                let _ = sender.output(ActionWorkerOutput::NotifyError(msg.clone()));
                                Err(msg)
                            }
                        }
                    }
                    Err(e) => {
                        let msg = format!("Sync failed {e}");
                        let _ = sender.output(ActionWorkerOutput::NotifyError(msg.clone()));
                        Err(msg)
                    }
                };

                let boxed: ActionResult = Arc::new(outcome);
                sender.input(ActionWorkerInput::TaskFinished(id, boxed));
            }
            Action::RefreshAllViews => {
                let _ = sender.output(ActionWorkerOutput::RefreshAllPages);

                let outcome: Result<(), String> = Ok(());
                let action_result: ActionResult = Arc::new(outcome);
                sender.input(ActionWorkerInput::TaskFinished(id, action_result));
            }
            Action::FetchShows => {
                let filter = ShowFilter {
                    any_downloaded: None,
                    completed: None,
                    title_or_description: None,
                    reverse_order: true,
                };
                let data = dbqueries::get_podcasts_filter(&[], &filter);
                let action_result: ActionResult = Arc::new(data);
                sender.input(ActionWorkerInput::TaskFinished(id, action_result));
            }
            Action::FetchRecents => {
                match dbqueries::get_episodes() {
                    Ok(ep) => {
                        let mut episodes: Vec<Episode> = ep.into_iter().take(50).collect();
                        episodes.sort_by(|a, b| b.epoch().cmp(&a.epoch()));

                        let mut grouped: BTreeMap<TimeBucket, Vec<Episode>> = BTreeMap::new();
                        for episode in episodes {
                            let bucket = TimeBucket::from_naive_datetime(episode.epoch());
                            grouped.entry(bucket).or_default().push(episode);
                        }

                        let action_result: ActionResult = Arc::new(grouped);
                        sender.input(ActionWorkerInput::TaskFinished(id, action_result));
                    }
                    Err(err) => {
                        let _ = sender.output(ActionWorkerOutput::NotifyError(err.to_string()));
                    }
                };
            }
            Action::FetchShowEpisodes(show_id) => match dbqueries::get_podcast_from_id(show_id) {
                Ok(show) => match dbqueries::get_pd_episodes(&show) {
                    Ok(episodes) => {
                        let action_result: ActionResult = Arc::new(episodes);
                        sender.input(ActionWorkerInput::TaskFinished(id, action_result));
                    }
                    Err(err) => {
                        let _ = sender.output(ActionWorkerOutput::NotifyError(err.to_string()));
                    }
                },
                Err(err) => {
                    let _ = sender.output(ActionWorkerOutput::NotifyError(err.to_string()));
                }
            },
        }
    }
}
