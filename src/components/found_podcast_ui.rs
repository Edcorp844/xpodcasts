use adw::prelude::*;
use podcasts_data::discovery::FoundPodcast;
use relm4::prelude::*;

use crate::util::cover_image::{ImageSize, fetch_cached_image};

#[derive(Debug)]
pub struct FoundPodcastsCard {
    podcast: FoundPodcast,
    texture: Option<adw::gdk::Texture>,
}

#[derive(Debug)]
pub enum FoundCardInput {
    ImageDownloaded(Option<adw::gdk::Texture>),
    OpenPodcastPage,
    Subscribe,
}

#[derive(Debug)]
pub enum FoundCardOutput {
    OpenPodcastPage(FoundPodcast),
    Subscribe(String),
}

#[derive(Debug)]
pub enum FoundPodcastCardCmdInput {
    // Input channel target variant triggered when an image download completes
    DownloadImage(Option<adw::gdk::Texture>),
    SubscribeFinished,
}

#[relm4::factory(pub)]
impl FactoryComponent for FoundPodcastsCard {
    type Init = FoundPodcast;
    type Input = FoundCardInput;
    type Output = FoundCardOutput;
    type CommandOutput = FoundPodcastCardCmdInput;
    type ParentWidget = gtk::FlowBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 6,
            set_hexpand: true,
            set_halign: gtk::Align::Fill,
            set_width_request: 100,

            gtk::Overlay {
                set_hexpand: true,
                set_vexpand: true,
                set_height_request: 200,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                    set_css_classes: &vec!["frame"],

                    inline_css: "background-color: mix(var(--window-bg-color), var(--card-fg-color), 0.1); border-radius: 12px; box-shadow: 0 12px 28px rgba(0, 0, 0, 0.32)",

                    gtk::Label {
                        set_label: &self.podcast.title.chars().take(2).collect::<String>().to_uppercase(),
                        set_css_classes: &vec!["title-1", "dimed"],
                        set_hexpand: true,
                        set_vexpand: true,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        inline_css: "opacity: 0.4;",
                    }
                },

                add_overlay = &gtk::Picture {
                    #[watch]
                    set_paintable: self.texture.as_ref().map(|t| t.upcast_ref::<adw::gdk::Paintable>()),
                    #[watch]
                    set_visible: self.texture.is_some(),

                    set_hexpand: true,
                    set_vexpand: true,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                    set_content_fit: gtk::ContentFit::Cover,
                    set_can_shrink: true,

                    inline_css: "border-radius: 12px",
                }
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Fill,
                set_spacing: 8,
                set_margin_top: 2,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 1,
                    set_halign: gtk::Align::Start, // Anchors the text content to the left

                    gtk::Label {
                        set_label: &self.podcast.title,
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_lines: 1,

                        inline_css: "font-weight: 600; font-size: 0.92rem",
                    },

                    gtk::Label {
                        set_label: &self.podcast.author,
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_lines: 1,
                        add_css_class: "dim-label",

                        inline_css: "font-size: 0.85rem; opacity: 0.7",
                    }
                },

                gtk::Box {
                    set_hexpand: true,
                },

                gtk::Button {
                    set_icon_name: "list-add-symbolic",
                    set_tooltip_text: Some("Follow"),
                    set_css_classes: &vec!["circular"],
                    set_valign: gtk::Align::Center,
                    set_halign: gtk::Align::End,

                    connect_clicked[sender] => move |_| {
                        sender.input(FoundCardInput::Subscribe);
                    }
                },
            },

            add_controller = gtk::GestureClick {
                connect_released[sender] => move |_, _, _, _| {
                    sender.input(FoundCardInput::OpenPodcastPage);
                }
            }
        }
    }

    fn init_model(podcast: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let url_string = podcast.art.clone();

        sender.oneshot_command(async move {
            let downloaded_texture = fetch_cached_image(&url_string, ImageSize::default()).await;
            FoundPodcastCardCmdInput::DownloadImage(downloaded_texture)
        });

        Self {
            podcast,
            texture: None,
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            FoundCardInput::ImageDownloaded(fetched_texture) => {
                self.texture = fetched_texture;
            }
            FoundCardInput::Subscribe => {
                let _ = sender.output(FoundCardOutput::Subscribe(self.podcast.feed.clone()));
            }
            FoundCardInput::OpenPodcastPage => {
                let _ = sender.output(FoundCardOutput::OpenPodcastPage(self.podcast.clone()));
            }
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: FactorySender<Self>) {
        match message {
            FoundPodcastCardCmdInput::DownloadImage(opt_texture) => {
                sender.input(FoundCardInput::ImageDownloaded(opt_texture));
            }
            FoundPodcastCardCmdInput::SubscribeFinished => {
                println!("Subscription handling processing completed.");
            }
        }
    }
}

impl FoundPodcastsCard {}
