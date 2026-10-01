use std::collections::HashMap;

use gst_play::PlayState;
use podcasts_data::discovery::{ALL_PLATFORM_IDS, FoundPodcast, SearchError, search};
use podcasts_data::{EpisodeId, dbqueries};
use relm4::adw::prelude::*;
use relm4::prelude::*;
use uuid::Uuid;

use crate::app_navigation_ext::PageController;
use crate::components::found_podcast_ui::{FoundCardOutput, FoundPodcastsCard};
use crate::pages::podcast::{PodcastPage, PodcastPageOutput};
use crate::workers::action_worker::worker::ActionResult;

#[derive(Debug)]
pub struct HomePage {
    pub podcasts: FactoryVecDeque<FoundPodcastsCard>,
    pub is_loading: bool,
    pub active_pages: HashMap<String, PageController>,
}

#[derive(Debug)]
pub enum HomePageCommand {
    Podcasts(Result<Vec<FoundPodcast>, SearchError>),
}

#[derive(Debug)]
pub enum HomePageInput {
    ActionFinished(Uuid, ActionResult),
    FetchPodcasts,
    PodcastsLoaded(Result<Vec<FoundPodcast>, SearchError>),
    Subscribe(String),
    PushPage(String),
    OpenPodcast(FoundPodcast),
    DownloadStarted(EpisodeId),
    DownloadCancled(EpisodeId),
    DownloadProgress(EpisodeId, f64),
    DownloadFinished(EpisodeId),
    ChangePlayBackState(PlayState, EpisodeId),
    PlayBackProgress(EpisodeId, f64, u64),
    ChangeEpisodeTo(EpisodeId),
}

#[derive(Debug)]
pub enum HomPageOutPut {
    ToggleSideBar,
    Subscribe(String),
    TogglePlay(EpisodeId),
    NotifyError(String),
    RequestDownload(EpisodeId),
    CancleDownload(EpisodeId),
    SetPlayNext(EpisodeId),
    AddToPlaylist(EpisodeId),
    //GotoEpisode(EpisodeId),
    RequestDeleteEpisode(EpisodeId),
}

#[relm4::component(pub)]
impl Component for HomePage {
    type Init = ();
    type Input = HomePageInput;
    type Output = HomPageOutPut;
    type CommandOutput = HomePageCommand;

