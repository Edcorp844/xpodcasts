use std::collections::HashMap;

use adw::prelude::*;

use gst_play::PlayState;
use podcasts_data::{
    EpisodeId, dbqueries,
    discovery::{FoundPodcast, SearchError, search},
};
use relm4::{Component, ComponentParts, ComponentSender, prelude::*};
use uuid::Uuid;

use crate::{
    app_navigation_ext::PageController,
    components::podcast_search_results::{
        PodcastResults, PodcastResultsInput, PodcastResultsOutput,
    },
    pages::podcast::{PodcastPage, PodcastPageOutput},
    settings::GenaralSettings,
    workers::action_worker::worker::ActionResult,
};

// 1. Define the possible pages you want to visit across your app
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageTarget {
    MainPodcastsList,
    PodcastDetails,
    EpisodeDescription,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultCategory {
    Podcast,
    library,
}

#[derive(Debug)]
pub struct SearchPage {
    pub current_page: PageTarget,
    result_category: ResultCategory,
    pub active_pages: HashMap<String, PageController>,
    podcast_results_widget: Controller<PodcastResults>,
}

#[derive(Debug)]
pub enum SearchPageInput {
    ActionFinished(Uuid, ActionResult),
    PushPage(String),
    OpenPodcast(FoundPodcast),
    PopPage,
    SwitchResultCategory(ResultCategory),
    PodcastsLoaded(Result<Vec<FoundPodcast>, SearchError>),
    UpdateQuery(String),
    TriggerSearch(String),
    Subscribe(String),
    DownloadStarted(EpisodeId),
    DownloadCancled(EpisodeId),
    DownloadProgress(EpisodeId, f64),
    DownloadFinished(EpisodeId),
    ChangePlayBackState(PlayState, EpisodeId),
    PlayBackProgress(EpisodeId, f64, u64),
    ChangeEpisodeTo(EpisodeId),
}

#[derive(Debug)]
pub enum SearchPageOutput {
    UpdateISSearching(bool),
    TogglePlay(EpisodeId),
    NotifyError(String),
    RequestDownload(EpisodeId),
    CancleDownload(EpisodeId),
    Subscribe(String),
    SetPlayNext(EpisodeId),
    AddToPlaylist(EpisodeId),
    RequestDeleteEpisode(EpisodeId),
}

#[derive(Debug)]
pub enum SearchPageCmdInput {
    Podcasts(Result<Vec<FoundPodcast>, SearchError>),
}

#[relm4::component(pub)]
impl Component for SearchPage {
    type Init = ();
    type Input = SearchPageInput;
    type Output = SearchPageOutput;
    type CommandOutput = SearchPageCmdInput;

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let podcast_results_widget =
            PodcastResults::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    PodcastResultsOutput::Subscribe(feed) => SearchPageInput::Subscribe(feed),
                    PodcastResultsOutput::OpenPodcast(podcast) => {
                        SearchPageInput::OpenPodcast(podcast)
                    }
                });
        let model = Self {
            current_page: PageTarget::MainPodcastsList,
            result_category: ResultCategory::Podcast,
            active_pages: HashMap::new(),
            podcast_results_widget,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input, // Keeps original ownership here
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match &message {
            SearchPageInput::Subscribe(feed) => {
                let _ = sender.output(SearchPageOutput::Subscribe(feed.clone()));
            }

            SearchPageInput::PodcastsLoaded(data) => match data {
                Ok(data) => {
                    let _ = sender.output(SearchPageOutput::UpdateISSearching(false));
                    self.podcast_results_widget
                        .emit(PodcastResultsInput::Results(data.clone()));
                }
                Err(_error) => {}
            },

            SearchPageInput::TriggerSearch(text) => {
                self.podcast_results_widget
                    .emit(PodcastResultsInput::SearchBegan);
                let _ = sender.output(SearchPageOutput::UpdateISSearching(true));
                let search_text = text.clone();
                println!("Searching: {search_text}");
                sender.oneshot_command(async move {
                    let search_platforms = GenaralSettings::new().get_search_platforms();
                    for id in search_platforms {
                        match dbqueries::set_discovery_setting(&id, true) {
                            Err(e) => {
                                println!("Error settings: {}", e);
                            }
                            Ok(_) => {}
                        }
                    }

                    SearchPageCmdInput::Podcasts(search(&search_text).await)
                });
            }
            SearchPageInput::UpdateQuery(_text) => {}
            SearchPageInput::OpenPodcast(podcast) => {
                let key = podcast.title.clone();
                let podcast_page = PodcastPage::builder().launch(podcast.clone()).forward(
                    sender.output_sender(),
                    |msg| match msg {
                        PodcastPageOutput::TogglePlay(episode) => {
                            SearchPageOutput::TogglePlay(episode)
                        }
                        PodcastPageOutput::Subscribe(feed) => SearchPageOutput::Subscribe(feed),
                        PodcastPageOutput::NotifyError(_) => todo!(),
                        PodcastPageOutput::RequestDownload(episode_id) => {
                            SearchPageOutput::RequestDownload(episode_id)
                        }
                        PodcastPageOutput::CancleDownload(episode_id) => {
                            SearchPageOutput::CancleDownload(episode_id)
                        }
                        PodcastPageOutput::SetPlayNext(episode_id) => {
                            SearchPageOutput::SetPlayNext(episode_id)
                        }
                        PodcastPageOutput::AddToPlaylist(episode_id) => {
                            SearchPageOutput::AddToPlaylist(episode_id)
                        }
                        PodcastPageOutput::GotoEpisode(episode_id) => todo!(),
                        PodcastPageOutput::RequestDeleteEpisode(episode_id) => {
                            SearchPageOutput::RequestDeleteEpisode(episode_id)
                        }
                    },
                );
                let controller = PageController::Podcast(podcast_page);
                self.active_pages.insert(key.to_string(), controller);
                sender.input(SearchPageInput::PushPage(key.to_string()));
            }
            SearchPageInput::PushPage(page) => {
                if let Some(PageController::Podcast(page_ctrl)) = self.active_pages.get(page) {
                    widgets.nav_view.push(page_ctrl.widget());
                }
            }
            SearchPageInput::PopPage => {
                widgets.nav_view.pop();
            }
            SearchPageInput::SwitchResultCategory(category) => {
                self.result_category = *category;
            }
            SearchPageInput::DownloadStarted(episode_id) => todo!(),
            SearchPageInput::DownloadCancled(episode_id) => todo!(),
            SearchPageInput::DownloadProgress(episode_id, _) => todo!(),
            SearchPageInput::DownloadFinished(episode_id) => {
                for (_, page) in &self.active_pages {
                    page.notify_download_finished(episode_id.clone());
                }
            }
            SearchPageInput::ChangePlayBackState(state, episode_id) => {
                for (_, page) in &self.active_pages {
                    page.notify_playing_state(episode_id.clone(), state.clone());
                }
            }
            SearchPageInput::PlayBackProgress(episode_id, pos, rem) => {
                for (_, page) in &self.active_pages {
                    page.notify_playback_progress(episode_id.clone(), pos.clone(), rem.clone());
                }
            }
            SearchPageInput::ChangeEpisodeTo(episode_id) => {
                for (_, page) in &self.active_pages {
                    page.notify_current_episode(episode_id.clone());
                }
            }
            SearchPageInput::ActionFinished(uuid, result) => {
                for (_, page) in &self.active_pages {
                    page.notify_action_finished(uuid.clone(), result.clone());
                }
            }
        }

        self.update(message, sender, root);
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            SearchPageCmdInput::Podcasts(data) => {
                sender.input(SearchPageInput::PodcastsLoaded(data))
            }
        }
    }

    view! {
        adw::NavigationPage {
            set_title: "Explore",
            set_tag: Some("main-navigator-root"),

            #[wrap(Some)]
            #[name = "nav_view"]
            set_child = &adw::NavigationView {

                // --- PAGE 1: Root List View ---
                #[name = "root_page"]
                add = &adw::NavigationPage {
                    set_title: "Podcasts",
                    set_tag: Some("root-podcasts-page"),

                    #[wrap(Some)]
                    set_child = &adw::ToolbarView {

                        add_top_bar = &adw::HeaderBar {
                            set_show_start_title_buttons: false,
                            set_show_end_title_buttons: false,


                            #[wrap(Some)]
                            set_title_widget = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 6,
                                set_hexpand: true,
                                set_halign: gtk::Align::Center,
                                set_valign: gtk::Align::Center,
                                set_size_request: (360, -1),

                                gtk::SearchEntry {
                                    set_placeholder_text: Some("Search for podcasts..."),
                                    set_hexpand: true,
                                    set_halign: gtk::Align::Fill,

                                    connect_search_changed[sender] => move |entry| {
                                        let text = entry.text().to_string();
                                        sender.input(SearchPageInput::UpdateQuery(text));
                                    },

                                    connect_activate[sender] => move |entry| {
                                        let text = entry.text().to_string();
                                        sender.input(SearchPageInput::TriggerSearch(text));
                                    }
                                },

                            },

                            pack_end=&adw::ToggleGroup {
                                set_homogeneous: true,
                                add_css_class: "round",

                                set_active_name: Some("podcasts"),

                                connect_active_name_notify[sender] => move |group| {
                                    if let Some(active_tab) = group.active_name() {
                                        match active_tab.as_str() {
                                            "podcasts" => {  sender.input(SearchPageInput::SwitchResultCategory(ResultCategory::Podcast)) }
                                            "library" => { sender.input(SearchPageInput::SwitchResultCategory(ResultCategory::Podcast))  }
                                            _ => {}
                                        }
                                    }
                                },

                                add = adw::Toggle {
                                    set_name: Some("podcasts"),
                                    set_label: Some("Podcasts"),

                                },

                                add = adw::Toggle {
                                    set_name: Some("library"),
                                    set_label: Some("Library"),

                                }
                            }
                        },

                       #[wrap(Some)]
                        set_content = model.podcast_results_widget.widget(),
                    }
                }
            },
        }
    }
}
