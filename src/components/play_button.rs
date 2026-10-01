use adw::prelude::*;
use relm4::{Component, prelude::*};

use crate::{components::progress_bar::{ProgressBar, ProgressBarInit, ProgressBarInput}, util::progress_color::ProgressColor};

#[derive(Debug)]
pub enum EpisodePlayingState {
    Playing,
    Paused,
    Stopped,
    Finished,
}

#[derive(Debug)]
pub struct PlayButton {
    play_progress_bar: Controller<ProgressBar>,
    label: String,
    playing_state: EpisodePlayingState,
}

#[derive(Debug)]
pub struct PlayButtonInitData {
    pub label: String,
    pub state: EpisodePlayingState,
    pub progress: f64,
}

#[derive(Debug)]
pub enum PlayButtonInput {
    SetLabel(String),
    UpdateProgress(f64),
    UpdatePlayingState(EpisodePlayingState),
}

#[derive(Debug)]
pub enum PlayButtonOutput {
    Clicked,
}

#[relm4::component(pub)]
impl Component for PlayButton {
    type Init = PlayButtonInitData;
    type Input = PlayButtonInput;
    type Output = PlayButtonOutput;
    type CommandOutput = ();

    view! {
         gtk::Button{
            set_css_classes: &vec!["pill"],
            set_halign: gtk::Align::Start,
            set_valign: gtk::Align::Start,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 6,

               gtk::Image {
                    #[watch]
                    set_icon_name: match &model.playing_state {
                        EpisodePlayingState::Playing => Some("media-playback-pause-symbolic"),
                        EpisodePlayingState::Paused | EpisodePlayingState::Stopped | EpisodePlayingState::Finished => {
                            Some("media-playback-start-symbolic")
                        }
                    }
                },

                gtk::Box{
                    #[watch]
                    set_visible: match &model.playing_state {
                        EpisodePlayingState::Playing | EpisodePlayingState::Paused => true,
                        EpisodePlayingState::Stopped | EpisodePlayingState::Finished => false,
                    },
                    model.play_progress_bar.widget() {
                        set_size_request: (50, 5),
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,

                    },
                },

                gtk::Label {
                    #[watch]
                    set_label: &model.label,
                },
            },

            connect_clicked[sender] => move |_| {
               let _ =  sender.output(PlayButtonOutput::Clicked);
            }
         }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let state = init;
        let play_progress_bar = ProgressBar::builder()
            .launch(ProgressBarInit {
                initial_fraction: state.progress,
                interactive: false,
                color: ProgressColor::Default,
            })
            .detach();

        let model = PlayButton {
            play_progress_bar,
            label: state.label,
            playing_state: state.state,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            PlayButtonInput::SetLabel(label) => self.label = label,
            PlayButtonInput::UpdateProgress(fraction) => self
                .play_progress_bar
                .emit(ProgressBarInput::SetFraction(fraction)),
            PlayButtonInput::UpdatePlayingState(state) => self.playing_state = state,
        }
    }
}
