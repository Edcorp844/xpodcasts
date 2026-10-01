use adw::prelude::*;
use gst_play::PlayState;
use podcasts_data::Episode;
use podcasts_data::EpisodeId;
use podcasts_data::dbqueries;
use relm4::prelude::*;

use crate::components::progress_bar::ProgressBar;
use crate::components::progress_bar::ProgressBarInit;
use crate::components::progress_bar::ProgressBarInput;
use crate::components::progress_bar::ProgressBarOutput;
use crate::components::volume_scale::VolumeControlModel;
use crate::components::volume_scale::VolumeControlOutput;
use crate::util::cover_image::ImageSize;
use crate::util::cover_image::fetch_cached_image;
use crate::util::progress_color::ProgressColor;

#[derive(Debug)]
pub enum PlayerPageView {
    Lyrics,
    PlayList,
    None,
}

pub struct MiniPlayerModel {
    pub volume_slider: Controller<VolumeControlModel>,
    texture: Option<adw::gdk::Texture>,
    current_state: PlayState,
    current_episode: Option<Episode>,
    play_progress_bar: Controller<ProgressBar>,
}

#[derive(Debug)]
pub enum MiniplayerModelInput {
    ImageDownloaded(Option<adw::gdk::Texture>),
    ChangePlayBackState(PlayState),
    SetCurrentEpisode(EpisodeId),
    UpdateProgress(f64),
    VolumeValue(f64),
}

#[derive(Debug)]
pub enum MiniplayerModelOutput {
    TogglePlay,
    NotifyError(String),
    SeekAudioPosition(f64),
    SetVolume(f64),
    ShowPlayerPage(PlayerPageView),
    RequestMute,
    RequestUnmute,
    RequestVolumeValue,
    Seekforward,
    SeekBakward,
}

#[derive(Debug)]
pub enum MiniPlayerModelCmdInput {
    DownloadImage(Option<adw::gdk::Texture>),
}

#[relm4::component(pub)]
impl Component for MiniPlayerModel {
    type Init = ();
    type Input = MiniplayerModelInput;
    type Output = MiniplayerModelOutput;
    type CommandOutput = MiniPlayerModelCmdInput;

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let volume_slider =
            VolumeControlModel::builder()
                .launch(0.1)
                .forward(sender.output_sender(), |output| match output {
                    VolumeControlOutput::VolumeChanged(new_vol) => {
                        MiniplayerModelOutput::SetVolume(new_vol)
                    }
                    VolumeControlOutput::SetMute => MiniplayerModelOutput::RequestMute,
                    VolumeControlOutput::Unmute => MiniplayerModelOutput::RequestUnmute,
                });

        let play_progress_bar = ProgressBar::builder()
            .launch(ProgressBarInit {
                initial_fraction: 0.0,
                interactive: true,
                color: ProgressColor::Default,
            })
            .forward(sender.output_sender(), |msg| match msg {
                ProgressBarOutput::FractionChanged(fraction) => {
                    MiniplayerModelOutput::SeekAudioPosition(fraction)
                }
            });

        let model = Self {
            volume_slider,
            texture: None,
            current_state: PlayState::Stopped,
            current_episode: None,
            play_progress_bar,
        };

        let _ = sender.output(MiniplayerModelOutput::RequestVolumeValue);

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            MiniplayerModelInput::ImageDownloaded(fetched_texture) => {
                self.texture = fetched_texture;
            }
            MiniplayerModelInput::ChangePlayBackState(state) => {
                self.current_state = state;
            }
            MiniplayerModelInput::SetCurrentEpisode(id) => {
                match dbqueries::get_episode_from_id(id) {
                    Ok(episode) => {
                        let image_uri_opt = episode.image_uri().map(|s| s.to_string());
                        self.current_episode = Some(episode);

                        if let Some(image_uri) = image_uri_opt {
                            sender.oneshot_command(async move {
                                let downloaded_texture =
                                    fetch_cached_image(&image_uri, ImageSize::from_dimesion(50))
                                        .await;

                                MiniPlayerModelCmdInput::DownloadImage(downloaded_texture)
                            });
                        } else {
                            self.texture = None;
                        }
                    }
                    Err(error) => {
                        // Forward the database infrastructure errors up to the application logger
                        let _ = sender.output(MiniplayerModelOutput::NotifyError(format!(
                            "Failed to resolve episode metadata: {:?}",
                            error
                        )));
                    }
                }
            }
            MiniplayerModelInput::UpdateProgress(fraction) => {
                self.play_progress_bar
                    .emit(ProgressBarInput::SetFraction(fraction));
            }
            MiniplayerModelInput::VolumeValue(val) => {
                self.play_progress_bar
                    .emit(ProgressBarInput::SetFraction(val));
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            MiniPlayerModelCmdInput::DownloadImage(opt_texture) => {
                sender.input(MiniplayerModelInput::ImageDownloaded(opt_texture));
            }
        }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            inline_css: "background: var(--secondary-sidebar-bg-color); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05), 0 4px 12px rgba(0, 0, 0, 0.04); border-bottom: 1px solid rgba(0, 0, 0, 0.03)",

