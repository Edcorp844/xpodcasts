use crate::components::podcast_section_tile::{PodcastSectionTile, PodcastSectionTileOutput};
use gettextrs::gettext;
use gtk::prelude::*;
use podcasts_data::{discovery::FoundPodcast, EpisodeId};
use relm4::{factory::FactoryVecDeque, prelude::*, Component, ComponentParts, ComponentSender};
use std::collections::HashMap;

#[derive(Debug)]
pub struct PodcastSection {
    podcasts: FactoryVecDeque<PodcastSectionTile>,
    index_by_id: HashMap<EpisodeId, relm4::factory::DynamicIndex>,
}

#[derive(Debug, Clone)]
pub enum PodcastSectionInput {
    SetPodcasts(Vec<FoundPodcast>),
}

#[derive(Debug, Clone)]
pub enum PodcastSectionOutput {
    OpenPodcastPage(FoundPodcast),
    Subscribe(String),
}

#[relm4::component(pub)]
impl Component for PodcastSection {
    type Init = ();
    type Input = PodcastSectionInput;
    type Output = PodcastSectionOutput;
    type CommandOutput = ();

    view! {
        gtk::Box {
            #[watch]
            set_visible: !model.podcasts.is_empty(),
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 20,

            gtk::Box {

                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Start,

                set_spacing: 4,
                inline_css: "margin-top: 16px;",

                gtk::Label {
                    set_label: &gettext("Trending Shows"),
                    add_css_class: "heading",
                    set_valign: gtk::Align::Center,
                },

                gtk::Image {
                    set_icon_name: Some("go-next-symbolic"),
                    inline_css: "opacity: 0.7; margin-left: 4px;",
                    set_valign: gtk::Align::Center,
                },
            },

            #[local_ref]
            podcast_grid -> gtk::FlowBox {

            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
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

        let model = Self {
            podcasts: FactoryVecDeque::builder().launch(grid).forward(
                sender.output_sender(),
                |msg| match msg {
                    PodcastSectionTileOutput::Subscribe(feed) => {
                        PodcastSectionOutput::Subscribe(feed)
                    }
                    PodcastSectionTileOutput::OpenPodcastPage(podcast) => {
                        PodcastSectionOutput::OpenPodcastPage(podcast)
                    }
                },
            ),
            index_by_id: HashMap::new(),
        };

        let podcast_grid = model.podcasts.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            PodcastSectionInput::SetPodcasts(podcasts) => {
                let mut guard = self.podcasts.guard();
                guard.clear();
                for podcast in podcasts {
                    guard.push_back(podcast);
                }
            }
        }
    }
}