    view! {
        adw::NavigationPage {
            set_title: "Podcasts",

            #[wrap(Some)]
            #[name = "nav_view"]
            set_child = &adw::NavigationView {

                // --- PAGE 1: Root List View ---
                #[name = "root_page"]
                add = &adw::NavigationPage {

                    #[wrap(Some)]
                    set_child = &adw::ToolbarView {

                        #[wrap(Some)]
                        set_content = &gtk::ScrolledWindow {
                                set_vexpand : true,
                                set_hscrollbar_policy: gtk::PolicyType::Never,

                                adw::Clamp {
                                    set_maximum_size: 1400,
                                    set_tightening_threshold: 1000,

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        gtk::Label {
                                            set_margin_top: 40,
                                            set_margin_horizontal: 20,
                                            set_label: "Home",
                                            set_halign:gtk::Align::Start,

                                            add_css_class: "title-1"
                                        },

                                        #[local_ref]
                                        podcast_grid -> gtk::FlowBox {
                                            set_margin_all: 20,
                                            #[watch]
                                            set_visible: !model.is_loading,
                                        }
                                    }
                                }
                            }
                    }
                }
            }
        }
    }

    fn init(
        _worker_sender: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Build your customized FlowBox layout container manually
        let grid = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .column_spacing(20)
            .row_spacing(40)
            .homogeneous(true)
            .build();

        // Attach your dynamic width column calculating callback hook
        let last_width = std::cell::Cell::new(0);
        grid.add_tick_callback(move |grid, _| {
            let width = grid.width();
            if width != last_width.get() {
                last_width.set(width);
                let columns = match width {
                    0..=500 => 2,
                    501..=800 => 3,
                    801..=1100 => 4,
                    _ => 5,
                };
                grid.set_min_children_per_line(columns);
                grid.set_max_children_per_line(columns);
            }
            gtk::glib::ControlFlow::Continue
        });

        let model = HomePage {
            podcasts: FactoryVecDeque::builder().launch(grid).forward(
                sender.input_sender(),
                |msg| match msg {
                    FoundCardOutput::Subscribe(feed) => HomePageInput::Subscribe(feed),
                    FoundCardOutput::OpenPodcastPage(podcast) => {
                        HomePageInput::OpenPodcast(podcast)
                    }
                },
            ),
            active_pages: HashMap::new(),
            is_loading: true,
        };

        // Resolve the reference for the local_ref macro parameter mapping step
        let podcast_grid = model.podcasts.widget();
        let widgets = view_output!();

        sender.input(HomePageInput::FetchPodcasts);

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
            HomePageInput::FetchPodcasts => {
                let search_texts = vec!["uganda", "news", "footboall", "Jesus", "phaneroo"];

                for text in search_texts {
                    // Remove '|text|'—the variable is automatically captured by the 'move' keyword
                    sender.oneshot_command(async move {
                        for id in ALL_PLATFORM_IDS {
                            match dbqueries::set_discovery_setting(id, true) {
                                Err(e) => {
                                    println!("Error settings: {}", e);
                                }
                                Ok(_) => {}
                            }
                        }

                        HomePageCommand::Podcasts(search(&text).await)
                    });
                }
            }

            HomePageInput::Subscribe(feed) => {
                let _ = sender.output(HomPageOutPut::Subscribe(feed));
            }
            HomePageInput::PodcastsLoaded(podcasts) => {
                match podcasts {
                    Ok(data) => {
                        let mut guard = self.podcasts.guard();
                        //guard.clear(); // Flush old elements cleanly
                        for podcast in data {
                            guard.push_back(podcast);
                        }
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        let _ = sender.output(HomPageOutPut::NotifyError(format!(
                            "Podcast Search Error: {}",
                            e.to_string()
                        )));
                    }
                }
                self.is_loading = false;
            }
            HomePageInput::OpenPodcast(podcast) => {
                let key = podcast.title.clone();
                let podcast_page = PodcastPage::builder().launch(podcast.clone()).forward(
                    sender.output_sender(),
                    |msg| match msg {
                        PodcastPageOutput::TogglePlay(episode) => {
                            HomPageOutPut::TogglePlay(episode)
                        }
                        PodcastPageOutput::Subscribe(feed) => HomPageOutPut::Subscribe(feed),
                        PodcastPageOutput::NotifyError(error) => HomPageOutPut::NotifyError(error),
                        PodcastPageOutput::RequestDownload(episode_id) => {
                            HomPageOutPut::RequestDownload(episode_id)
                        }
                        PodcastPageOutput::CancleDownload(episode_id) => {
                            HomPageOutPut::CancleDownload(episode_id)
                        }
                        PodcastPageOutput::SetPlayNext(episode_id) => {
                            HomPageOutPut::SetPlayNext(episode_id)
                        }
                        PodcastPageOutput::AddToPlaylist(episode_id) => {
                            HomPageOutPut::AddToPlaylist(episode_id)
                        }
                        PodcastPageOutput::RequestDeleteEpisode(episode_id) => {
                            HomPageOutPut::RequestDeleteEpisode(episode_id)
                        }
                        _ => HomPageOutPut::NotifyError(format!("")),
                    },
                );
                let controller = PageController::Podcast(podcast_page);
                self.active_pages.insert(key.to_string(), controller);
                sender.input(HomePageInput::PushPage(key.to_string()));
            }
            HomePageInput::PushPage(ref page) => {
                if let Some(PageController::Podcast(page_ctrl)) = self.active_pages.get(page) {
                    widgets.nav_view.push(page_ctrl.widget());
                }
            }
            HomePageInput::DownloadStarted(episode_id) => {
                for (_, page) in &self.active_pages {
                    page.notify_download_started(episode_id.clone());
                }
            }
            HomePageInput::DownloadCancled(episode_id) => {
                println!("Cancle Donwload for {:?} reached home page", episode_id);
            }
            HomePageInput::DownloadProgress(episode_id, fraction) => {
                for (_, page) in &self.active_pages {
                    page.notify_download_progress(episode_id, fraction);
                }
            }
            HomePageInput::DownloadFinished(episode_id) => {
                for (_, page) in &self.active_pages {
                    page.notify_download_finished(episode_id);
                }
            }
            HomePageInput::ChangePlayBackState(state, episode_id) => {
                for (_, page) in &self.active_pages {
                    page.notify_playing_state(episode_id, state);
                }
            }
            HomePageInput::PlayBackProgress(episode_id, pos, rem) => {
                for (_, page) in &self.active_pages {
                    page.notify_playback_progress(episode_id, pos, rem);
                }
            }
            HomePageInput::ChangeEpisodeTo(episode_id) => {
                for (_, page) in &self.active_pages {
                    page.notify_current_episode(episode_id);
                }
            }
            HomePageInput::ActionFinished(uuid, result) => {
                for (_, page) in &self.active_pages {
                    page.notify_action_finished(uuid, result.clone());
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            HomePageCommand::Podcasts(data) => sender.input(HomePageInput::PodcastsLoaded(data)),
        }
    }
}
