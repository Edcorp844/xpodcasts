use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
    sync::Arc,
};

use adw::prelude::*;
use chrono::Datelike;
use gst_play::PlayState;
use podcasts_data::{Episode, EpisodeId, ShowId};
use relm4::{Component, prelude::*};
use uuid::Uuid;

use crate::{
    components::episode_group::{GroupedEpisodes, GroupedEpisodesInput, GroupedEpisodesOutput},
    workers::action_worker::worker::{Action, ActionResult},
};

/// Shared, mutable state that both the component's normal update loop and
/// the detached idle-callback chunks need to touch. Wrapping it once here
/// avoids smuggling non-Debug/non-Clone Controllers through Input messages.
#[derive(Default)]
struct GroupState {
    groups: Vec<Controller<GroupedEpisodes>>,
    pending: VecDeque<(i32, Vec<Episode>)>,
}

#[derive(Debug)]
pub struct AllEpisodesPage {
    #[allow(dead_code)]
    state: Rc<RefCell<GroupState>>,
    task_id: Uuid,
}

impl std::fmt::Debug for GroupState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupState")
            .field("groups_len", &self.groups.len())
            .field("pending_len", &self.pending.len())
            .finish()
    }
}

#[derive(Debug)]
pub enum AllEpisodesPageInput {
    SetEpisodes(Vec<Episode>),
    DownloadStarted(EpisodeId),
    DownloadCancled(EpisodeId),
    DownloadProgress(EpisodeId, f64),
    DownloadFinished(EpisodeId),
    ChangePlayBackState(PlayState, EpisodeId),
    PlayBackProgress(EpisodeId, f64, u64),
    ChangeEpisodeTo(EpisodeId),
    EpisodeDeleted(EpisodeId),
    ActionFinished(Uuid, ActionResult),
}

#[derive(Debug)]
pub enum AllEpisodesPageOutput {
    TogglePlay(EpisodeId),
    RequestDownload(EpisodeId),
    CancleDownload(EpisodeId),
    SetPlayNext(EpisodeId),
    AddToPlaylist(EpisodeId),
    RequestDeleteEpisode(EpisodeId),
    ExecuteAction(Uuid, Action),
    NotifyError(String),
}

#[derive(Debug)]
pub enum AllEpisodesPageCmdOutput {
    EpisodesGrouped(BTreeMap<i32, Vec<Episode>>),
}

#[relm4::component(pub)]
impl Component for AllEpisodesPage {
    type Init = ShowId;
    type Input = AllEpisodesPageInput;
    type Output = AllEpisodesPageOutput;
    type CommandOutput = AllEpisodesPageCmdOutput;

