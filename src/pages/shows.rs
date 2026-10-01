use std::sync::Arc;

use adw::prelude::*;
use gst_play::PlayState;
use podcasts_data::{
    EpisodeId, Show, ShowId,
    dbqueries::{self, ShowFilter},
    errors::DataError,
};
use relm4::{Component, ComponentParts, ComponentSender, prelude::*};
use uuid::Uuid;

use crate::{
    components::show_card::{ShowCard, ShowCardOutput},
    pages::show::{ShowPage, ShowPageInput, ShowPageOutput},
    workers::action_worker::worker::{Action, ActionResult},
};

#[derive(Debug)]
pub struct ShowsPage {
    shows: FactoryVecDeque<ShowCard>,
    open_show_pages: Vec<Controller<ShowPage>>,
    task_id: Uuid,
}

#[derive(Debug)]
pub enum ShowsPageInput {
    FetchShows,
    ActionFinished(Uuid, ActionResult),
    ShowsLoaded(Result<Vec<Show>, DataError>),
    GotoShow(ShowId),
    DownloadStarted(EpisodeId),
    DownloadCancled(EpisodeId),
    DownloadProgress(EpisodeId, f64),
    DownloadFinished(EpisodeId),
    ChangePlayBackState(PlayState, EpisodeId),
    PlayBackProgress(EpisodeId, f64, u64),
    ChangeEpisodeTo(EpisodeId),
}

#[derive(Debug)]
pub enum ShowsPageOutput {
    TogglePlay(EpisodeId),
    NotifyError(String),
    RequestDownload(EpisodeId),
    CancleDownload(EpisodeId),
    SetPlayNext(EpisodeId),
    AddToPlaylist(EpisodeId),
    RequestDeleteEpisode(EpisodeId),
    Execute(Uuid, Action),
}

#[derive(Debug)]
pub enum ShowsPageCommand {
    //Shows(data),
}

#[relm4::component(pub)]
impl Component for ShowsPage {
    type Init = ();
    type Input = ShowsPageInput;
    type Output = ShowsPageOutput;
    type CommandOutput = ShowsPageCommand;

    fn init(
        _worker_sender: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let grid = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .column_spacing(20)
            .row_spacing(40)
            .homogeneous(true)
            .valign(gtk::Align::Start)
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

        let model = ShowsPage {
            shows: FactoryVecDeque::builder()
                .launch(grid)
                .forward(sender.input_sender(), |msg| match msg {
                    ShowCardOutput::GotoShow(show) => ShowsPageInput::GotoShow(show),
                }),
            open_show_pages: Vec::new(),
            task_id: uuid::Uuid::new_v4(),
        };

        let show_grid = model.shows.widget();

        let widgets = view_output!();

        let _ = sender.output(ShowsPageOutput::Execute(model.task_id, Action::FetchShows));

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
            ShowsPageInput::FetchShows => {
                let filter = ShowFilter {
                    any_downloaded: None,
                    completed: None,
                    title_or_description: None,
                    reverse_order: true,
                };
                let data = dbqueries::get_podcasts_filter(&[], &filter);
                sender.input(ShowsPageInput::ShowsLoaded(data));
            }

            ShowsPageInput::ShowsLoaded(shows) => match shows {
                Ok(data) => {
                    let mut guard = self.shows.guard();
                    guard.clear();
                    for show in data {
                        guard.push_back(show);
                    }
                }
                Err(error) => {
                    let _ = sender.output(ShowsPageOutput::NotifyError(error.to_string()));
                }
            },

            ShowsPageInput::GotoShow(id) => {
                let show_page =
                    ShowPage::builder()
                        .launch(id.clone())
                        .forward(sender.output_sender(), |msg| match msg {
                            ShowPageOutput::NotifyError(error) => {
                                ShowsPageOutput::NotifyError(error)
                            }
                            ShowPageOutput::TogglePlay(id) => ShowsPageOutput::TogglePlay(id),
                            ShowPageOutput::RequestDownload(episode_id) => {
                                ShowsPageOutput::RequestDownload(episode_id)
                            }
                            ShowPageOutput::CancleDownload(episode_id) => {
                                ShowsPageOutput::CancleDownload(episode_id)
                            }
                            ShowPageOutput::SetPlayNext(episode_id) => {
                                ShowsPageOutput::SetPlayNext(episode_id)
                            }
                            ShowPageOutput::AddToPlaylist(episode_id) => {
                                ShowsPageOutput::AddToPlaylist(episode_id)
                            }
                            ShowPageOutput::RequestDeleteEpisode(episode_id) => {
                                ShowsPageOutput::RequestDeleteEpisode(episode_id)
                            }
                            ShowPageOutput::Execute(uuid, action) => {
                                ShowsPageOutput::Execute(uuid, action)
                            }
                        });

                widgets.nav_view.push(show_page.widget());
                self.open_show_pages.push(show_page);
            }
            ShowsPageInput::DownloadStarted(episode_id) => {
                for page in &self.open_show_pages {
                    page.emit(ShowPageInput::DownloadStarted(episode_id));
                }
            }
            ShowsPageInput::DownloadCancled(episode_id) => {
                for page in &self.open_show_pages {
                    page.emit(ShowPageInput::DownloadCancled(episode_id));
                }
            }
            ShowsPageInput::DownloadProgress(episode_id, fraction) => {
                for page in &self.open_show_pages {
                    page.emit(ShowPageInput::DownloadProgress(episode_id, fraction));
                }
            }
            ShowsPageInput::DownloadFinished(episode_id) => {
                for page in &self.open_show_pages {
                    page.emit(ShowPageInput::DownloadFinished(episode_id));
                }
            }
            ShowsPageInput::ChangePlayBackState(state, episode_id) => {
                for page in &self.open_show_pages {
                    page.emit(ShowPageInput::ChangePlayBackState(state, episode_id));
                }
            }
            ShowsPageInput::PlayBackProgress(episode_id, pos, rem) => {
                for page in &self.open_show_pages {
                    page.emit(ShowPageInput::PlayBackProgress(episode_id, pos, rem));
                }
            }
            ShowsPageInput::ChangeEpisodeTo(episode_id) => {
                for page in &self.open_show_pages {
                    page.emit(ShowPageInput::ChangeEpisodeTo(episode_id));
                }
            }
            ShowsPageInput::ActionFinished(id, result) => {
                if id == self.task_id {
                    if let Ok(shows_arc) = result.downcast::<Result<Vec<Show>, DataError>>() {
                        if let Ok(shows) = Arc::try_unwrap(shows_arc) {
                            sender.input(ShowsPageInput::ShowsLoaded(shows));
                        }
                    }
                } else {
                    for page in &self.open_show_pages {
                        page.emit(ShowPageInput::ActionFinished(id, result.clone()));
                    }
                }
            }
        }

        self.update_view(widgets, sender.clone());
    }

    view! {
        adw::NavigationPage {
            set_title: "Podcasts",

            #[wrap(Some)]
            #[name = "nav_view"]
            set_child = &adw::NavigationView{
                add=&adw::NavigationPage {

                    #[wrap(Some)]
                    set_child = &adw::ToolbarView {
                        #[wrap(Some)]
                        set_content = &gtk::Box{
                            set_orientation: gtk::Orientation::Vertical,

                            gtk::ScrolledWindow {
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
                                            set_label: "Shows",
                                            set_halign:gtk::Align::Start,

                                            add_css_class: "title-1"
                                        },

                                        #[local_ref]
                                        show_grid -> gtk::FlowBox {
                                            set_margin_all: 20,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

        }
    }
    }
}
