use adw::prelude::*;
use podcasts_data::discovery::FoundPodcast;
use relm4::prelude::*;

use crate::components::podcats_list_item::{PodcastListItem, PodcastListItemOutput};

#[derive(Debug)]
pub struct PodcastResults {
    pub podcasts: FactoryVecDeque<PodcastListItem>,
}

#[derive(Debug)]
pub enum PodcastResultsInput {
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

        let model = Self { podcasts };

        let search_results = model.podcasts.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            PodcastResultsInput::Results(podcasts) => {
                let mut guard = self.podcasts.guard();
                guard.clear();

                for podcast in podcasts.iter().take(10) {
                    guard.push_back(podcast.clone());
                }
            }
        }
    }

    view! {
    adw::Clamp{
            set_maximum_size: 1100,
            set_tightening_threshold: 900,

            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_vscrollbar_policy: gtk::PolicyType::Automatic,
                set_hexpand: true,
                set_vexpand: true,

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
