use adw::prelude::*;
use podcasts_data::discovery::FoundPodcast;
use relm4::prelude::*;

use crate::components::podcats_list_item::{PodcastListItem, PodcastListItemOutput};

#[derive(Debug)]
pub struct PodcastResults {
    // A FactoryVecDeque coordinates your children (PodcastListItem) inside the ListBox
    pub podcasts: FactoryVecDeque<PodcastListItem>,
    loading: bool,
}

#[derive(Debug)]
pub enum PodcastResultsInput {
    SearchBegan,
    Results(Vec<FoundPodcast>),
}

#[derive(Debug)]
pub enum PodcastResultsOutput {
    Subscribe(String),
    OpenPodcast(FoundPodcast),
}

#[relm4::component(pub)]
impl SimpleComponent for PodcastResults {
    type Init = ();
    type Input = PodcastResultsInput;
    type Output = PodcastResultsOutput;
    type Widgets = PodcastResultsWidgets;

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Initialize the FactoryVecDeque container bound to the parent gtk::ListBox widget
        let podcasts = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.output_sender(), |output_msg| match output_msg {
                // Intercept the individual row clicks and map them directly to your parent enum
                PodcastListItemOutput::Subscribe(feed_url) => {
                    PodcastResultsOutput::Subscribe(feed_url)
                }
                PodcastListItemOutput::OpenPodcastPage(podcast) => {
                    PodcastResultsOutput::OpenPodcast(podcast)
                }
            });

        let model = Self {
            podcasts,
            loading: false,
        };

        let search_results = model.podcasts.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            PodcastResultsInput::SearchBegan => {
                self.loading = true;
            }
            PodcastResultsInput::Results(podcasts) => {
                self.loading = false;

                let mut guard = self.podcasts.guard();
                guard.clear();

                for podcast in podcasts.iter().take(10) {
                    guard.push_back(podcast.clone());
                }
            }
        }
    }

    view! {
        #[name = "content_stack"]
        gtk::Stack {
            // FIX: Watch the loading property and change the active page string identifier automatically
            #[watch]
            set_visible_child_name: if model.loading { "loading-page" } else { "results-page" },

            // --- PAGE 1: SPINNER LOADING STATE ---
            add_named[Some("loading-page")] = &gtk::CenterBox {
                set_hexpand: true,
                set_vexpand: true,
                set_halign: gtk::Align::Center,
                set_halign: gtk::Align::Center,

                #[wrap(Some)]
                set_center_widget = &gtk::Box{
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                    set_halign: gtk::Align::Center,
                    set_spacing: 16,

                    adw::Spinner {
                        set_size_request: (100, 100)
                    },

                    gtk::Label{
                        set_label: "Searching podcasts ...",
                        set_wrap: true,
                        add_css_class: "title-4"
                    }

                },
            },

            add_named[Some("results-page")]=&adw::Clamp{
                set_maximum_size: 1100,
                set_tightening_threshold: 900,

                gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_vscrollbar_policy: gtk::PolicyType::Automatic,
                    set_hexpand: true,
                    set_vexpand: true,

                    // Pull the automatically constructed list widget layout directly from the factory manager
                    #[local_ref]
                    search_results -> gtk::ListBox {
                        add_css_class: "boxed-list",
                        set_selection_mode: gtk::SelectionMode::None,
                        set_margin_all: 12,
                    }
                }
            }
        }
    }
}
