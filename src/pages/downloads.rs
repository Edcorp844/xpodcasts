use std::collections::HashMap;

use adw::prelude::*;
use gst_play::PlayState;
use podcasts_data::{
    EpisodeId, EpisodeModel, EpisodeWidgetModel,
    dbqueries::{self, ShowFilter},
};
use relm4::{Component, prelude::*};
use uuid::Uuid;

use crate::{
    components::downloaded_episode_list_item::{
        DownloadedEpisodeListItem, DownloadedEpisodeListItemInput, DownloadedEpisodeListItemOutput,
    },
    workers::action_worker::worker::ActionResult,
};

#[derive(Debug)]
pub struct DownloadsPage {
    episodes: FactoryVecDeque<DownloadedEpisodeListItem>,
    index_by_id: HashMap<EpisodeId, relm4::factory::DynamicIndex>,
    is_loading: bool,
}

#[derive(Debug, Clone)]
pub enum DownloadsPageInput {
    ActionFinished(Uuid, ActionResult),
    FetchDownloads,
    GottenEpisodes(Vec<EpisodeWidgetModel>),
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
pub enum DownloadsPageOutput {
    TogglePlay(EpisodeId),
    NotifyError(String),
    RequestDeleteEpisode(EpisodeId),
}

#[relm4::component(pub)]
impl Component for DownloadsPage {
    type Init = ();
    type Input = DownloadsPageInput;
    type Output = DownloadsPageOutput;
    type CommandOutput = ();

    view! {
        adw::NavigationPage {
            set_title: "Downloads Page",

           #[wrap(Some)]
            set_child = &adw::ToolbarView {

               #[wrap(Some)]
                set_content= &gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    adw::Clamp {
                        set_maximum_size: 1100,
                        set_tightening_threshold: 900,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 12,
                            set_spacing: 6,

                             gtk::Label {
                                set_margin_top: 40,
                                set_margin_horizontal: 20,
                                set_label: "Downloads",
                                set_halign:gtk::Align::Start,

                                add_css_class: "title-1"
                            },

                            #[local_ref]
                            episodes_container -> gtk::ListBox {
                                #[watch]
                                set_visible: !model.episodes.is_empty(),
                                set_margin_all: 20,
                                add_css_class: "boxed-list",
                            },

                           adw::StatusPage {
                                #[watch]
                                set_visible: model.episodes.is_empty(),

                                set_title: "You downloaded episodes will appear here",
                                //set_description: Some("You downloaded episodes will appear here"),
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
        let episodes_parent = gtk::ListBox::builder().build();
        let model = DownloadsPage {
            episodes: FactoryVecDeque::builder().launch(episodes_parent).forward(
                sender.output_sender(),
                |msg| match msg {
                    DownloadedEpisodeListItemOutput::TogglePlay(id) => {
                        DownloadsPageOutput::TogglePlay(id)
                    }
                    DownloadedEpisodeListItemOutput::RequestDeleteEpisode(episode_id) => {
                        DownloadsPageOutput::RequestDeleteEpisode(episode_id)
                    }
                    DownloadedEpisodeListItemOutput::NotifyError(error) => {
                        DownloadsPageOutput::NotifyError(error)
                    }
                },
            ),
            is_loading: true,
            index_by_id: HashMap::new(),
        };

        let episodes_container = model.episodes.widget();

        let widgets = view_output!();

        sender.input(DownloadsPageInput::FetchDownloads);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            DownloadsPageInput::FetchDownloads => {
                self.is_loading = true;
                let filter = ShowFilter {
                    any_downloaded: Some(true),
                    completed: None,
                    title_or_description: None,
                    reverse_order: true,
                };
                match dbqueries::get_podcasts_filter(&[], &filter) {
                    Ok(shows) => {
                        for show in shows.iter() {
                            let filter = dbqueries::EpisodeFilter {
                                downloaded: Some(true),
                                played: None,
                                search: None,
                                reverse_order: false,
                            };

                            match dbqueries::get_pd_episode_widgets(show, &filter) {
                                Ok(episodes) => {
                                    sender.input(DownloadsPageInput::GottenEpisodes(episodes));
                                }
                                Err(error) => {
                                    let _ = sender.output(DownloadsPageOutput::NotifyError(
                                        error.to_string(),
                                    ));
                                }
                            }
                        }
                    }
                    Err(error) => {
                        let _ = sender.output(DownloadsPageOutput::NotifyError(error.to_string()));
                    }
                }

                self.is_loading = false;
            }
            DownloadsPageInput::GottenEpisodes(episodes) => {
                let mut guard = self.episodes.guard();
                //guard.clear();

                for episode in episodes.iter() {
                    let index = guard.push_back(episode.clone());
                    self.index_by_id.insert(episode.id(), index);
                }
            }
            DownloadsPageInput::DownloadStarted(episode_id) => {
                dbg!(episode_id);
            }
            DownloadsPageInput::DownloadCancled(_episode_id) => {}
            DownloadsPageInput::DownloadProgress(_episode_id, _) => {}
            DownloadsPageInput::DownloadFinished(_episode_id) => {
                sender.input(DownloadsPageInput::FetchDownloads)
            }
            DownloadsPageInput::ChangePlayBackState(play_state, episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        DownloadedEpisodeListItemInput::ChangePlayBackState(play_state),
                    );
                }
            }
            DownloadsPageInput::PlayBackProgress(episode_id, pos, rem) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    self.episodes.send(
                        index.current_index(),
                        DownloadedEpisodeListItemInput::PlayBackProgress(pos, rem),
                    );
                }
            }
            DownloadsPageInput::ChangeEpisodeTo(episode_id) => {
                self.episodes
                    .broadcast(DownloadedEpisodeListItemInput::ChangeEpisodeTo(episode_id));
            }
            DownloadsPageInput::EpisodeDeleted(episode_id) => {
                if let Some(index) = self.index_by_id.get(&episode_id) {
                    let mut guard = self.episodes.guard();
                    guard.remove(index.current_index());
                }
            }
            DownloadsPageInput::ActionFinished(uuid, result) => {}
        }
    }
}
