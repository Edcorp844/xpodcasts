use std::collections::HashMap;

use adw::prelude::*;
use podcasts_data::{Episode, EpisodeId};
use relm4::{Component, ComponentParts, factory::FactoryVecDeque};

use crate::components::episode_list_item::{
    EpisodeListItem, EpisodeListItemInput, EpisodeListItemOutput,
};

#[derive(Debug)]
pub struct GroupedEpisodes {
    group_title: String,
    episodes: FactoryVecDeque<EpisodeListItem>,
    index_by_id: HashMap<EpisodeId, relm4::factory::DynamicIndex>,
}

#[derive(Debug)]
pub enum GroupedEpisodesInput {
    AppendEpisodes(Vec<Episode>),
    DownloadStarted(EpisodeId),
    DownloadCancled(EpisodeId),
    DownloadProgress(EpisodeId, f64),
    DownloadFinished(EpisodeId),
    ChangePlayBackState(gst_play::PlayState, EpisodeId),
    PlayBackProgress(EpisodeId, f64, u64),
    ChangeEpisodeTo(EpisodeId),
    EpisodeDeleted(EpisodeId),
}

#[derive(Debug)]
pub enum GroupedEpisodesOutput {
    TogglePlay(EpisodeId),
    RequestDownload(EpisodeId),
    CancleDownload(EpisodeId),
    SetPlayNext(EpisodeId),
    AddToPlaylist(EpisodeId),
    GotoEpisode(EpisodeId),
    RequestDeleteEpisode(EpisodeId),
    NotifyError(String),
}

#[relm4::component(pub)]
impl Component for GroupedEpisodes {
    type Init = (String, Vec<Episode>);
    type Input = GroupedEpisodesInput;
    type Output = GroupedEpisodesOutput;
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_halign: gtk::Align::Start,
            set_spacing: 20,
            set_margin_top: 40,
            set_hexpand: true,
            set_vexpand: true,


            gtk::Label {
                set_label: &model.group_title,
                add_css_class: "title-4",
                set_halign: gtk::Align::Start,
                set_xalign: 0.0,
            },

            #[local_ref]
            episodes_widget -> gtk::ListBox {
                add_css_class: "boxed-list",
                set_vexpand: true,
            },
        }
    }

    fn init(
        (year, episodes_list): Self::Init,
        root: Self::Root,
        sender: relm4::prelude::ComponentSender<Self>,
    ) -> relm4::prelude::ComponentParts<Self> {
        let model = Self {
            group_title: year,
            episodes: FactoryVecDeque::builder()
                .launch(gtk::ListBox::builder().build())
                .forward(sender.output_sender(), |msg| match msg {
                    EpisodeListItemOutput::TogglePlay(episode_id) => {
                        GroupedEpisodesOutput::TogglePlay(episode_id)
                    }
                    EpisodeListItemOutput::RequestDownload(episode_id) => {
                        GroupedEpisodesOutput::RequestDeleteEpisode(episode_id)
                    }
                    EpisodeListItemOutput::CancleDownload(episode_id) => {
                        GroupedEpisodesOutput::CancleDownload(episode_id)
                    }
                    EpisodeListItemOutput::SetPlayNext(episode_id) => {
                        GroupedEpisodesOutput::SetPlayNext(episode_id)
                    }
                    EpisodeListItemOutput::AddToPlaylist(episode_id) => {
                        GroupedEpisodesOutput::AddToPlaylist(episode_id)
                    }
                    EpisodeListItemOutput::GotoEpisode(episode_id) => {
                        GroupedEpisodesOutput::GotoEpisode(episode_id)
                    }
                    EpisodeListItemOutput::RequestDeleteEpisode(episode_id) => {
                        GroupedEpisodesOutput::RequestDeleteEpisode(episode_id)
                    }
                    EpisodeListItemOutput::NotifyError(error) => {
                        GroupedEpisodesOutput::NotifyError(error)
                    }
                }),
            index_by_id: HashMap::new(),
        };

        let episodes_widget = model.episodes.widget();

        let widgets = view_output!();

        sender.input(GroupedEpisodesInput::AppendEpisodes(episodes_list));

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: relm4::prelude::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            GroupedEpisodesInput::AppendEpisodes(episodes) => {
                let mut guard = self.episodes.guard();
                guard.clear();
                for episode in episodes {
                    let index = guard.push_back(episode.clone());
                    self.index_by_id.insert(episode.id(), index);
                }
            }
            GroupedEpisodesInput::DownloadStarted(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes
                        .send(index.current_index(), EpisodeListItemInput::DownloadStarted);
                }
            }
            GroupedEpisodesInput::DownloadCancled(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes
                        .send(index.current_index(), EpisodeListItemInput::DownloadCancled);
                }
            }
            GroupedEpisodesInput::DownloadProgress(episode_id, fraction) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::DownloadProgress(fraction),
                    );
                }
            }
            GroupedEpisodesInput::DownloadFinished(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::DownloadFinished,
                    );
                }
            }
            GroupedEpisodesInput::ChangePlayBackState(play_state, episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::ChangePlayBackState(play_state),
                    );
                }
            }
            GroupedEpisodesInput::PlayBackProgress(episode_id, pos, rem) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        EpisodeListItemInput::PlayBackProgress(pos, rem),
                    );
                }
            }
            GroupedEpisodesInput::ChangeEpisodeTo(episode_id) => {
                self.episodes
                    .broadcast(EpisodeListItemInput::ChangeEpisodeTo(episode_id));
            }
            GroupedEpisodesInput::EpisodeDeleted(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes
                        .send(index.current_index(), EpisodeListItemInput::EpisodeDeleted);
                }
            }
        }

        self.update_view(widgets, sender);
    }
}
