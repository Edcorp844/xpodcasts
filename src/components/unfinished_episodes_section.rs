use std::collections::HashMap;

use gettextrs::gettext;
use gtk::prelude::*;
use podcasts_data::{Episode, EpisodeId};
use relm4::{factory::FactoryVecDeque, prelude::*, Component, ComponentParts, ComponentSender};

use crate::components::unfinished_episode_card::UnfinishedEpisodeCard;

#[derive(Debug)]
pub struct UnfinishedEpisodesSection {
    episodes: FactoryVecDeque<UnfinishedEpisodeCard>,
    index_by_id: HashMap<EpisodeId, relm4::factory::DynamicIndex>,
}

#[derive(Debug, Clone)]
pub enum UnfinishedEpisodesSectionInput {
    SetEpisodes(Vec<Episode>),
}

#[derive(Debug, Clone)]
pub enum UnfinishedEpisodesSectionOutput {}

#[relm4::component(pub)]
impl Component for UnfinishedEpisodesSection {
    type Init = ();
    type Input = UnfinishedEpisodesSectionInput;
    type Output = UnfinishedEpisodesSectionOutput;
    type CommandOutput = ();

    view! {
        gtk::Box{
            #[watch]
            set_visible: !model.episodes.is_empty(),
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 20,
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Start,
                set_spacing: 4,
                inline_css: "margin-top: 16px;",

                gtk::Label {
                    set_label: &gettext("Continue listening"),
                    add_css_class: "heading",
                    set_valign: gtk::Align::Center,
                },

                gtk::Image {
                    set_icon_name: Some("go-next-symbolic"),
                    inline_css: "opacity: 0.7; margin-left: 4px;",
                    set_valign: gtk::Align::Center,
                },

                add_controller = gtk::GestureClick {
                    connect_released[sender] => move |_, _, _, _| {

                    }
                }
            },

            gtk::ScrolledWindow{
                set_hscrollbar_policy: gtk::PolicyType::External,
                set_vscrollbar_policy: gtk::PolicyType::Never,
                set_propagate_natural_height: true,
                
                #[local_ref]
                episodes_container -> gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_margin_bottom: 20,
                    #[watch]
                    set_visible: !model.episodes.is_empty(),
                    set_spacing: 20,
                },
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let episodes_parent = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        let model = Self {
            episodes: FactoryVecDeque::builder().launch(episodes_parent).detach(),
            index_by_id: HashMap::new(),
        };
        let episodes_container = model.episodes.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            UnfinishedEpisodesSectionInput::SetEpisodes(episodes) => {
                let mut guard = self.episodes.guard();
                guard.clear();

                for episode in episodes.iter() {
                    let index = guard.push_back(episode.clone());
                    self.index_by_id.insert(episode.id(), index);
                }
            }
        }
    }
}