            adw::HeaderBar {
                set_valign: gtk::Align::Start,

                // --- Media Playback Controls ---
                pack_start = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,

                    gtk::Separator { set_hexpand: true, add_css_class: "spacer" },

                    gtk::Button {
                        set_icon_name: "media-seek-backward-symbolic",
                        set_tooltip_text: Some("Rewind 15 seconds"),
                        set_valign: gtk::Align::Center,
                        set_css_classes: &["circular", "flat"],

                        #[watch]
                        set_sensitive: model.current_state != gst_play::PlayState::Stopped,

                        connect_clicked[sender]=>move |_| {
                            let _  = sender.output(MiniplayerModelOutput::SeekBakward);
                        }
                    },

                     match model.current_state {
                        gst_play::PlayState::Buffering => adw::Spinner{
                            set_size_request: (24, 24),
                            set_margin_start: 6,
                            set_margin_end: 6,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                        }
                        _=> gtk::Button {
                            set_tooltip_text: Some("Play"),
                            add_css_class: "circular",
                            set_size_request: (50, 50),
                            set_margin_start: 6,
                            set_margin_end: 6,

                            #[watch]
                            set_sensitive: model.current_state != gst_play::PlayState::Stopped,

                            #[wrap(Some)]
                            set_child = &gtk::Image {
                                #[watch]
                                set_icon_name: if model.current_state == gst_play::PlayState::Playing {
                                    Some("media-playback-pause-symbolic")
                                } else {
                                    Some("media-playback-start-symbolic")
                                },
                                set_icon_size: gtk::IconSize::Large,
                            },

                            connect_clicked[sender] => move |_| {
                                let _ = sender.output(MiniplayerModelOutput::TogglePlay);
                            }
                        },
                    },

                    gtk::Button {
                        set_icon_name: "media-seek-forward-symbolic",
                        set_tooltip_text: Some("Fast forward 30 seconds"),
                        set_valign: gtk::Align::Center,
                        set_css_classes: &["circular", "flat"],

                        #[watch]
                        set_sensitive: model.current_state != gst_play::PlayState::Stopped,

                        connect_clicked[sender]=>move |_| {
                            let _  = sender.output(MiniplayerModelOutput::Seekforward);
                        }
                    },



                    gtk::Separator { set_hexpand: true, add_css_class: "spacer" },
                },


                #[wrap(Some)]
                set_title_widget =&gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,
                    set_halign: gtk::Align::Fill,
                    set_hexpand: true,
                    set_vexpand: false,
                    add_css_class: "frame",

                   // gtk::Separator { set_hexpand: true, add_css_class: "spacer" },

                    gtk::Box {
                        set_width_request: 50,
                        set_height_request: 50,
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Center,
                        set_hexpand: false,
                        set_vexpand: false,
                        add_css_class: "suggested-action",
                        inline_css: "background: rgba(255, 255, 255, 0.7)",

                        gtk::Picture {
                            #[watch]
                            set_paintable: model.texture.as_ref().map(|t| t.upcast_ref::<adw::gdk::Paintable>()),
                            #[watch]
                            set_visible: model.texture.is_some(),
                            set_hexpand: false,
                            set_vexpand: false,
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Fill,
                            set_content_fit: gtk::ContentFit::Cover,
                            set_can_shrink: true,



                        },

                        add_controller = gtk::GestureClick {
                            connect_released[sender] => move |_, _, _, _| {
                                let _ = sender.output(MiniplayerModelOutput::ShowPlayerPage(PlayerPageView::None));
                            }
                        }
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_halign: gtk::Align::Start,

                        gtk::Label {
                            #[watch]
                            set_label:match &model.current_episode {
                                Some(episode) => episode.title(),
                                None => "",
                            },
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                            set_lines: 1,
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Center,
                            set_css_classes: &vec!["caption"],
                        },

                        gtk::Label {
                            set_label: "July 5",
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Center,
                            set_css_classes: &vec!["caption", "dimmed"],
                        },

                        model.play_progress_bar.widget() {
                            set_height_request: 2,
                            set_halign: gtk::Align::Fill,
                            set_valign: gtk::Align::Center,
                        },
                    },

                },

                // --- Volume Slider on the Right End ---
                pack_end = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_valign: gtk::Align::Center,

                    gtk::Separator { set_hexpand: true, add_css_class: "spacer" },

                    gtk::Button{
                        set_icon_name: "annotations-text-symbolic",
                        set_tooltip_text: Some("Lyrics"),


                        connect_clicked[sender]=>move |_| {
                            let _  = sender.output(MiniplayerModelOutput::ShowPlayerPage(PlayerPageView::Lyrics));
                        }
                    },
                    gtk::Separator { set_hexpand: true, add_css_class: "spacer" },

                    gtk::Button{
                        set_icon_name: "view-list-symbolic",
                        set_tooltip_text: Some("Play List"),


                        connect_clicked[sender]=>move |_| {
                            let _  = sender.output(MiniplayerModelOutput::ShowPlayerPage(PlayerPageView::PlayList));
                        }
                    },

                    gtk::Separator { set_hexpand: true, add_css_class: "spacer" },

                    model.volume_slider.widget(),

                    gtk::Separator { set_hexpand: true, add_css_class: "spacer" },
                }
            }
        },
    }
}
