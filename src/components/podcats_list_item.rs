use adw::prelude::*;
use podcasts_data::discovery::FoundPodcast;
use relm4::{
    FactorySender, RelmWidgetExt,
    factory::{DynamicIndex, FactoryComponent},
};

use crate::util::cover_image::{ImageSize, fetch_cached_image};

#[derive(Debug)]
pub struct PodcastListItem {
    podcast: FoundPodcast,
    texture: Option<adw::gdk::Texture>,
}

#[derive(Debug)]
pub enum PodcastListItemInput {
    ImageDownloaded(Option<adw::gdk::Texture>),
    Subscribe,
    OpenPodcastPage,
}

#[derive(Debug)]
pub enum PodcastListItemOutput {
    Subscribe(String),
    OpenPodcastPage(FoundPodcast),
}

#[derive(Debug)]
pub enum PodcastListItemCmdInput {
    DownloadImage(Option<adw::gdk::Texture>),
}

#[relm4::factory(pub)]
impl FactoryComponent for PodcastListItem {
    type Init = FoundPodcast;
    type Input = PodcastListItemInput;
    type Output = PodcastListItemOutput;
    type CommandOutput = PodcastListItemCmdInput;
    type ParentWidget = gtk::ListBox;

    fn init_model(podcast: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let image_url = podcast.clone().art;
        sender.oneshot_command(async move {
            let downloaded_texture = fetch_cached_image(&image_url, ImageSize::default()).await;

            PodcastListItemCmdInput::DownloadImage(downloaded_texture)
        });

        Self {
            podcast,
            texture: None,
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            PodcastListItemInput::ImageDownloaded(fetched_texture) => {
                self.texture = fetched_texture;
            }
            PodcastListItemInput::Subscribe => {
                let _ = sender.output(PodcastListItemOutput::Subscribe(self.podcast.feed.clone()));
            }
            PodcastListItemInput::OpenPodcastPage => {
                let _ = sender.output(PodcastListItemOutput::OpenPodcastPage(self.podcast.clone()));
            }
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: FactorySender<Self>) {
        match message {
            PodcastListItemCmdInput::DownloadImage(opt_texture) => {
                sender.input(PodcastListItemInput::ImageDownloaded(opt_texture));
            }
        }
    }

    view! {
        gtk::Box {
            set_halign: gtk::Align::Fill,
            set_orientation: gtk::Orientation::Horizontal,
            set_margin_all: 16,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Start, 
                set_spacing: 16,

                gtk::Overlay {
                    set_height_request: 150,
                    set_width_request: 150,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Start,

                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_halign: gtk::Align::Fill,
                        set_valign: gtk::Align::Fill,
                        inline_css: "
                            background-color: mix(@window_bg_color, @card_fg_color, 0.1);
                            border-radius: 16px;
                            box-shadow: 0 12px 28px rgba(0, 0, 0, 0.32);
                            border: 1px solid alpha(@borders, 0.8);
                        ",

                        gtk::Label {
                            #[watch]
                            set_label: &self.podcast.title.trim().chars().take(2).collect::<String>().to_uppercase(),
                            add_css_class: "title-large",
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            inline_css: "color: @dim_label_opacity; opacity: 0.25; font-weight: 800;",
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
                        inline_css: "border-radius: 16px;",
                    }
                },


                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Start,

                    gtk::Label {
                        set_label: &self.podcast.title,
                        add_css_class: "title-4",
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        set_wrap: true
                    },

                    gtk::Label {
                        set_label: &self.podcast.author,
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        set_wrap: true
                    },

                    gtk::Label {
                        #[watch]
                        set_use_markup: true,
                        #[watch]
                        set_markup: &{
                            let desc = &self.podcast.description;
                            let raw_markup = html2text::config::plain()
                                .string_from_read(desc.as_bytes(), desc.len())
                                .unwrap_or_else(|_| desc.to_string());
                            raw_markup.replace('\n', " ").replace('\r', " ")
                        },
                        set_halign: gtk::Align::Start,
                        set_xalign: 0.0,
                        set_wrap: true,
                        set_lines: 2,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_css_classes: &vec!["dimmed", "body"]
                    },

                    gtk::Label {
                        set_label: &match &self.podcast.last_publication {
                            Some(date) => {
                                format!("Updated: {}", date.format("%-d %B %Y"))
                            }
                            None => "".to_string(),
                        },
                        add_css_class: "caption",
                        set_xalign: 0.0,
                        set_halign: gtk::Align::Start,
                        set_wrap: true
                    },

                    gtk::Separator {
                        set_vexpand: true,
                        add_css_class: "spacer",
                    }
                },
            },

            gtk::Box {
                set_hexpand: true, 
            },

            gtk::Box {
                set_halign: gtk::Align::End,
                set_valign: gtk::Align::Center,

                gtk::Button {
                    set_icon_name: "list-add-symbolic",
                    set_tooltip_text: Some("Follow"),
                    set_css_classes: &vec!["circular", "suggested-action"],
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,

                    connect_clicked[sender] => move |_| {
                        sender.input(PodcastListItemInput::Subscribe);
                    }
                },
            },

            add_controller = gtk::GestureClick {
                connect_released[sender] => move |_, _, _, _| {
                    sender.input(PodcastListItemInput::OpenPodcastPage);
                }
            }
        }
    }

}
