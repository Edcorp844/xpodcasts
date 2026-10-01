use std::{collections::BTreeMap, sync::Arc};

use adw::prelude::*;
use chrono::{Datelike, Duration, Local, NaiveDateTime};
use gst_play::PlayState;
use podcasts_data::{Episode, EpisodeId, dbqueries};
use relm4::{Component, prelude::*};
use uuid::Uuid;

use crate::{
    components::episode_group::{GroupedEpisodes, GroupedEpisodesInput, GroupedEpisodesOutput},
    workers::action_worker::worker::{Action, ActionResult},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeBucket {
    Today = 0,
    Yesterday = 1,
    ThisWeek = 2,
    ThisMonth = 3,
    LastMonth = 4,
    ThisYear = 5,
    Older = 6,
}

impl TimeBucket {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimeBucket::Today => "Today",
            TimeBucket::Yesterday => "Yesterday",
            TimeBucket::ThisWeek => "This Week",
            TimeBucket::ThisMonth => "This Month",
            TimeBucket::LastMonth => "Last Month",
            TimeBucket::ThisYear => "This Year",
            TimeBucket::Older => "Older",
        }
    }

    /// Determines which time bucket an episode belongs to based on its NaiveDateTime
    pub fn from_naive_datetime(dt: NaiveDateTime) -> Self {
        let now = Local::now();
        // Extract naive date for local timezone comparisons
        let now_date = now.date_naive();
        let ep_date = dt.date();

        if ep_date == now_date {
            return TimeBucket::Today;
        }
        if ep_date == now_date - Duration::days(1) {
            return TimeBucket::Yesterday;
        }

        if ep_date.year() == now_date.year() {
            if ep_date.month() == now_date.month() {
                if now_date.signed_duration_since(ep_date) < Duration::days(7) {
                    return TimeBucket::ThisWeek;
                }
                return TimeBucket::ThisMonth;
            }
            if ep_date.month() + 1 == now_date.month() {
                return TimeBucket::LastMonth;
            }
            return TimeBucket::ThisYear;
        }

        if ep_date.year() + 1 == now_date.year() && ep_date.month() == 12 && now_date.month() == 1 {
            return TimeBucket::LastMonth;
        }

        TimeBucket::Older
    }
}

#[derive(Debug)]
pub struct RecentlyUpdatedPage {
    groups: Vec<Controller<GroupedEpisodes>>,
    task_id: Uuid,
}

#[derive(Debug, Clone)]
pub enum RecentlyUpdatedPageInput {
    ActionFinished(Uuid, ActionResult),
    GottenEpisodes(BTreeMap<TimeBucket, Vec<Episode>>),
    DownloadStarted(EpisodeId),
    DownloadCancled(EpisodeId),
    DownloadProgress(EpisodeId, f64),
    DownloadFinished(EpisodeId),
    ChangePlayBackState(PlayState, EpisodeId),
    PlayBackProgress(EpisodeId, f64, u64),
    ChangeEpisodeTo(EpisodeId),
    EpisodeDeleted(EpisodeId),
}

#[derive(Debug, Clone)]
pub enum RecentlyUpdatedPageOutput {
    ExcuteAction(Uuid, Action),
    TogglePlay(EpisodeId),
    RequestDownload(EpisodeId),
    CancleDownload(EpisodeId),
    SetPlayNext(EpisodeId),
    AddToPlaylist(EpisodeId),
    RequestDeleteEpisode(EpisodeId),
    NotifyError(String),
}

#[relm4::component(pub)]
impl Component for RecentlyUpdatedPage {
    type Init = ();
    type Input = RecentlyUpdatedPageInput;
    type Output = RecentlyUpdatedPageOutput;
    type CommandOutput = ();

