use gtk::prelude::*;
use relm4::prelude::*;

use crate::{
    components::progress_bar::{ProgressBar, ProgressBarInit, ProgressBarOutput},
    util::progress_color::ProgressColor,
};

pub struct VolumeControlModel {
    volume_bar: Controller<ProgressBar>,
    is_muted: bool,
    current_volume: f64,
}

#[derive(Debug)]
pub enum VolumeControlInput {
    SetVolume(f64),
    IncreaseVolume,
    DectreaseVolume,
    Muted,
    Unmuted,
}

#[derive(Debug)]
pub enum VolumeControlOutput {
    VolumeChanged(f64),
    SetMute,
    Unmute,
}

#[relm4::component(pub)]
impl Component for VolumeControlModel {
    type Init = f64;
    type Input = VolumeControlInput;
    type Output = VolumeControlOutput;
    type CommandOutput = ();

    fn init(
        initial_volume: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let volume_bar = ProgressBar::builder()
            .launch(ProgressBarInit {
                initial_fraction: initial_volume,
                interactive: true,
                color: ProgressColor::Default,
            })
            .forward(sender.output_sender(), |msg| match msg {
                ProgressBarOutput::FractionChanged(fraction) => {
                    VolumeControlOutput::VolumeChanged(fraction)
                }
            });

        let model = VolumeControlModel {
            volume_bar,
            current_volume: initial_volume,
            is_muted: false,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 8,

            gtk::MenuButton {
                set_icon_name: "audio-volume-low-symbolic",

                 #[wrap(Some)]
                set_popover = &gtk::Popover {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_margin_all: 4,

                        gtk::Box {
                        gtk::Image{
                            set_icon_name: Some("audio-volume-low-symbolic"),
                            set_sensitive: model.current_volume <= 0.0,

                            // add_controller = &gtk::GestureClick {
                            //     connect_pressed[sender] => move |_gesture, _n_press, _x, _y| {
                            //         sender.input(VolumeControlInput::DectreaseVolume);
                            //     }
                            // }
                        },
                    },

                        model.volume_bar.widget(){
                            set_size_request: (200, 10),
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                        },

                        gtk::Image{
                            set_icon_name: Some("audio-volume-high-symbolic"),
                            set_sensitive: model.current_volume >= 1.0,

                            // add_controller = &gtk::GestureClick {
                            //     connect_pressed[sender] => move |_gesture, _n_press, _x, _y| {
                            //         sender.input(VolumeControlInput::IncreaseVolume);
                            //     }
                            // }
                        },
                    }
                }
            }


        }
    }
}
