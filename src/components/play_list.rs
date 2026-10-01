use gtk::prelude::*;
use podcasts_data::{EpisodeId, dbqueries};
use relm4::{Component, ComponentParts, prelude::*};

use crate::components::playlist_episode_list_item::{
    PlayListEpisodeListItem, PlayListEpisodeListItemOutput,
};

#[derive(Debug)]
pub struct PlayListComponent {
    episodes: FactoryVecDeque<PlayListEpisodeListItem>,
}

#[derive(Debug)]
pub enum PlayListComponentInput {
    UpdatePlaylist(Vec<EpisodeId>, Option<usize>),
}

#[derive(Debug)]
pub enum PlayListComponentOutput {
    SetPlayNext(EpisodeId),
    SetPlayNow(EpisodeId),
    RemoveFromPlayList(EpisodeId),
    NotifyError(String),
}

#[relm4::component(pub)]
impl Component for PlayListComponent {
    type Init = ();
    type Input = PlayListComponentInput;
    type Output = PlayListComponentOutput;
    type CommandOutput = ();

    view! {
        adw::Clamp {
            set_maximum_size: 600,
            set_halign: gtk::Align::Center,
            set_hexpand: true,
            set_vexpand: true,
            set_margin_all: 24,

                gtk::Box{
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,
                set_vexpand: true,
                set_margin_all: 24,

                gtk::Label {
                    set_label: "● ● ○",
                    set_halign: gtk::Align::Start,
                    set_margin_bottom: 12,
                    set_margin_horizontal: 24,
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    #[local_ref]
                    episodes_container -> gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::Single,
                        add_css_class: "navigation-sidebar",
                        inline_css: "background: transparent;",
                        #[watch]
                        set_visible: !model.episodes.is_empty(),
                    },
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: relm4::prelude::ComponentSender<Self>,
    ) -> relm4::prelude::ComponentParts<Self> {
        let episodes_parent = gtk::ListBox::builder().build();
        let model = PlayListComponent {
            episodes: FactoryVecDeque::builder().launch(episodes_parent).forward(
                sender.output_sender(),
                |msg| match msg {
                    PlayListEpisodeListItemOutput::SetPlayNext(episode_id) => {
                        PlayListComponentOutput::SetPlayNext(episode_id)
                    }
                    PlayListEpisodeListItemOutput::SetPlayNow(episode_id) => {
                        PlayListComponentOutput::SetPlayNow(episode_id)
                    }
                    PlayListEpisodeListItemOutput::RemoveFromPlayList(episode_id) => {
                        PlayListComponentOutput::RemoveFromPlayList(episode_id)
                    }
                    PlayListEpisodeListItemOutput::NotifyError(error) => {
                        PlayListComponentOutput::NotifyError(error)
                    }
                },
            ),
        };

        let episodes_container = model.episodes.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            PlayListComponentInput::UpdatePlaylist(episode_ids, pos) => {
                let mut guard = self.episodes.guard();
                guard.clear();

                for (position, id) in episode_ids.iter().enumerate() {
                    match dbqueries::get_episode_from_id(id.clone()) {
                        Ok(episode) => {
                            let is_playing = match pos {
                                Some(pos) => pos == position,
                                None => false,
                            };
                            guard.push_back((episode, is_playing));
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }
}