    view! {
        adw::NavigationPage {
            set_title: "Episodes",

           #[wrap(Some)]
            set_child = &adw::ToolbarView {

                 add_top_bar=&adw::HeaderBar {
                    set_show_title: false,
                 },

                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    adw::Clamp {
                        set_maximum_size: 1100,
                        set_tightening_threshold: 900,

                        #[name="group_parent"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 12,
                            set_spacing: 6,
                        }
                    }

                }
            }
        }
    }

    fn init(
        show_id: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AllEpisodesPage {
            state: Rc::new(RefCell::new(GroupState::default())),
            task_id: Uuid::new_v4(),
        };
        let widgets = view_output!();

        let task_id = model.task_id;

        let sender_for_shown = sender.clone();
        root.connect_shown(move |_| {
            let _ = sender_for_shown.output(AllEpisodesPageOutput::ExecuteAction(
                task_id,
                Action::FetchShowEpisodes(show_id),
            ));
        });

        let state_for_cleanup = model.state.clone();
        let group_parent_for_cleanup = widgets.group_parent.clone();
        root.connect_hidden(move |_| {
            let mut state = state_for_cleanup.borrow_mut();
            for group in state.groups.drain(..) {
                group_parent_for_cleanup.remove(group.widget());
            }
            state.pending.clear();
        });

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AllEpisodesPageInput::SetEpisodes(episodes) => {
                // Sorting + bucketing thousands of episodes is real CPU
                // work — do it off the main thread so the UI doesn't
                // freeze while it happens.
                sender.oneshot_command(async move {
                    let grouped = relm4::spawn_blocking(move || {
                        let mut episodes = episodes;
                        episodes.sort_by(|a, b| b.epoch().cmp(&a.epoch()));

                        let mut grouped: BTreeMap<i32, Vec<Episode>> = BTreeMap::new();
                        for episode in episodes {
                            let year = episode.epoch().year();
                            grouped.entry(year).or_default().push(episode);
                        }
                        grouped
                    })
                    .await
                    .unwrap_or_default();

                    AllEpisodesPageCmdOutput::EpisodesGrouped(grouped)
                });
            }
            AllEpisodesPageInput::DownloadStarted(episode_id) => {
                for group in &self.state.borrow().groups {
                    group.emit(GroupedEpisodesInput::DownloadStarted(episode_id));
                }
            }
            AllEpisodesPageInput::DownloadCancled(episode_id) => {
                for group in &self.state.borrow().groups {
                    group.emit(GroupedEpisodesInput::DownloadCancled(episode_id));
                }
            }
            AllEpisodesPageInput::DownloadProgress(episode_id, fraction) => {
                for group in &self.state.borrow().groups {
                    group.emit(GroupedEpisodesInput::DownloadProgress(episode_id, fraction));
                }
            }
            AllEpisodesPageInput::DownloadFinished(episode_id) => {
                for group in &self.state.borrow().groups {
                    group.emit(GroupedEpisodesInput::DownloadFinished(episode_id));
                }
            }
            AllEpisodesPageInput::ChangePlayBackState(play_state, episode_id) => {
                for group in &self.state.borrow().groups {
                    group.emit(GroupedEpisodesInput::ChangePlayBackState(
                        play_state, episode_id,
                    ));
                }
            }
            AllEpisodesPageInput::PlayBackProgress(episode_id, pos, rem) => {
                for group in &self.state.borrow().groups {
                    group.emit(GroupedEpisodesInput::PlayBackProgress(episode_id, pos, rem));
                }
            }
            AllEpisodesPageInput::ChangeEpisodeTo(episode_id) => {
                for group in &self.state.borrow().groups {
                    group.emit(GroupedEpisodesInput::ChangeEpisodeTo(episode_id));
                }
            }
            AllEpisodesPageInput::EpisodeDeleted(episode_id) => {
                for group in &self.state.borrow().groups {
                    group.emit(GroupedEpisodesInput::EpisodeDeleted(episode_id));
                }
            }
            AllEpisodesPageInput::ActionFinished(uuid, result) => {
                if uuid == self.task_id {
                    if let Ok(episodes_arc) = result.downcast::<Vec<Episode>>() {
                        if let Ok(episodes) = Arc::try_unwrap(episodes_arc) {
                            sender.input(AllEpisodesPageInput::SetEpisodes(episodes));
                        }
                    }
                }
            }
        }
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AllEpisodesPageCmdOutput::EpisodesGrouped(grouped) => {
                {
                    let mut state = self.state.borrow_mut();

                    // Drop old controllers AND remove their widgets —
                    // clearing the Vec alone leaves stale widgets attached
                    // to group_parent forever.
                    for old_group in state.groups.drain(..) {
                        widgets.group_parent.remove(old_group.widget());
                    }

                    state.pending.clear();
                    state.pending.extend(grouped.into_iter().rev());
                }

                Self::process_next_chunk(self.state.clone(), widgets.group_parent.clone(), sender);
            }
        }
    }
}

impl AllEpisodesPage {
    /// Pops one year-group off the pending queue, builds its
    /// GroupedEpisodes component, appends its widget, and reschedules
    /// itself on the GLib idle queue if more remain — yielding back to
    /// the main loop between each one so the UI stays responsive even
    /// with tens of thousands of episodes across many years.
    fn process_next_chunk(
        state: Rc<RefCell<GroupState>>,
        group_parent: gtk::Box,
        sender: ComponentSender<Self>,
    ) {
        gtk::glib::idle_add_local_once(move || {
            let next = state.borrow_mut().pending.pop_front();

            if let Some((year, episodes)) = next {
                let group = GroupedEpisodes::builder()
                    .launch((format!("{year}"), episodes))
                    .forward(sender.output_sender(), |msg| match msg {
                        GroupedEpisodesOutput::TogglePlay(episode_id) => {
                            AllEpisodesPageOutput::TogglePlay(episode_id)
                        }
                        GroupedEpisodesOutput::RequestDownload(episode_id) => {
                            AllEpisodesPageOutput::RequestDownload(episode_id)
                        }
                        GroupedEpisodesOutput::CancleDownload(episode_id) => {
                            AllEpisodesPageOutput::CancleDownload(episode_id)
                        }
                        GroupedEpisodesOutput::SetPlayNext(episode_id) => {
                            AllEpisodesPageOutput::SetPlayNext(episode_id)
                        }
                        GroupedEpisodesOutput::AddToPlaylist(episode_id) => {
                            AllEpisodesPageOutput::AddToPlaylist(episode_id)
                        }
                        GroupedEpisodesOutput::GotoEpisode(_episode_id) => todo!(),
                        GroupedEpisodesOutput::RequestDeleteEpisode(episode_id) => {
                            AllEpisodesPageOutput::RequestDeleteEpisode(episode_id)
                        }
                        GroupedEpisodesOutput::NotifyError(error) => {
                            AllEpisodesPageOutput::NotifyError(error)
                        }
                    });

                group_parent.append(group.widget());
                state.borrow_mut().groups.push(group);

                let more_remaining = !state.borrow().pending.is_empty();
                if more_remaining {
                    Self::process_next_chunk(state.clone(), group_parent.clone(), sender.clone());
                }
            }
        });
    }
}