    view! {
        adw::NavigationPage {
            set_title: "Recently Updated",

            #[wrap(Some)]
            set_child = &adw::ToolbarView {

                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    adw::Clamp {
                        set_maximum_size: 1100,
                        set_tightening_threshold: 900,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 12,
                            set_spacing: 6,
                            set_halign: gtk::Align::Start,

                            gtk::Label {
                                set_margin_top: 40,
                                set_margin_horizontal: 20,
                                set_label: "Recently Updated",
                                set_halign: gtk::Align::Start,
                                set_xalign: 0.0,
                                add_css_class: "title-1"
                            },

                            #[name = "episodes_container"]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 16,
                                set_margin_horizontal: 20,
                                #[watch]
                                set_visible: !model.groups.is_empty(),
                            },

                            adw::StatusPage {
                                #[watch]
                                set_visible: false,

                                set_title: "Your recently updated podcast episodes will appear here",
                                set_icon_name: Some("media-optical-symbolic"),
                                set_vexpand: true,
                                set_hexpand: true,
                            },
                        }
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = RecentlyUpdatedPage {
            groups: Vec::new(),
            task_id: Uuid::new_v4(),
        };

        let widgets = view_output!();
        sender.output(RecentlyUpdatedPageOutput::ExcuteAction(
            model.task_id,
            Action::FetchRecents,
        ));

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            RecentlyUpdatedPageInput::GottenEpisodes(grouped) => {
                while let Some(child) = widgets.episodes_container.first_child() {
                    widgets.episodes_container.remove(&child);
                }
                self.groups.clear();

                for (bucket, bucket_episodes) in grouped {
                    let group_title = bucket.as_str().to_string();

                    let group = GroupedEpisodes::builder()
                        .launch((group_title, bucket_episodes))
                        .forward(sender.output_sender(), |msg| match msg {
                            GroupedEpisodesOutput::TogglePlay(episode_id) => {
                                RecentlyUpdatedPageOutput::TogglePlay(episode_id)
                            }
                            GroupedEpisodesOutput::RequestDownload(episode_id) => {
                                RecentlyUpdatedPageOutput::RequestDownload(episode_id)
                            }
                            GroupedEpisodesOutput::CancleDownload(episode_id) => {
                                RecentlyUpdatedPageOutput::CancleDownload(episode_id)
                            }
                            GroupedEpisodesOutput::SetPlayNext(episode_id) => {
                                RecentlyUpdatedPageOutput::SetPlayNext(episode_id)
                            }
                            GroupedEpisodesOutput::AddToPlaylist(episode_id) => {
                                RecentlyUpdatedPageOutput::AddToPlaylist(episode_id)
                            }
                            GroupedEpisodesOutput::GotoEpisode(_episode_id) => todo!(),
                            GroupedEpisodesOutput::RequestDeleteEpisode(episode_id) => {
                                RecentlyUpdatedPageOutput::RequestDeleteEpisode(episode_id)
                            }
                            GroupedEpisodesOutput::NotifyError(error) => {
                                RecentlyUpdatedPageOutput::NotifyError(error)
                            }
                        });

                    widgets.episodes_container.append(group.widget());
                    self.groups.push(group);
                }
            }

            RecentlyUpdatedPageInput::DownloadStarted(id) => {
                for group in &self.groups {
                    group.emit(GroupedEpisodesInput::DownloadStarted(id));
                }
            }
            RecentlyUpdatedPageInput::DownloadCancled(id) => {
                for group in &self.groups {
                    group.emit(GroupedEpisodesInput::DownloadCancled(id));
                }
            }
            RecentlyUpdatedPageInput::DownloadProgress(id, progress) => {
                for group in &self.groups {
                    group.emit(GroupedEpisodesInput::DownloadProgress(id, progress));
                }
            }
            RecentlyUpdatedPageInput::DownloadFinished(id) => {
                for group in &self.groups {
                    group.emit(GroupedEpisodesInput::DownloadFinished(id));
                }
            }
            RecentlyUpdatedPageInput::ChangePlayBackState(state, id) => {
                for group in &self.groups {
                    group.emit(GroupedEpisodesInput::ChangePlayBackState(state.clone(), id));
                }
            }
            RecentlyUpdatedPageInput::PlayBackProgress(id, progress, duration) => {
                for group in &self.groups {
                    group.emit(GroupedEpisodesInput::PlayBackProgress(
                        id, progress, duration,
                    ));
                }
            }
            RecentlyUpdatedPageInput::ChangeEpisodeTo(id) => {
                for group in &self.groups {
                    group.emit(GroupedEpisodesInput::ChangeEpisodeTo(id));
                }
            }
            RecentlyUpdatedPageInput::EpisodeDeleted(id) => {
                for group in &self.groups {
                    group.emit(GroupedEpisodesInput::EpisodeDeleted(id));
                }
            }
            RecentlyUpdatedPageInput::ActionFinished(uuid, result) => {
                if uuid == self.task_id {
                    if let Ok(grouped_arc) = result.downcast::<BTreeMap<TimeBucket, Vec<Episode>>>()
                    {
                        if let Ok(grouped) = Arc::try_unwrap(grouped_arc) {
                            sender.input(RecentlyUpdatedPageInput::GottenEpisodes(grouped));
                        }
                    }
                }
            }
        }

        self.update_view(widgets, sender);
    }
}
