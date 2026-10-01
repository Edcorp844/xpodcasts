use log::{error, info};
use podcasts_data::{FEED_MANAGER, Source, dbqueries};
use relm4::ComponentSender;

use crate::{
    action::Action,
    workers::action_worker::service::{ActionWorker, ActionWorkerInput, ActionWorkerOutput},
};

impl ActionWorker {
    pub(crate) async fn subscribe(sender: ComponentSender<Self>, feed: String) {
        let mut error_source = None; // <- auto unsub from this
        if let Err(e) = async {
            let source =
                dbqueries::get_source_from_uri(&feed).or_else(|_| Source::from_url(&feed))?;
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
            sender.input(ActionWorkerInput::Execute(Action::QuickSyncNextcloud));
            sender.input(ActionWorkerInput::Execute(Action::RefreshAllViews));
            Ok::<(), anyhow::Error>(())
        }
        .await
        {
            error!("Failed to subscribe: {feed} {e}");
            // auto unsubscribe
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
            // TODO show the actual error (like "content didn't start with rss feed"),
            // but pipeline doesn't pass useful errors yet
            println!("error: {}", e);
            let _ = sender.output(ActionWorkerOutput::NotifyError(format!(
                "Failed to subscribe to feed: {}",
                feed
            )));
        }
    }
}
