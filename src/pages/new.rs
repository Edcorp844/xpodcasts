use adw::prelude::*;
use gst_play::PlayState;
use podcasts_data::{
    EpisodeId, Show,
    dbqueries::{self, ShowFilter},
    errors::DataError,
};
use relm4::{Component, ComponentParts, ComponentSender, prelude::*};
use uuid::Uuid;

use crate::{components::show_card::ShowCard, workers::action_worker::worker::ActionResult};

#[derive(Debug)]
pub struct NewPage {
    shows: FactoryVecDeque<ShowCard>,
    is_loading: bool,
}

#[derive(Debug)]
pub enum NewPageInput {
    ActionFinished(Uuid, ActionResult),
    FetchShows,
    ShowsLoaded(Result<Vec<Show>, DataError>),
    DownloadStarted(EpisodeId),
    DownloadCancled(EpisodeId),
    DownloadProgress(EpisodeId, f64),
    DownloadFinished(EpisodeId),
    ChangePlayBackState(PlayState, EpisodeId),
    PlayBackProgress(EpisodeId, f64, u64),
    ChangeEpisodeTo(EpisodeId),
}

#[derive(Debug)]
pub enum NewPageOutput {}

#[derive(Debug)]
pub enum NewPageCommand {
    //Shows(data),
}

#[relm4::component(pub)]
impl Component for NewPage {
    type Init = ();
    type Input = NewPageInput;
    type Output = NewPageOutput;
    type CommandOutput = NewPageCommand;

    fn init(
        _worker_sender: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
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

        let model = NewPage {
            shows: FactoryVecDeque::builder().launch(grid).detach(),
            is_loading: true,
        };

        let show_grid = model.shows.widget();

        let widgets = view_output!();

        sender.input(NewPageInput::FetchShows);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            NewPageInput::FetchShows => {
                // let ignore = get_ignored_shows()?;
                let filter = ShowFilter {
                    any_downloaded: None,
                    completed: None,
                    title_or_description: None,
                    reverse_order: false,
                };
                let data = dbqueries::get_podcasts_filter(&[], &filter);
                sender.input(NewPageInput::ShowsLoaded(data));
            }

            // Captures background thread work payload safely
            NewPageInput::ShowsLoaded(shows) => {
                match shows {
                    Ok(data) => {
                        let mut guard = self.shows.guard();
                        //guard.clear(); // Flush old elements cleanly
                        for show in data {
                            guard.push_back(show);
                        }
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
                self.is_loading = false;
            }
            NewPageInput::DownloadStarted(episode_id) => {}
            NewPageInput::DownloadCancled(episode_id) => {}
            NewPageInput::DownloadProgress(episode_id, _) => {}
            NewPageInput::DownloadFinished(episode_id) => {}
            NewPageInput::ChangePlayBackState(play_state, episode_id) => {}
            NewPageInput::PlayBackProgress(episode_id, pos, rem) => {}
            NewPageInput::ChangeEpisodeTo(episode_id) => {}
            NewPageInput::ActionFinished(uuid, any) => {}
        }
    }

    // fn update_cmd(
    //     &mut self,
    //     message: Self::CommandOutput,
    //     sender: ComponentSender<Self>,
    //     _: &Self::Root,
    // ) {
    //     match message {
    //         HomePageCommand::Shows(data) => sender.input(HomePageInput::PodcastsLoaded(data)),
    //     }
    // }

    view! {
        adw::NavigationPage {
            set_title: "Podcasts",

            #[wrap(Some)]
            set_child = &adw::NavigationPage {

                    #[wrap(Some)]
                    set_child = &adw::ToolbarView {

                        #[wrap(Some)]
                        set_content = &gtk::Box{
                            set_orientation: gtk::Orientation::Vertical,
                            gtk::Separator {
                                add_css_class: "tahoe-shimmer-line",
                                #[watch]
                                set_visible: model.is_loading,
                                set_halign: gtk::Align::Fill,
                                inline_css: " min-height: 2px;
                                    border: none;  background: linear-gradient(90deg, 
                                        rgba(0, 122, 255, 0) 0%, 
                                        #007AFF 25%, 
                                        #AF52DE 50%, 
                                        #FF2D55 75%, 
                                        rgba(255, 45, 85, 0) 100%
                                    );
                                    background-size: 200% 100%; animation: s from { background-position: 0% 0%; }
                                    to { background-position: 200% 0%; }"

                            },

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
}
