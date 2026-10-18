use gettextrs::gettext;
use gtk::prelude::*;
use podcasts_data::{Episode, EpisodeId};
use relm4::{factory::FactoryVecDeque, prelude::*, Component, ComponentParts, ComponentSender};
use std::collections::HashMap;

use crate::components::episode_section_tile::EpisodeSectionTile;



#[derive(Debug)]
pub struct EpisodeSection {
    // One FactoryVecDeque per column
    col_1: FactoryVecDeque<EpisodeSectionTile>,
    col_2: FactoryVecDeque<EpisodeSectionTile>,
    col_3: FactoryVecDeque<EpisodeSectionTile>,
    col_4: FactoryVecDeque<EpisodeSectionTile>,
    index_by_id: HashMap<EpisodeId, relm4::factory::DynamicIndex>,
}

#[derive(Debug, Clone)]
pub enum EpisodeSectionInput {
    SetEpisodes(Vec<Episode>),
}

#[derive(Debug, Clone)]
pub enum EpisodeSectionOutput {}

#[relm4::component(pub)]
impl Component for EpisodeSection {
    type Init = ();
    type Input = EpisodeSectionInput;
    type Output = EpisodeSectionOutput;
    type CommandOutput = ();

    view! {
        gtk::Box {
            #[watch]
            set_visible: !model.col_1.is_empty() || !model.col_2.is_empty() || !model.col_3.is_empty(),
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 20,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Start,
                set_spacing: 4,
                inline_css: "margin-top: 16px;",

                gtk::Label {
                    set_label: &gettext("You May have Missed"),
                    add_css_class: "title-3",
                    set_valign: gtk::Align::Center,
                },

                gtk::Image {
                    set_icon_name: Some("go-next-symbolic"),
                    inline_css: "opacity: 0.7; margin-left: 4px;",
                    set_valign: gtk::Align::Center,
                },
            },

            gtk::ScrolledWindow {
                set_hscrollbar_policy: gtk::PolicyType::External,
                set_vscrollbar_policy: gtk::PolicyType::Never,
                set_propagate_natural_height: true,
                set_margin_bottom: 20,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 16,


                    #[local_ref]
                    episodes_container1 -> gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,
                    },

                    #[local_ref]
                    episodes_container2 -> gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                         set_spacing: 8,
                    },

                    #[local_ref]
                    episodes_container3 -> gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                         set_spacing: 8,
                    },

                    #[local_ref]
                    episodes_container4 -> gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                         set_spacing: 8,
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
        // Create 3 vertical parent boxes
        let parent_1 = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let parent_2 = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let parent_3 = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let parent_4 = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let model = Self {
            col_1: FactoryVecDeque::builder().launch(parent_1).detach(),
            col_2: FactoryVecDeque::builder().launch(parent_2).detach(),
            col_3: FactoryVecDeque::builder().launch(parent_3).detach(),
            col_4: FactoryVecDeque::builder().launch(parent_4).detach(),
            index_by_id: HashMap::new(),
        };

        let episodes_container1 = model.col_1.widget();
        let episodes_container2 = model.col_2.widget();
        let episodes_container3 = model.col_3.widget();
        let episodes_container4 = model.col_4.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            EpisodeSectionInput::SetEpisodes(episodes) => {
                let mut guard_1 = self.col_1.guard();
                let mut guard_2 = self.col_2.guard();
                let mut guard_3 = self.col_3.guard();
                let mut guard_4 = self.col_4.guard();

                guard_1.clear();
                guard_2.clear();
                guard_3.clear();
                guard_4.clear();
                self.index_by_id.clear();

                // Chunk the episodes into groups of 3 (3 rows per column)
                for (col_index, chunk) in episodes.chunks(3).enumerate() {
                    match col_index % 4 {
                        // Column 1
                        0 => {
                            for episode in chunk {
                                let idx = guard_1.push_back(episode.clone());
                                self.index_by_id.insert(episode.id(), idx);
                            }
                        }
                        // Column 2
                        1 => {
                            for episode in chunk {
                                let idx = guard_2.push_back(episode.clone());
                                self.index_by_id.insert(episode.id(), idx);
                            }
                        }
                        // Column 3
                        2 => {
                            for episode in chunk {
                                let idx = guard_3.push_back(episode.clone());
                                self.index_by_id.insert(episode.id(), idx);
                            }
                        }
                        // Column 4
                        3 => {
                            for episode in chunk {
                                let idx = guard_4.push_back(episode.clone());
                                self.index_by_id.insert(episode.id(), idx);
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}
